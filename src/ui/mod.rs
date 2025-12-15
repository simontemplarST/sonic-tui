use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use ratatui_image::StatefulImage;

use crate::app::{App, FocusPanel, InputMode, NowPlayingView, RepeatMode, SearchFocus};

pub fn draw(f: &mut Frame, app: &mut App) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // Main content
            Constraint::Length(2), // Player bar (progress + help)
        ])
        .split(f.area());

    // Check if in now playing mode
    if app.input_mode == InputMode::NowPlaying {
        draw_now_playing_view(f, app, main_chunks[0]);
    } else {
        // Main area split: left (library) and right (queue + art)
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(60), // Library browser
                Constraint::Percentage(40), // Right panel (queue + art)
            ])
            .split(main_chunks[0]);

        // Library: 3-column browser
        draw_library(f, app, content_chunks[0]);

        // Right panel: queue on top, album art below
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(60), // Queue
                Constraint::Percentage(40), // Album art / lyrics
            ])
            .split(content_chunks[1]);

        draw_queue(f, app, right_chunks[0]);
        draw_album_art(f, app, right_chunks[1]);
    }

    // Player bar
    draw_player(f, app, main_chunks[1]);

    // Draw search overlay if in search mode
    if app.input_mode == InputMode::Search {
        draw_search_input(f, app);
    }

    // Draw search results if in search results mode
    if app.input_mode == InputMode::SearchResults {
        draw_search_results(f, app);
    }

    // Draw playlists modal
    if app.input_mode == InputMode::Playlists {
        draw_playlists_modal(f, app);
    }

    // Draw help modal
    if app.input_mode == InputMode::Help {
        draw_help_modal(f);
    }

    // Draw add to playlist modal
    if app.input_mode == InputMode::AddToPlaylist {
        draw_add_to_playlist_modal(f, app);
    }

    // Draw create playlist modal
    if app.input_mode == InputMode::CreatePlaylist {
        draw_create_playlist_modal(f, app);
    }
}

fn draw_now_playing_view(f: &mut Frame, app: &mut App, area: Rect) {
    // Split into left (album art) and right (lyrics or queue)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Album art
            Constraint::Percentage(50), // Lyrics or Queue
        ])
        .split(area);

    // Draw album art with song info
    draw_now_playing_art(f, app, chunks[0]);

    // Draw lyrics or queue based on toggle
    match app.now_playing_view {
        NowPlayingView::Lyrics => draw_now_playing_lyrics(f, app, chunks[1]),
        NowPlayingView::Queue => draw_now_playing_queue(f, app, chunks[1]),
    }
}

fn draw_now_playing_art(f: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Now Playing")
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some(song) = &app.player.current_song {
        // Vertical layout: album art on top, song info below
        let info_height = 6;

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),              // Album art space
                Constraint::Length(info_height), // Song info
            ])
            .split(inner);

        // Render album art
        if let Some(ref mut protocol) = app.cover_art_protocol {
            let image_widget = StatefulImage::new(None);
            f.render_stateful_widget(image_widget, layout[0], protocol);
        }

        // Song info below
        let album = song.album.as_deref().unwrap_or("Unknown Album");
        let artist = song.artist.as_deref().unwrap_or("Unknown Artist");

        let info = vec![
            Line::from(""),
            Line::from(Span::styled(
                &song.title,
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::White),
            )),
            Line::from(Span::styled(artist, Style::default().fg(Color::Cyan))),
            Line::from(Span::styled(album, Style::default().fg(Color::DarkGray))),
        ];

        let info_widget = Paragraph::new(info)
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(info_widget, layout[1]);
    } else {
        // Center "No track playing" message
        let v_center = inner.height / 2;
        let centered = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(v_center),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(inner);

        let content = Line::from(Span::styled(
            "No track playing",
            Style::default().fg(Color::DarkGray),
        ));

        let paragraph = Paragraph::new(content)
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(paragraph, centered[1]);
    }
}

fn draw_now_playing_lyrics(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Lyrics (Tab: toggle queue)")
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.lyrics_lines.is_empty() {
        let v_center = inner.height / 2;
        let centered = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(v_center),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(inner);

        let content = if app.player.current_song.is_some() {
            "No lyrics available"
        } else {
            ""
        };
        let paragraph = Paragraph::new(content)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(paragraph, centered[1]);
        return;
    }

    let current_line = app.current_lyrics_line();
    let visible_height = inner.height as usize;

    // Calculate scroll offset to keep current line centered
    let scroll_offset = if current_line > visible_height / 2 {
        current_line - visible_height / 2
    } else {
        0
    };

    // Build lyrics display with highlighting
    let lines: Vec<Line> = app
        .lyrics_lines
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(visible_height)
        .map(|(i, line)| {
            if i == current_line {
                Line::from(Span::styled(
                    line.as_str(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
            } else if i == current_line.saturating_sub(1) || i == current_line + 1 {
                Line::from(Span::styled(
                    line.as_str(),
                    Style::default().fg(Color::White),
                ))
            } else {
                Line::from(Span::styled(
                    line.as_str(),
                    Style::default().fg(Color::DarkGray),
                ))
            }
        })
        .collect();

    let paragraph = Paragraph::new(lines).alignment(ratatui::layout::Alignment::Center);
    f.render_widget(paragraph, inner);
}

fn draw_now_playing_queue(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Queue (Tab: toggle lyrics)")
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let items: Vec<ListItem> = app
        .player
        .queue
        .iter()
        .enumerate()
        .map(|(i, song)| {
            let playing = i == app.player.queue_position && app.player.current_song.is_some();
            let prefix = if playing { "▶ " } else { "  " };
            let artist = song.artist.as_deref().unwrap_or("Unknown");
            let style = if playing {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };
            ListItem::new(format!("{}{} - {}", prefix, song.title, artist)).style(style)
        })
        .collect();

    if items.is_empty() {
        let v_center = inner.height / 2;
        let centered = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(v_center),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(inner);

        let content = Line::from(Span::styled(
            "Queue is empty",
            Style::default().fg(Color::DarkGray),
        ));

        let paragraph = Paragraph::new(content)
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(paragraph, centered[1]);
        return;
    }

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    state.select(Some(app.selected_queue));

    f.render_stateful_widget(list, inner, &mut state);
}

fn draw_create_playlist_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 20, f.area());
    f.render_widget(ratatui::widgets::Clear, area);

    // Just show a small input box
    let input_area = Rect {
        x: area.x,
        y: area.y + area.height / 2 - 1,
        width: area.width,
        height: 3,
    };

    let input = Paragraph::new(app.new_playlist_name.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("New Playlist Name")
                .border_style(Style::default().fg(Color::Yellow)),
        );

    f.render_widget(input, input_area);
}

fn draw_add_to_playlist_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 60, f.area());
    f.render_widget(ratatui::widgets::Clear, area);

    let song_name = app.add_to_playlist_song
        .as_ref()
        .map(|s| s.title.as_str())
        .unwrap_or("Unknown");

    let items: Vec<ListItem> = app
        .playlists
        .iter()
        .map(|p| ListItem::new(p.name.clone()))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Add \"{}\" to playlist (n: new)", song_name))
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    state.select(if app.playlists.is_empty() {
        None
    } else {
        Some(app.selected_playlist)
    });

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_help_modal(f: &mut Frame) {
    let area = centered_rect(70, 80, f.area());
    f.render_widget(ratatui::widgets::Clear, area);

    let help_text = vec![
        Line::from(Span::styled("Navigation", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))),
        Line::from("  Tab, h/l     Switch panels"),
        Line::from("  j/k, ↑/↓     Navigate up/down"),
        Line::from("  g/G          Go to top/bottom"),
        Line::from("  Enter        Select / Play"),
        Line::from("  Esc          Back / Close"),
        Line::from(""),
        Line::from(Span::styled("Playback", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))),
        Line::from("  Space        Play / Pause"),
        Line::from("  n            Next track"),
        Line::from("  p            Previous track"),
        Line::from("  +/-          Volume up/down"),
        Line::from(""),
        Line::from(Span::styled("Queue", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))),
        Line::from("  e            Add to queue"),
        Line::from("  E            Play next"),
        Line::from("  a            Add album to queue"),
        Line::from("  A            Add to playlist"),
        Line::from("  s            Toggle shuffle"),
        Line::from("  r            Cycle repeat mode"),
        Line::from("  ←/→          Seek -/+ 5s"),
        Line::from(""),
        Line::from(Span::styled("Views", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))),
        Line::from("  /            Search"),
        Line::from("  P            Playlists"),
        Line::from("  f            Now Playing view"),
        Line::from("  ?            This help"),
        Line::from(""),
        Line::from(Span::styled("Now Playing View", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))),
        Line::from("  Tab/t        Toggle lyrics/queue"),
        Line::from("  Esc/f/q      Exit to main view"),
        Line::from("  j/k          Navigate queue"),
        Line::from("  Enter        Play from queue"),
        Line::from(""),
        Line::from(Span::styled("Other", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))),
        Line::from("  q            Quit"),
        Line::from(""),
        Line::from(Span::styled("Press Esc to close", Style::default().fg(Color::DarkGray))),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Help")
                .border_style(Style::default().fg(Color::Yellow)),
        );

    f.render_widget(paragraph, area);
}

fn draw_playlists_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 70, f.area());
    f.render_widget(ratatui::widgets::Clear, area);

    let items: Vec<ListItem> = app
        .playlists
        .iter()
        .map(|p| {
            let count = p.song_count
                .map(|c| format!(" ({} tracks)", c))
                .unwrap_or_default();
            let duration = p.duration
                .map(|d| {
                    let mins = d / 60;
                    let hours = mins / 60;
                    let mins = mins % 60;
                    if hours > 0 {
                        format!(" - {}h {}m", hours, mins)
                    } else {
                        format!(" - {}m", mins)
                    }
                })
                .unwrap_or_default();
            ListItem::new(format!("{}{}{}", p.name, count, duration))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Playlists ({}) - Enter: play, e: enqueue, d: delete", app.playlists.len()))
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    state.select(if app.playlists.is_empty() {
        None
    } else {
        Some(app.selected_playlist)
    });

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_library(f: &mut Frame, app: &App, area: Rect) {
    // Split into 2 columns: artists on left, albums/songs stacked on right
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(60),
        ])
        .split(area);

    draw_artists_panel(f, app, columns[0]);

    // Right column: albums on top, songs on bottom
    let right_stack = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(60),
        ])
        .split(columns[1]);

    draw_albums_panel(f, app, right_stack[0]);
    draw_songs_panel(f, app, right_stack[1]);
}

fn panel_style(focused: bool) -> Style {
    if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Gray)
    }
}

fn draw_artists_panel(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == FocusPanel::Artists;

    let items: Vec<ListItem> = app
        .artists
        .iter()
        .map(|artist| ListItem::new(artist.name.clone()))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Artists")
                .border_style(panel_style(focused)),
        )
        .highlight_style(
            Style::default()
                .bg(if focused { Color::DarkGray } else { Color::Black })
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(if focused { "▶ " } else { "  " });

    let mut state = ListState::default();
    state.select(Some(app.selected_artist));

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_albums_panel(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == FocusPanel::Albums;

    let items: Vec<ListItem> = app
        .albums
        .iter()
        .map(|album| {
            let year = album.year.map(|y| format!(" ({})", y)).unwrap_or_default();
            ListItem::new(format!("{}{}", album.name, year))
        })
        .collect();

    let title = if app.albums.is_empty() {
        "Albums".to_string()
    } else {
        format!("Albums ({})", app.albums.len())
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(panel_style(focused)),
        )
        .highlight_style(
            Style::default()
                .bg(if focused { Color::DarkGray } else { Color::Black })
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(if focused { "▶ " } else { "  " });

    let mut state = ListState::default();
    state.select(if app.albums.is_empty() { None } else { Some(app.selected_album) });

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_songs_panel(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == FocusPanel::Songs;

    let items: Vec<ListItem> = app
        .songs
        .iter()
        .map(|song| {
            let duration = song
                .duration
                .map(|d| format_duration(d))
                .unwrap_or_default();
            let track = song
                .track
                .map(|t| format!("{:2}. ", t))
                .unwrap_or_else(|| "".to_string());
            ListItem::new(format!("{}{} [{}]", track, song.title, duration))
        })
        .collect();

    let title = if app.songs.is_empty() {
        "Songs".to_string()
    } else {
        format!("Songs ({})", app.songs.len())
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(panel_style(focused)),
        )
        .highlight_style(
            Style::default()
                .bg(if focused { Color::DarkGray } else { Color::Black })
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(if focused { "▶ " } else { "  " });

    let mut state = ListState::default();
    state.select(if app.songs.is_empty() { None } else { Some(app.selected_song) });

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_queue(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == FocusPanel::Queue;

    let items: Vec<ListItem> = app
        .player
        .queue
        .iter()
        .enumerate()
        .map(|(i, song)| {
            let playing = i == app.player.queue_position && app.player.current_song.is_some();
            let prefix = if playing { "▶ " } else { "  " };
            let artist = song.artist.as_deref().unwrap_or("Unknown");
            let style = if playing {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };
            ListItem::new(format!("{}{} - {}", prefix, song.title, artist)).style(style)
        })
        .collect();

    let title = format!("Queue ({})", app.player.queue.len());

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(panel_style(focused)),
        )
        .highlight_style(
            Style::default()
                .bg(if focused { Color::DarkGray } else { Color::Black })
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(if focused { "▶ " } else { "  " });

    let mut state = ListState::default();
    state.select(if app.player.queue.is_empty() {
        None
    } else {
        Some(app.selected_queue)
    });

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_album_art(f: &mut Frame, app: &mut App, area: Rect) {
    // Split into album art (left) and lyrics (right)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Album art + info
            Constraint::Percentage(50), // Lyrics
        ])
        .split(area);

    draw_now_playing_info(f, app, chunks[0]);
    draw_lyrics(f, app, chunks[1]);
}

fn draw_now_playing_info(f: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Now Playing")
        .border_style(Style::default().fg(Color::Gray));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some(song) = &app.player.current_song {
        // Calculate content height (art + info)
        let art_height = inner.height.saturating_sub(4).min(inner.width / 2); // Keep aspect ratio
        let info_height = 3;
        let total_height = art_height + info_height;

        // Vertical centering
        let v_padding = inner.height.saturating_sub(total_height) / 2;

        let centered = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(v_padding),
                Constraint::Length(art_height),
                Constraint::Length(info_height),
                Constraint::Min(0),
            ])
            .split(inner);

        // Render cached album art (centered horizontally)
        if let Some(ref mut protocol) = app.cover_art_protocol {
            // Center the image horizontally
            let art_area = centered[1];
            let art_width = art_height * 2; // Approximate aspect ratio for terminal chars
            let h_padding = art_area.width.saturating_sub(art_width) / 2;

            let centered_art = Rect {
                x: art_area.x + h_padding,
                y: art_area.y,
                width: art_width.min(art_area.width),
                height: art_area.height,
            };

            let image_widget = StatefulImage::new(None);
            f.render_stateful_widget(image_widget, centered_art, protocol);
        }

        // Song info below the art (centered)
        let album = song.album.as_deref().unwrap_or("Unknown Album");
        let artist = song.artist.as_deref().unwrap_or("Unknown Artist");

        let info = vec![
            Line::from(Span::styled(
                &song.title,
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(artist),
            Line::from(Span::styled(album, Style::default().fg(Color::DarkGray))),
        ];

        let info_widget = Paragraph::new(info)
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: true });
        f.render_widget(info_widget, centered[2]);
    } else {
        // Center "No track playing" message
        let v_center = inner.height / 2;
        let centered = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(v_center),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(inner);

        let content = Line::from(Span::styled(
            "No track playing",
            Style::default().fg(Color::DarkGray),
        ));

        let paragraph = Paragraph::new(content)
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(paragraph, centered[1]);
    }
}

fn draw_lyrics(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Lyrics")
        .border_style(Style::default().fg(Color::Gray));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.lyrics_lines.is_empty() {
        let content = if app.player.current_song.is_some() {
            "No lyrics available"
        } else {
            ""
        };
        let paragraph = Paragraph::new(content)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(paragraph, inner);
        return;
    }

    let current_line = app.current_lyrics_line();
    let visible_height = inner.height as usize;

    // Calculate scroll offset to keep current line centered
    let scroll_offset = if current_line > visible_height / 2 {
        current_line - visible_height / 2
    } else {
        0
    };

    // Build lyrics display with highlighting
    let lines: Vec<Line> = app
        .lyrics_lines
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(visible_height)
        .map(|(i, line)| {
            if i == current_line {
                Line::from(Span::styled(
                    line.as_str(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
            } else if i == current_line.saturating_sub(1) || i == current_line + 1 {
                Line::from(Span::styled(
                    line.as_str(),
                    Style::default().fg(Color::White),
                ))
            } else {
                Line::from(Span::styled(
                    line.as_str(),
                    Style::default().fg(Color::DarkGray),
                ))
            }
        })
        .collect();

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);
}

fn draw_player(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Progress bar
            Constraint::Length(1), // Help
        ])
        .split(area);

    // Progress bar
    let position = app.player.position();
    let duration = app.player.duration();
    let progress = if duration > 0.0 {
        (position / duration * 100.0) as u16
    } else {
        0
    };

    // Build status indicators
    let shuffle_indicator = if app.shuffle { " 🔀" } else { "" };
    let repeat_indicator = match app.repeat {
        RepeatMode::Off => "",
        RepeatMode::All => " 🔁",
        RepeatMode::One => " 🔂",
    };

    let status = if app.player.current_song.is_some() {
        if app.player.is_paused() { "⏸ " } else { "▶ " }
    } else {
        "  "
    };

    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black))
        .percent(progress)
        .label(format!(
            "{}{} / {} │ Vol: {}%{}{}",
            status,
            format_duration(position as i32),
            format_duration(duration as i32),
            app.player.volume,
            shuffle_indicator,
            repeat_indicator
        ));

    f.render_widget(gauge, chunks[0]);

    // Help line
    let help = " Tab: panels │ j/k: nav │ Enter: play │ e: queue │ Space: pause │ /: search │ P: playlists │ f: now playing │ ?: help";
    let help_widget = Paragraph::new(help)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help_widget, chunks[1]);
}

fn draw_search_input(f: &mut Frame, app: &App) {
    // Small centered box for search input
    let area = f.area();
    let width = 50.min(area.width.saturating_sub(4));
    let x = (area.width - width) / 2;
    let y = area.height / 3;

    let search_area = Rect {
        x: area.x + x,
        y: area.y + y,
        width,
        height: 3,
    };

    // Clear the area
    f.render_widget(ratatui::widgets::Clear, search_area);

    let input = Paragraph::new(app.search_query.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Search")
                .border_style(Style::default().fg(Color::Yellow)),
        );

    f.render_widget(input, search_area);
}

fn draw_search_results(f: &mut Frame, app: &App) {
    // Large overlay for search results
    let area = centered_rect(80, 80, f.area());
    f.render_widget(ratatui::widgets::Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!("Search Results: \"{}\" (Esc to close)", app.search_query))
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Split into 3 columns
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(inner);

    // Artists column
    let artist_items: Vec<ListItem> = app
        .search_artists
        .iter()
        .map(|a| ListItem::new(a.name.clone()))
        .collect();

    let artists_focused = app.search_focus == SearchFocus::Artists;
    let artists_list = List::new(artist_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Artists ({})", app.search_artists.len()))
                .border_style(if artists_focused {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::Gray)
                }),
        )
        .highlight_style(
            Style::default()
                .bg(if artists_focused { Color::DarkGray } else { Color::Black })
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(if artists_focused { "▶ " } else { "  " });

    let mut artist_state = ListState::default();
    artist_state.select(if app.search_artists.is_empty() {
        None
    } else {
        Some(app.search_selected_artist)
    });
    f.render_stateful_widget(artists_list, columns[0], &mut artist_state);

    // Albums column
    let album_items: Vec<ListItem> = app
        .search_albums
        .iter()
        .map(|a| {
            let artist = a.artist.as_deref().unwrap_or("Unknown");
            ListItem::new(format!("{}\n  {}", a.name, artist))
        })
        .collect();

    let albums_focused = app.search_focus == SearchFocus::Albums;
    let albums_list = List::new(album_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Albums ({})", app.search_albums.len()))
                .border_style(if albums_focused {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::Gray)
                }),
        )
        .highlight_style(
            Style::default()
                .bg(if albums_focused { Color::DarkGray } else { Color::Black })
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(if albums_focused { "▶ " } else { "  " });

    let mut album_state = ListState::default();
    album_state.select(if app.search_albums.is_empty() {
        None
    } else {
        Some(app.search_selected_album)
    });
    f.render_stateful_widget(albums_list, columns[1], &mut album_state);

    // Songs column
    let song_items: Vec<ListItem> = app
        .search_songs
        .iter()
        .map(|s| {
            let artist = s.artist.as_deref().unwrap_or("Unknown");
            ListItem::new(format!("{}\n  {}", s.title, artist))
        })
        .collect();

    let songs_focused = app.search_focus == SearchFocus::Songs;
    let songs_list = List::new(song_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Songs ({})", app.search_songs.len()))
                .border_style(if songs_focused {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::Gray)
                }),
        )
        .highlight_style(
            Style::default()
                .bg(if songs_focused { Color::DarkGray } else { Color::Black })
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(if songs_focused { "▶ " } else { "  " });

    let mut song_state = ListState::default();
    song_state.select(if app.search_songs.is_empty() {
        None
    } else {
        Some(app.search_selected_song)
    });
    f.render_stateful_widget(songs_list, columns[2], &mut song_state);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn format_duration(seconds: i32) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{mins}:{secs:02}")
}
