use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    #[serde(default)]
    pub audio: AudioConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub discord: DiscordConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub url: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    #[serde(default = "default_volume")]
    pub volume: i32,
    #[serde(default)]
    pub max_bitrate: Option<i32>,
    #[serde(default)]
    pub gapless: bool,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            volume: 100,
            max_bitrate: None,
            gapless: true,
        }
    }
}

fn default_volume() -> i32 {
    100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_true")]
    pub show_cover_art: bool,
    #[serde(default)]
    pub color_from_cover: bool,
    #[serde(default)]
    pub accent_color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub application_id: Option<String>,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_cover_art: true,
            color_from_cover: false,
            accent_color: None,
        }
    }
}

fn default_true() -> bool {
    true
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            application_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    #[serde(default = "default_cache_dir")]
    pub directory: PathBuf,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            directory: default_cache_dir(),
            enabled: true,
        }
    }
}

fn default_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sonic-tui")
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            return Err(anyhow!(
                "Config file not found at {}. Please create one using config.example.yaml as a template.",
                config_path.display()
            ));
        }

        let content = std::fs::read_to_string(&config_path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow!("Could not find config directory"))?
            .join("sonic-tui");

        std::fs::create_dir_all(&config_dir)?;
        Ok(config_dir.join("config.yaml"))
    }

    #[allow(dead_code)]
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        let content = serde_yaml::to_string(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }
}
