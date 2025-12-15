# sonic-tui

A terminal-based music player for OpenSubsonic and Navidrome servers, inspired by [jellyfin-tui](https://github.com/dhonus/jellyfin-tui).

![Screenshot](screenshot.png)

## Features

- **Multi-panel interface** - Browse artists, albums, songs, and queue simultaneously
- **Now Playing view** - Immersive fullscreen view with album art and lyrics/queue
- **Album art display** - Sixel/Kitty protocol support for inline images
- **Lyrics** - Synced lyrics from server with LRCLIB fallback
- **Discord Rich Presence** - Show what you're listening to on Discord
- **Queue management** - Add, remove, shuffle, and reorder tracks
- **Playlist support** - Browse, create, edit, and delete playlists
- **Search** - Search across artists, albums, and songs
- **Scrobbling** - Server-side scrobbling support
- **Gapless playback** - Seamless transitions between tracks
- **Vim-style keybindings** - Navigate with hjkl

## Requirements

- A terminal with Sixel or Kitty image protocol support (for album art)
- mpv (libmpv) for audio playback
- An OpenSubsonic-compatible server (Navidrome, Airsonic, etc.)

## Installation

### From source

```bash
# Clone the repository
git clone https://github.com/yourusername/sonic-tui.git
cd sonic-tui

# Build and install
cargo install --path .
```

### Using cargo

```bash
cargo install sonic-tui
```

### NixOS

A `shell.nix` is provided for development:

```bash
nix-shell
cargo build --release
```

## Configuration

Create a configuration file at `~/.config/sonic-tui/config.yaml`:

```yaml
server:
  url: "https://your-navidrome-server.com"
  username: "your-username"
  password: "your-password"

audio:
  volume: 100
  max_bitrate: null  # null for original quality, or set to 320, 256, etc.
  gapless: true

ui:
  show_cover_art: true
  color_from_cover: false
  accent_color: null  # null for default, or hex color like "#ff5500"

cache:
  directory: "~/.cache/sonic-tui"
  enabled: true

discord:
  enabled: false
  # application_id: "YOUR_DISCORD_APPLICATION_ID"  # Get from https://discord.com/developers/applications
```

### Discord Rich Presence (Optional)

To show what you're listening to on Discord:

1. Go to https://discord.com/developers/applications
2. Click "New Application" and name it (e.g., "Music Player")
3. Copy the Application ID from the General Information page
4. Add it to your config file:
   ```yaml
   discord:
     enabled: true
     application_id: "1234567890123456789"
   ```
5. (Optional) Upload an image called "music" under Rich Presence → Art Assets for a custom icon

## Keybindings

### Global

| Key | Action |
|-----|--------|
| `q` | Quit |
| `?` | Show help |
| `/` | Search |
| `P` | Playlists |
| `f` | Now Playing view |
| `Space` | Play/Pause |
| `n` | Next track |
| `p` | Previous track |
| `s` | Toggle shuffle |
| `r` | Cycle repeat mode |
| `+` / `=` | Volume up |
| `-` | Volume down |
| `Left` / `Right` | Seek backward/forward |

### Navigation

| Key | Action |
|-----|--------|
| `h` / `Left` | Focus left panel |
| `l` / `Right` | Focus right panel |
| `j` / `Down` | Move down in list |
| `k` / `Up` | Move up in list |
| `Tab` | Cycle focus between panels |
| `Enter` | Select / Play |
| `g` | Go to top of list |
| `G` | Go to bottom of list |

### Queue

| Key | Action |
|-----|--------|
| `e` | Add to queue |
| `E` | Add to queue (play next) |
| `a` | Add album to queue |
| `A` | Add song to playlist |

### Now Playing View

| Key | Action |
|-----|--------|
| `Tab` / `t` | Toggle lyrics/queue |
| `Esc` / `f` / `q` | Exit to main view |
| `j` / `k` | Navigate queue (when queue is shown) |
| `Enter` | Play selected song from queue |
| `Space` / `n` / `p` / `←` / `→` | Playback controls still work |

### Playlists

| Key | Action |
|-----|--------|
| `Enter` | Play playlist |
| `e` | Enqueue playlist |
| `d` | Delete playlist |
| `n` | Create new playlist (when adding song) |

### Search

| Key | Action |
|-----|--------|
| `Tab` | Cycle between Artists/Albums/Songs |
| `Enter` | Select result |
| `Esc` | Close search |

## Dependencies

### Runtime

- mpv (for audio playback)
- A Sixel-capable terminal (optional, for album art):
  - kitty
  - WezTerm
  - foot
  - mlterm
  - xterm (with Sixel support enabled)

### Build

- Rust 1.70+
- pkg-config
- OpenSSL development headers
- mpv development headers

#### Debian/Ubuntu

```bash
sudo apt install pkg-config libssl-dev libmpv-dev
```

#### Fedora

```bash
sudo dnf install pkg-config openssl-devel mpv-libs-devel
```

#### Arch Linux

```bash
sudo pacman -S pkg-config openssl mpv
```

#### macOS

```bash
brew install pkg-config openssl mpv
```

## Troubleshooting

### No album art showing

Make sure your terminal supports the Sixel or Kitty image protocol. You can test with:

```bash
# For Sixel
img2sixel /path/to/image.png

# For Kitty
kitty +kitten icat /path/to/image.png
```

### Audio not playing

Ensure mpv is installed and working:

```bash
mpv --version
mpv /path/to/audio/file.mp3
```

### Connection errors

Verify your server URL includes the protocol (http:// or https://) and check your credentials.

## License

GPL-3.0

## Acknowledgments

- [jellyfin-tui](https://github.com/dhonus/jellyfin-tui) - Inspiration and reference
- [ratatui](https://github.com/ratatui-org/ratatui) - Terminal UI framework
- [libmpv](https://mpv.io/) - Audio playback
