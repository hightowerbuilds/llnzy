use std::sync::mpsc;

use alacritty_terminal::event::{Event as TermEvent, EventListener};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Point};
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::term::{self, Config as TermConfig, Term, TermMode};
use alacritty_terminal::vte::ansi::{Color as AnsiColor, NamedColor, Processor};

use crate::config::{indexed_color, Config};

/// Terminal events forwarded to the main thread.
#[derive(Debug)]
pub enum TerminalEvent {
    Title(String),
    ResetTitle,
    Bell,
    ClipboardStore(String),
    PtyWrite(String),
    ChildExit(i32),
}

/// Event listener that forwards terminal events through a channel.
struct EventProxy {
    tx: mpsc::Sender<TerminalEvent>,
}

impl EventListener for EventProxy {
    fn send_event(&self, event: TermEvent) {
        let mapped = match event {
            TermEvent::Title(t) => Some(TerminalEvent::Title(t)),
            TermEvent::ResetTitle => Some(TerminalEvent::ResetTitle),
            TermEvent::Bell => Some(TerminalEvent::Bell),
            TermEvent::ClipboardStore(_, s) => Some(TerminalEvent::ClipboardStore(s)),
            TermEvent::PtyWrite(s) => Some(TerminalEvent::PtyWrite(s)),
            TermEvent::ChildExit(status) => {
                let code = status.code().unwrap_or(-1);
                Some(TerminalEvent::ChildExit(code))
            }
            _ => None,
        };
        if let Some(ev) = mapped {
            let _ = self.tx.send(ev);
        }
    }
}

/// Size information for the terminal.
#[derive(Clone, Copy)]
pub struct TermSize {
    cols: usize,
    rows: usize,
}

impl TermSize {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self { cols, rows }
    }
}

impl Dimensions for TermSize {
    fn total_lines(&self) -> usize {
        self.rows
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.cols
    }
}

pub struct Terminal {
    term: Term<EventProxy>,
    processor: Processor,
    event_rx: mpsc::Receiver<TerminalEvent>,
}

impl Terminal {
    pub fn new(cols: u16, rows: u16) -> Self {
        let config = TermConfig::default();
        let size = TermSize::new(cols as usize, rows as usize);
        let (tx, rx) = mpsc::channel();
        let term = Term::new(config, &size, EventProxy { tx });
        let processor = Processor::new();

        Terminal {
            term,
            processor,
            event_rx: rx,
        }
    }

    /// Drain pending terminal events (title changes, bell, clipboard, etc.)
    pub fn drain_events(&self) -> Vec<TerminalEvent> {
        let mut events = Vec::new();
        while let Ok(ev) = self.event_rx.try_recv() {
            events.push(ev);
        }
        events
    }

    /// Feed raw bytes from the PTY into the terminal emulator.
    pub fn process(&mut self, bytes: &[u8]) {
        self.processor.advance(&mut self.term, bytes);
    }

    /// Resize the terminal grid.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        let size = TermSize::new(cols as usize, rows as usize);
        self.term.resize(size);
    }

    /// Get the number of columns and rows.
    pub fn size(&self) -> (usize, usize) {
        let grid = self.term.grid();
        (grid.columns(), grid.screen_lines())
    }

    // --- Scrollback ---

    /// Scroll the display by delta lines (positive = up into history).
    pub fn scroll(&mut self, delta: i32) {
        self.term.scroll_display(Scroll::Delta(delta));
    }

    /// Scroll one page up.
    pub fn scroll_page_up(&mut self) {
        self.term.scroll_display(Scroll::PageUp);
    }

    /// Scroll one page down.
    pub fn scroll_page_down(&mut self) {
        self.term.scroll_display(Scroll::PageDown);
    }

    /// Scroll to the bottom (latest output).
    pub fn scroll_to_bottom(&mut self) {
        self.term.scroll_display(Scroll::Bottom);
    }

    // --- Cell access (viewport-aware) ---

    /// Get a reference to the cell at the given viewport position,
    /// accounting for display_offset (scrollback).
    pub fn cell(&self, row: usize, col: usize) -> &Cell {
        let display_offset = self.term.grid().display_offset();
        let point = term::viewport_to_point(display_offset, Point::new(row, Column(col)));
        &self.term.grid()[point]
    }

    /// Get the character at a given viewport cell.
    pub fn cell_char(&self, row: usize, col: usize) -> char {
        self.cell(row, col).c
    }

    /// Get the cell flags (bold, italic, underline, etc.)
    pub fn cell_flags(&self, row: usize, col: usize) -> Flags {
        self.cell(row, col).flags
    }

    /// Check if a cell has the INVERSE flag and swap fg/bg accordingly.
    pub fn resolve_fg_with_attrs(&self, row: usize, col: usize, config: &Config) -> [u8; 3] {
        let cell = self.cell(row, col);
        let inverse = cell.flags.contains(Flags::INVERSE);
        if inverse {
            resolve_color(&cell.bg, config, false)
        } else {
            resolve_color(&cell.fg, config, true)
        }
    }

    pub fn resolve_bg_with_attrs(&self, row: usize, col: usize, config: &Config) -> [u8; 3] {
        let cell = self.cell(row, col);
        let inverse = cell.flags.contains(Flags::INVERSE);
        if inverse {
            resolve_color(&cell.fg, config, true)
        } else {
            resolve_color(&cell.bg, config, false)
        }
    }

    /// Collect decoration rects (underlines, strikethrough) for the visible grid.
    pub fn decoration_rects(
        &self,
        config: &Config,
        cell_w: f32,
        cell_h: f32,
    ) -> Vec<(f32, f32, f32, f32, [f32; 4])> {
        let (cols, rows) = self.size();
        let mut rects = Vec::new();

        for row in 0..rows {
            for col in 0..cols {
                let flags = self.cell_flags(row, col);
                let fg = self.resolve_fg_with_attrs(row, col, config);
                let color = [
                    fg[0] as f32 / 255.0,
                    fg[1] as f32 / 255.0,
                    fg[2] as f32 / 255.0,
                    1.0,
                ];
                let x = col as f32 * cell_w;
                let y = row as f32 * cell_h;

                // Underline variants
                if flags.contains(Flags::UNDERLINE) {
                    rects.push((x, y + cell_h - 2.0, cell_w, 1.0, color));
                } else if flags.contains(Flags::DOUBLE_UNDERLINE) {
                    rects.push((x, y + cell_h - 4.0, cell_w, 1.0, color));
                    rects.push((x, y + cell_h - 1.0, cell_w, 1.0, color));
                } else if flags.contains(Flags::UNDERCURL) {
                    // Approximate curly underline with stepped segments
                    let segments = 4;
                    let seg_w = cell_w / segments as f32;
                    for i in 0..segments {
                        let offset = if i % 2 == 0 { -1.5 } else { 0.5 };
                        rects.push((
                            x + i as f32 * seg_w,
                            y + cell_h - 2.0 + offset,
                            seg_w,
                            1.0,
                            color,
                        ));
                    }
                } else if flags.contains(Flags::DOTTED_UNDERLINE) {
                    // Dotted: alternating small rects
                    let dot_w = (cell_w / 4.0).max(1.0);
                    let mut dx = 0.0;
                    while dx < cell_w {
                        rects.push((x + dx, y + cell_h - 2.0, dot_w, 1.0, color));
                        dx += dot_w * 2.0;
                    }
                } else if flags.contains(Flags::DASHED_UNDERLINE) {
                    // Dashed: longer segments with gaps
                    let dash_w = (cell_w / 2.0).max(1.0);
                    rects.push((x, y + cell_h - 2.0, dash_w, 1.0, color));
                }

                // Strikethrough
                if flags.contains(Flags::STRIKEOUT) {
                    rects.push((x, y + cell_h * 0.5, cell_w, 1.0, color));
                }
            }
        }

        rects
    }

    /// Get the hyperlink URI for a cell, if any (OSC 8).
    pub fn cell_hyperlink(&self, row: usize, col: usize) -> Option<String> {
        self.cell(row, col).hyperlink().map(|h| h.uri().to_string())
    }

    // --- Terminal mode queries ---

    pub fn mouse_mode(&self) -> bool {
        self.term.mode().intersects(TermMode::MOUSE_MODE)
    }

    pub fn sgr_mouse(&self) -> bool {
        self.term.mode().contains(TermMode::SGR_MOUSE)
    }

    pub fn bracketed_paste(&self) -> bool {
        self.term.mode().contains(TermMode::BRACKETED_PASTE)
    }

    pub fn app_cursor(&self) -> bool {
        self.term.mode().contains(TermMode::APP_CURSOR)
    }

    /// Get the cursor position in viewport coordinates, or None if not visible.
    pub fn cursor_point(&self) -> Option<(usize, usize)> {
        let cursor = self.term.grid().cursor.point;
        let display_offset = self.term.grid().display_offset();
        term::point_to_viewport(display_offset, cursor).map(|p| (p.line, p.column.0))
    }

    /// Collect background rects for cells with non-default backgrounds.
    pub fn background_rects(
        &self,
        config: &Config,
        cell_w: f32,
        cell_h: f32,
    ) -> Vec<(f32, f32, f32, f32, [f32; 4])> {
        let (cols, rows) = self.size();
        let bg_f = config.bg();
        let default_bg = [
            (bg_f[0] * 255.0) as u8,
            (bg_f[1] * 255.0) as u8,
            (bg_f[2] * 255.0) as u8,
        ];
        let mut rects = Vec::new();

        for row in 0..rows {
            let mut col = 0;
            while col < cols {
                let bg = self.resolve_bg_with_attrs(row, col, config);
                if bg != default_bg {
                    // Batch consecutive cells with the same background
                    let start_col = col;
                    while col < cols && self.resolve_bg_with_attrs(row, col, config) == bg {
                        col += 1;
                    }
                    let x = start_col as f32 * cell_w;
                    let y = row as f32 * cell_h;
                    let w = (col - start_col) as f32 * cell_w;
                    let color = [
                        bg[0] as f32 / 255.0,
                        bg[1] as f32 / 255.0,
                        bg[2] as f32 / 255.0,
                        1.0,
                    ];
                    rects.push((x, y, w, cell_h, color));
                } else {
                    col += 1;
                }
            }
        }

        rects
    }
}

fn resolve_color(color: &AnsiColor, config: &Config, is_fg: bool) -> [u8; 3] {
    match color {
        AnsiColor::Named(named) => resolve_named(*named, config, is_fg),
        AnsiColor::Spec(rgb) => [rgb.r, rgb.g, rgb.b],
        AnsiColor::Indexed(idx) => indexed_color(*idx, &config.colors),
    }
}

fn resolve_named(named: NamedColor, config: &Config, is_fg: bool) -> [u8; 3] {
    let scheme = &config.colors;
    match named {
        NamedColor::Black => scheme.ansi[0],
        NamedColor::Red => scheme.ansi[1],
        NamedColor::Green => scheme.ansi[2],
        NamedColor::Yellow => scheme.ansi[3],
        NamedColor::Blue => scheme.ansi[4],
        NamedColor::Magenta => scheme.ansi[5],
        NamedColor::Cyan => scheme.ansi[6],
        NamedColor::White => scheme.ansi[7],
        NamedColor::BrightBlack => scheme.ansi[8],
        NamedColor::BrightRed => scheme.ansi[9],
        NamedColor::BrightGreen => scheme.ansi[10],
        NamedColor::BrightYellow => scheme.ansi[11],
        NamedColor::BrightBlue => scheme.ansi[12],
        NamedColor::BrightMagenta => scheme.ansi[13],
        NamedColor::BrightCyan => scheme.ansi[14],
        NamedColor::BrightWhite => scheme.ansi[15],
        NamedColor::Foreground => {
            if is_fg {
                scheme.foreground
            } else {
                scheme.background
            }
        }
        NamedColor::Background => {
            if is_fg {
                scheme.foreground
            } else {
                scheme.background
            }
        }
        _ => {
            if is_fg {
                scheme.foreground
            } else {
                scheme.background
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Terminal creation and sizing ──

    #[test]
    fn create_terminal() {
        let term = Terminal::new(80, 24);
        assert_eq!(term.size(), (80, 24));
    }

    #[test]
    fn create_terminal_small() {
        let term = Terminal::new(10, 5);
        assert_eq!(term.size(), (10, 5));
    }

    #[test]
    fn resize_terminal() {
        let mut term = Terminal::new(80, 24);
        term.resize(120, 40);
        assert_eq!(term.size(), (120, 40));
    }

    // ── Cell access ──

    #[test]
    fn empty_cells_are_space() {
        let term = Terminal::new(80, 24);
        assert_eq!(term.cell_char(0, 0), ' ');
        assert_eq!(term.cell_char(23, 79), ' ');
    }

    #[test]
    fn process_text_sets_cells() {
        let mut term = Terminal::new(80, 24);
        term.process(b"Hello");
        assert_eq!(term.cell_char(0, 0), 'H');
        assert_eq!(term.cell_char(0, 1), 'e');
        assert_eq!(term.cell_char(0, 2), 'l');
        assert_eq!(term.cell_char(0, 3), 'l');
        assert_eq!(term.cell_char(0, 4), 'o');
        assert_eq!(term.cell_char(0, 5), ' '); // unwritten
    }

    #[test]
    fn process_newline_moves_to_next_row() {
        let mut term = Terminal::new(80, 24);
        term.process(b"A\r\nB");
        assert_eq!(term.cell_char(0, 0), 'A');
        assert_eq!(term.cell_char(1, 0), 'B');
    }

    // ── Cursor position ──

    #[test]
    fn cursor_starts_at_origin() {
        let term = Terminal::new(80, 24);
        assert_eq!(term.cursor_point(), Some((0, 0)));
    }

    #[test]
    fn cursor_advances_with_text() {
        let mut term = Terminal::new(80, 24);
        term.process(b"ABC");
        assert_eq!(term.cursor_point(), Some((0, 3)));
    }

    // ── Terminal modes ──

    #[test]
    fn default_modes() {
        let term = Terminal::new(80, 24);
        assert!(!term.app_cursor());
        assert!(!term.mouse_mode());
        assert!(!term.sgr_mouse());
        assert!(!term.bracketed_paste());
    }

    #[test]
    fn enable_app_cursor_mode() {
        let mut term = Terminal::new(80, 24);
        term.process(b"\x1b[?1h"); // DECCKM on
        assert!(term.app_cursor());
    }

    #[test]
    fn disable_app_cursor_mode() {
        let mut term = Terminal::new(80, 24);
        term.process(b"\x1b[?1h"); // on
        term.process(b"\x1b[?1l"); // off
        assert!(!term.app_cursor());
    }

    #[test]
    fn enable_bracketed_paste() {
        let mut term = Terminal::new(80, 24);
        term.process(b"\x1b[?2004h");
        assert!(term.bracketed_paste());
    }

    #[test]
    fn enable_mouse_mode() {
        let mut term = Terminal::new(80, 24);
        term.process(b"\x1b[?1000h"); // X10 mouse
        assert!(term.mouse_mode());
    }

    #[test]
    fn enable_sgr_mouse() {
        let mut term = Terminal::new(80, 24);
        term.process(b"\x1b[?1006h");
        assert!(term.sgr_mouse());
    }

    // ── Events ──

    #[test]
    fn title_event_from_osc() {
        let mut term = Terminal::new(80, 24);
        term.process(b"\x1b]0;My Title\x07");
        let events = term.drain_events();
        let has_title = events
            .iter()
            .any(|e| matches!(e, TerminalEvent::Title(t) if t == "My Title"));
        assert!(has_title);
    }

    #[test]
    fn bell_event() {
        let mut term = Terminal::new(80, 24);
        term.process(b"\x07");
        let events = term.drain_events();
        let has_bell = events.iter().any(|e| matches!(e, TerminalEvent::Bell));
        assert!(has_bell);
    }

    #[test]
    fn drain_events_empties_queue() {
        let mut term = Terminal::new(80, 24);
        term.process(b"\x07"); // bell
        let events = term.drain_events();
        assert!(!events.is_empty());
        let events2 = term.drain_events();
        assert!(events2.is_empty());
    }

    // ── Color resolution ──

    #[test]
    fn resolve_named_ansi_colors() {
        let config = Config::default();
        assert_eq!(
            resolve_named(NamedColor::Red, &config, true),
            config.colors.ansi[1]
        );
        assert_eq!(
            resolve_named(NamedColor::Blue, &config, true),
            config.colors.ansi[4]
        );
        assert_eq!(
            resolve_named(NamedColor::BrightWhite, &config, true),
            config.colors.ansi[15]
        );
    }

    #[test]
    fn resolve_named_foreground() {
        let config = Config::default();
        assert_eq!(
            resolve_named(NamedColor::Foreground, &config, true),
            config.colors.foreground
        );
        assert_eq!(
            resolve_named(NamedColor::Foreground, &config, false),
            config.colors.background
        );
    }

    #[test]
    fn resolve_color_spec_rgb() {
        let config = Config::default();
        let rgb = alacritty_terminal::vte::ansi::Rgb {
            r: 100,
            g: 150,
            b: 200,
        };
        let color = AnsiColor::Spec(rgb);
        assert_eq!(resolve_color(&color, &config, true), [100, 150, 200]);
    }

    #[test]
    fn resolve_color_indexed() {
        let config = Config::default();
        let color = AnsiColor::Indexed(1); // red
        assert_eq!(resolve_color(&color, &config, true), config.colors.ansi[1]);
    }

    // ── Cell flags after ANSI processing ──

    #[test]
    fn bold_flag_set() {
        let mut term = Terminal::new(80, 24);
        term.process(b"\x1b[1mX");
        let flags = term.cell_flags(0, 0);
        assert!(flags.contains(Flags::BOLD));
    }

    #[test]
    fn italic_flag_set() {
        let mut term = Terminal::new(80, 24);
        term.process(b"\x1b[3mX");
        let flags = term.cell_flags(0, 0);
        assert!(flags.contains(Flags::ITALIC));
    }

    #[test]
    fn underline_flag_set() {
        let mut term = Terminal::new(80, 24);
        term.process(b"\x1b[4mX");
        let flags = term.cell_flags(0, 0);
        assert!(flags.contains(Flags::UNDERLINE));
    }

    #[test]
    fn inverse_flag_set() {
        let mut term = Terminal::new(80, 24);
        term.process(b"\x1b[7mX");
        let flags = term.cell_flags(0, 0);
        assert!(flags.contains(Flags::INVERSE));
    }

    #[test]
    fn strikeout_flag_set() {
        let mut term = Terminal::new(80, 24);
        term.process(b"\x1b[9mX");
        let flags = term.cell_flags(0, 0);
        assert!(flags.contains(Flags::STRIKEOUT));
    }

    // ── Scrollback ──

    #[test]
    fn scroll_to_bottom_after_scroll_up() {
        let mut term = Terminal::new(80, 24);
        // Fill terminal to create scrollback
        for _ in 0..30 {
            term.process(b"\r\n");
        }
        term.scroll(5); // scroll up
        term.scroll_to_bottom();
        // Cursor should be visible again
        assert!(term.cursor_point().is_some());
    }

    // ── Foreground/background with inverse ──

    #[test]
    fn resolve_fg_bg_normal() {
        let mut term = Terminal::new(80, 24);
        let config = Config::default();
        term.process(b"X");
        let fg = term.resolve_fg_with_attrs(0, 0, &config);
        let bg = term.resolve_bg_with_attrs(0, 0, &config);
        // Normal cell: fg = scheme foreground, bg = scheme background
        assert_eq!(fg, config.colors.foreground);
        assert_eq!(bg, config.colors.background);
    }

    #[test]
    fn resolve_fg_bg_inverse_swaps() {
        let mut term = Terminal::new(80, 24);
        let config = Config::default();
        term.process(b"\x1b[7mX"); // INVERSE
        let fg = term.resolve_fg_with_attrs(0, 0, &config);
        let bg = term.resolve_bg_with_attrs(0, 0, &config);
        // Inverse: fg and bg are swapped
        assert_eq!(fg, config.colors.background);
        assert_eq!(bg, config.colors.foreground);
    }
}
