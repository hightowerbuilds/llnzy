use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Point};
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::term::{self, TermMode};

use crate::config::Config;

use super::colors::resolve_color;
use super::detect_urls;
use super::{TermSize, Terminal};

impl Terminal {
    /// Resize the terminal grid.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        let size = TermSize::new(cols as usize, rows as usize);
        self.term.resize(size);
        self.bump_selection_revision_if_visible();
    }

    /// Get the number of columns and rows.
    pub fn size(&self) -> (usize, usize) {
        let grid = self.term.grid();
        (grid.columns(), grid.screen_lines())
    }

    /// Scroll the display by delta lines (positive = up into history).
    pub fn scroll(&mut self, delta: i32) {
        self.term.scroll_display(Scroll::Delta(delta));
        self.bump_selection_revision_if_visible();
    }

    /// Scroll one page up.
    pub fn scroll_page_up(&mut self) {
        self.term.scroll_display(Scroll::PageUp);
        self.bump_selection_revision_if_visible();
    }

    /// Scroll one page down.
    pub fn scroll_page_down(&mut self) {
        self.term.scroll_display(Scroll::PageDown);
        self.bump_selection_revision_if_visible();
    }

    /// Scroll to the bottom (latest output).
    pub fn scroll_to_bottom(&mut self) {
        self.term.scroll_display(Scroll::Bottom);
        self.bump_selection_revision_if_visible();
    }

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

    pub fn display_offset(&self) -> usize {
        self.term.grid().display_offset()
    }

    pub fn history_size(&self) -> usize {
        self.term.grid().history_size()
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

                if flags.contains(Flags::UNDERLINE) {
                    rects.push((x, y + cell_h - 2.0, cell_w, 1.0, color));
                } else if flags.contains(Flags::DOUBLE_UNDERLINE) {
                    rects.push((x, y + cell_h - 4.0, cell_w, 1.0, color));
                    rects.push((x, y + cell_h - 1.0, cell_w, 1.0, color));
                } else if flags.contains(Flags::UNDERCURL) {
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
                    let dot_w = (cell_w / 4.0).max(1.0);
                    let mut dx = 0.0;
                    while dx < cell_w {
                        rects.push((x + dx, y + cell_h - 2.0, dot_w, 1.0, color));
                        dx += dot_w * 2.0;
                    }
                } else if flags.contains(Flags::DASHED_UNDERLINE) {
                    let dash_w = (cell_w / 2.0).max(1.0);
                    rects.push((x, y + cell_h - 2.0, dash_w, 1.0, color));
                }

                if flags.contains(Flags::STRIKEOUT) {
                    rects.push((x, y + cell_h * 0.5, cell_w, 1.0, color));
                }
            }
        }

        rects
    }

    /// Collect the text content of a viewport row as a string.
    pub fn row_text(&self, row: usize) -> String {
        let (cols, _) = self.size();
        (0..cols).map(|c| self.cell_char(row, c)).collect()
    }

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

    pub fn alt_screen(&self) -> bool {
        self.term.mode().contains(TermMode::ALT_SCREEN)
    }

    pub fn alternate_scroll(&self) -> bool {
        self.term.mode().contains(TermMode::ALTERNATE_SCROLL)
    }

    /// Get the cursor position in viewport coordinates, or None if not visible.
    pub fn cursor_point(&self) -> Option<(usize, usize)> {
        let cursor = self.term.grid().cursor.point;
        let display_offset = self.term.grid().display_offset();
        term::point_to_viewport(display_offset, cursor).map(|p| (p.line, p.column.0))
    }

    /// Collect underline decoration rects for detected URLs in the visible grid.
    pub fn url_decoration_rects(
        &self,
        cell_w: f32,
        cell_h: f32,
    ) -> Vec<(f32, f32, f32, f32, [f32; 4])> {
        let (cols, rows) = self.size();
        let mut rects = Vec::new();
        let color = [0.4, 0.65, 0.9, 0.7];

        for row in 0..rows {
            let line_text = self.row_text(row);
            for (start, end, _url) in detect_urls(&line_text) {
                let start = start.min(cols);
                let end = end.min(cols);
                if start >= end {
                    continue;
                }
                let x = start as f32 * cell_w;
                let y = row as f32 * cell_h + cell_h - 2.0;
                let w = (end - start) as f32 * cell_w;
                rects.push((x, y, w, 1.0, color));
            }
        }

        rects
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
