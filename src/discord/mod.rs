use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::api::Song;

pub struct DiscordPresence {
    client: DiscordIpcClient,
    connected: bool,
}

impl DiscordPresence {
    pub fn new(application_id: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut client = DiscordIpcClient::new(application_id)?;

        // Try to connect, but don't fail if Discord isn't running
        let connected = match client.connect() {
            Ok(_) => {
                tracing::info!("Connected to Discord Rich Presence");
                true
            }
            Err(e) => {
                tracing::warn!("Failed to connect to Discord: {}", e);
                false
            }
        };

        Ok(Self { client, connected })
    }

    pub fn update_song(&mut self, song: &Song, paused: bool) -> Result<(), Box<dyn std::error::Error>> {
        if !self.connected {
            return Ok(());
        }

        // Details is the top line - show song title prominently
        let details = if paused {
            format!("⏸ {}", song.title)
        } else {
            format!("🎵 {}", song.title)
        };

        // State is the second line - show artist
        let state = song.artist.as_deref().unwrap_or("Unknown Artist").to_string();

        let large_text = song.album.as_deref().unwrap_or("Unknown Album");

        let start_timestamp = if !paused {
            Some(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64)
        } else {
            None
        };

        let mut activity_builder = activity::Activity::new()
            .details(&details)
            .state(&state)
            .assets(
                activity::Assets::new()
                    .large_image("music") // You can upload an icon in Discord Developer Portal
                    .large_text(large_text)
            );

        if let Some(timestamp) = start_timestamp {
            activity_builder = activity_builder.timestamps(
                activity::Timestamps::new().start(timestamp)
            );
        }

        match self.client.set_activity(activity_builder) {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::warn!("Failed to update Discord presence: {}", e);
                self.connected = false;
                Ok(())
            }
        }
    }

    pub fn clear(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.connected {
            return Ok(());
        }

        match self.client.clear_activity() {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::warn!("Failed to clear Discord presence: {}", e);
                self.connected = false;
                Ok(())
            }
        }
    }

    pub fn reconnect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.connected {
            return Ok(());
        }

        match self.client.connect() {
            Ok(_) => {
                tracing::info!("Reconnected to Discord Rich Presence");
                self.connected = true;
                Ok(())
            }
            Err(e) => {
                tracing::debug!("Failed to reconnect to Discord: {}", e);
                Ok(())
            }
        }
    }
}

impl Drop for DiscordPresence {
    fn drop(&mut self) {
        if self.connected {
            let _ = self.client.close();
        }
    }
}
