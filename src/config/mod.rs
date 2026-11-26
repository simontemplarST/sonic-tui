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
    #[serde(default)]
    pub keybindings: KeybindingsConfig,
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
    #[serde(default = "default_true")]
    pub show_visualizer: bool,
    /// Cover art protocol: "auto" (default), "sixel", "kitty", "iterm2" or "halfblocks".
    /// The SONIC_TUI_IMAGE_PROTOCOL environment variable overrides it.
    #[serde(default)]
    pub image_protocol: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub application_id: Option<String>,
    #[serde(default = "default_pause_timeout")]
    pub pause_timeout_minutes: u64,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_cover_art: true,
            color_from_cover: false,
            accent_color: None,
            show_visualizer: true,
            image_protocol: None,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_pause_timeout() -> u64 {
    5
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            application_id: None,
            pause_timeout_minutes: 5,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingsConfig {
    // Navigation
    #[serde(default = "default_key_quit")]
    pub quit: String,
    #[serde(default = "default_key_help")]
    pub help: String,
    #[serde(default = "default_key_search")]
    pub search: String,
    #[serde(default = "default_key_filter")]
    pub filter: String,
    #[serde(default = "default_key_focus_left")]
    pub focus_left: String,
    #[serde(default = "default_key_focus_right")]
    pub focus_right: String,

    // Playback
    #[serde(default = "default_key_play_pause")]
    pub play_pause: String,
    #[serde(default = "default_key_stop")]
    pub stop: String,
    #[serde(default = "default_key_next")]
    pub next: String,
    #[serde(default = "default_key_previous")]
    pub previous: String,
    #[serde(default = "default_key_seek_forward")]
    pub seek_forward: String,
    #[serde(default = "default_key_seek_backward")]
    pub seek_backward: String,
    #[serde(default = "default_key_volume_up")]
    pub volume_up: String,
    #[serde(default = "default_key_volume_down")]
    pub volume_down: String,
    #[serde(default = "default_key_shuffle")]
    pub shuffle: String,
    #[serde(default = "default_key_repeat")]
    pub repeat: String,

    // Queue
    #[serde(default = "default_key_enqueue")]
    pub enqueue: String,
    #[serde(default = "default_key_enqueue_next")]
    pub enqueue_next: String,
    #[serde(default = "default_key_enqueue_album")]
    pub enqueue_album: String,
    #[serde(default = "default_key_clear_queue")]
    pub clear_queue: String,
    #[serde(default = "default_key_remove_from_queue")]
    pub remove_from_queue: String,

    // Features
    #[serde(default = "default_key_star")]
    pub star: String,
    #[serde(default = "default_key_lyrics")]
    pub lyrics: String,
    #[serde(default = "default_key_add_to_playlist")]
    pub add_to_playlist: String,
    #[serde(default = "default_key_now_playing")]
    pub now_playing: String,
    #[serde(default = "default_key_favorites")]
    pub favorites: String,
    #[serde(default = "default_key_history")]
    pub history: String,
    #[serde(default = "default_key_statistics")]
    pub statistics: String,
    #[serde(default = "default_key_radio")]
    pub radio: String,

    // Sorting
    #[serde(default = "default_key_sort")]
    pub sort: String,
}

impl Default for KeybindingsConfig {
    fn default() -> Self {
        Self {
            quit: default_key_quit(),
            help: default_key_help(),
            search: default_key_search(),
            filter: default_key_filter(),
            focus_left: default_key_focus_left(),
            focus_right: default_key_focus_right(),
            play_pause: default_key_play_pause(),
            stop: default_key_stop(),
            next: default_key_next(),
            previous: default_key_previous(),
            seek_forward: default_key_seek_forward(),
            seek_backward: default_key_seek_backward(),
            volume_up: default_key_volume_up(),
            volume_down: default_key_volume_down(),
            shuffle: default_key_shuffle(),
            repeat: default_key_repeat(),
            enqueue: default_key_enqueue(),
            enqueue_next: default_key_enqueue_next(),
            enqueue_album: default_key_enqueue_album(),
            clear_queue: default_key_clear_queue(),
            remove_from_queue: default_key_remove_from_queue(),
            star: default_key_star(),
            lyrics: default_key_lyrics(),
            add_to_playlist: default_key_add_to_playlist(),
            now_playing: default_key_now_playing(),
            favorites: default_key_favorites(),
            history: default_key_history(),
            statistics: default_key_statistics(),
            radio: default_key_radio(),
            sort: default_key_sort(),
        }
    }
}

// Default keybinding functions
fn default_key_quit() -> String { "q".to_string() }
fn default_key_help() -> String { "?".to_string() }
fn default_key_search() -> String { "/".to_string() }
fn default_key_filter() -> String { "\\".to_string() }
fn default_key_focus_left() -> String { "h".to_string() }
fn default_key_focus_right() -> String { "l".to_string() }
fn default_key_play_pause() -> String { " ".to_string() }
fn default_key_stop() -> String { "".to_string() }  // Not used
fn default_key_next() -> String { "n".to_string() }
fn default_key_previous() -> String { "p".to_string() }
fn default_key_seek_forward() -> String { "".to_string() }  // Arrow keys handled separately
fn default_key_seek_backward() -> String { "".to_string() }  // Arrow keys handled separately
fn default_key_volume_up() -> String { "+".to_string() }
fn default_key_volume_down() -> String { "-".to_string() }
fn default_key_shuffle() -> String { "s".to_string() }
fn default_key_repeat() -> String { "r".to_string() }
fn default_key_enqueue() -> String { "e".to_string() }
fn default_key_enqueue_next() -> String { "E".to_string() }
fn default_key_enqueue_album() -> String { "a".to_string() }
fn default_key_clear_queue() -> String { "x".to_string() }
fn default_key_remove_from_queue() -> String { "d".to_string() }
fn default_key_star() -> String { "S".to_string() }
fn default_key_lyrics() -> String { "".to_string() }  // Not currently implemented
fn default_key_add_to_playlist() -> String { "A".to_string() }
fn default_key_now_playing() -> String { "f".to_string() }
fn default_key_favorites() -> String { "F".to_string() }
fn default_key_history() -> String { "H".to_string() }
fn default_key_statistics() -> String { "I".to_string() }
fn default_key_radio() -> String { "R".to_string() }
fn default_key_sort() -> String { "o".to_string() }

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

        // Validate keybindings for duplicates
        config.validate_keybindings()?;

        Ok(config)
    }

    fn validate_keybindings(&self) -> Result<()> {
        use std::collections::HashMap;

        let kb = &self.keybindings;
        let mut key_map: HashMap<&str, Vec<&str>> = HashMap::new();

        // Collect all keybindings
        let bindings = vec![
            (&kb.quit, "quit"),
            (&kb.help, "help"),
            (&kb.search, "search"),
            (&kb.filter, "filter"),
            (&kb.focus_left, "focus_left"),
            (&kb.focus_right, "focus_right"),
            (&kb.play_pause, "play_pause"),
            (&kb.stop, "stop"),
            (&kb.next, "next"),
            (&kb.previous, "previous"),
            (&kb.seek_forward, "seek_forward"),
            (&kb.seek_backward, "seek_backward"),
            (&kb.volume_up, "volume_up"),
            (&kb.volume_down, "volume_down"),
            (&kb.shuffle, "shuffle"),
            (&kb.repeat, "repeat"),
            (&kb.enqueue, "enqueue"),
            (&kb.enqueue_next, "enqueue_next"),
            (&kb.enqueue_album, "enqueue_album"),
            (&kb.clear_queue, "clear_queue"),
            (&kb.remove_from_queue, "remove_from_queue"),
            (&kb.star, "star"),
            (&kb.lyrics, "lyrics"),
            (&kb.add_to_playlist, "add_to_playlist"),
            (&kb.now_playing, "now_playing"),
            (&kb.favorites, "favorites"),
            (&kb.history, "history"),
            (&kb.statistics, "statistics"),
            (&kb.radio, "radio"),
            (&kb.sort, "sort"),
        ];

        for (key, action) in bindings {
            // Skip empty keybindings
            if !key.is_empty() {
                key_map.entry(key.as_str()).or_default().push(action);
            }
        }

        // Check for duplicates
        for (key, actions) in key_map {
            if actions.len() > 1 {
                return Err(anyhow!(
                    "Duplicate keybinding '{}' found for actions: {}",
                    key,
                    actions.join(", ")
                ));
            }
        }

        Ok(())
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
