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
}

impl Player {
    pub fn new(volume: i32, max_bitrate: Option<i32>) -> Result<Self> {
        let mpv = Mpv::new().map_err(|e| anyhow!("Failed to create mpv: {:?}", e))?;

        // Configure mpv
        mpv_err(mpv.set_property("vo", "null"))?;
        mpv_err(mpv.set_property("video", "no"))?;
        mpv_err(mpv.set_property("volume", volume as i64))?;
        mpv_err(mpv.set_property("gapless-audio", "yes"))?;

        Ok(Self {
            mpv,
            current_song: None,
            queue: Vec::new(),
            queue_position: 0,
            volume,
            max_bitrate,
        })
    }

    pub fn play_url(&mut self, url: &str, song: Song) -> Result<()> {
        mpv_err(self.mpv.command("loadfile", &[url, "replace"]))?;
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

    pub fn max_bitrate(&self) -> Option<i32> {
        self.max_bitrate
    }
}
