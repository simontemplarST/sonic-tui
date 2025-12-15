use anyhow::Result;
use std::path::PathBuf;

/// Cache manager for storing library metadata and downloaded songs (for future offline mode)
#[allow(dead_code)]
pub struct Cache {
    directory: PathBuf,
    enabled: bool,
}

#[allow(dead_code)]
impl Cache {
    pub fn new(directory: PathBuf, enabled: bool) -> Result<Self> {
        if enabled {
            std::fs::create_dir_all(&directory)?;
        }

        Ok(Self { directory, enabled })
    }

    /// Get the path for a cached cover art image
    pub fn cover_art_path(&self, id: &str) -> PathBuf {
        self.directory.join("covers").join(format!("{id}.jpg"))
    }

    /// Get the path for a cached song
    pub fn song_path(&self, id: &str, suffix: &str) -> PathBuf {
        self.directory
            .join("songs")
            .join(format!("{id}.{suffix}"))
    }

    /// Check if a song is cached
    pub fn is_song_cached(&self, id: &str, suffix: &str) -> bool {
        self.song_path(id, suffix).exists()
    }

    /// Get the database path for library metadata
    pub fn database_path(&self) -> PathBuf {
        self.directory.join("library.json")
    }

    /// Clear all cached data
    pub fn clear(&self) -> Result<()> {
        if self.directory.exists() {
            std::fs::remove_dir_all(&self.directory)?;
            std::fs::create_dir_all(&self.directory)?;
        }
        Ok(())
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}
