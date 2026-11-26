use anyhow::{anyhow, Result};
use libmpv2::Mpv;

use crate::api::Song;

// Helper to convert libmpv errors to anyhow
fn mpv_err<T>(result: Result<T, libmpv2::Error>) -> Result<T> {
    result.map_err(|e| anyhow!("mpv error: {:?}", e))
}

pub struct Player {
    mpv: Mpv,
    pub current_song: Option<Song>,
    pub queue: Vec<Song>,
    pub queue_position: usize,
    pub volume: i32,
    max_bitrate: Option<i32>,
    // Gapless: the queue entry (index, song) appended to mpv's playlist after the current track.
    gapless: bool,
    preloaded: Option<(usize, Song)>,
}

impl Player {
    pub fn new(volume: i32, max_bitrate: Option<i32>, gapless: bool) -> Result<Self> {
        let mpv = Mpv::new().map_err(|e| anyhow!("Failed to create mpv: {:?}", e))?;

        // Configure mpv
        mpv_err(mpv.set_property("vo", "null"))?;
        mpv_err(mpv.set_property("video", "no"))?;
        mpv_err(mpv.set_property("volume", volume as i64))?;
        mpv_err(mpv.set_property("gapless-audio", if gapless { "yes" } else { "no" }))?;
        // Open the next playlist entry (an HTTP stream) before the current one ends.
        mpv_err(mpv.set_property("prefetch-playlist", if gapless { "yes" } else { "no" }))?;

        Ok(Self {
            mpv,
            current_song: None,
            queue: Vec::new(),
            queue_position: 0,
            volume,
            max_bitrate,
            gapless,
            preloaded: None,
        })
    }

    pub fn play_url(&mut self, url: &str, song: Song) -> Result<()> {
        mpv_err(self.mpv.command("loadfile", &[url, "replace"]))?;
        self.preloaded = None; // replace leaves mpv with a one-entry playlist
        self.current_song = Some(song);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn play(&self) -> Result<()> {
        mpv_err(self.mpv.set_property("pause", false))
    }

    #[allow(dead_code)]
    pub fn pause(&self) -> Result<()> {
        mpv_err(self.mpv.set_property("pause", true))
    }

    pub fn toggle_pause(&self) -> Result<()> {
        let paused: bool = mpv_err(self.mpv.get_property("pause"))?;
        mpv_err(self.mpv.set_property("pause", !paused))
    }

    #[allow(dead_code)]
    pub fn stop(&mut self) -> Result<()> {
        mpv_err(self.mpv.command("stop", &[]))?;
        self.preloaded = None;
        self.current_song = None;
        Ok(())
    }

    pub fn seek(&self, seconds: i64) -> Result<()> {
        mpv_err(self.mpv.command("seek", &[&seconds.to_string(), "relative"]))
    }

    pub fn seek_absolute(&self, seconds: f64) -> Result<()> {
        mpv_err(self.mpv.command("seek", &[&seconds.to_string(), "absolute"]))
    }

    pub fn set_volume(&mut self, volume: i32) -> Result<()> {
        let volume = volume.clamp(0, 100);
        self.volume = volume;
        mpv_err(self.mpv.set_property("volume", volume as i64))
    }

    pub fn volume_up(&mut self) -> Result<()> {
        self.set_volume(self.volume + 5)
    }

    pub fn volume_down(&mut self) -> Result<()> {
        self.set_volume(self.volume - 5)
    }

    pub fn is_paused(&self) -> bool {
        self.mpv.get_property("pause").unwrap_or(true)
    }

    pub fn position(&self) -> f64 {
        self.mpv.get_property("time-pos").unwrap_or(0.0)
    }

    pub fn duration(&self) -> f64 {
        self.mpv.get_property("duration").unwrap_or(0.0)
    }

    pub fn is_idle(&self) -> bool {
        self.mpv.get_property("idle-active").unwrap_or(true)
    }

    // Queue management
    pub fn enqueue(&mut self, song: Song) {
        self.queue.push(song);
    }

    pub fn enqueue_next(&mut self, song: Song) {
        let insert_pos = if self.queue.is_empty() {
            0
        } else {
            (self.queue_position + 1).min(self.queue.len())
        };
        self.queue.insert(insert_pos, song);
    }

    pub fn clear_queue(&mut self) {
        self.queue.clear();
        self.queue_position = 0;
    }

    pub fn remove_from_queue(&mut self, index: usize) -> Option<Song> {
        if index < self.queue.len() {
            let removed = self.queue.remove(index);
            // Adjust queue position if necessary
            if index < self.queue_position {
                self.queue_position = self.queue_position.saturating_sub(1);
            } else if index == self.queue_position && index >= self.queue.len() {
                self.queue_position = self.queue.len().saturating_sub(1);
            }
            Some(removed)
        } else {
            None
        }
    }

    pub fn move_queue_item_up(&mut self, index: usize) -> bool {
        if index > 0 && index < self.queue.len() {
            self.queue.swap(index, index - 1);
            // Adjust queue position if it was one of the swapped items
            if self.queue_position == index {
                self.queue_position = index - 1;
            } else if self.queue_position == index - 1 {
                self.queue_position = index;
            }
            true
        } else {
            false
        }
    }

    pub fn move_queue_item_down(&mut self, index: usize) -> bool {
        if index < self.queue.len().saturating_sub(1) {
            self.queue.swap(index, index + 1);
            // Adjust queue position if it was one of the swapped items
            if self.queue_position == index {
                self.queue_position = index + 1;
            } else if self.queue_position == index + 1 {
                self.queue_position = index;
            }
            true
        } else {
            false
        }
    }

    pub fn play_from_queue(&mut self, index: usize, get_url: impl Fn(&Song) -> String) -> Result<()> {
        if index < self.queue.len() {
            self.queue_position = index;
            let song = self.queue[index].clone();
            let url = get_url(&song);
            self.play_url(&url, song)?;
        }
        Ok(())
    }

    pub fn next(&mut self, get_url: impl Fn(&Song) -> String) -> Result<bool> {
        if self.queue_position + 1 < self.queue.len() {
            self.queue_position += 1;
            let song = self.queue[self.queue_position].clone();
            let url = get_url(&song);
            self.play_url(&url, song)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn previous(&mut self, get_url: impl Fn(&Song) -> String) -> Result<bool> {
        // If more than 3 seconds into song, restart it
        if self.position() > 3.0 {
            self.seek_absolute(0.0)?;
            return Ok(true);
        }

        if self.queue_position > 0 {
            self.queue_position -= 1;
            let song = self.queue[self.queue_position].clone();
            let url = get_url(&song);
            self.play_url(&url, song)?;
            Ok(true)
        } else {
            self.seek_absolute(0.0)?;
            Ok(false)
        }
    }

    pub fn gapless(&self) -> bool {
        self.gapless
    }

    /// Keep mpv's playlist as [current, next] so mpv plays straight into the next track.
    /// `next` is (queue index, song, stream url), or None when nothing should follow.
    pub fn sync_preload(&mut self, next: Option<(usize, Song, String)>) {
        let same = match (&self.preloaded, &next) {
            (Some((_, a)), Some((_, b, _))) => a.id == b.id,
            (None, None) => true,
            _ => false,
        };
        if same {
            if let (Some(p), Some((i, _, _))) = (self.preloaded.as_mut(), next.as_ref()) {
                p.0 = *i; // queue edits can move the same song to another index
            }
            return;
        }
        if self.preloaded.take().is_some() {
            let _ = self.mpv.command("playlist-remove", &["1"]);
        }
        if let Some((i, song, url)) = next {
            if self.mpv.command("loadfile", &[&url, "append"]).is_ok() {
                self.preloaded = Some((i, song));
            }
        }
    }

    /// True when mpv has moved on to the preloaded track; current_song/queue_position then follow it.
    pub fn take_advance(&mut self) -> bool {
        let pos: i64 = self.mpv.get_property("playlist-pos").unwrap_or(-1);
        if pos >= 1 {
            if let Some((i, song)) = self.preloaded.take() {
                let _ = self.mpv.command("playlist-remove", &["0"]);
                self.queue_position = i;
                self.current_song = Some(song);
                return true;
            }
        }
        false
    }

    pub fn max_bitrate(&self) -> Option<i32> {
        self.max_bitrate
    }
}
