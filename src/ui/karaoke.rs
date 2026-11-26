//! Karaoke lyrics view: the current line in big block letters with a fill that sweeps across it as it is sung,
//! the previous line above and the next few lines below.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};
use tui_big_text::{BigText, PixelSize};

use crate::app::App;

/// Big-text sizes, largest first, with the size of one glyph in terminal cells (width, height).
/// `Full` (8x8 cells) is left out: with tall terminal cells it looks stretched; `HalfHeight` keeps letters in proportion.
const SIZES: [(PixelSize, u16, u16); 3] = [
    (PixelSize::HalfHeight, 8, 4),
    (PixelSize::Quadrant, 4, 4),
    (PixelSize::Sextant, 4, 3),
];

/// Seconds per character used to estimate how long a line takes to sing (LRC only times the start of each line).
const SECS_PER_CHAR: f64 = 0.09;
/// Show a 3-2-1 countdown when the next line starts within this many seconds after a gap.
const COUNTDOWN_SECS: f64 = 3.0;

/// Where playback is within synced lyrics.
#[derive(Debug, PartialEq)]
pub(crate) struct Progress {
    /// Index of the line being sung, or None before the first line.
    pub line: Option<usize>,
    /// How much of that line has been sung, 0.0..=1.0.
    pub fill: f64,
    /// Seconds until the next line starts, if there is one.
    pub next_in: Option<f64>,
}

pub(crate) fn progress(stamps: &[f64], lines: &[String], pos: f64, duration: f64) -> Progress {
    let line = stamps.iter().rposition(|&t| pos >= t);
    let next_t = match line {
        Some(i) => stamps.get(i + 1).copied(),
        None => stamps.first().copied(),
    };
    let fill = match line {
        Some(i) => {
            let start = stamps[i];
            let end = next_t.unwrap_or(if duration > start { duration } else { start + 5.0 });
            let chars = lines.get(i).map(|l| l.trim().chars().count()).unwrap_or(0) as f64;
            // Sung length: about SECS_PER_CHAR per character, at least a second, never past the next line.
            let sung = (chars * SECS_PER_CHAR).max(1.0).min((end - start).max(0.1));
            ((pos - start) / sung).clamp(0.0, 1.0)
        }
        None => 0.0,
    };
    Progress { line, fill, next_in: next_t.map(|t| t - pos) }
}

/// Replace typographic punctuation with ASCII so the 8x8 font (ASCII only) can draw it.
/// Returns None if anything else non-ASCII remains (accents, other scripts): those lines use normal text.
pub(crate) fn big_text_safe(s: &str) -> Option<String> {
    let mut out = String::with_capacity(s.len());
    for c in s.trim().chars() {
        match c {
            '\u{2018}' | '\u{2019}' | '\u{02BC}' => out.push('\''),
            '\u{201C}' | '\u{201D}' => out.push('"'),
            '\u{2013}' | '\u{2014}' => out.push('-'),
            '\u{2026}' => out.push_str("..."),
            '\u{00A0}' => out.push(' '),
            c if c.is_ascii() && !c.is_ascii_control() => out.push(c),
            _ => return None,
        }
    }
    Some(out)
}

/// Greedy word wrap to `width` characters per row; words longer than a row are split.
pub(crate) fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut rows: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        let mut word = word.to_string();
        while word.chars().count() > width {
            if !cur.is_empty() {
                rows.push(std::mem::take(&mut cur));
            }
            let head: String = word.chars().take(width).collect();
            word = word.chars().skip(width).collect();
            rows.push(head);
        }
        if word.is_empty() {
            continue;
        }
        let needed = if cur.is_empty() { word.chars().count() } else { cur.chars().count() + 1 + word.chars().count() };
        if needed <= width {
            if !cur.is_empty() {
                cur.push(' ');
            }
            cur.push_str(&word);
        } else {
            rows.push(std::mem::replace(&mut cur, word));
        }
    }
    if !cur.is_empty() {
        rows.push(cur);
    }
    rows
}

/// Largest big-text size at which `text` fits in `w`x`h` cells: (size, glyph height, wrapped rows).
fn fit(text: &str, w: u16, h: u16) -> Option<(PixelSize, u16, Vec<String>)> {
    SIZES.iter().find_map(|&(size, gw, gh)| {
        let cols = (w / gw) as usize;
        if cols < 4 {
            return None;
        }
        let rows = wrap(text, cols);
        (rows.len() as u16 * gh <= h).then_some((size, gh, rows))
    })
}

/// Split rows into sung/unsung spans: the first `sung` characters (across rows) get `hi`, the rest `lo`.
fn fill_lines(rows: &[String], sung: usize, hi: Style, lo: Style) -> Vec<Line<'static>> {
    let mut left = sung;
    rows.iter()
        .map(|row| {
            let n = row.chars().count();
            let k = left.min(n);
            left -= k;
            let a: String = row.chars().take(k).collect();
            let b: String = row.chars().skip(k).collect();
            Line::from(vec![Span::styled(a, hi), Span::styled(b, lo)])
        })
        .collect()
}

fn accent(app: &App) -> Color {
    app.config
        .ui
        .accent_color
        .as_deref()
        .and_then(|s| {
            let h = s.trim().trim_start_matches('#');
            if h.len() != 6 {
                return None;
            }
            let v = u32::from_str_radix(h, 16).ok()?;
            Some(Color::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
        })
        .unwrap_or(Color::Cyan)
}

fn centered(text: impl Into<Line<'static>>) -> Paragraph<'static> {
    Paragraph::new(text.into()).alignment(Alignment::Center)
}

/// Draw `rows` as big text (with the karaoke fill) centred in `area`; falls back to wrapped bold text.
fn draw_big(f: &mut Frame, area: Rect, text: &str, fill: f64, hi: Style, lo: Style) {
    let chars: usize = text.chars().filter(|c| !c.is_whitespace()).count();
    let sung_of = |rows: &[String]| {
        // Count sung characters over the wrapped rows (spaces at row breaks are dropped by wrapping).
        let total: usize = rows.iter().map(|r| r.chars().count()).sum();
        let ratio = if chars == 0 { 0.0 } else { fill };
        (ratio * total as f64).round() as usize
    };
    if let Some(safe) = big_text_safe(text) {
        if let Some((size, gh, rows)) = fit(&safe, area.width, area.height) {
            let h = rows.len() as u16 * gh;
            let y = area.y + (area.height - h) / 2;
            let lines = fill_lines(&rows, sung_of(&rows), hi, lo);
            let big = BigText::builder().pixel_size(size).lines(lines).centered().build();
            f.render_widget(big, Rect::new(area.x, y, area.width, h));
            return;
        }
    }
    // Normal text fallback (non-ASCII line, or a terminal too small for big text).
    let rows = wrap(text, area.width.max(1) as usize);
    let h = (rows.len() as u16).min(area.height);
    let y = area.y + (area.height - h) / 2;
    let lines = fill_lines(&rows, sung_of(&rows), hi.add_modifier(Modifier::BOLD), lo.add_modifier(Modifier::BOLD));
    f.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center).wrap(Wrap { trim: true }),
        Rect::new(area.x, y, area.width, h),
    );
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let hi = Style::default().fg(accent(app));
    let lo = Style::default().fg(Color::Gray);
    let dim = Style::default().fg(Color::DarkGray);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    // Header: song, and the lyrics offset when it isn't zero.
    let mut header = match &app.player.current_song {
        Some(s) => format!("♪ {} — {}", s.title, s.artist.as_deref().unwrap_or("Unknown artist")),
        None => "Nothing playing".to_string(),
    };
    if app.lyrics_offset != 0.0 {
        header.push_str(&format!("   (lyrics {:+.2}s)", app.lyrics_offset));
    }
    if !app.lyrics_lines.is_empty() && app.lyrics_timestamps.is_empty() {
        header.push_str("   (unsynced lyrics: no karaoke timing)");
    }
    f.render_widget(centered(Span::styled(header, dim)), chunks[0]);
    let body = chunks[2];

    if app.lyrics_lines.is_empty() {
        let msg = if app.player.current_song.is_some() { "No lyrics for this song" } else { "" };
        draw_big(f, body, "~ ~ ~", 0.0, lo, dim);
        let y = body.y + body.height.saturating_sub(2);
        f.render_widget(centered(Span::styled(msg, dim)), Rect::new(body.x, y, body.width, 1));
        return;
    }

    // Unsynced lyrics: a plain scrolling list around the estimated line.
    if app.lyrics_timestamps.is_empty() {
        let cur = app.current_lyrics_line();
        let half = body.height as usize / 2;
        let lines: Vec<Line> = app
            .lyrics_lines
            .iter()
            .enumerate()
            .skip(cur.saturating_sub(half))
            .take(body.height as usize)
            .map(|(i, l)| Line::from(Span::styled(l.clone(), if i == cur { hi.add_modifier(Modifier::BOLD) } else { dim })))
            .collect();
        f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), body);
        return;
    }

    let p = progress(&app.lyrics_timestamps, &app.lyrics_lines, app.lyrics_position(), app.player.duration());
    let line_at = |i: usize| app.lyrics_lines.get(i).map(|s| s.trim().to_string()).unwrap_or_default();

    // previous (1) · gap (1) · big text (rest) · gap (1) · next lines (3)
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(body);

    // Previous non-empty line, dimmed.
    if let Some(i) = p.line {
        if let Some(prev) = (0..i).rev().map(line_at).find(|l| !l.is_empty()) {
            f.render_widget(centered(Span::styled(prev, dim)), rows[0]);
        }
    }

    // Current line, or a countdown / rest marker during intros and instrumental breaks.
    let current = p.line.map(line_at).unwrap_or_default();
    if !current.is_empty() {
        draw_big(f, rows[2], &current, p.fill, hi, lo);
    } else if let Some(n) = p.next_in.filter(|&n| n > 0.0 && n <= COUNTDOWN_SECS) {
        draw_big(f, rows[2], &format!("{}", n.ceil() as u32), 1.0, hi, lo);
    } else {
        draw_big(f, rows[2], "~ ~ ~", 0.0, lo, dim);
    }

    // Next three non-empty lines, fading out.
    let start = p.line.map(|i| i + 1).unwrap_or(0);
    let upcoming: Vec<String> = (start..app.lyrics_lines.len()).map(line_at).filter(|l| !l.is_empty()).take(3).collect();
    let styles = [Style::default().fg(Color::White), lo, dim];
    let lines: Vec<Line> = upcoming
        .into_iter()
        .zip(styles)
        .map(|(l, s)| Line::from(Span::styled(l, s)))
        .collect();
    f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), rows[4]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_greedy_and_splits_long_words() {
        assert_eq!(wrap("the quick brown fox", 9), vec!["the quick", "brown fox"]);
        assert_eq!(wrap("abcdefghij", 4), vec!["abcd", "efgh", "ij"]);
        assert_eq!(wrap("   ", 5), Vec::<String>::new());
    }

    #[test]
    fn big_text_safe_maps_punctuation_and_rejects_other_scripts() {
        assert_eq!(big_text_safe("Don\u{2019}t stop\u{2026}").as_deref(), Some("Don't stop..."));
        assert_eq!(big_text_safe("caf\u{00E9}"), None);
    }

    #[test]
    fn progress_tracks_line_and_fill() {
        let stamps = [10.0, 20.0, 22.0];
        let lines: Vec<String> = ["a line of twenty chr", "", "last"].iter().map(|s| s.to_string()).collect();
        // Before the first line: no line, counting down.
        let p = progress(&stamps, &lines, 8.5, 100.0);
        assert_eq!(p.line, None);
        assert_eq!(p.next_in, Some(1.5));
        // 20 chars * 0.09 = 1.8 s sung; 0.9 s in = half filled.
        let p = progress(&stamps, &lines, 10.9, 100.0);
        assert_eq!(p.line, Some(0));
        assert!((p.fill - 0.5).abs() < 1e-9);
        // Fill never passes the next line's start; after it, fully sung.
        assert_eq!(progress(&stamps, &lines, 15.0, 100.0).fill, 1.0);
        // Last line: at least 1 s to sing.
        let p = progress(&stamps, &lines, 22.5, 100.0);
        assert_eq!(p.line, Some(2));
        assert!((p.fill - 0.5).abs() < 1e-9);
        assert_eq!(p.next_in, None);
    }
}
