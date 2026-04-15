use alacritty_terminal::event::EventListener;
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Point};
use alacritty_terminal::term::cell::Cell;
use alacritty_terminal::term::{self, Config as TermConfig, Term};
use alacritty_terminal::vte::ansi::{Color as AnsiColor, NamedColor, Processor};

use crate::config::{indexed_color, Config};

/// Event listener that discards terminal events (bell, title change, etc.)
struct EventProxy;

impl EventListener for EventProxy {
    fn send_event(&self, _event: alacritty_terminal::event::Event) {}
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
}

impl Terminal {
    pub fn new(cols: u16, rows: u16) -> Self {
        let config = TermConfig::default();
        let size = TermSize::new(cols as usize, rows as usize);
        let term = Term::new(config, &size, EventProxy);
        let processor = Processor::new();

        Terminal { term, processor }
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

    /// Current scroll offset (0 = at bottom).
    pub fn display_offset(&self) -> usize {
        self.term.grid().display_offset()
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

    /// Resolve a cell's foreground color to RGB.
    pub fn resolve_fg(&self, row: usize, col: usize, config: &Config) -> [u8; 3] {
        let cell = self.cell(row, col);
        resolve_color(&cell.fg, config, true)
    }

    /// Resolve a cell's background color to RGB.
    pub fn resolve_bg(&self, row: usize, col: usize, config: &Config) -> [u8; 3] {
        let cell = self.cell(row, col);
        resolve_color(&cell.bg, config, false)
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
        let default_bg = [
            (config.bg[0] * 255.0) as u8,
            (config.bg[1] * 255.0) as u8,
            (config.bg[2] * 255.0) as u8,
        ];
        let mut rects = Vec::new();

        for row in 0..rows {
            let mut col = 0;
            while col < cols {
                let bg = self.resolve_bg(row, col, config);
                if bg != default_bg {
                    // Batch consecutive cells with the same background
                    let start_col = col;
                    while col < cols && self.resolve_bg(row, col, config) == bg {
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
        AnsiColor::Indexed(idx) => indexed_color(*idx),
    }
}

fn resolve_named(named: NamedColor, config: &Config, is_fg: bool) -> [u8; 3] {
    match named {
        NamedColor::Black => [0, 0, 0],
        NamedColor::Red => [170, 0, 0],
        NamedColor::Green => [0, 170, 0],
        NamedColor::Yellow => [170, 170, 0],
        NamedColor::Blue => [0, 0, 170],
        NamedColor::Magenta => [170, 0, 170],
        NamedColor::Cyan => [0, 170, 170],
        NamedColor::White => [170, 170, 170],
        NamedColor::BrightBlack => [85, 85, 85],
        NamedColor::BrightRed => [255, 85, 85],
        NamedColor::BrightGreen => [85, 255, 85],
        NamedColor::BrightYellow => [255, 255, 85],
        NamedColor::BrightBlue => [85, 85, 255],
        NamedColor::BrightMagenta => [255, 85, 255],
        NamedColor::BrightCyan => [85, 255, 255],
        NamedColor::BrightWhite => [255, 255, 255],
        NamedColor::Foreground | NamedColor::Background | _ => {
            if is_fg {
                config.fg
            } else {
                let bg = config.bg;
                [(bg[0] * 255.0) as u8, (bg[1] * 255.0) as u8, (bg[2] * 255.0) as u8]
            }
        }
    }
}
