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

    // Draw filter overlay if in filter mode
    if app.input_mode == InputMode::Filter {
        draw_filter_input(f, app);
    }

    // Draw search results if in search results mode
    if app.input_mode == InputMode::SearchResults {
        draw_search_results(f, app);
    }

    // Draw favorites if in favorites mode
    if app.input_mode == InputMode::Favorites {
        draw_favorites(f, app);
    }

    // Draw history if in history mode
    if app.input_mode == InputMode::History {
        draw_history(f, app);
    }

    // Draw statistics if in statistics mode
    if app.input_mode == InputMode::Statistics {
        draw_statistics(f, app);
    }

    // Draw playlists modal
    if app.input_mode == InputMode::Playlists {
        draw_playlists_modal(f, app);
    }

    // Draw help modal
    if app.input_mode == InputMode::Help {
        draw_help_modal(f, app);
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
        // Vertical layout: album art on top, song info, visualizer below
        let info_height = 6;
        let visualizer_height = 8;

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),                      // Album art space
                Constraint::Length(info_height),         // Song info
                Constraint::Length(visualizer_height),   // Visualizer
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

        // Render visualizer if enabled
        if app.config.ui.show_visualizer {
            draw_visualizer(f, app, layout[2]);
        }
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

fn draw_visualizer(f: &mut Frame, app: &App, area: Rect) {
    let bar_count = app.visualizer_bars.len();
    let bar_width = area.width as usize / bar_count.max(1);

    if bar_width == 0 {
        return;
    }

    // Create lines for each row of the visualizer
    let mut lines = vec![];
    for row in 0..area.height {
        let mut line_content = String::new();
        for (_i, &bar_height) in app.visualizer_bars.iter().enumerate() {
            // Calculate if this row should show the bar
            let bar_pixels = (bar_height * area.height as f32) as u16;
            let from_bottom = area.height - row - 1;

            let char = if from_bottom < bar_pixels {
                "█"
            } else {
                " "
            };

            // Repeat for bar width
            for _ in 0..bar_width {
                line_content.push_str(char);
            }
        }

        // Color gradient based on height
        let from_bottom = area.height - row - 1;
        let intensity = from_bottom as f32 / area.height.max(1) as f32;
        let color = if intensity > 0.7 {
            Color::Red
        } else if intensity > 0.4 {
            Color::Yellow
        } else {
            Color::Green
        };

        lines.push(Line::from(Span::styled(line_content, Style::default().fg(color))));
    }

    let visualizer_widget = Paragraph::new(lines)
        .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(visualizer_widget, area);
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

    // Center the selection in the viewport
    if !app.playlists.is_empty() {
        let visible_height = area.height.saturating_sub(2) as usize; // Subtract borders
        if app.selected_playlist > visible_height / 2 {
            let offset = app.selected_playlist.saturating_sub(visible_height / 2);
            *state.offset_mut() = offset;
        }
    }

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_help_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(70, 80, f.area());
    f.render_widget(ratatui::widgets::Clear, area);

    let kb = &app.config.keybindings;

    let help_text = vec![
        Line::from(Span::styled("Navigation", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))),
        Line::from(format!("  Tab, {}/{}    Switch panels", kb.focus_left, kb.focus_right)),
        Line::from("  j/k, ↑/↓     Navigate up/down"),
        Line::from("  g/G          Go to top/bottom"),
        Line::from("  Enter        Select / Play"),
        Line::from("  Esc          Back / Close"),
        Line::from(""),
        Line::from(Span::styled("Playback", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))),
        Line::from(format!("  {}            Play / Pause", if kb.play_pause == " " { "Space" } else { &kb.play_pause })),
        Line::from(format!("  {}            Next track", kb.next)),
        Line::from(format!("  {}            Previous track", kb.previous)),
        Line::from(format!("  {}/{}          Volume up/down", kb.volume_up, kb.volume_down)),
        Line::from(""),
        Line::from(Span::styled("Queue", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))),
        Line::from(format!("  {}            Add to queue", kb.enqueue)),
        Line::from(format!("  {}            Play next", kb.enqueue_next)),
        Line::from(format!("  {}            Add artist/album to queue", kb.enqueue_album)),
        Line::from("               (on Artists: all songs from artist, on Albums/Songs: all songs from album)"),
        Line::from(format!("  {}            Add to playlist", kb.add_to_playlist)),
        Line::from(format!("  {}            Toggle shuffle", kb.shuffle)),
        Line::from(format!("  {}            Cycle repeat mode", kb.repeat)),
        Line::from(format!("  {}            Start/stop radio (similar songs)", kb.radio)),
        Line::from("  ←/→          Seek -/+ 5s"),
        Line::from(""),
        Line::from(Span::styled("Queue Management (in Queue)", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))),
        Line::from(format!("  {}            Remove from queue", kb.remove_from_queue)),
        Line::from(format!("  {}            Clear queue", kb.clear_queue)),
        Line::from("  J/K          Move item down/up"),
        Line::from("  w            Save queue as playlist"),
        Line::from(""),
        Line::from(Span::styled("Views", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))),
        Line::from(format!("  {}            Search", kb.search)),
        Line::from("  P            Playlists"),
        Line::from(format!("  {}            Favorites", kb.favorites)),
        Line::from(format!("  {}            History", kb.history)),
        Line::from(format!("  {}            Statistics & Insights", kb.statistics)),
        Line::from(format!("  {}            Now Playing view", kb.now_playing)),
        Line::from(format!("  {}            This help", kb.help)),
        Line::from(""),
        Line::from(Span::styled("Filter & Sort", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))),
        Line::from(format!("  {}            Filter current panel", if kb.filter == "\\" { "\\\\" } else { &kb.filter })),
        Line::from("  c            Clear filter"),
        Line::from(format!("  {}            Cycle sort mode (focus panel first)", kb.sort)),
        Line::from("               Artists: A-Z → Z-A → Album Count"),
        Line::from("               Albums: A-Z → Z-A → Year → Artist"),
        Line::from("               Songs: Track → A-Z → Z-A → Artist → Album → Duration"),
        Line::from("               Current mode shown in panel title (e.g., Artists [A-Z])"),
        Line::from(""),
        Line::from(Span::styled("Favorites", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))),
        Line::from(format!("  {}            Star/unstar current item", kb.star)),
        Line::from(""),
        Line::from(Span::styled("Now Playing View", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))),
        Line::from("  Tab/t        Toggle lyrics/queue"),
        Line::from(format!("  Esc/{}/{}      Exit to main view", kb.now_playing, kb.quit)),
        Line::from("  j/k          Navigate queue"),
        Line::from("  Enter        Play from queue"),
        Line::from(""),
        Line::from(Span::styled("Other", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))),
        Line::from(format!("  {}            Quit", kb.quit)),
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

    // Center the selection in the viewport
    if !app.playlists.is_empty() {
        let visible_height = area.height.saturating_sub(2) as usize; // Subtract borders
        if app.selected_playlist > visible_height / 2 {
            let offset = app.selected_playlist.saturating_sub(visible_height / 2);
            *state.offset_mut() = offset;
        }
    }

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

    let filtered_artists = app.get_filtered_artists();
    let items: Vec<ListItem> = filtered_artists
        .iter()
        .map(|artist| {
            let star = if artist.starred.is_some() { "⭐ " } else { "" };
            ListItem::new(format!("{}{}", star, artist.name))
        })
        .collect();

    // Build title with sort mode and filter indicator
    use crate::app::ArtistSortMode;
    let sort_str = match app.artist_sort_mode {
        ArtistSortMode::NameAsc => "A-Z",
        ArtistSortMode::NameDesc => "Z-A",
        ArtistSortMode::AlbumCount => "Count",
    };
    let filter_str = if !app.filter_query.is_empty() {
        format!(" [Filter: {}]", app.filter_query)
    } else {
        String::new()
    };
    let title = format!("Artists [{}]{}", sort_str, filter_str);

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
    let clamped_selection = if filtered_artists.is_empty() {
        0
    } else {
        app.selected_artist.min(filtered_artists.len().saturating_sub(1))
    };
    state.select(if filtered_artists.is_empty() { None } else { Some(clamped_selection) });

    // Center the selection in the viewport
    if !filtered_artists.is_empty() {
        let visible_height = area.height.saturating_sub(2) as usize; // Subtract borders
        if clamped_selection > visible_height / 2 {
            let offset = clamped_selection.saturating_sub(visible_height / 2);
            *state.offset_mut() = offset;
        }
    }

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_albums_panel(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == FocusPanel::Albums;

    let filtered_albums = app.get_filtered_albums();
    let items: Vec<ListItem> = filtered_albums
        .iter()
        .map(|album| {
            let star = if album.starred.is_some() { "⭐ " } else { "" };
            let year = album.year.map(|y| format!(" ({})", y)).unwrap_or_default();
            ListItem::new(format!("{}{}{}", star, album.name, year))
        })
        .collect();

    // Build title with sort mode and filter indicator
    use crate::app::AlbumSortMode;
    let sort_str = match app.album_sort_mode {
        AlbumSortMode::NameAsc => "A-Z",
        AlbumSortMode::NameDesc => "Z-A",
        AlbumSortMode::Year => "Year",
        AlbumSortMode::Artist => "Artist",
    };
    let filter_str = if !app.filter_query.is_empty() {
        format!(" [Filter: {}]", app.filter_query)
    } else {
        String::new()
    };
    let title = if filtered_albums.is_empty() {
        format!("Albums [{}]{}", sort_str, filter_str)
    } else {
        format!("Albums ({}) [{}]{}", filtered_albums.len(), sort_str, filter_str)
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
    state.select(if filtered_albums.is_empty() { None } else { Some(app.selected_album.min(filtered_albums.len().saturating_sub(1))) });

    // Center the selection in the viewport
    if !filtered_albums.is_empty() {
        let visible_height = area.height.saturating_sub(2) as usize; // Subtract borders
        let clamped_selection = app.selected_album.min(filtered_albums.len().saturating_sub(1));
        if clamped_selection > visible_height / 2 {
            let offset = clamped_selection.saturating_sub(visible_height / 2);
            *state.offset_mut() = offset;
        }
    }

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_songs_panel(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == FocusPanel::Songs;

    let filtered_songs = app.get_filtered_songs();
    let items: Vec<ListItem> = filtered_songs
        .iter()
        .map(|song| {
            let star = if song.starred.is_some() { "⭐ " } else { "" };
            let duration = song
                .duration
                .map(|d| format_duration(d))
                .unwrap_or_default();

            // Show disc-track format for multi-disc albums
            let track = if let Some(disc) = song.disc_number {
                if disc > 1 {
                    song.track
                        .map(|t| format!("{}-{:02}. ", disc, t))
                        .unwrap_or_else(|| format!("{}. ", disc))
                } else {
                    song.track
                        .map(|t| format!("{:2}. ", t))
                        .unwrap_or_else(|| "".to_string())
                }
            } else {
                song.track
                    .map(|t| format!("{:2}. ", t))
                    .unwrap_or_else(|| "".to_string())
            };

            ListItem::new(format!("{}{}{} [{}]", star, track, song.title, duration))
        })
        .collect();

    // Build title with sort mode and filter indicator
    use crate::app::SongSortMode;
    let sort_str = match app.song_sort_mode {
        SongSortMode::Track => "Track",
        SongSortMode::TitleAsc => "A-Z",
        SongSortMode::TitleDesc => "Z-A",
        SongSortMode::Artist => "Artist",
        SongSortMode::Album => "Album",
        SongSortMode::Duration => "Duration",
    };
    let filter_str = if !app.filter_query.is_empty() {
        format!(" [Filter: {}]", app.filter_query)
    } else {
        String::new()
    };
    let title = if filtered_songs.is_empty() {
        format!("Songs [{}]{}", sort_str, filter_str)
    } else {
        format!("Songs ({}) [{}]{}", filtered_songs.len(), sort_str, filter_str)
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
    state.select(if filtered_songs.is_empty() { None } else { Some(app.selected_song.min(filtered_songs.len().saturating_sub(1))) });

    // Center the selection in the viewport
    if !filtered_songs.is_empty() {
        let visible_height = area.height.saturating_sub(2) as usize; // Subtract borders
        let clamped_selection = app.selected_song.min(filtered_songs.len().saturating_sub(1));
        if clamped_selection > visible_height / 2 {
            let offset = clamped_selection.saturating_sub(visible_height / 2);
            *state.offset_mut() = offset;
        }
    }

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

    let title = if focused {
        format!("Queue ({}) [d:delete x:clear J/K:move w:save]", app.player.queue.len())
    } else {
        format!("Queue ({})", app.player.queue.len())
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
    state.select(if app.player.queue.is_empty() {
        None
    } else {
        Some(app.selected_queue)
    });

    // Center the selection in the viewport
    if !app.player.queue.is_empty() {
        let visible_height = area.height.saturating_sub(2) as usize; // Subtract borders
        if app.selected_queue > visible_height / 2 {
            let offset = app.selected_queue.saturating_sub(visible_height / 2);
            *state.offset_mut() = offset;
        }
    }

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
    let radio_indicator = if app.radio_mode { " 📻" } else { "" };

    let status = if app.player.current_song.is_some() {
        if app.player.is_paused() { "⏸ " } else { "▶ " }
    } else {
        "  "
    };

    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black))
        .percent(progress)
        .label(format!(
            "{}{} / {} │ Vol: {}%{}{}{}",
            status,
            format_duration(position as i32),
            format_duration(duration as i32),
            app.player.volume,
            shuffle_indicator,
            repeat_indicator,
            radio_indicator
        ));

    f.render_widget(gauge, chunks[0]);

    // Help line
    let help = "Tab: panels │ j/k: nav │ Enter: play │ e: queue │ Space: pause │ /: search │ o: sort │ P: playlists │ f: now playing │ ?: help";
    let help_widget = Paragraph::new(help)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(ratatui::layout::Alignment::Center);
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

fn draw_filter_input(f: &mut Frame, app: &App) {
    // Small centered box for filter input
    let area = f.area();
    let width = 50.min(area.width.saturating_sub(4));
    let x = (area.width - width) / 2;
    let y = area.height / 3;

    let filter_area = Rect {
        x: area.x + x,
        y: area.y + y,
        width,
        height: 3,
    };

    // Clear the area
    f.render_widget(ratatui::widgets::Clear, filter_area);

    let input = Paragraph::new(app.filter_query.as_str())
        .style(Style::default().fg(Color::Cyan))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Filter (press Enter to apply, Esc to cancel)")
                .border_style(Style::default().fg(Color::Cyan)),
        );

    f.render_widget(input, filter_area);
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

    // Center the selection in the viewport
    if !app.search_artists.is_empty() {
        let visible_height = columns[0].height.saturating_sub(2) as usize; // Subtract borders
        if app.search_selected_artist > visible_height / 2 {
            let offset = app.search_selected_artist.saturating_sub(visible_height / 2);
            *artist_state.offset_mut() = offset;
        }
    }

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

    // Center the selection in the viewport
    if !app.search_albums.is_empty() {
        let visible_height = columns[1].height.saturating_sub(2) as usize; // Subtract borders
        if app.search_selected_album > visible_height / 2 {
            let offset = app.search_selected_album.saturating_sub(visible_height / 2);
            *album_state.offset_mut() = offset;
        }
    }

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

    // Center the selection in the viewport
    if !app.search_songs.is_empty() {
        let visible_height = columns[2].height.saturating_sub(2) as usize; // Subtract borders
        if app.search_selected_song > visible_height / 2 {
            let offset = app.search_selected_song.saturating_sub(visible_height / 2);
            *song_state.offset_mut() = offset;
        }
    }

    f.render_stateful_widget(songs_list, columns[2], &mut song_state);
}

fn draw_favorites(f: &mut Frame, app: &App) {
    use crate::app::FavoritesFocus;

    // Large overlay for favorites
    let area = centered_rect(80, 80, f.area());
    f.render_widget(ratatui::widgets::Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Favorites (S: unstar, Esc to close)")
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
        .favorites_artists
        .iter()
        .map(|a| ListItem::new(format!("⭐ {}", a.name)))
        .collect();

    let artists_focused = app.favorites_focus == FavoritesFocus::Artists;
    let artists_list = List::new(artist_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Artists ({})", app.favorites_artists.len()))
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
    artist_state.select(if app.favorites_artists.is_empty() {
        None
    } else {
        Some(app.favorites_selected_artist)
    });

    // Center the selection in the viewport
    if !app.favorites_artists.is_empty() {
        let visible_height = columns[0].height.saturating_sub(2) as usize;
        if app.favorites_selected_artist > visible_height / 2 {
            let offset = app.favorites_selected_artist.saturating_sub(visible_height / 2);
            *artist_state.offset_mut() = offset;
        }
    }

    f.render_stateful_widget(artists_list, columns[0], &mut artist_state);

    // Albums column
    let album_items: Vec<ListItem> = app
        .favorites_albums
        .iter()
        .map(|a| {
            let artist = a.artist.as_deref().unwrap_or("Unknown");
            ListItem::new(format!("⭐ {}\n  {}", a.name, artist))
        })
        .collect();

    let albums_focused = app.favorites_focus == FavoritesFocus::Albums;
    let albums_list = List::new(album_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Albums ({})", app.favorites_albums.len()))
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
    album_state.select(if app.favorites_albums.is_empty() {
        None
    } else {
        Some(app.favorites_selected_album)
    });

    // Center the selection in the viewport
    if !app.favorites_albums.is_empty() {
        let visible_height = columns[1].height.saturating_sub(2) as usize;
        if app.favorites_selected_album > visible_height / 2 {
            let offset = app.favorites_selected_album.saturating_sub(visible_height / 2);
            *album_state.offset_mut() = offset;
        }
    }

    f.render_stateful_widget(albums_list, columns[1], &mut album_state);

    // Songs column
    let song_items: Vec<ListItem> = app
        .favorites_songs
        .iter()
        .map(|s| {
            let artist = s.artist.as_deref().unwrap_or("Unknown");
            ListItem::new(format!("⭐ {}\n  {}", s.title, artist))
        })
        .collect();

    let songs_focused = app.favorites_focus == FavoritesFocus::Songs;
    let songs_list = List::new(song_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Songs ({})", app.favorites_songs.len()))
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
    song_state.select(if app.favorites_songs.is_empty() {
        None
    } else {
        Some(app.favorites_selected_song)
    });

    // Center the selection in the viewport
    if !app.favorites_songs.is_empty() {
        let visible_height = columns[2].height.saturating_sub(2) as usize;
        if app.favorites_selected_song > visible_height / 2 {
            let offset = app.favorites_selected_song.saturating_sub(visible_height / 2);
            *song_state.offset_mut() = offset;
        }
    }

    f.render_stateful_widget(songs_list, columns[2], &mut song_state);
}

fn draw_history(f: &mut Frame, app: &App) {
    use chrono::{DateTime, Local};

    // Large overlay for history
    let area = centered_rect(70, 80, f.area());
    f.render_widget(ratatui::widgets::Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Playback History (c: clear, Esc to close)")
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Create list items with timestamps
    let items: Vec<ListItem> = app
        .history
        .iter()
        .map(|entry| {
            let artist = entry.song.artist.as_deref().unwrap_or("Unknown");
            let datetime: DateTime<Local> = entry.played_at.into();
            let time_str = datetime.format("%H:%M:%S").to_string();
            let line1 = format!("{} - {}", entry.song.title, artist);
            let line2 = format!("  Played at: {}", time_str);
            ListItem::new(format!("{}\n{}", line1, line2))
        })
        .collect();

    let title = if app.history.is_empty() {
        "No playback history".to_string()
    } else {
        format!("{} songs played", app.history.len())
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    state.select(if app.history.is_empty() {
        None
    } else {
        Some(app.history_selected)
    });

    // Center the selection in the viewport
    if !app.history.is_empty() {
        let visible_height = inner.height.saturating_sub(2) as usize; // Subtract borders
        if app.history_selected > visible_height / 2 {
            let offset = app.history_selected.saturating_sub(visible_height / 2);
            *state.offset_mut() = offset;
        }
    }

    f.render_stateful_widget(list, inner, &mut state);
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

fn draw_statistics(f: &mut Frame, app: &App) {
    let area = centered_rect(90, 90, f.area());
    f.render_widget(ratatui::widgets::Clear, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Percentage(40),  // Top songs/albums/artists
            Constraint::Percentage(40),  // Recently added
            Constraint::Length(2),  // Summary
        ])
        .split(area);

    // Title
    let title = Paragraph::new("Statistics & Insights")
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow)))
        .style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(title, chunks[0]);

    // Top stats in columns
    let stats_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(chunks[1]);

    // Top Songs
    let top_songs: Vec<Line> = app.top_songs.iter().enumerate().map(|(i, stat)| {
        Line::from(format!("{}. {} ({}×)", i + 1, stat.song.title, stat.play_count))
    }).collect();
    let songs_widget = Paragraph::new(top_songs)
        .block(Block::default().borders(Borders::ALL).title("Top Songs"));
    f.render_widget(songs_widget, stats_chunks[0]);

    // Top Albums
    let top_albums: Vec<Line> = app.top_albums.iter().enumerate().map(|(i, stat)| {
        Line::from(format!("{}. {} - {} ({}×)", i + 1, stat.album_name, stat.artist_name, stat.play_count))
    }).collect();
    let albums_widget = Paragraph::new(top_albums)
        .block(Block::default().borders(Borders::ALL).title("Top Albums"));
    f.render_widget(albums_widget, stats_chunks[1]);

    // Top Artists
    let top_artists: Vec<Line> = app.top_artists.iter().enumerate().map(|(i, stat)| {
        Line::from(format!("{}. {} ({}×)", i + 1, stat.artist_name, stat.play_count))
    }).collect();
    let artists_widget = Paragraph::new(top_artists)
        .block(Block::default().borders(Borders::ALL).title("Top Artists"));
    f.render_widget(artists_widget, stats_chunks[2]);

    // Recently Added Albums
    let recent: Vec<Line> = app.recently_added_albums.iter().take(10).map(|album| {
        let artist = album.artist.as_deref().unwrap_or("Unknown");
        Line::from(format!("{} - {}", album.name, artist))
    }).collect();
    let recent_widget = Paragraph::new(recent)
        .block(Block::default().borders(Borders::ALL).title("Recently Added Albums"));
    f.render_widget(recent_widget, chunks[2]);

    // Summary
    let hours = app.total_playtime_minutes / 60;
    let mins = app.total_playtime_minutes % 60;
    let summary = Paragraph::new(format!(
        "Total Playtime: {}h {}m  │  History Entries: {}  │  Press Esc or I to close",
        hours, mins, app.history.len()
    ))
    .block(Block::default().borders(Borders::ALL))
    .style(Style::default().fg(Color::DarkGray));
    f.render_widget(summary, chunks[3]);
}

fn format_duration(seconds: i32) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{mins}:{secs:02}")
}
