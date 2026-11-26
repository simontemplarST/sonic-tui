use anyhow::{anyhow, Result};
use md5;
use rand::Rng;
use reqwest::Client;

use super::models::*;

const CLIENT_NAME: &str = "sonic-tui";
const API_VERSION: &str = "1.16.1";

#[derive(Clone)]
pub struct SubsonicClient {
    client: Client,
    base_url: String,
    username: String,
    password: String,
}

impl SubsonicClient {
    pub fn new(base_url: &str, username: &str, password: &str) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            username: username.to_string(),
            password: password.to_string(),
        })
    }

    fn generate_auth_params(&self) -> Vec<(&str, String)> {
        let salt: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(12)
            .map(char::from)
            .collect();

        let token = format!("{:x}", md5::compute(format!("{}{}", self.password, salt)));

        vec![
            ("u", self.username.clone()),
            ("t", token),
            ("s", salt),
            ("v", API_VERSION.to_string()),
            ("c", CLIENT_NAME.to_string()),
            ("f", "json".to_string()),
        ]
    }

    async fn get<T>(&self, endpoint: &str, params: &[(&str, &str)]) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let url = format!("{}/rest/{}", self.base_url, endpoint);
        let mut all_params = self.generate_auth_params();
        for (k, v) in params {
            all_params.push((k, v.to_string()));
        }

        let response = self
            .client
            .get(&url)
            .query(&all_params)
            .send()
            .await?
            .json::<SubsonicResponse<T>>()
            .await?;

        if response.response.status != "ok" {
            if let Some(error) = response.response.error {
                return Err(anyhow!("API error {}: {}", error.code, error.message));
            }
            return Err(anyhow!("Unknown API error"));
        }

        Ok(response.response.data)
    }

    /// Test connection to server
    pub async fn ping(&self) -> Result<()> {
        let _: Empty = self.get("ping", &[]).await?;
        Ok(())
    }

    /// Get all artists
    pub async fn get_artists(&self) -> Result<Vec<Artist>> {
        let response: ArtistsResponse = self.get("getArtists", &[]).await?;
        Ok(response
            .artists
            .index
            .into_iter()
            .flat_map(|idx| idx.artists)
            .collect())
    }

    /// Get artist with albums
    pub async fn get_artist(&self, id: &str) -> Result<ArtistDetail> {
        let response: ArtistResponse = self.get("getArtist", &[("id", id)]).await?;
        Ok(response.artist)
    }

    /// Get album with songs
    pub async fn get_album(&self, id: &str) -> Result<AlbumDetail> {
        let response: AlbumResponse = self.get("getAlbum", &[("id", id)]).await?;
        Ok(response.album)
    }

    /// Search for artists, albums, and songs
    pub async fn search(&self, query: &str) -> Result<SearchResult3> {
        let response: SearchResult3Response = self
            .get(
                "search3",
                &[
                    ("query", query),
                    ("artistCount", "20"),
                    ("albumCount", "20"),
                    ("songCount", "50"),
                ],
            )
            .await?;
        Ok(response.search_result)
    }

    /// Get all playlists
    pub async fn get_playlists(&self) -> Result<Vec<Playlist>> {
        let response: PlaylistsResponse = self.get("getPlaylists", &[]).await?;
        Ok(response.playlists.playlists)
    }

    /// Get playlist with songs
    pub async fn get_playlist(&self, id: &str) -> Result<PlaylistDetail> {
        let response: PlaylistResponse = self.get("getPlaylist", &[("id", id)]).await?;
        Ok(response.playlist)
    }

    /// Create a new playlist
    pub async fn create_playlist(&self, name: &str) -> Result<()> {
        let _: Empty = self.get("createPlaylist", &[("name", name)]).await?;
        Ok(())
    }

    /// Add songs to playlist
    pub async fn update_playlist(&self, playlist_id: &str, song_ids: &[String]) -> Result<()> {
        let mut params: Vec<(&str, &str)> = vec![("playlistId", playlist_id)];
        for id in song_ids {
            params.push(("songIdToAdd", id));
        }
        let _: Empty = self.get("updatePlaylist", &params).await?;
        Ok(())
    }

    /// Delete playlist
    pub async fn delete_playlist(&self, id: &str) -> Result<()> {
        let _: Empty = self.get("deletePlaylist", &[("id", id)]).await?;
        Ok(())
    }

    /// Get lyrics for a song
    pub async fn get_lyrics(&self, artist: &str, title: &str) -> Result<Option<String>> {
        let response: LyricsResponse = self
            .get("getLyrics", &[("artist", artist), ("title", title)])
            .await?;
        Ok(response.lyrics.and_then(|l| l.value))
    }

    /// Lyrics by song id (OpenSubsonic songLyrics). Returns (lines, timestamps in seconds; empty if unsynced).
    pub async fn get_lyrics_by_song_id(&self, id: &str) -> Result<Option<(Vec<String>, Vec<f64>)>> {
        let r: LyricsListResponse = self.get("getLyricsBySongId", &[("id", id)]).await?;
        let list = match r.lyrics_list {
            Some(l) => l.structured_lyrics,
            None => return Ok(None),
        };
        let pick = list
            .iter()
            .find(|l| l.synced && !l.line.is_empty())
            .or_else(|| list.iter().find(|l| !l.line.is_empty()));
        let Some(l) = pick else { return Ok(None) };
        // OpenSubsonic: positive offset (ms) shows lyrics sooner.
        let offset = l.offset.unwrap_or(0.0);
        let lines = l.line.iter().map(|x| x.value.clone()).collect();
        let stamps = if l.synced {
            l.line.iter().map(|x| ((x.start.unwrap_or(0.0) - offset) / 1000.0).max(0.0)).collect()
        } else {
            Vec::new()
        };
        Ok(Some((lines, stamps)))
    }

    /// Scrobble a song (mark as played)
    pub async fn scrobble(&self, id: &str, submission: bool) -> Result<()> {
        let submission_str = if submission { "true" } else { "false" };
        let _: Empty = self
            .get("scrobble", &[("id", id), ("submission", submission_str)])
            .await?;
        Ok(())
    }

    /// Get stream URL for a song
    pub fn get_stream_url(&self, id: &str, max_bitrate: Option<i32>) -> String {
        let mut params = self.generate_auth_params();
        params.push(("id", id.to_string()));
        if let Some(bitrate) = max_bitrate {
            params.push(("maxBitRate", bitrate.to_string()));
        }

        let query: Vec<String> = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect();

        format!("{}/rest/stream?{}", self.base_url, query.join("&"))
    }

    /// Get cover art URL
    pub fn get_cover_art_url(&self, id: &str, size: Option<i32>) -> String {
        let mut params = self.generate_auth_params();
        params.push(("id", id.to_string()));
        if let Some(s) = size {
            params.push(("size", s.to_string()));
        }

        let query: Vec<String> = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect();

        format!("{}/rest/getCoverArt?{}", self.base_url, query.join("&"))
    }

    /// Get download URL for a song (for future offline mode)
    #[allow(dead_code)]
    pub fn get_download_url(&self, id: &str) -> String {
        let mut params = self.generate_auth_params();
        params.push(("id", id.to_string()));

        let query: Vec<String> = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect();

        format!("{}/rest/download?{}", self.base_url, query.join("&"))
    }

    /// Star an item (artist, album, or song)
    pub async fn star(&self, id: &str, item_type: &str) -> Result<()> {
        let param_name = match item_type {
            "artist" => "artistId",
            "album" => "albumId",
            "song" => "id",
            _ => return Err(anyhow!("Invalid item type: {}", item_type)),
        };
        let _: Empty = self.get("star", &[(param_name, id)]).await?;
        Ok(())
    }

    /// Unstar an item (artist, album, or song)
    pub async fn unstar(&self, id: &str, item_type: &str) -> Result<()> {
        let param_name = match item_type {
            "artist" => "artistId",
            "album" => "albumId",
            "song" => "id",
            _ => return Err(anyhow!("Invalid item type: {}", item_type)),
        };
        let _: Empty = self.get("unstar", &[(param_name, id)]).await?;
        Ok(())
    }

    /// Get all starred items
    pub async fn get_starred(&self) -> Result<Starred2> {
        let response: StarredResponse = self.get("getStarred2", &[]).await?;
        Ok(response.starred2)
    }

    /// Get similar songs
    pub async fn get_similar_songs(&self, id: &str, count: usize) -> Result<Vec<Song>> {
        let response: SimilarSongsResponse = self
            .get(
                "getSimilarSongs2",
                &[("id", id), ("count", &count.to_string())],
            )
            .await?;
        Ok(response.similar_songs.songs)
    }

    /// Get recently added albums
    pub async fn get_recently_added_albums(&self, count: usize) -> Result<Vec<Album>> {
        let response: AlbumList2Response = self
            .get(
                "getAlbumList2",
                &[("type", "newest"), ("size", &count.to_string())],
            )
            .await?;
        Ok(response.album_list.albums)
    }
}
