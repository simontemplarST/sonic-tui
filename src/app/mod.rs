use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use rand::seq::SliceRandom;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use std::time::{Instant, SystemTime};

use crate::api::{Album, Artist, AlbumDetail, Playlist, Song, SubsonicClient};
use crate::config::Config;
use crate::discord::DiscordPresence;
use crate::player::Player;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusPanel {
    Artists,
    Albums,
    Songs,
    Queue,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
    Search,
    SearchResults,
    Playlists,
    Help,
    AddToPlaylist,
    CreatePlaylist,
    NowPlaying,
    Favorites,
    History,
    Filter,
    Statistics,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NowPlayingView {
    Lyrics,
    Queue,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RepeatMode {
    Off,
    All,
    One,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchFocus {
    Artists,
    Albums,
    Songs,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FavoritesFocus {
    Artists,
    Albums,
    Songs,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ArtistSortMode {
    NameAsc,
    NameDesc,
    AlbumCount,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AlbumSortMode {
    NameAsc,
    NameDesc,
    Year,
    Artist,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SongSortMode {
    TitleAsc,
    TitleDesc,
    Artist,
    Album,
    Track,
    Duration,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HistoryEntry {
    pub song: Song,
    pub played_at: SystemTime,
}

#[derive(Debug, Clone)]
pub struct SongStats {
    pub song: Song,
    pub play_count: usize,
}

#[derive(Debug, Clone)]
pub struct AlbumStats {
    pub album_name: String,
    pub artist_name: String,
    pub play_count: usize,
}

#[derive(Debug, Clone)]
pub struct ArtistStats {
    pub artist_name: String,
    pub play_count: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Preferences {
    artist_sort_mode: ArtistSortMode,
    album_sort_mode: AlbumSortMode,
    song_sort_mode: SongSortMode,
}

pub struct App {
    pub client: SubsonicClient,
    pub player: Player,
    #[allow(dead_code)]
    pub config: Config,

    // Focus state
    pub focus: FocusPanel,
    pub input_mode: InputMode,
    pub search_query: String,

    // Library data
    pub artists: Vec<Artist>,
    pub albums: Vec<Album>,
    pub songs: Vec<Song>,
    pub playlists: Vec<Playlist>,

    // Current selection
    pub selected_artist: usize,
    pub selected_album: usize,
    pub selected_song: usize,
    pub selected_playlist: usize,
    pub selected_queue: usize,

    // Current context
    pub current_artist: Option<Artist>,
    pub current_album: Option<AlbumDetail>,

    // Search results
    pub search_artists: Vec<Artist>,
    pub search_albums: Vec<Album>,
    pub search_songs: Vec<Song>,
    pub search_focus: SearchFocus,
    pub search_selected_artist: usize,
    pub search_selected_album: usize,
    pub search_selected_song: usize,

    // Favorites
    pub favorites_artists: Vec<Artist>,
    pub favorites_albums: Vec<Album>,
    pub favorites_songs: Vec<Song>,
    pub favorites_focus: FavoritesFocus,
    pub favorites_selected_artist: usize,
    pub favorites_selected_album: usize,
    pub favorites_selected_song: usize,

    // Playback history
    pub history: Vec<HistoryEntry>,
    pub history_selected: usize,

    // Statistics
    pub top_songs: Vec<SongStats>,
    pub top_albums: Vec<AlbumStats>,
    pub top_artists: Vec<ArtistStats>,
    pub recently_added_albums: Vec<Album>,
    pub total_playtime_minutes: u64,

    // Filtering and sorting
    pub filter_query: String,
    pub artist_sort_mode: ArtistSortMode,
    pub album_sort_mode: AlbumSortMode,
    pub song_sort_mode: SongSortMode,

    // Status
    pub status_message: Option<String>,
    pub loading: bool,

    // Playback modes
    pub shuffle: bool,
    pub repeat: RepeatMode,
    pub radio_mode: bool,
    radio_seed_song_id: Option<String>, // Track which song started the radio

    // Add to playlist
    pub add_to_playlist_song: Option<Song>,
    pub new_playlist_name: String,

    // Scrobble tracking
    pub scrobbled_song_id: Option<String>,

    // History tracking
    last_history_song_id: Option<String>,

    // Cover art
    pub cover_art_data: Option<Vec<u8>>,
    pub cover_art_protocol: Option<Box<dyn StatefulProtocol>>,

    // Lyrics
    pub lyrics: Option<String>,
    pub lyrics_lines: Vec<String>,
    pub lyrics_timestamps: Vec<f64>, // Timestamps in seconds for synced lyrics

    // Now playing view
    pub now_playing_view: NowPlayingView,

    // Discord Rich Presence
    pub discord: Option<DiscordPresence>,
    last_discord_update: Option<String>, // Track last song ID to avoid redundant updates
    pause_started_at: Option<Instant>, // Track when the current song was paused

    // Audio visualizer
    pub visualizer_bars: Vec<f32>, // Bar heights for visualizer (0.0 to 1.0)
    visualizer_tick: u64,
}

impl App {
    pub async fn new(config: Config) -> Result<Self> {
        let client = SubsonicClient::new(
            &config.server.url,
            &config.server.username,
            &config.server.password,
        )?;

        // Test connection
        client.ping().await?;

        let player = Player::new(config.audio.volume, config.audio.max_bitrate)?;

        // Initialize Discord Rich Presence if enabled
        let discord = if config.discord.enabled {
            if let Some(ref app_id) = config.discord.application_id {
                match DiscordPresence::new(app_id) {
                    Ok(presence) => Some(presence),
                    Err(e) => {
                        tracing::warn!("Failed to initialize Discord presence: {}", e);
                        None
                    }
                }
            } else {
                tracing::warn!("Discord enabled but no application_id provided");
                None
            }
        } else {
            None
        };

        let mut app = Self {
            client,
            player,
            config,
            focus: FocusPanel::Artists,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            artists: Vec::new(),
            albums: Vec::new(),
            songs: Vec::new(),
            playlists: Vec::new(),
            selected_artist: 0,
            selected_album: 0,
            selected_song: 0,
            selected_playlist: 0,
            selected_queue: 0,
            current_artist: None,
            current_album: None,
            search_artists: Vec::new(),
            search_albums: Vec::new(),
            search_songs: Vec::new(),
            search_focus: SearchFocus::Artists,
            search_selected_artist: 0,
            search_selected_album: 0,
            search_selected_song: 0,
            favorites_artists: Vec::new(),
            favorites_albums: Vec::new(),
            favorites_songs: Vec::new(),
            favorites_focus: FavoritesFocus::Artists,
            favorites_selected_artist: 0,
            favorites_selected_album: 0,
            favorites_selected_song: 0,
            history: Vec::new(),
            history_selected: 0,
            top_songs: Vec::new(),
            top_albums: Vec::new(),
            top_artists: Vec::new(),
            recently_added_albums: Vec::new(),
            total_playtime_minutes: 0,
            filter_query: String::new(),
            artist_sort_mode: ArtistSortMode::NameAsc,
            album_sort_mode: AlbumSortMode::NameAsc,
            song_sort_mode: SongSortMode::Track,
            status_message: Some("Loading library...".to_string()),
            loading: true,
            cover_art_data: None,
            cover_art_protocol: None,
            lyrics: None,
            lyrics_lines: Vec::new(),
            lyrics_timestamps: Vec::new(),
            shuffle: false,
            repeat: RepeatMode::Off,
            radio_mode: false,
            radio_seed_song_id: None,
            add_to_playlist_song: None,
            new_playlist_name: String::new(),
            scrobbled_song_id: None,
            last_history_song_id: None,
            now_playing_view: NowPlayingView::Lyrics,
            discord,
            last_discord_update: None,
            pause_started_at: None,
            visualizer_bars: vec![0.0; 32], // 32 bars
            visualizer_tick: 0,
        };

        // Load history from disk
        let _ = app.load_history();

        // Load preferences from disk
        let _ = app.load_preferences();

        // Load initial data
        app.load_artists().await?;
        app.load_playlists().await?;
        app.status_message = None;
        app.loading = false;

        Ok(app)
    }

    pub async fn load_artists(&mut self) -> Result<()> {
        self.artists = self.client.get_artists().await?;
        self.sort_artists();
        Ok(())
    }

    pub async fn load_playlists(&mut self) -> Result<()> {
        self.playlists = self.client.get_playlists().await?;
        Ok(())
    }

    pub async fn load_favorites(&mut self) -> Result<()> {
        let starred = self.client.get_starred().await?;
        self.favorites_artists = starred.artists;
        self.favorites_albums = starred.albums;
        self.favorites_songs = starred.songs;
        Ok(())
    }

    // Sorting methods
    pub fn sort_artists(&mut self) {
        match self.artist_sort_mode {
            ArtistSortMode::NameAsc => {
                self.artists.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            }
            ArtistSortMode::NameDesc => {
                self.artists.sort_by(|a, b| b.name.to_lowercase().cmp(&a.name.to_lowercase()));
            }
            ArtistSortMode::AlbumCount => {
                self.artists.sort_by(|a, b| {
                    b.album_count.unwrap_or(0).cmp(&a.album_count.unwrap_or(0))
                });
            }
        }
    }

    pub fn sort_albums(&mut self) {
        match self.album_sort_mode {
            AlbumSortMode::NameAsc => {
                self.albums.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            }
            AlbumSortMode::NameDesc => {
                self.albums.sort_by(|a, b| b.name.to_lowercase().cmp(&a.name.to_lowercase()));
            }
            AlbumSortMode::Year => {
                self.albums.sort_by(|a, b| {
                    b.year.unwrap_or(0).cmp(&a.year.unwrap_or(0))
                });
            }
            AlbumSortMode::Artist => {
                self.albums.sort_by(|a, b| {
                    let artist_a = a.artist.as_deref().unwrap_or("").to_lowercase();
                    let artist_b = b.artist.as_deref().unwrap_or("").to_lowercase();
                    artist_a.cmp(&artist_b)
                });
            }
        }
    }

    pub fn sort_songs(&mut self) {
        match self.song_sort_mode {
            SongSortMode::TitleAsc => {
                self.songs.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
            }
            SongSortMode::TitleDesc => {
                self.songs.sort_by(|a, b| b.title.to_lowercase().cmp(&a.title.to_lowercase()));
            }
            SongSortMode::Artist => {
                self.songs.sort_by(|a, b| {
                    let artist_a = a.artist.as_deref().unwrap_or("").to_lowercase();
                    let artist_b = b.artist.as_deref().unwrap_or("").to_lowercase();
                    artist_a.cmp(&artist_b)
                });
            }
            SongSortMode::Album => {
                self.songs.sort_by(|a, b| {
                    let album_a = a.album.as_deref().unwrap_or("").to_lowercase();
                    let album_b = b.album.as_deref().unwrap_or("").to_lowercase();
                    album_a.cmp(&album_b)
                });
            }
            SongSortMode::Track => {
                self.songs.sort_by(|a, b| {
                    // Sort by disc number first, then track number
                    let disc_a = a.disc_number.unwrap_or(1);
                    let disc_b = b.disc_number.unwrap_or(1);
                    let track_a = a.track.unwrap_or(9999);
                    let track_b = b.track.unwrap_or(9999);

                    disc_a.cmp(&disc_b).then(track_a.cmp(&track_b))
                });
            }
            SongSortMode::Duration => {
                self.songs.sort_by(|a, b| {
                    b.duration.unwrap_or(0).cmp(&a.duration.unwrap_or(0))
                });
            }
        }
    }

    // Filtering methods - return filtered lists
    pub fn get_filtered_artists(&self) -> Vec<Artist> {
        if self.filter_query.is_empty() {
            self.artists.clone()
        } else {
            let query = self.filter_query.to_lowercase();
            self.artists
                .iter()
                .filter(|artist| artist.name.to_lowercase().contains(&query))
                .cloned()
                .collect()
        }
    }

    pub fn get_filtered_albums(&self) -> Vec<Album> {
        if self.filter_query.is_empty() {
            self.albums.clone()
        } else {
            let query = self.filter_query.to_lowercase();
            self.albums
                .iter()
                .filter(|album| {
                    album.name.to_lowercase().contains(&query)
                        || album.artist.as_ref().map_or(false, |a| a.to_lowercase().contains(&query))
                })
                .cloned()
                .collect()
        }
    }

    pub fn get_filtered_songs(&self) -> Vec<Song> {
        if self.filter_query.is_empty() {
            self.songs.clone()
        } else {
            let query = self.filter_query.to_lowercase();
            self.songs
                .iter()
                .filter(|song| {
                    song.title.to_lowercase().contains(&query)
                        || song.artist.as_ref().map_or(false, |a| a.to_lowercase().contains(&query))
                        || song.album.as_ref().map_or(false, |a| a.to_lowercase().contains(&query))
                })
                .cloned()
                .collect()
        }
    }

    // Cycle through sort modes
    pub fn cycle_artist_sort(&mut self) {
        self.artist_sort_mode = match self.artist_sort_mode {
            ArtistSortMode::NameAsc => ArtistSortMode::NameDesc,
            ArtistSortMode::NameDesc => ArtistSortMode::AlbumCount,
            ArtistSortMode::AlbumCount => ArtistSortMode::NameAsc,
        };
        self.sort_artists();
        let _ = self.save_preferences();
    }

    pub fn cycle_album_sort(&mut self) {
        self.album_sort_mode = match self.album_sort_mode {
            AlbumSortMode::NameAsc => AlbumSortMode::NameDesc,
            AlbumSortMode::NameDesc => AlbumSortMode::Year,
            AlbumSortMode::Year => AlbumSortMode::Artist,
            AlbumSortMode::Artist => AlbumSortMode::NameAsc,
        };
        self.sort_albums();
        let _ = self.save_preferences();
    }

    pub fn cycle_song_sort(&mut self) {
        self.song_sort_mode = match self.song_sort_mode {
            SongSortMode::Track => SongSortMode::TitleAsc,
            SongSortMode::TitleAsc => SongSortMode::TitleDesc,
            SongSortMode::TitleDesc => SongSortMode::Artist,
            SongSortMode::Artist => SongSortMode::Album,
            SongSortMode::Album => SongSortMode::Duration,
            SongSortMode::Duration => SongSortMode::Track,
        };
        self.sort_songs();
        let _ = self.save_preferences();
    }

    fn history_file_path() -> Result<std::path::PathBuf> {
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find cache directory"))?
            .join("sonic-tui");
        std::fs::create_dir_all(&cache_dir)?;
        Ok(cache_dir.join("history.json"))
    }

    fn preferences_file_path() -> Result<std::path::PathBuf> {
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find cache directory"))?
            .join("sonic-tui");
        std::fs::create_dir_all(&cache_dir)?;
        Ok(cache_dir.join("preferences.json"))
    }

    pub fn load_preferences(&mut self) -> Result<()> {
        let path = Self::preferences_file_path()?;
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let prefs: Preferences = serde_json::from_str(&content)?;
            self.artist_sort_mode = prefs.artist_sort_mode;
            self.album_sort_mode = prefs.album_sort_mode;
            self.song_sort_mode = prefs.song_sort_mode;
        }
        Ok(())
    }

    pub fn save_preferences(&self) -> Result<()> {
        let path = Self::preferences_file_path()?;
        let prefs = Preferences {
            artist_sort_mode: self.artist_sort_mode,
            album_sort_mode: self.album_sort_mode,
            song_sort_mode: self.song_sort_mode,
        };
        let content = serde_json::to_string_pretty(&prefs)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn load_history(&mut self) -> Result<()> {
        let path = Self::history_file_path()?;
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let mut loaded: Vec<HistoryEntry> = serde_json::from_str(&content).unwrap_or_default();
            // Limit to last 100 entries (most recent)
            if loaded.len() > 100 {
                loaded = loaded.into_iter().take(100).collect();
            }
            self.history = loaded;
        }
        Ok(())
    }

    pub fn save_history(&self) -> Result<()> {
        let path = Self::history_file_path()?;
        let content = serde_json::to_string_pretty(&self.history)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn add_to_history(&mut self, song: Song) {
        // Add to beginning of history (most recent first)
        self.history.insert(0, HistoryEntry {
            song,
            played_at: SystemTime::now(),
        });

        // Keep only last 100 entries
        if self.history.len() > 100 {
            self.history.truncate(100);
        }

        // Save to disk
        let _ = self.save_history();
    }

    pub async fn calculate_statistics(&mut self) -> Result<()> {
        use std::collections::HashMap;

        // Calculate play counts for songs, albums, and artists
        let mut song_counts: HashMap<String, (Song, usize)> = HashMap::new();
        let mut album_counts: HashMap<String, (String, String, usize)> = HashMap::new(); // (album_name, artist_name, count)
        let mut artist_counts: HashMap<String, usize> = HashMap::new();
        let mut total_duration = 0u64;

        for entry in &self.history {
            let song_id = entry.song.id.clone();

            // Count song plays
            song_counts
                .entry(song_id)
                .and_modify(|(_, count)| *count += 1)
                .or_insert((entry.song.clone(), 1));

            // Count album plays
            if let Some(ref album_name) = entry.song.album {
                let artist_name = entry.song.artist.clone().unwrap_or_else(|| "Unknown Artist".to_string());
                album_counts
                    .entry(album_name.clone())
                    .and_modify(|(_, _, count)| *count += 1)
                    .or_insert((album_name.clone(), artist_name, 1));
            }

            // Count artist plays
            if let Some(ref artist_name) = entry.song.artist {
                *artist_counts.entry(artist_name.clone()).or_insert(0) += 1;
            }

            // Sum total playtime
            total_duration += entry.song.duration.unwrap_or(0) as u64;
        }

        // Convert to sorted vectors (top 10)
        let mut top_songs: Vec<SongStats> = song_counts
            .into_iter()
            .map(|(_, (song, count))| SongStats { song, play_count: count })
            .collect();
        top_songs.sort_by(|a, b| b.play_count.cmp(&a.play_count));
        top_songs.truncate(10);

        let mut top_albums: Vec<AlbumStats> = album_counts
            .into_iter()
            .map(|(_, (album_name, artist_name, count))| AlbumStats {
                album_name,
                artist_name,
                play_count: count,
            })
            .collect();
        top_albums.sort_by(|a, b| b.play_count.cmp(&a.play_count));
        top_albums.truncate(10);

        let mut top_artists: Vec<ArtistStats> = artist_counts
            .into_iter()
            .map(|(artist_name, count)| ArtistStats {
                artist_name,
                play_count: count,
            })
            .collect();
        top_artists.sort_by(|a, b| b.play_count.cmp(&a.play_count));
        top_artists.truncate(10);

        self.top_songs = top_songs;
        self.top_albums = top_albums;
        self.top_artists = top_artists;
        self.total_playtime_minutes = total_duration / 60;

        // Fetch recently added albums
        self.recently_added_albums = self.client.get_recently_added_albums(20).await?;

        Ok(())
    }

    pub async fn load_artist_albums(&mut self, artist_id: &str) -> Result<()> {
        let artist = self.client.get_artist(artist_id).await?;
        self.albums = artist.albums;
        tracing::debug!("Loaded {} albums for artist {}", self.albums.len(), artist.name);

        // Debug: check for duplicate album IDs
        let mut album_ids: Vec<String> = self.albums.iter().map(|a| a.id.clone()).collect();
        album_ids.sort();
        let original_len = album_ids.len();
        album_ids.dedup();
        if album_ids.len() != original_len {
            tracing::warn!("Found {} duplicate albums for artist {}", original_len - album_ids.len(), artist.name);
            // Log album names to see what's duplicated
            for album in &self.albums {
                tracing::debug!("  Album: {} (id: {})", album.name, album.id);
            }
        }

        self.sort_albums();
        self.current_artist = Some(Artist {
            id: artist.id,
            name: artist.name,
            album_count: artist.album_count,
            cover_art: None,
            starred: artist.starred,
        });
        self.selected_album = 0;
        Ok(())
    }

    pub async fn load_album_songs(&mut self, album_id: &str) -> Result<()> {
        let album = self.client.get_album(album_id).await?;
        self.songs = album.songs.clone();
        tracing::debug!("Loaded {} songs for album {} ({})", self.songs.len(), album.name, album_id);

        // Debug: check for duplicate song IDs
        let mut song_ids: Vec<String> = self.songs.iter().map(|s| s.id.clone()).collect();
        song_ids.sort();
        let original_len = song_ids.len();
        song_ids.dedup();
        if song_ids.len() != original_len {
            tracing::warn!("Found {} duplicate songs in album {}", original_len - song_ids.len(), album.name);
        }

        self.sort_songs();
        self.current_album = Some(album);
        self.selected_song = 0;
        Ok(())
    }

    pub async fn search(&mut self, query: &str) -> Result<()> {
        let results = self.client.search(query).await?;
        self.search_artists = results.artists;
        self.search_albums = results.albums;
        self.search_songs = results.songs;
        Ok(())
    }

    /// Check if a key event matches a configured keybinding
    fn key_matches(&self, key: &KeyEvent, binding: &str) -> bool {
        match key.code {
            KeyCode::Char(c) => {
                let key_str = c.to_string();
                binding == key_str
            }
            _ => false,
        }
    }

    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
        // Handle search input mode
        if self.input_mode == InputMode::Search {
            match key.code {
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                    self.search_query.clear();
                }
                KeyCode::Enter => {
                    if !self.search_query.is_empty() {
                        self.search(&self.search_query.clone()).await?;
                        self.search_focus = SearchFocus::Artists;
                        self.search_selected_artist = 0;
                        self.search_selected_album = 0;
                        self.search_selected_song = 0;
                        self.input_mode = InputMode::SearchResults;
                    } else {
                        self.input_mode = InputMode::Normal;
                    }
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                }
                _ => {}
            }
            return Ok(false);
        }

        // Filter input mode
        if self.input_mode == InputMode::Filter {
            match key.code {
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                    self.filter_query.clear();
                }
                KeyCode::Enter => {
                    self.input_mode = InputMode::Normal;
                    // Filter stays active
                }
                KeyCode::Backspace => {
                    self.filter_query.pop();
                }
                KeyCode::Char(c) => {
                    self.filter_query.push(c);
                }
                _ => {}
            }
            return Ok(false);
        }

        // Help mode
        if self.input_mode == InputMode::Help {
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')) {
                self.input_mode = InputMode::Normal;
            }
            return Ok(false);
        }

        // Now playing mode
        if self.input_mode == InputMode::NowPlaying {
            return self.handle_now_playing_mode(key).await;
        }

        // Playlist mode
        if self.input_mode == InputMode::Playlists {
            return self.handle_playlist_mode(key).await;
        }

        // Favorites mode
        if self.input_mode == InputMode::Favorites {
            return self.handle_favorites_mode(key).await;
        }

        // History mode
        if self.input_mode == InputMode::History {
            return self.handle_history_mode(key).await;
        }

        // Statistics mode
        if self.input_mode == InputMode::Statistics {
            return self.handle_statistics_mode(key).await;
        }

        // Add to playlist mode
        if self.input_mode == InputMode::AddToPlaylist {
            return self.handle_add_to_playlist_mode(key).await;
        }

        // Create playlist mode
        if self.input_mode == InputMode::CreatePlaylist {
            match key.code {
                KeyCode::Esc => {
                    self.input_mode = InputMode::AddToPlaylist;
                    self.new_playlist_name.clear();
                }
                KeyCode::Enter => {
                    if !self.new_playlist_name.is_empty() {
                        // Clone data before mutable borrows
                        let playlist_name = self.new_playlist_name.clone();
                        let song_data = self.add_to_playlist_song.clone();

                        // Create the playlist
                        self.client.create_playlist(&playlist_name).await?;

                        // Add the song to it
                        if let Some(song) = song_data {
                            // Reload playlists to get the new one
                            self.load_playlists().await?;

                            // Find the new playlist and add the song
                            if let Some(playlist) = self.playlists.iter().find(|p| p.name == playlist_name) {
                                self.client.update_playlist(&playlist.id, &[song.id.clone()]).await?;
                                self.status_message = Some(format!(
                                    "Created \"{}\" and added \"{}\"",
                                    playlist_name, song.title
                                ));
                            }
                        }

                        self.new_playlist_name.clear();
                        self.add_to_playlist_song = None;
                        self.input_mode = InputMode::Normal;
                    }
                }
                KeyCode::Backspace => {
                    self.new_playlist_name.pop();
                }
                KeyCode::Char(c) => {
                    self.new_playlist_name.push(c);
                }
                _ => {}
            }
            return Ok(false);
        }

        // Search results mode
        if self.input_mode == InputMode::SearchResults {
            match key.code {
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                    self.search_focus = match self.search_focus {
                        SearchFocus::Artists => SearchFocus::Albums,
                        SearchFocus::Albums => SearchFocus::Songs,
                        SearchFocus::Songs => SearchFocus::Artists,
                    };
                }
                KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                    self.search_focus = match self.search_focus {
                        SearchFocus::Artists => SearchFocus::Songs,
                        SearchFocus::Albums => SearchFocus::Artists,
                        SearchFocus::Songs => SearchFocus::Albums,
                    };
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    match self.search_focus {
                        SearchFocus::Artists => {
                            if !self.search_artists.is_empty() {
                                self.search_selected_artist = (self.search_selected_artist + 1)
                                    .min(self.search_artists.len() - 1);
                            }
                        }
                        SearchFocus::Albums => {
                            if !self.search_albums.is_empty() {
                                self.search_selected_album = (self.search_selected_album + 1)
                                    .min(self.search_albums.len() - 1);
                            }
                        }
                        SearchFocus::Songs => {
                            if !self.search_songs.is_empty() {
                                self.search_selected_song = (self.search_selected_song + 1)
                                    .min(self.search_songs.len() - 1);
                            }
                        }
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    match self.search_focus {
                        SearchFocus::Artists => {
                            self.search_selected_artist = self.search_selected_artist.saturating_sub(1);
                        }
                        SearchFocus::Albums => {
                            self.search_selected_album = self.search_selected_album.saturating_sub(1);
                        }
                        SearchFocus::Songs => {
                            self.search_selected_song = self.search_selected_song.saturating_sub(1);
                        }
                    }
                }
                KeyCode::Enter => {
                    self.handle_search_select().await?;
                }
                KeyCode::Char('e') => {
                    // Enqueue selected song
                    if let SearchFocus::Songs = self.search_focus {
                        if let Some(song) = self.search_songs.get(self.search_selected_song) {
                            self.player.enqueue(song.clone());
                            self.status_message = Some(format!("Added \"{}\" to queue", song.title));
                        }
                    }
                }
                _ => {}
            }
            return Ok(false);
        }

        // Normal mode
        match key.code {
            // Quit
            _ if self.key_matches(&key, &self.config.keybindings.quit) => return Ok(true),

            // Panel switching
            KeyCode::Tab => self.next_panel(),
            KeyCode::BackTab => self.prev_panel(),
            _ if self.key_matches(&key, &self.config.keybindings.focus_left) => self.prev_panel(),
            _ if self.key_matches(&key, &self.config.keybindings.focus_right) => self.next_panel(),

            // Navigation
            KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_selection(-1),
            KeyCode::Char('g') => self.move_to_top(),
            KeyCode::Char('G') => self.move_to_bottom(),

            // Search
            _ if self.key_matches(&key, &self.config.keybindings.search) => {
                self.input_mode = InputMode::Search;
                self.search_query.clear();
            }

            // Filter
            _ if self.key_matches(&key, &self.config.keybindings.filter) => {
                self.input_mode = InputMode::Filter;
            }

            // Clear filter
            KeyCode::Char('c') => {
                if !self.filter_query.is_empty() {
                    self.filter_query.clear();
                    self.status_message = Some("Filter cleared".to_string());
                }
            }

            // Cycle sort mode
            _ if self.key_matches(&key, &self.config.keybindings.sort) => {
                match self.focus {
                    FocusPanel::Artists => {
                        self.cycle_artist_sort();
                        let mode_str = match self.artist_sort_mode {
                            ArtistSortMode::NameAsc => "Name (A-Z)",
                            ArtistSortMode::NameDesc => "Name (Z-A)",
                            ArtistSortMode::AlbumCount => "Album Count",
                        };
                        self.status_message = Some(format!("Sort: {}", mode_str));
                    }
                    FocusPanel::Albums => {
                        self.cycle_album_sort();
                        let mode_str = match self.album_sort_mode {
                            AlbumSortMode::NameAsc => "Name (A-Z)",
                            AlbumSortMode::NameDesc => "Name (Z-A)",
                            AlbumSortMode::Year => "Year",
                            AlbumSortMode::Artist => "Artist",
                        };
                        self.status_message = Some(format!("Sort: {}", mode_str));
                    }
                    FocusPanel::Songs => {
                        self.cycle_song_sort();
                        let mode_str = match self.song_sort_mode {
                            SongSortMode::Track => "Track",
                            SongSortMode::TitleAsc => "Title (A-Z)",
                            SongSortMode::TitleDesc => "Title (Z-A)",
                            SongSortMode::Artist => "Artist",
                            SongSortMode::Album => "Album",
                            SongSortMode::Duration => "Duration",
                        };
                        self.status_message = Some(format!("Sort: {}", mode_str));
                    }
                    FocusPanel::Queue => {
                        self.status_message = Some("Cannot sort queue".to_string());
                    }
                }
            }

            // Enter/Select
            KeyCode::Enter => self.handle_enter().await?,

            // Playback
            _ if self.key_matches(&key, &self.config.keybindings.play_pause) => {
                self.player.toggle_pause()?;
            }
            _ if self.key_matches(&key, &self.config.keybindings.next) => {
                if self.player.current_song.is_some() || !self.player.queue.is_empty() {
                    let client = self.client.clone();
                    let bitrate = self.player.max_bitrate();
                    self.player.next(|song| client.get_stream_url(&song.id, bitrate))?;
                }
            }
            _ if self.key_matches(&key, &self.config.keybindings.previous) => {
                if self.player.current_song.is_some() {
                    let client = self.client.clone();
                    let bitrate = self.player.max_bitrate();
                    self.player.previous(|song| client.get_stream_url(&song.id, bitrate))?;
                }
            }

            // Seek with arrow keys
            KeyCode::Left => {
                if self.player.current_song.is_some() {
                    self.player.seek(-5)?;
                }
            }
            KeyCode::Right => {
                if self.player.current_song.is_some() {
                    self.player.seek(5)?;
                }
            }

            // Shuffle
            _ if self.key_matches(&key, &self.config.keybindings.shuffle) => {
                self.toggle_shuffle();
            }

            // Repeat
            _ if self.key_matches(&key, &self.config.keybindings.repeat) => {
                self.cycle_repeat();
            }

            // Radio mode
            _ if self.key_matches(&key, &self.config.keybindings.radio) => {
                if self.radio_mode {
                    self.stop_radio().await?;
                } else {
                    // Start radio from current song or selected song
                    let song_id = if let Some(ref song) = self.player.current_song {
                        Some(song.id.clone())
                    } else if self.focus == FocusPanel::Songs {
                        let filtered_songs = self.get_filtered_songs();
                        filtered_songs.get(self.selected_song).map(|s| s.id.clone())
                    } else {
                        None
                    };

                    if let Some(id) = song_id {
                        self.start_radio(id).await?;
                    } else {
                        self.status_message = Some("No song selected for radio".to_string());
                    }
                }
            }

            // Volume
            _ if self.key_matches(&key, &self.config.keybindings.volume_up) => {
                self.player.volume_up()?;
            }
            KeyCode::Char('=') => {
                self.player.volume_up()?;
            }
            _ if self.key_matches(&key, &self.config.keybindings.volume_down) => {
                self.player.volume_down()?;
            }

            // Enqueue
            _ if self.key_matches(&key, &self.config.keybindings.enqueue) => self.enqueue_selected().await?,
            _ if self.key_matches(&key, &self.config.keybindings.enqueue_next) => self.enqueue_next_selected().await?,

            // Add all songs from album to queue
            _ if self.key_matches(&key, &self.config.keybindings.enqueue_album) => self.enqueue_album().await?,

            // Add to playlist
            _ if self.key_matches(&key, &self.config.keybindings.add_to_playlist) => {
                let song = match self.focus {
                    FocusPanel::Songs => {
                        let filtered_songs = self.get_filtered_songs();
                        filtered_songs.get(self.selected_song).cloned()
                    }
                    FocusPanel::Queue => self.player.queue.get(self.selected_queue).cloned(),
                    _ => None,
                };
                if let Some(s) = song {
                    self.add_to_playlist_song = Some(s);
                    self.load_playlists().await?;
                    self.selected_playlist = 0;
                    self.input_mode = InputMode::AddToPlaylist;
                }
            }

            // Queue management
            _ if self.key_matches(&key, &self.config.keybindings.remove_from_queue) => {
                if self.focus == FocusPanel::Queue {
                    self.remove_from_queue();
                }
            }
            _ if self.key_matches(&key, &self.config.keybindings.clear_queue) => {
                if self.focus == FocusPanel::Queue {
                    self.clear_queue();
                }
            }
            KeyCode::Char('K') => {
                if self.focus == FocusPanel::Queue {
                    self.move_queue_item_up();
                }
            }
            KeyCode::Char('J') => {
                if self.focus == FocusPanel::Queue {
                    self.move_queue_item_down();
                }
            }
            KeyCode::Char('w') => {
                if self.focus == FocusPanel::Queue {
                    self.save_queue_as_playlist().await?;
                }
            }

            // Star/unstar current item
            _ if self.key_matches(&key, &self.config.keybindings.star) => {
                self.handle_star_toggle().await?;
            }

            // Playlists
            KeyCode::Char('P') => {
                self.load_playlists().await?;
                self.selected_playlist = 0;
                self.input_mode = InputMode::Playlists;
            }

            // Favorites
            _ if self.key_matches(&key, &self.config.keybindings.favorites) => {
                self.load_favorites().await?;
                self.favorites_selected_artist = 0;
                self.favorites_selected_album = 0;
                self.favorites_selected_song = 0;
                self.input_mode = InputMode::Favorites;
            }

            // History
            _ if self.key_matches(&key, &self.config.keybindings.history) => {
                self.history_selected = 0;
                self.input_mode = InputMode::History;
            }

            // Statistics
            _ if self.key_matches(&key, &self.config.keybindings.statistics) => {
                self.calculate_statistics().await?;
                self.input_mode = InputMode::Statistics;
            }

            // Help
            _ if self.key_matches(&key, &self.config.keybindings.help) => {
                self.input_mode = InputMode::Help;
            }

            // Now playing view
            _ if self.key_matches(&key, &self.config.keybindings.now_playing) => {
                self.input_mode = InputMode::NowPlaying;
            }

            _ => {}
        }

        Ok(false)
    }

    async fn handle_playlist_mode(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.playlists.is_empty() {
                    self.selected_playlist = (self.selected_playlist + 1)
                        .min(self.playlists.len() - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected_playlist = self.selected_playlist.saturating_sub(1);
            }
            KeyCode::Char('g') => {
                self.selected_playlist = 0;
            }
            KeyCode::Char('G') => {
                self.selected_playlist = self.playlists.len().saturating_sub(1);
            }
            KeyCode::Enter => {
                // Load playlist songs and play
                if let Some(playlist) = self.playlists.get(self.selected_playlist) {
                    let playlist_id = playlist.id.clone();
                    let playlist_name = playlist.name.clone();

                    let detail = self.client.get_playlist(&playlist_id).await?;
                    if !detail.entries.is_empty() {
                        self.player.clear_queue();
                        for song in &detail.entries {
                            self.player.enqueue(song.clone());
                        }

                        let first_song = detail.entries[0].clone();
                        let client = self.client.clone();
                        let bitrate = self.player.max_bitrate();
                        self.player.play_from_queue(0, |song| {
                            client.get_stream_url(&song.id, bitrate)
                        })?;

                        // Load cover art and lyrics
                        if let Some(cover_id) = &first_song.cover_art {
                            self.load_cover_art(cover_id).await?;
                        }
                        let artist = first_song.artist.as_deref().unwrap_or("");
                        self.load_lyrics(artist, &first_song.title).await?;

                        self.status_message = Some(format!("Playing playlist: {}", playlist_name));
                    }
                    self.input_mode = InputMode::Normal;
                    self.focus = FocusPanel::Queue;
                }
            }
            KeyCode::Char('e') => {
                // Add playlist to queue without playing
                if let Some(playlist) = self.playlists.get(self.selected_playlist) {
                    let playlist_name = playlist.name.clone();
                    let detail = self.client.get_playlist(&playlist.id).await?;
                    for song in detail.entries {
                        self.player.enqueue(song);
                    }
                    self.status_message = Some(format!("Added playlist \"{}\" to queue", playlist_name));
                    self.input_mode = InputMode::Normal;
                    self.focus = FocusPanel::Queue;
                }
            }
            KeyCode::Char('d') => {
                // Delete playlist
                if let Some(playlist) = self.playlists.get(self.selected_playlist) {
                    let playlist_id = playlist.id.clone();
                    let playlist_name = playlist.name.clone();
                    self.client.delete_playlist(&playlist_id).await?;
                    self.load_playlists().await?;
                    self.selected_playlist = self.selected_playlist.min(self.playlists.len().saturating_sub(1));
                    self.status_message = Some(format!("Deleted playlist \"{}\"", playlist_name));
                }
            }
            _ => {}
        }
        Ok(false)
    }

    async fn handle_add_to_playlist_mode(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.add_to_playlist_song = None;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.playlists.is_empty() {
                    self.selected_playlist = (self.selected_playlist + 1)
                        .min(self.playlists.len() - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected_playlist = self.selected_playlist.saturating_sub(1);
            }
            KeyCode::Char('n') => {
                // Create new playlist
                self.new_playlist_name.clear();
                self.input_mode = InputMode::CreatePlaylist;
            }
            KeyCode::Enter => {
                if let Some(song) = &self.add_to_playlist_song {
                    if let Some(playlist) = self.playlists.get(self.selected_playlist) {
                        let playlist_id = playlist.id.clone();
                        let playlist_name = playlist.name.clone();
                        let song_id = song.id.clone();
                        let song_title = song.title.clone();

                        self.client.update_playlist(&playlist_id, &[song_id]).await?;
                        self.status_message = Some(format!(
                            "Added \"{}\" to \"{}\"",
                            song_title, playlist_name
                        ));
                    }
                }
                self.input_mode = InputMode::Normal;
                self.add_to_playlist_song = None;
            }
            _ => {}
        }
        Ok(false)
    }

    async fn handle_favorites_mode(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                self.favorites_focus = match self.favorites_focus {
                    FavoritesFocus::Artists => FavoritesFocus::Albums,
                    FavoritesFocus::Albums => FavoritesFocus::Songs,
                    FavoritesFocus::Songs => FavoritesFocus::Artists,
                };
            }
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                self.favorites_focus = match self.favorites_focus {
                    FavoritesFocus::Artists => FavoritesFocus::Songs,
                    FavoritesFocus::Albums => FavoritesFocus::Artists,
                    FavoritesFocus::Songs => FavoritesFocus::Albums,
                };
            }
            KeyCode::Char('j') | KeyCode::Down => {
                match self.favorites_focus {
                    FavoritesFocus::Artists => {
                        if !self.favorites_artists.is_empty() {
                            self.favorites_selected_artist = (self.favorites_selected_artist + 1)
                                .min(self.favorites_artists.len() - 1);
                        }
                    }
                    FavoritesFocus::Albums => {
                        if !self.favorites_albums.is_empty() {
                            self.favorites_selected_album = (self.favorites_selected_album + 1)
                                .min(self.favorites_albums.len() - 1);
                        }
                    }
                    FavoritesFocus::Songs => {
                        if !self.favorites_songs.is_empty() {
                            self.favorites_selected_song = (self.favorites_selected_song + 1)
                                .min(self.favorites_songs.len() - 1);
                        }
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                match self.favorites_focus {
                    FavoritesFocus::Artists => {
                        self.favorites_selected_artist = self.favorites_selected_artist.saturating_sub(1);
                    }
                    FavoritesFocus::Albums => {
                        self.favorites_selected_album = self.favorites_selected_album.saturating_sub(1);
                    }
                    FavoritesFocus::Songs => {
                        self.favorites_selected_song = self.favorites_selected_song.saturating_sub(1);
                    }
                }
            }
            KeyCode::Char('g') => {
                match self.favorites_focus {
                    FavoritesFocus::Artists => self.favorites_selected_artist = 0,
                    FavoritesFocus::Albums => self.favorites_selected_album = 0,
                    FavoritesFocus::Songs => self.favorites_selected_song = 0,
                }
            }
            KeyCode::Char('G') => {
                match self.favorites_focus {
                    FavoritesFocus::Artists => {
                        self.favorites_selected_artist = self.favorites_artists.len().saturating_sub(1);
                    }
                    FavoritesFocus::Albums => {
                        self.favorites_selected_album = self.favorites_albums.len().saturating_sub(1);
                    }
                    FavoritesFocus::Songs => {
                        self.favorites_selected_song = self.favorites_songs.len().saturating_sub(1);
                    }
                }
            }
            KeyCode::Enter => {
                self.handle_favorites_select().await?;
            }
            KeyCode::Char('e') => {
                // Enqueue selected item
                if let FavoritesFocus::Songs = self.favorites_focus {
                    if let Some(song) = self.favorites_songs.get(self.favorites_selected_song) {
                        self.player.enqueue(song.clone());
                        self.status_message = Some(format!("Added \"{}\" to queue", song.title));
                    }
                }
            }
            KeyCode::Char('S') => {
                // Unstar (remove from favorites) the selected item
                self.handle_unstar_favorite().await?;
            }
            _ => {}
        }
        Ok(false)
    }

    async fn handle_history_mode(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.history.is_empty() {
                    self.history_selected = (self.history_selected + 1)
                        .min(self.history.len() - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.history_selected = self.history_selected.saturating_sub(1);
            }
            KeyCode::Char('g') => {
                self.history_selected = 0;
            }
            KeyCode::Char('G') => {
                self.history_selected = self.history.len().saturating_sub(1);
            }
            KeyCode::Enter => {
                // Play selected song from history
                if let Some(entry) = self.history.get(self.history_selected) {
                    let song = entry.song.clone();
                    self.player.clear_queue();
                    self.player.enqueue(song.clone());
                    let client = self.client.clone();
                    let bitrate = self.player.max_bitrate();
                    self.player.play_from_queue(0, |s| {
                        client.get_stream_url(&s.id, bitrate)
                    })?;

                    // Load cover art and lyrics
                    if let Some(cover_id) = &song.cover_art {
                        self.load_cover_art(cover_id).await?;
                    }
                    let artist = song.artist.as_deref().unwrap_or("");
                    self.load_lyrics(artist, &song.title).await?;

                    self.input_mode = InputMode::Normal;
                }
            }
            KeyCode::Char('e') => {
                // Enqueue selected song from history
                if let Some(entry) = self.history.get(self.history_selected) {
                    self.player.enqueue(entry.song.clone());
                    self.status_message = Some(format!("Added \"{}\" to queue", entry.song.title));
                }
            }
            KeyCode::Char('c') => {
                // Clear history
                self.history.clear();
                self.history_selected = 0;
                let _ = self.save_history();
                self.status_message = Some("History cleared".to_string());
            }
            _ => {}
        }
        Ok(false)
    }

    async fn handle_statistics_mode(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('I') => {
                self.input_mode = InputMode::Normal;
            }
            _ => {}
        }
        Ok(false)
    }

    async fn handle_now_playing_mode(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            // Exit now playing view
            KeyCode::Esc | KeyCode::Char('f') | KeyCode::Char('q') => {
                self.input_mode = InputMode::Normal;
            }
            // Toggle between lyrics and queue
            KeyCode::Tab | KeyCode::Char('t') => {
                self.now_playing_view = match self.now_playing_view {
                    NowPlayingView::Lyrics => NowPlayingView::Queue,
                    NowPlayingView::Queue => NowPlayingView::Lyrics,
                };
            }
            // Navigate queue if queue view is active
            KeyCode::Char('j') | KeyCode::Down => {
                if self.now_playing_view == NowPlayingView::Queue && !self.player.queue.is_empty() {
                    self.selected_queue = (self.selected_queue + 1).min(self.player.queue.len() - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.now_playing_view == NowPlayingView::Queue {
                    self.selected_queue = self.selected_queue.saturating_sub(1);
                }
            }
            KeyCode::Char('g') => {
                if self.now_playing_view == NowPlayingView::Queue {
                    self.selected_queue = 0;
                }
            }
            KeyCode::Char('G') => {
                if self.now_playing_view == NowPlayingView::Queue {
                    self.selected_queue = self.player.queue.len().saturating_sub(1);
                }
            }
            // Play selected song from queue
            KeyCode::Enter => {
                if self.now_playing_view == NowPlayingView::Queue {
                    if let Some(song) = self.player.queue.get(self.selected_queue).cloned() {
                        let client = self.client.clone();
                        let bitrate = self.player.max_bitrate();
                        self.player.play_from_queue(self.selected_queue, |song| {
                            client.get_stream_url(&song.id, bitrate)
                        })?;

                        // Load cover art and lyrics
                        if let Some(cover_id) = &song.cover_art {
                            self.load_cover_art(cover_id).await?;
                        }
                        let artist = song.artist.as_deref().unwrap_or("");
                        self.load_lyrics(artist, &song.title).await?;
                    }
                }
            }
            // Playback controls
            KeyCode::Char(' ') => {
                self.player.toggle_pause()?;
            }
            KeyCode::Char('n') => {
                if self.player.current_song.is_some() || !self.player.queue.is_empty() {
                    let client = self.client.clone();
                    let bitrate = self.player.max_bitrate();
                    self.player.next(|song| client.get_stream_url(&song.id, bitrate))?;
                }
            }
            KeyCode::Char('p') => {
                if self.player.current_song.is_some() {
                    let client = self.client.clone();
                    let bitrate = self.player.max_bitrate();
                    self.player.previous(|song| client.get_stream_url(&song.id, bitrate))?;
                }
            }
            KeyCode::Left => {
                if self.player.current_song.is_some() {
                    self.player.seek(-5)?;
                }
            }
            KeyCode::Right => {
                if self.player.current_song.is_some() {
                    self.player.seek(5)?;
                }
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.player.volume_up()?;
            }
            KeyCode::Char('-') => {
                self.player.volume_down()?;
            }
            _ => {}
        }
        Ok(false)
    }

    fn shuffle_queue(&mut self) {
        if self.player.queue.len() > 1 {
            let current = if self.player.current_song.is_some() {
                Some(self.player.queue.remove(self.player.queue_position))
            } else {
                None
            };

            let mut rng = rand::thread_rng();
            self.player.queue.shuffle(&mut rng);

            if let Some(song) = current {
                self.player.queue.insert(0, song);
                self.player.queue_position = 0;
            }
        }
    }

    fn next_panel(&mut self) {
        self.focus = match self.focus {
            FocusPanel::Artists => FocusPanel::Albums,
            FocusPanel::Albums => FocusPanel::Songs,
            FocusPanel::Songs => FocusPanel::Queue,
            FocusPanel::Queue => FocusPanel::Artists,
        };
    }

    fn prev_panel(&mut self) {
        self.focus = match self.focus {
            FocusPanel::Artists => FocusPanel::Queue,
            FocusPanel::Albums => FocusPanel::Artists,
            FocusPanel::Songs => FocusPanel::Albums,
            FocusPanel::Queue => FocusPanel::Songs,
        };
    }

    fn move_selection(&mut self, delta: i32) {
        // Get lengths first to avoid borrow checker issues
        let len = match self.focus {
            FocusPanel::Artists => self.get_filtered_artists().len(),
            FocusPanel::Albums => self.get_filtered_albums().len(),
            FocusPanel::Songs => self.get_filtered_songs().len(),
            FocusPanel::Queue => self.player.queue.len(),
        };

        if len == 0 {
            return;
        }

        let selected = match self.focus {
            FocusPanel::Artists => &mut self.selected_artist,
            FocusPanel::Albums => &mut self.selected_album,
            FocusPanel::Songs => &mut self.selected_song,
            FocusPanel::Queue => &mut self.selected_queue,
        };

        let new_val = *selected as i32 + delta;
        *selected = new_val.clamp(0, len as i32 - 1) as usize;
    }

    fn move_to_top(&mut self) {
        match self.focus {
            FocusPanel::Artists => self.selected_artist = 0,
            FocusPanel::Albums => self.selected_album = 0,
            FocusPanel::Songs => self.selected_song = 0,
            FocusPanel::Queue => self.selected_queue = 0,
        }
    }

    fn move_to_bottom(&mut self) {
        match self.focus {
            FocusPanel::Artists => {
                self.selected_artist = self.get_filtered_artists().len().saturating_sub(1);
            }
            FocusPanel::Albums => {
                self.selected_album = self.get_filtered_albums().len().saturating_sub(1);
            }
            FocusPanel::Songs => {
                self.selected_song = self.get_filtered_songs().len().saturating_sub(1);
            }
            FocusPanel::Queue => {
                self.selected_queue = self.player.queue.len().saturating_sub(1);
            }
        }
    }

    async fn handle_enter(&mut self) -> Result<()> {
        match self.focus {
            FocusPanel::Artists => {
                let filtered_artists = self.get_filtered_artists();
                if let Some(artist) = filtered_artists.get(self.selected_artist) {
                    self.load_artist_albums(&artist.id.clone()).await?;
                    // Clear filter when navigating
                    self.filter_query.clear();
                    // Auto-advance to albums panel
                    self.focus = FocusPanel::Albums;
                }
            }
            FocusPanel::Albums => {
                let filtered_albums = self.get_filtered_albums();
                if let Some(album) = filtered_albums.get(self.selected_album) {
                    self.load_album_songs(&album.id.clone()).await?;
                    // Clear filter when navigating
                    self.filter_query.clear();
                    // Auto-advance to songs panel
                    self.focus = FocusPanel::Songs;
                }
            }
            FocusPanel::Songs => {
                let filtered_songs = self.get_filtered_songs();
                if let Some(song) = filtered_songs.get(self.selected_song).cloned() {
                    // Add to queue and play
                    self.player.clear_queue();
                    for s in &filtered_songs {
                        self.player.enqueue(s.clone());
                    }
                    let client = self.client.clone();
                    let bitrate = self.player.max_bitrate();
                    self.player.play_from_queue(self.selected_song, |song| {
                        client.get_stream_url(&song.id, bitrate)
                    })?;

                    // Load cover art
                    if let Some(cover_id) = &song.cover_art {
                        self.load_cover_art(cover_id).await?;
                    }

                    // Load lyrics
                    let artist = song.artist.as_deref().unwrap_or("");
                    let title = &song.title;
                    self.load_lyrics(artist, title).await?;
                }
            }
            FocusPanel::Queue => {
                if let Some(song) = self.player.queue.get(self.selected_queue).cloned() {
                    let client = self.client.clone();
                    let bitrate = self.player.max_bitrate();
                    self.player.play_from_queue(self.selected_queue, |song| {
                        client.get_stream_url(&song.id, bitrate)
                    })?;

                    // Load cover art
                    if let Some(cover_id) = &song.cover_art {
                        self.load_cover_art(cover_id).await?;
                    }

                    // Load lyrics
                    let artist = song.artist.as_deref().unwrap_or("");
                    let title = &song.title;
                    self.load_lyrics(artist, title).await?;
                }
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn play_song(&mut self, song: Song) -> Result<()> {
        let url = self
            .client
            .get_stream_url(&song.id, self.player.max_bitrate());
        self.player.play_url(&url, song)?;
        Ok(())
    }

    async fn handle_search_select(&mut self) -> Result<()> {
        match self.search_focus {
            SearchFocus::Artists => {
                if let Some(artist) = self.search_artists.get(self.search_selected_artist) {
                    self.load_artist_albums(&artist.id.clone()).await?;
                    self.focus = FocusPanel::Albums;
                    self.input_mode = InputMode::Normal;
                }
            }
            SearchFocus::Albums => {
                if let Some(album) = self.search_albums.get(self.search_selected_album) {
                    self.load_album_songs(&album.id.clone()).await?;
                    self.focus = FocusPanel::Songs;
                    self.input_mode = InputMode::Normal;
                }
            }
            SearchFocus::Songs => {
                if let Some(song) = self.search_songs.get(self.search_selected_song).cloned() {
                    // Play the song directly
                    self.player.clear_queue();
                    self.player.enqueue(song.clone());
                    let client = self.client.clone();
                    let bitrate = self.player.max_bitrate();
                    self.player.play_from_queue(0, |song| {
                        client.get_stream_url(&song.id, bitrate)
                    })?;

                    // Load cover art and lyrics
                    if let Some(cover_id) = &song.cover_art {
                        self.load_cover_art(cover_id).await?;
                    }
                    let artist = song.artist.as_deref().unwrap_or("");
                    let title = &song.title;
                    self.load_lyrics(artist, title).await?;

                    self.input_mode = InputMode::Normal;
                }
            }
        }
        Ok(())
    }

    async fn handle_favorites_select(&mut self) -> Result<()> {
        match self.favorites_focus {
            FavoritesFocus::Artists => {
                if let Some(artist) = self.favorites_artists.get(self.favorites_selected_artist) {
                    self.load_artist_albums(&artist.id.clone()).await?;
                    self.focus = FocusPanel::Albums;
                    self.input_mode = InputMode::Normal;
                }
            }
            FavoritesFocus::Albums => {
                if let Some(album) = self.favorites_albums.get(self.favorites_selected_album) {
                    self.load_album_songs(&album.id.clone()).await?;
                    self.focus = FocusPanel::Songs;
                    self.input_mode = InputMode::Normal;
                }
            }
            FavoritesFocus::Songs => {
                if let Some(song) = self.favorites_songs.get(self.favorites_selected_song).cloned() {
                    // Play the song directly
                    self.player.clear_queue();
                    self.player.enqueue(song.clone());
                    let client = self.client.clone();
                    let bitrate = self.player.max_bitrate();
                    self.player.play_from_queue(0, |song| {
                        client.get_stream_url(&song.id, bitrate)
                    })?;

                    // Load cover art and lyrics
                    if let Some(cover_id) = &song.cover_art {
                        self.load_cover_art(cover_id).await?;
                    }
                    let artist = song.artist.as_deref().unwrap_or("");
                    let title = &song.title;
                    self.load_lyrics(artist, title).await?;

                    self.input_mode = InputMode::Normal;
                }
            }
        }
        Ok(())
    }

    async fn handle_unstar_favorite(&mut self) -> Result<()> {
        match self.favorites_focus {
            FavoritesFocus::Artists => {
                if let Some(artist) = self.favorites_artists.get(self.favorites_selected_artist) {
                    let name = artist.name.clone();
                    self.client.unstar(&artist.id, "artist").await?;
                    self.load_favorites().await?;
                    self.favorites_selected_artist = self.favorites_selected_artist
                        .min(self.favorites_artists.len().saturating_sub(1));
                    self.status_message = Some(format!("Unstarred artist: {}", name));
                }
            }
            FavoritesFocus::Albums => {
                if let Some(album) = self.favorites_albums.get(self.favorites_selected_album) {
                    let name = album.name.clone();
                    self.client.unstar(&album.id, "album").await?;
                    self.load_favorites().await?;
                    self.favorites_selected_album = self.favorites_selected_album
                        .min(self.favorites_albums.len().saturating_sub(1));
                    self.status_message = Some(format!("Unstarred album: {}", name));
                }
            }
            FavoritesFocus::Songs => {
                if let Some(song) = self.favorites_songs.get(self.favorites_selected_song) {
                    let title = song.title.clone();
                    self.client.unstar(&song.id, "song").await?;
                    self.load_favorites().await?;
                    self.favorites_selected_song = self.favorites_selected_song
                        .min(self.favorites_songs.len().saturating_sub(1));
                    self.status_message = Some(format!("Unstarred song: {}", title));
                }
            }
        }
        Ok(())
    }

    async fn handle_star_toggle(&mut self) -> Result<()> {
        match self.focus {
            FocusPanel::Artists => {
                let filtered_artists = self.get_filtered_artists();
                if let Some(artist) = filtered_artists.get(self.selected_artist) {
                    let name = artist.name.clone();
                    let id = artist.id.clone();
                    let saved_selection = self.selected_artist;
                    if artist.starred.is_some() {
                        self.client.unstar(&id, "artist").await?;
                        self.status_message = Some(format!("Unstarred artist: {}", name));
                    } else {
                        self.client.star(&id, "artist").await?;
                        self.status_message = Some(format!("Starred artist: {}", name));
                    }
                    // Reload artists to get updated starred status
                    self.load_artists().await?;
                    // Restore selection
                    let filtered_len = self.get_filtered_artists().len();
                    self.selected_artist = saved_selection.min(filtered_len.saturating_sub(1));
                }
            }
            FocusPanel::Albums => {
                let filtered_albums = self.get_filtered_albums();
                if let Some(album) = filtered_albums.get(self.selected_album) {
                    let name = album.name.clone();
                    let id = album.id.clone();
                    let saved_selection = self.selected_album;
                    if album.starred.is_some() {
                        self.client.unstar(&id, "album").await?;
                        self.status_message = Some(format!("Unstarred album: {}", name));
                    } else {
                        self.client.star(&id, "album").await?;
                        self.status_message = Some(format!("Starred album: {}", name));
                    }
                    // Reload current artist's albums to get updated starred status
                    if let Some(artist) = &self.current_artist {
                        self.load_artist_albums(&artist.id.clone()).await?;
                        // Restore selection
                        let filtered_len = self.get_filtered_albums().len();
                        self.selected_album = saved_selection.min(filtered_len.saturating_sub(1));
                    }
                }
            }
            FocusPanel::Songs => {
                let filtered_songs = self.get_filtered_songs();
                if let Some(song) = filtered_songs.get(self.selected_song) {
                    let title = song.title.clone();
                    let id = song.id.clone();
                    let saved_selection = self.selected_song;
                    if song.starred.is_some() {
                        self.client.unstar(&id, "song").await?;
                        self.status_message = Some(format!("Unstarred song: {}", title));
                    } else {
                        self.client.star(&id, "song").await?;
                        self.status_message = Some(format!("Starred song: {}", title));
                    }
                    // Reload current album's songs to get updated starred status
                    if let Some(album) = &self.current_album {
                        self.load_album_songs(&album.id.clone()).await?;
                        // Restore selection
                        let filtered_len = self.get_filtered_songs().len();
                        self.selected_song = saved_selection.min(filtered_len.saturating_sub(1));
                    }
                }
            }
            FocusPanel::Queue => {
                if let Some(song) = self.player.queue.get(self.selected_queue) {
                    let title = song.title.clone();
                    if song.starred.is_some() {
                        self.client.unstar(&song.id, "song").await?;
                        self.status_message = Some(format!("Unstarred song: {}", title));
                    } else {
                        self.client.star(&song.id, "song").await?;
                        self.status_message = Some(format!("Starred song: {}", title));
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn load_cover_art(&mut self, cover_id: &str) -> Result<()> {
        let url = self.client.get_cover_art_url(cover_id, Some(300));
        match reqwest::get(&url).await {
            Ok(response) => {
                if let Ok(bytes) = response.bytes().await {
                    self.cover_art_data = Some(bytes.to_vec());

                    // Pre-process image for display
                    if let Ok(img) = image::ImageReader::new(std::io::Cursor::new(&self.cover_art_data.as_ref().unwrap()))
                        .with_guessed_format()
                        .and_then(|r| r.decode().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)))
                    {
                        let mut picker = Picker::from_termios().unwrap_or_else(|_| Picker::new((8, 16)));
                        picker.guess_protocol();
                        let protocol = picker.new_resize_protocol(img);
                        self.cover_art_protocol = Some(protocol);
                    } else {
                        self.cover_art_protocol = None;
                    }
                }
            }
            Err(_) => {
                self.cover_art_data = None;
                self.cover_art_protocol = None;
            }
        }
        Ok(())
    }

    pub async fn load_lyrics(&mut self, artist: &str, title: &str) -> Result<()> {
        // First try server lyrics
        match self.client.get_lyrics(artist, title).await {
            Ok(Some(lyrics)) => {
                self.lyrics_lines = lyrics.lines().map(|s| s.to_string()).collect();
                self.lyrics_timestamps = Vec::new(); // No timestamps from server
                self.lyrics = Some(lyrics);
                return Ok(());
            }
            _ => {}
        }

        // Fallback to LRCLIB for synced lyrics
        self.fetch_lrclib_lyrics(artist, title).await
    }

    async fn fetch_lrclib_lyrics(&mut self, artist: &str, title: &str) -> Result<()> {
        let url = format!(
            "https://lrclib.net/api/get?artist_name={}&track_name={}",
            urlencoding::encode(artist),
            urlencoding::encode(title)
        );

        let response = match reqwest::get(&url).await {
            Ok(r) => r,
            Err(_) => {
                self.clear_lyrics();
                return Ok(());
            }
        };

        if !response.status().is_success() {
            self.clear_lyrics();
            return Ok(());
        }

        #[derive(serde::Deserialize)]
        struct LrcLibResponse {
            #[serde(rename = "syncedLyrics")]
            synced_lyrics: Option<String>,
            #[serde(rename = "plainLyrics")]
            plain_lyrics: Option<String>,
        }

        let data: LrcLibResponse = match response.json().await {
            Ok(d) => d,
            Err(_) => {
                self.clear_lyrics();
                return Ok(());
            }
        };

        // Prefer synced lyrics
        if let Some(synced) = data.synced_lyrics {
            self.parse_lrc(&synced);
        } else if let Some(plain) = data.plain_lyrics {
            self.lyrics_lines = plain.lines().map(|s| s.to_string()).collect();
            self.lyrics_timestamps = Vec::new();
            self.lyrics = Some(plain);
        } else {
            self.clear_lyrics();
        }

        Ok(())
    }

    fn parse_lrc(&mut self, lrc: &str) {
        self.lyrics_lines = Vec::new();
        self.lyrics_timestamps = Vec::new();

        for line in lrc.lines() {
            // Parse LRC format: [mm:ss.xx] lyrics text
            if let Some(rest) = line.strip_prefix('[') {
                if let Some(bracket_end) = rest.find(']') {
                    let timestamp_str = &rest[..bracket_end];
                    let text = rest[bracket_end + 1..].trim();

                    // Parse timestamp mm:ss.xx
                    if let Some(timestamp) = self.parse_lrc_timestamp(timestamp_str) {
                        self.lyrics_timestamps.push(timestamp);
                        self.lyrics_lines.push(text.to_string());
                    }
                }
            }
        }

        self.lyrics = Some(lrc.to_string());
    }

    fn parse_lrc_timestamp(&self, s: &str) -> Option<f64> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() == 2 {
            let minutes: f64 = parts[0].parse().ok()?;
            let seconds: f64 = parts[1].parse().ok()?;
            Some(minutes * 60.0 + seconds)
        } else {
            None
        }
    }

    fn clear_lyrics(&mut self) {
        self.lyrics = None;
        self.lyrics_lines = Vec::new();
        self.lyrics_timestamps = Vec::new();
    }

    /// Calculate which line of lyrics should be highlighted based on song progress
    pub fn current_lyrics_line(&self) -> usize {
        if self.lyrics_lines.is_empty() {
            return 0;
        }

        let position = self.player.position();

        // If we have timestamps, use them for accurate sync
        if !self.lyrics_timestamps.is_empty() {
            for (i, &timestamp) in self.lyrics_timestamps.iter().enumerate().rev() {
                if position >= timestamp {
                    return i;
                }
            }
            return 0;
        }

        // Fallback to percentage-based estimation
        let duration = self.player.duration();
        if duration <= 0.0 {
            return 0;
        }

        let progress = position / duration;
        let line = (progress * self.lyrics_lines.len() as f64) as usize;
        line.min(self.lyrics_lines.len().saturating_sub(1))
    }

    async fn enqueue_selected(&mut self) -> Result<()> {
        match self.focus {
            FocusPanel::Songs => {
                let filtered_songs = self.get_filtered_songs();
                if let Some(song) = filtered_songs.get(self.selected_song) {
                    self.player.enqueue(song.clone());
                    self.status_message = Some(format!("Added \"{}\" to queue", song.title));
                }
            }
            FocusPanel::Queue => {
                // Maybe remove from queue?
            }
            _ => {}
        }
        Ok(())
    }

    async fn enqueue_next_selected(&mut self) -> Result<()> {
        if let FocusPanel::Songs = self.focus {
            let filtered_songs = self.get_filtered_songs();
            if let Some(song) = filtered_songs.get(self.selected_song) {
                self.player.enqueue_next(song.clone());
                self.status_message = Some(format!("\"{}\" will play next", song.title));
            }
        }
        Ok(())
    }

    async fn enqueue_album(&mut self) -> Result<()> {
        let filtered_songs = self.get_filtered_songs();
        if !filtered_songs.is_empty() {
            for song in &filtered_songs {
                self.player.enqueue(song.clone());
            }
            self.status_message = Some(format!("Added {} songs to queue", filtered_songs.len()));
        }
        Ok(())
    }

    // Queue management
    fn clear_queue(&mut self) {
        self.player.clear_queue();
        self.selected_queue = 0;
        self.status_message = Some("Queue cleared".to_string());
    }

    fn remove_from_queue(&mut self) {
        if let Some(removed) = self.player.remove_from_queue(self.selected_queue) {
            self.status_message = Some(format!("Removed \"{}\" from queue", removed.title));
            // Adjust selection
            if self.selected_queue >= self.player.queue.len() && self.selected_queue > 0 {
                self.selected_queue -= 1;
            }
        }
    }

    fn move_queue_item_up(&mut self) {
        if self.player.move_queue_item_up(self.selected_queue) {
            if self.selected_queue > 0 {
                self.selected_queue -= 1;
            }
            self.status_message = Some("Moved up".to_string());
        }
    }

    fn move_queue_item_down(&mut self) {
        if self.player.move_queue_item_down(self.selected_queue) {
            if self.selected_queue < self.player.queue.len() - 1 {
                self.selected_queue += 1;
            }
            self.status_message = Some("Moved down".to_string());
        }
    }

    async fn save_queue_as_playlist(&mut self) -> Result<()> {
        if self.player.queue.is_empty() {
            self.status_message = Some("Queue is empty".to_string());
            return Ok(());
        }

        // Generate a playlist name based on current time
        use chrono::Local;
        let now = Local::now();
        let playlist_name = format!("Queue {}", now.format("%Y-%m-%d %H:%M"));

        // Create the playlist
        self.client.create_playlist(&playlist_name).await?;

        // Reload playlists to get the new one
        self.load_playlists().await?;

        // Find the new playlist and add all songs from queue
        if let Some(playlist) = self.playlists.iter().find(|p| p.name == playlist_name) {
            let song_ids: Vec<String> = self.player.queue.iter().map(|s| s.id.clone()).collect();
            self.client.update_playlist(&playlist.id, &song_ids).await?;
            self.status_message = Some(format!("Saved {} songs as \"{}\"", song_ids.len(), playlist_name));
        }

        Ok(())
    }

    fn toggle_shuffle(&mut self) {
        self.shuffle = !self.shuffle;
        if self.shuffle {
            self.shuffle_queue();
            self.status_message = Some("Shuffle enabled".to_string());
        } else {
            self.status_message = Some("Shuffle disabled".to_string());
        }
    }

    fn cycle_repeat(&mut self) {
        self.repeat = match self.repeat {
            RepeatMode::Off => RepeatMode::All,
            RepeatMode::All => RepeatMode::One,
            RepeatMode::One => RepeatMode::Off,
        };
        let mode_str = match self.repeat {
            RepeatMode::Off => "off",
            RepeatMode::All => "all",
            RepeatMode::One => "one",
        };
        self.status_message = Some(format!("Repeat: {}", mode_str));
    }

    async fn start_radio(&mut self, song_id: String) -> Result<()> {
        // Fetch similar songs
        let similar_songs = self.client.get_similar_songs(&song_id, 50).await?;

        if similar_songs.is_empty() {
            self.status_message = Some("No similar songs found".to_string());
            return Ok(());
        }

        // Clear queue and add similar songs
        self.player.clear_queue();
        for song in &similar_songs {
            self.player.enqueue(song.clone());
        }

        // Start playing first song
        if !similar_songs.is_empty() {
            let client = self.client.clone();
            let bitrate = self.player.max_bitrate();
            self.player.play_from_queue(0, |song| {
                client.get_stream_url(&song.id, bitrate)
            })?;

            // Load cover art and lyrics
            if let Some(song) = self.player.current_song.clone() {
                if let Some(cover_id) = &song.cover_art {
                    self.load_cover_art(cover_id).await?;
                }
                let artist = song.artist.as_deref().unwrap_or("");
                self.load_lyrics(artist, &song.title).await?;
            }
        }

        // Enable radio mode
        self.radio_mode = true;
        self.radio_seed_song_id = Some(song_id);
        self.status_message = Some(format!("Radio started ({} songs)", similar_songs.len()));

        Ok(())
    }

    async fn stop_radio(&mut self) -> Result<()> {
        self.radio_mode = false;
        self.radio_seed_song_id = None;

        // Clone the current song data to avoid borrow checker issues
        let current_song = self.player.current_song.clone();

        // If a song is currently playing, reload its album to the queue
        if let Some(ref song) = current_song {
            if let Some(ref album_id) = song.album_id {
                let song_id = song.id.clone();

                // Load the album
                let album = self.client.get_album(album_id).await?;
                let album_name = album.name.clone();

                // Clear queue and add album songs
                self.player.clear_queue();
                for album_song in &album.songs {
                    self.player.enqueue(album_song.clone());
                }

                // Find the current song's position in the album
                if let Some(pos) = album.songs.iter().position(|s| s.id == song_id) {
                    self.player.queue_position = pos;
                }

                self.status_message = Some(format!("Radio stopped - playing album \"{}\"", album_name));
            } else {
                // No album info, just clear queue
                self.player.clear_queue();
                self.status_message = Some("Radio stopped - queue cleared".to_string());
            }
        } else {
            // No song playing, just clear queue
            self.player.clear_queue();
            self.status_message = Some("Radio stopped".to_string());
        }

        Ok(())
    }

    async fn refill_radio_queue(&mut self) -> Result<()> {
        // Only refill if in radio mode and queue is getting low
        if !self.radio_mode || self.player.queue.len() > 10 {
            return Ok(());
        }

        // Get the seed song ID or use current song
        let seed_id = if let Some(ref seed) = self.radio_seed_song_id {
            seed.clone()
        } else if let Some(ref song) = self.player.current_song {
            song.id.clone()
        } else {
            return Ok(());
        };

        // Fetch more similar songs
        let similar_songs = self.client.get_similar_songs(&seed_id, 25).await?;

        // Add to queue
        for song in &similar_songs {
            self.player.enqueue(song.clone());
        }

        if !similar_songs.is_empty() {
            tracing::debug!("Refilled radio queue with {} songs", similar_songs.len());
        }

        Ok(())
    }

    pub async fn tick(&mut self) -> Result<()> {
        // Update Discord Rich Presence
        self.update_discord_presence();

        // Track new songs in history
        if let Some(song) = self.player.current_song.clone() {
            if self.last_history_song_id.as_ref() != Some(&song.id) {
                let song_id = song.id.clone();
                self.add_to_history(song);
                self.last_history_song_id = Some(song_id);
            }
        } else {
            self.last_history_song_id = None;
        }

        // Scrobble near end of song (90%) - only once per song
        if let Some(song) = &self.player.current_song {
            let already_scrobbled = self.scrobbled_song_id.as_ref() == Some(&song.id);
            if !already_scrobbled {
                let position = self.player.position();
                let duration = self.player.duration();
                if duration > 0.0 && position > duration * 0.9 {
                    let _ = self.client.scrobble(&song.id, true).await;
                    self.scrobbled_song_id = Some(song.id.clone());
                }
            }
        } else {
            // Clear scrobble tracking when no song playing
            self.scrobbled_song_id = None;
        }

        // Check if current song ended
        if self.player.current_song.is_some() && self.player.is_idle() {
            match self.repeat {
                RepeatMode::One => {
                    // Replay current song
                    if let Some(song) = self.player.queue.get(self.player.queue_position).cloned() {
                        let client = self.client.clone();
                        let bitrate = self.player.max_bitrate();
                        let url = client.get_stream_url(&song.id, bitrate);
                        self.player.play_url(&url, song.clone())?;

                        // Load cover art and lyrics for replayed song
                        if let Some(cover_id) = &song.cover_art {
                            self.load_cover_art(cover_id).await?;
                        }
                        let artist = song.artist.as_deref().unwrap_or("");
                        self.load_lyrics(artist, &song.title).await?;
                    }
                }
                RepeatMode::All => {
                    let client = self.client.clone();
                    let bitrate = self.player.max_bitrate();
                    let played = self.player.next(|song| client.get_stream_url(&song.id, bitrate))?;
                    if !played && !self.player.queue.is_empty() {
                        // Wrap around to beginning
                        self.player.play_from_queue(0, |song| {
                            client.get_stream_url(&song.id, bitrate)
                        })?;

                        // Load cover art and lyrics for wrapped song
                        if let Some(song) = self.player.current_song.clone() {
                            if let Some(cover_id) = &song.cover_art {
                                self.load_cover_art(cover_id).await?;
                            }
                            let artist = song.artist.as_deref().unwrap_or("");
                            self.load_lyrics(artist, &song.title).await?;
                        }
                    } else if played {
                        // Load cover art and lyrics for next song
                        if let Some(song) = self.player.current_song.clone() {
                            if let Some(cover_id) = &song.cover_art {
                                self.load_cover_art(cover_id).await?;
                            }
                            let artist = song.artist.as_deref().unwrap_or("");
                            self.load_lyrics(artist, &song.title).await?;
                        }
                    }
                }
                RepeatMode::Off => {
                    let client = self.client.clone();
                    let bitrate = self.player.max_bitrate();
                    let played = self.player.next(|song| client.get_stream_url(&song.id, bitrate))?;

                    // Load cover art and lyrics for next song
                    if played {
                        if let Some(song) = self.player.current_song.clone() {
                            if let Some(cover_id) = &song.cover_art {
                                self.load_cover_art(cover_id).await?;
                            }
                            let artist = song.artist.as_deref().unwrap_or("");
                            self.load_lyrics(artist, &song.title).await?;
                        }
                    }
                }
            }
        }

        // Refill radio queue if needed
        self.refill_radio_queue().await?;

        // Update visualizer
        self.update_visualizer();

        Ok(())
    }

    fn update_visualizer(&mut self) {
        self.visualizer_tick = self.visualizer_tick.wrapping_add(1);

        let is_playing = self.player.current_song.is_some() && !self.player.is_paused();

        if is_playing {
            // Use playback position to vary the animation
            let position = self.player.position();
            let duration = self.player.duration().max(1.0);
            let progress = (position / duration).min(1.0);

            // Animate bars when playing with variation based on position
            for (i, bar) in self.visualizer_bars.iter_mut().enumerate() {
                // Multiple overlapping waves with different speeds
                let time = self.visualizer_tick as f32 * 0.15;
                let pos_offset = position as f32 * 0.5; // Use actual playback position

                // Create multiple frequency components
                let freq1 = 0.2 + (i as f32 * 0.03);
                let freq2 = 0.4 + ((31 - i) as f32 * 0.02);
                let freq3 = 0.1 + (i as f32 * 0.08);

                let wave1 = ((time + pos_offset) * freq1).sin().abs();
                let wave2 = ((time + pos_offset * 1.5) * freq2).cos().abs();
                let wave3 = ((time + pos_offset * 0.7) * freq3).sin().abs();

                // Combine waves with varying weights based on progress
                let energy_boost = ((progress as f32) * 6.28).sin().abs() * 0.3 + 0.7;
                let combined = (wave1 * 0.4 + wave2 * 0.35 + wave3 * 0.25) * energy_boost;

                // Add position-based randomness that changes over time
                use std::collections::hash_map::RandomState;
                use std::hash::{BuildHasher, Hash, Hasher};
                let mut hasher = RandomState::new().build_hasher();
                ((position as u64 / 2) + i as u64).hash(&mut hasher);
                let random_factor = (hasher.finish() % 100) as f32 / 100.0;

                *bar = (combined * 0.75 + random_factor * 0.25).min(1.0);
            }
        } else {
            // Decay bars when paused
            for bar in self.visualizer_bars.iter_mut() {
                *bar *= 0.85; // Decay factor
                if *bar < 0.01 {
                    *bar = 0.0;
                }
            }
        }
    }

    fn update_discord_presence(&mut self) {
        if let Some(ref mut discord) = self.discord {
            // Try to reconnect if disconnected
            let _ = discord.reconnect();

            if let Some(song) = &self.player.current_song {
                let is_paused = self.player.is_paused();

                // Track pause duration
                if is_paused {
                    // If just paused, record the time
                    if self.pause_started_at.is_none() {
                        self.pause_started_at = Some(Instant::now());
                    }

                    // Check if pause timeout has been exceeded
                    if let Some(pause_start) = self.pause_started_at {
                        let pause_duration = pause_start.elapsed();
                        let timeout = std::time::Duration::from_secs(
                            self.config.discord.pause_timeout_minutes * 60
                        );

                        if pause_duration >= timeout {
                            // Clear presence after timeout
                            if self.last_discord_update.is_some() {
                                let _ = discord.clear();
                                self.last_discord_update = None;
                            }
                            return;
                        }
                    }
                } else {
                    // Clear pause tracking when playing
                    self.pause_started_at = None;
                }

                // Only update if song changed or pause state changed
                let song_key = format!("{}:{}", song.id, is_paused);
                if self.last_discord_update.as_ref() != Some(&song_key) {
                    if let Err(e) = discord.update_song(song, is_paused) {
                        tracing::warn!("Failed to update Discord presence: {}", e);
                    }
                    self.last_discord_update = Some(song_key);
                }
            } else {
                // No song playing, clear presence and pause tracking
                self.pause_started_at = None;
                if self.last_discord_update.is_some() {
                    let _ = discord.clear();
                    self.last_discord_update = None;
                }
            }
        }
    }
}
