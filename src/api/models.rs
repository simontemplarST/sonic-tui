use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsonicResponse<T> {
    #[serde(rename = "subsonic-response")]
    pub response: SubsonicResponseInner<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsonicResponseInner<T> {
    pub status: String,
    pub version: String,
    #[serde(flatten)]
    pub data: T,
    pub error: Option<SubsonicError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsonicError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Empty {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtistsResponse {
    pub artists: ArtistsContainer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtistsContainer {
    #[serde(rename = "index", default)]
    pub index: Vec<ArtistIndex>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtistIndex {
    pub name: String,
    #[serde(rename = "artist", default)]
    pub artists: Vec<Artist>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artist {
    pub id: String,
    pub name: String,
    #[serde(rename = "albumCount")]
    pub album_count: Option<i32>,
    #[serde(rename = "coverArt")]
    pub cover_art: Option<String>,
    #[serde(default)]
    pub starred: Option<String>, // ISO 8601 timestamp when starred
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtistResponse {
    pub artist: ArtistDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtistDetail {
    pub id: String,
    pub name: String,
    #[serde(rename = "albumCount")]
    pub album_count: Option<i32>,
    #[serde(default)]
    pub starred: Option<String>, // ISO 8601 timestamp when starred
    #[serde(rename = "album", default)]
    pub albums: Vec<Album>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Album {
    pub id: String,
    pub name: String,
    pub artist: Option<String>,
    #[serde(rename = "artistId")]
    pub artist_id: Option<String>,
    #[serde(rename = "coverArt")]
    pub cover_art: Option<String>,
    #[serde(rename = "songCount")]
    pub song_count: Option<i32>,
    pub duration: Option<i32>,
    pub year: Option<i32>,
    pub genre: Option<String>,
    #[serde(default)]
    pub starred: Option<String>, // ISO 8601 timestamp when starred
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumResponse {
    pub album: AlbumDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumDetail {
    pub id: String,
    pub name: String,
    pub artist: Option<String>,
    #[serde(rename = "artistId")]
    pub artist_id: Option<String>,
    #[serde(rename = "coverArt")]
    pub cover_art: Option<String>,
    #[serde(rename = "songCount")]
    pub song_count: Option<i32>,
    pub duration: Option<i32>,
    pub year: Option<i32>,
    pub genre: Option<String>,
    #[serde(default)]
    pub starred: Option<String>, // ISO 8601 timestamp when starred
    #[serde(rename = "song", default)]
    pub songs: Vec<Song>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Song {
    pub id: String,
    pub title: String,
    pub album: Option<String>,
    #[serde(rename = "albumId")]
    pub album_id: Option<String>,
    pub artist: Option<String>,
    #[serde(rename = "artistId")]
    pub artist_id: Option<String>,
    pub track: Option<i32>,
    #[serde(rename = "discNumber")]
    pub disc_number: Option<i32>,
    pub year: Option<i32>,
    pub genre: Option<String>,
    #[serde(rename = "coverArt")]
    pub cover_art: Option<String>,
    pub duration: Option<i32>,
    #[serde(rename = "bitRate")]
    pub bit_rate: Option<i32>,
    pub path: Option<String>,
    pub suffix: Option<String>,
    #[serde(default)]
    pub starred: Option<String>, // ISO 8601 timestamp when starred
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult3Response {
    #[serde(rename = "searchResult3")]
    pub search_result: SearchResult3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult3 {
    #[serde(rename = "artist", default)]
    pub artists: Vec<Artist>,
    #[serde(rename = "album", default)]
    pub albums: Vec<Album>,
    #[serde(rename = "song", default)]
    pub songs: Vec<Song>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistsResponse {
    pub playlists: PlaylistsContainer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistsContainer {
    #[serde(rename = "playlist", default)]
    pub playlists: Vec<Playlist>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub comment: Option<String>,
    #[serde(rename = "songCount")]
    pub song_count: Option<i32>,
    pub duration: Option<i32>,
    pub public: Option<bool>,
    pub owner: Option<String>,
    #[serde(rename = "coverArt")]
    pub cover_art: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistResponse {
    pub playlist: PlaylistDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistDetail {
    pub id: String,
    pub name: String,
    #[serde(rename = "entry", default)]
    pub entries: Vec<Song>,
}

// OpenSubsonic getLyricsBySongId (songLyrics extension): structured, optionally synced lyrics.
#[derive(Debug, Clone, Deserialize)]
pub struct LyricsListResponse {
    #[serde(rename = "lyricsList")]
    pub lyrics_list: Option<LyricsList>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LyricsList {
    #[serde(rename = "structuredLyrics", default)]
    pub structured_lyrics: Vec<StructuredLyrics>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StructuredLyrics {
    #[serde(default)]
    pub synced: bool,
    #[serde(default)]
    pub offset: Option<f64>,
    #[serde(default)]
    pub line: Vec<LyricLine>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LyricLine {
    pub start: Option<f64>,
    #[serde(default)]
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsResponse {
    pub lyrics: Option<Lyrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lyrics {
    pub artist: Option<String>,
    pub title: Option<String>,
    #[serde(alias = "$value")]
    pub value: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NowPlayingResponse {
    #[serde(rename = "nowPlaying")]
    pub now_playing: NowPlayingContainer,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NowPlayingContainer {
    #[serde(rename = "entry", default)]
    pub entries: Vec<NowPlayingEntry>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NowPlayingEntry {
    #[serde(flatten)]
    pub song: Song,
    pub username: Option<String>,
    #[serde(rename = "minutesAgo")]
    pub minutes_ago: Option<i32>,
    #[serde(rename = "playerId")]
    pub player_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarredResponse {
    pub starred2: Starred2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Starred2 {
    #[serde(rename = "artist", default)]
    pub artists: Vec<Artist>,
    #[serde(rename = "album", default)]
    pub albums: Vec<Album>,
    #[serde(rename = "song", default)]
    pub songs: Vec<Song>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarSongsResponse {
    #[serde(rename = "similarSongs2")]
    pub similar_songs: SimilarSongs2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarSongs2 {
    #[serde(rename = "song", default)]
    pub songs: Vec<Song>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumList2Response {
    #[serde(rename = "albumList2")]
    pub album_list: AlbumList2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumList2 {
    #[serde(rename = "album", default)]
    pub albums: Vec<Album>,
}
