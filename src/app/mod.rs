use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use rand::seq::SliceRandom;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;

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

    // Status
    pub status_message: Option<String>,
    pub loading: bool,

    // Playback modes
    pub shuffle: bool,
    pub repeat: RepeatMode,

    // Add to playlist
    pub add_to_playlist_song: Option<Song>,
    pub new_playlist_name: String,

    // Scrobble tracking
    pub scrobbled_song_id: Option<String>,

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
            status_message: Some("Loading library...".to_string()),
            loading: true,
            cover_art_data: None,
            cover_art_protocol: None,
            lyrics: None,
            lyrics_lines: Vec::new(),
            lyrics_timestamps: Vec::new(),
            shuffle: false,
            repeat: RepeatMode::Off,
            add_to_playlist_song: None,
            new_playlist_name: String::new(),
            scrobbled_song_id: None,
            now_playing_view: NowPlayingView::Lyrics,
            discord,
            last_discord_update: None,
        };

        // Load initial data
        app.load_artists().await?;
        app.load_playlists().await?;
        app.status_message = None;
        app.loading = false;

        Ok(app)
    }

    pub async fn load_artists(&mut self) -> Result<()> {
        self.artists = self.client.get_artists().await?;
        Ok(())
    }

    pub async fn load_playlists(&mut self) -> Result<()> {
        self.playlists = self.client.get_playlists().await?;
        Ok(())
    }

    pub async fn load_artist_albums(&mut self, artist_id: &str) -> Result<()> {
        let artist = self.client.get_artist(artist_id).await?;
        self.albums = artist.albums;
        self.current_artist = Some(Artist {
            id: artist.id,
            name: artist.name,
            album_count: artist.album_count,
            cover_art: None,
        });
        self.selected_album = 0;
        Ok(())
    }

    pub async fn load_album_songs(&mut self, album_id: &str) -> Result<()> {
        let album = self.client.get_album(album_id).await?;
        self.songs = album.songs.clone();
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
            KeyCode::Char('q') => return Ok(true),

            // Panel switching
            KeyCode::Tab => self.next_panel(),
            KeyCode::BackTab => self.prev_panel(),
            KeyCode::Char('h') => self.prev_panel(),
            KeyCode::Char('l') => self.next_panel(),

            // Navigation
            KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_selection(-1),
            KeyCode::Char('g') => self.move_to_top(),
            KeyCode::Char('G') => self.move_to_bottom(),

            // Search
            KeyCode::Char('/') => {
                self.input_mode = InputMode::Search;
                self.search_query.clear();
            }

            // Enter/Select
            KeyCode::Enter => self.handle_enter().await?,

            // Playback
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
            KeyCode::Char('s') => {
                self.shuffle = !self.shuffle;
                if self.shuffle {
                    self.shuffle_queue();
                    self.status_message = Some("Shuffle: ON".to_string());
                } else {
                    self.status_message = Some("Shuffle: OFF".to_string());
                }
            }

            // Repeat
            KeyCode::Char('r') => {
                self.repeat = match self.repeat {
                    RepeatMode::Off => RepeatMode::All,
                    RepeatMode::All => RepeatMode::One,
                    RepeatMode::One => RepeatMode::Off,
                };
                let msg = match self.repeat {
                    RepeatMode::Off => "Repeat: OFF",
                    RepeatMode::All => "Repeat: ALL",
                    RepeatMode::One => "Repeat: ONE",
                };
                self.status_message = Some(msg.to_string());
            }

            // Volume
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.player.volume_up()?;
            }
            KeyCode::Char('-') => {
                self.player.volume_down()?;
            }

            // Enqueue
            KeyCode::Char('e') => self.enqueue_selected().await?,
            KeyCode::Char('E') => self.enqueue_next_selected().await?,

            // Add all songs from album to queue
            KeyCode::Char('a') => self.enqueue_album().await?,

            // Add to playlist
            KeyCode::Char('A') => {
                let song = match self.focus {
                    FocusPanel::Songs => self.songs.get(self.selected_song).cloned(),
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

            // Playlists
            KeyCode::Char('P') => {
                self.load_playlists().await?;
                self.selected_playlist = 0;
                self.input_mode = InputMode::Playlists;
            }

            // Help
            KeyCode::Char('?') => {
                self.input_mode = InputMode::Help;
            }

            // Now playing view
            KeyCode::Char('f') => {
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
        let (selected, len) = match self.focus {
            FocusPanel::Artists => (&mut self.selected_artist, self.artists.len()),
            FocusPanel::Albums => (&mut self.selected_album, self.albums.len()),
            FocusPanel::Songs => (&mut self.selected_song, self.songs.len()),
            FocusPanel::Queue => (&mut self.selected_queue, self.player.queue.len()),
        };

        if len == 0 {
            return;
        }

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
                self.selected_artist = self.artists.len().saturating_sub(1);
            }
            FocusPanel::Albums => {
                self.selected_album = self.albums.len().saturating_sub(1);
            }
            FocusPanel::Songs => {
                self.selected_song = self.songs.len().saturating_sub(1);
            }
            FocusPanel::Queue => {
                self.selected_queue = self.player.queue.len().saturating_sub(1);
            }
        }
    }

    async fn handle_enter(&mut self) -> Result<()> {
        match self.focus {
            FocusPanel::Artists => {
                if let Some(artist) = self.artists.get(self.selected_artist) {
                    self.load_artist_albums(&artist.id.clone()).await?;
                    // Auto-advance to albums panel
                    self.focus = FocusPanel::Albums;
                }
            }
            FocusPanel::Albums => {
                if let Some(album) = self.albums.get(self.selected_album) {
                    self.load_album_songs(&album.id.clone()).await?;
                    // Auto-advance to songs panel
                    self.focus = FocusPanel::Songs;
                }
            }
            FocusPanel::Songs => {
                if let Some(song) = self.songs.get(self.selected_song).cloned() {
                    // Add to queue and play
                    self.player.clear_queue();
                    for s in &self.songs {
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
                if let Some(song) = self.songs.get(self.selected_song) {
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
            if let Some(song) = self.songs.get(self.selected_song) {
                self.player.enqueue_next(song.clone());
                self.status_message = Some(format!("\"{}\" will play next", song.title));
            }
        }
        Ok(())
    }

    async fn enqueue_album(&mut self) -> Result<()> {
        if !self.songs.is_empty() {
            for song in &self.songs {
                self.player.enqueue(song.clone());
            }
            self.status_message = Some(format!("Added {} songs to queue", self.songs.len()));
        }
        Ok(())
    }

    pub async fn tick(&mut self) -> Result<()> {
        // Update Discord Rich Presence
        self.update_discord_presence();

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

        Ok(())
    }

    fn update_discord_presence(&mut self) {
        if let Some(ref mut discord) = self.discord {
            // Try to reconnect if disconnected
            let _ = discord.reconnect();

            if let Some(song) = &self.player.current_song {
                // Only update if song changed or pause state changed
                let song_key = format!("{}:{}", song.id, self.player.is_paused());
                if self.last_discord_update.as_ref() != Some(&song_key) {
                    if let Err(e) = discord.update_song(song, self.player.is_paused()) {
                        tracing::warn!("Failed to update Discord presence: {}", e);
                    }
                    self.last_discord_update = Some(song_key);
                }
            } else {
                // No song playing, clear presence
                if self.last_discord_update.is_some() {
                    let _ = discord.clear();
                    self.last_discord_update = None;
                }
            }
        }
    }
}
