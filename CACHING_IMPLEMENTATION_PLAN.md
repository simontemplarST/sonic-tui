# Background Caching Implementation Plan

## Current Status

As of the current implementation, sonic-tui uses **on-demand loading** for cover art and lyrics:
- Cover art and lyrics load only when manually selected
- Automatic track transitions show stale/no artwork
- No UI freezes during playback transitions ✅
- Responsive user experience for manual interactions ✅

## The Problem

When implementing naive prefetching in `tick()`, we encountered a fundamental architectural limitation:

```rust
pub async fn tick(&mut self) -> Result<()> {
    // This blocks the entire UI for 3+ seconds!
    self.prefetch_next_song().await?;
}
```

**Issue**: The event loop is single-threaded and awaiting network requests blocks UI rendering.

## The Solution: True Background Tasks

### Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    Main Thread                           │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Event Loop (crossterm + ratatui)                 │  │
│  │  - Polls input every 100ms                        │  │
│  │  - Renders UI (non-blocking)                      │  │
│  │  - Checks cache for results                       │  │
│  └───────────────────────────────────────────────────┘  │
│                        ▲                                 │
│                        │ Results via mpsc channel        │
│                        │                                 │
└────────────────────────┼─────────────────────────────────┘
                         │
┌────────────────────────┼─────────────────────────────────┐
│              Background Task (tokio::spawn)              │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Prefetch Worker                                  │  │
│  │  - Listens for prefetch requests                  │  │
│  │  - Fetches cover art + lyrics in parallel        │  │
│  │  - Pre-processes images (decode + protocol)      │  │
│  │  - Sends results back via channel                │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### Implementation Steps

#### 1. Add Dependencies

```toml
# Cargo.toml
[dependencies]
tokio = { version = "1", features = ["full", "sync"] }  # Add "sync" for channels
```

#### 2. Create Cache Types

```rust
// src/cache/mod.rs (expand existing module)

use tokio::sync::mpsc;
use crate::api::Song;
use ratatui_image::protocol::StatefulProtocol;

/// Prefetch request sent from main thread to background worker
#[derive(Debug, Clone)]
pub struct PrefetchRequest {
    pub song: Song,
}

/// Prefetch result sent from background worker to main thread
#[derive(Debug)]
pub struct PrefetchResult {
    pub song_id: String,
    pub cover_art_data: Option<Vec<u8>>,
    pub cover_art_protocol: Option<Box<dyn StatefulProtocol>>,
    pub lyrics_lines: Vec<String>,
    pub lyrics_timestamps: Vec<f64>,
}

/// Cache manager that coordinates background prefetching
pub struct CacheManager {
    request_tx: mpsc::UnboundedSender<PrefetchRequest>,
    result_rx: mpsc::UnboundedReceiver<PrefetchResult>,
}

impl CacheManager {
    pub fn new(client: SubsonicClient) -> Self {
        let (request_tx, mut request_rx) = mpsc::unbounded_channel();
        let (result_tx, result_rx) = mpsc::unbounded_channel();

        // Spawn background worker
        tokio::spawn(async move {
            while let Some(request) = request_rx.recv().await {
                // Fetch in background
                if let Some(result) = Self::fetch_song_data(request, &client).await {
                    let _ = result_tx.send(result);
                }
            }
        });

        Self {
            request_tx,
            result_rx,
        }
    }

    /// Request prefetch for a song (non-blocking)
    pub fn prefetch(&self, song: Song) {
        let _ = self.request_tx.send(PrefetchRequest { song });
    }

    /// Check if any prefetch results are ready (non-blocking)
    pub fn poll_results(&mut self) -> Vec<PrefetchResult> {
        let mut results = Vec::new();
        while let Ok(result) = self.result_rx.try_recv() {
            results.push(result);
        }
        results
    }

    async fn fetch_song_data(
        request: PrefetchRequest,
        client: &SubsonicClient,
    ) -> Option<PrefetchResult> {
        let song = request.song;

        // Fetch cover art + pre-process in parallel with lyrics
        let cover_future = async {
            if let Some(cover_id) = &song.cover_art {
                let url = client.get_cover_art_url(cover_id, Some(300));
                if let Ok(response) = reqwest::get(&url).await {
                    if let Ok(bytes) = response.bytes().await {
                        let bytes_vec = bytes.to_vec();

                        // Pre-process image
                        if let Ok(img) = image::ImageReader::new(std::io::Cursor::new(&bytes_vec))
                            .with_guessed_format()
                            .and_then(|r| r.decode())
                        {
                            let mut picker = Picker::from_termios()
                                .unwrap_or_else(|_| Picker::new((8, 16)));
                            picker.guess_protocol();
                            let protocol = picker.new_resize_protocol(img);
                            return Some((bytes_vec, protocol));
                        }
                    }
                }
            }
            None
        };

        let lyrics_future = async {
            let artist = song.artist.as_deref().unwrap_or("");
            let title = &song.title;

            // Try server first
            if let Ok(Some(lyrics)) = client.get_lyrics(artist, title).await {
                return (
                    lyrics.lines().map(|s| s.to_string()).collect(),
                    Vec::new()
                );
            }

            // Try LRCLIB fallback
            // ... (implement LRCLIB fetch)

            (Vec::new(), Vec::new())
        };

        // Fetch both in parallel
        let (cover_result, (lyrics_lines, lyrics_timestamps)) =
            tokio::join!(cover_future, lyrics_future);

        Some(PrefetchResult {
            song_id: song.id,
            cover_art_data: cover_result.as_ref().map(|(data, _)| data.clone()),
            cover_art_protocol: cover_result.map(|(_, proto)| proto),
            lyrics_lines,
            lyrics_timestamps,
        })
    }
}
```

#### 3. Integrate into App

```rust
// src/app/mod.rs

pub struct App {
    // ... existing fields ...

    cache_manager: CacheManager,
    cached_results: HashMap<String, PrefetchResult>,
}

impl App {
    pub async fn new(config: Config) -> Result<Self> {
        // ... existing initialization ...

        let cache_manager = CacheManager::new(client.clone());

        Ok(Self {
            // ... existing fields ...
            cache_manager,
            cached_results: HashMap::new(),
        })
    }

    pub async fn tick(&mut self) -> Result<()> {
        // Poll for prefetch results (non-blocking!)
        for result in self.cache_manager.poll_results() {
            self.cached_results.insert(result.song_id.clone(), result);
        }

        // Check if current song has cached data ready
        if let Some(current_song) = &self.player.current_song {
            if let Some(cached) = self.cached_results.remove(&current_song.id) {
                // Instant swap - already pre-processed!
                self.cover_art_data = cached.cover_art_data;
                self.cover_art_protocol = cached.cover_art_protocol;
                self.lyrics_lines = cached.lyrics_lines;
                self.lyrics_timestamps = cached.lyrics_timestamps;
            }
        }

        // Request prefetch for next song (non-blocking!)
        let next_position = self.player.queue_position + 1;
        if let Some(next_song) = self.player.queue.get(next_position) {
            if !self.cached_results.contains_key(&next_song.id) {
                self.cache_manager.prefetch(next_song.clone());
            }
        }

        // ... rest of tick() ...
    }
}
```

#### 4. Handle Edge Cases

**Problem: StatefulProtocol isn't Send**

The ratatui-image protocol types may not be `Send`, making them incompatible with tokio::spawn.

**Solution 1**: Store raw bytes in cache, process protocol on main thread
```rust
pub struct PrefetchResult {
    pub cover_art_data: Option<Vec<u8>>,  // ✅ Send
    // pub cover_art_protocol: Option<Box<dyn StatefulProtocol>>, // ❌ Not Send
}

// Process on main thread when swapping:
if let Some(cached) = self.cached_results.remove(&current_song.id) {
    self.cover_art_data = cached.cover_art_data;

    // Process protocol NOW on main thread
    if let Some(ref bytes) = self.cover_art_data {
        if let Ok(img) = image::load_from_memory(bytes) {
            let mut picker = Picker::from_termios().unwrap_or_else(|_| Picker::new((8, 16)));
            picker.guess_protocol();
            self.cover_art_protocol = Some(picker.new_resize_protocol(img));
        }
    }
}
```

**Solution 2**: Use parking_lot for thread-safe protocol storage (if possible)

#### 5. Memory Management

Implement LRU cache to prevent unbounded memory growth:

```rust
use std::collections::VecDeque;

const MAX_CACHE_SIZE: usize = 5; // Cache 5 songs

impl App {
    fn manage_cache(&mut self) {
        // Remove old entries if cache too large
        if self.cached_results.len() > MAX_CACHE_SIZE {
            // Remove entries not in queue
            let queue_ids: HashSet<_> = self.player.queue
                .iter()
                .map(|s| &s.id)
                .collect();

            self.cached_results.retain(|id, _| queue_ids.contains(id));
        }
    }
}
```

### Benefits

✅ **Zero UI freezes** - All network/processing happens in background
✅ **Instant transitions** - Cover art pre-loaded and pre-processed
✅ **Efficient** - Only prefetches next song, not entire queue
✅ **Resilient** - Falls back gracefully if prefetch isn't ready
✅ **Parallel fetching** - Cover art + lyrics fetch simultaneously

### Estimated Implementation Time

- **Core cache manager**: 1-2 hours
- **Integration + testing**: 1-2 hours
- **Edge case handling** (Send trait, etc.): 1-2 hours
- **Memory management + cleanup**: 30 min

**Total**: 4-6 hours of focused development

### Testing Strategy

1. **Test instant transitions**: Let songs play through naturally
2. **Test quick skips**: Rapidly skip songs before prefetch completes (should show stale art briefly)
3. **Test memory usage**: Monitor RSS after playing 50+ songs
4. **Test error handling**: Disconnect network mid-prefetch
5. **Test pause/resume**: Ensure prefetch stops when paused

### Future Enhancements

- **Disk cache**: Persist cover art to `~/.cache/sonic-tui/` for offline viewing
- **Configurable prefetch distance**: Prefetch 2-3 songs ahead
- **Adaptive prefetching**: Prefetch more aggressively on Wi-Fi vs mobile
- **Cover art size selection**: Prefetch lower resolution for bandwidth savings

## References

- [tokio channels documentation](https://docs.rs/tokio/latest/tokio/sync/mpsc/index.html)
- [ratatui-image protocol trait](https://docs.rs/ratatui-image/latest/ratatui_image/protocol/trait.StatefulProtocol.html)
- Similar implementation: [spotify-tui's caching](https://github.com/Rigellute/spotify-tui)

---

**Document Status**: Ready for implementation
**Last Updated**: 2026-02-08
**Priority**: Medium (quality-of-life improvement)
