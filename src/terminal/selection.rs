use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::{Selection as TermSelection, SelectionRange, SelectionType};
use alacritty_terminal::term;

use super::Terminal;

impl Terminal {
    pub(super) fn viewport_point(&self, row: usize, col: usize) -> Point {
        let display_offset = self.term.grid().display_offset();
        term::viewport_to_point(display_offset, Point::new(row, Column(col)))
    }

    pub fn start_selection(&mut self, row: usize, col: usize) {
        let point = self.viewport_point(row, col);
        self.term.selection = Some(TermSelection::new(SelectionType::Simple, point, Side::Left));
        self.selection_anchor = Some((row, col));
        self.selection_end = Some((row, col));
        self.bump_selection_revision();
    }

    pub fn update_selection(&mut self, row: usize, col: usize) -> bool {
        let Some((anchor_row, anchor_col)) = self.selection_anchor else {
            return false;
        };
        if self.selection_end == Some((row, col)) {
            return false;
        }
        let anchor = self.viewport_point(anchor_row, anchor_col);
        let point = self.viewport_point(row, col);
        let dragging_backward = (row, col) < (anchor_row, anchor_col);
        let (anchor_side, point_side) = if dragging_backward {
            (Side::Right, Side::Left)
        } else {
            (Side::Left, Side::Right)
        };
        let mut selection = TermSelection::new(SelectionType::Simple, anchor, anchor_side);
        selection.update(point, point_side);
        self.term.selection = Some(selection);
        self.selection_end = Some((row, col));
        self.bump_selection_revision();
        true
    }

    pub fn clear_selection(&mut self) {
        let had_selection = self.term.selection.is_some()
            || self.selection_anchor.is_some()
            || self.selection_end.is_some();
        self.term.selection = None;
        self.selection_anchor = None;
        self.selection_end = None;
        if had_selection {
            self.bump_selection_revision();
        }
    }

    pub fn has_selection(&self) -> bool {
        self.term
            .selection
            .as_ref()
            .is_some_and(|selection| !selection.is_empty())
    }

    pub fn selected_text(&self) -> Option<String> {
        let range = self.selection_range()?;
        Some(self.selection_range_to_string(range))
    }

    pub fn selection_revision(&self) -> u64 {
        self.selection_revision
    }

    fn selection_range(&self) -> Option<SelectionRange> {
        self.term
            .selection
            .as_ref()
            .and_then(|selection| (!selection.is_empty()).then(|| selection.to_range(&self.term)))
            .flatten()
    }

    fn selection_range_to_string(&self, range: SelectionRange) -> String {
        let (cols, _) = self.size();
        if cols == 0 {
            return String::new();
        }

        let mut lines = Vec::new();
        let max_col = cols.saturating_sub(1);
        for line in range.start.line.0..=range.end.line.0 {
            let col_start = if range.is_block || line == range.start.line.0 {
                range.start.column.0.min(max_col)
            } else {
                0
            };
            let col_end = if range.is_block || line == range.end.line.0 {
                range.end.column.0.min(max_col)
            } else {
                max_col
            };
            if col_end < col_start {
                lines.push(String::new());
                continue;
            }

            let mut text = String::new();
            for col in col_start..=col_end {
                let c = self.term.grid()[Point::new(Line(line), Column(col))].c;
                if c != '\0' {
                    text.push(c);
                }
            }
            lines.push(text.trim_end().to_string());
        }

        lines.join("\n")
    }

    pub fn select_all(&mut self) {
        let (cols, rows) = self.size();
        if cols == 0 || rows == 0 {
            self.clear_selection();
            return;
        }

        self.start_selection(0, 0);
        self.update_selection(rows.saturating_sub(1), cols.saturating_sub(1));
    }

    pub fn select_word(&mut self, row: usize, col: usize) {
        let point = self.viewport_point(row, col);
        self.term.selection = Some(TermSelection::new(
            SelectionType::Semantic,
            point,
            Side::Left,
        ));
        self.selection_anchor = None;
        self.selection_end = Some((row, col));
        self.bump_selection_revision();
    }

    pub fn select_line(&mut self, row: usize) {
        let point = self.viewport_point(row, 0);
        self.term.selection = Some(TermSelection::new(SelectionType::Lines, point, Side::Left));
        self.selection_anchor = None;
        self.selection_end = Some((row, 0));
        self.bump_selection_revision();
    }

    pub fn selection_rects(
        &self,
        cell_w: f32,
        cell_h: f32,
        sel_color: [u8; 3],
        sel_alpha: f32,
    ) -> Vec<(f32, f32, f32, f32, [f32; 4])> {
        let Some(selection_range) = self.selection_range() else {
            return Vec::new();
        };

        let (cols, rows) = self.size();
        if cols == 0 || rows == 0 {
            return Vec::new();
        }

        let color = [
            sel_color[0] as f32 / 255.0,
            sel_color[1] as f32 / 255.0,
            sel_color[2] as f32 / 255.0,
            sel_alpha,
        ];
        let mut rects = Vec::new();

        for line in selection_range.start.line.0..=selection_range.end.line.0 {
            let point = Point::new(Line(line), Column(0));
            let Some(viewport_row) =
                term::point_to_viewport(self.term.grid().display_offset(), point)
                    .map(|point| point.line)
            else {
                continue;
            };
            if viewport_row >= rows {
                continue;
            }

            let max_col = cols.saturating_sub(1);
            let col_start = if selection_range.is_block || line == selection_range.start.line.0 {
                selection_range.start.column.0.min(max_col)
            } else {
                0
            };
            let col_end = if selection_range.is_block || line == selection_range.end.line.0 {
                selection_range.end.column.0.min(max_col)
            } else {
                max_col
            };
            if col_end < col_start {
                continue;
            }

            rects.push((
                col_start as f32 * cell_w,
                viewport_row as f32 * cell_h,
                (col_end - col_start + 1) as f32 * cell_w,
                cell_h,
                color,
            ));
        }

        rects
    }

    fn bump_selection_revision(&mut self) {
        self.selection_revision = self.selection_revision.wrapping_add(1);
    }

    pub(super) fn bump_selection_revision_if_visible(&mut self) {
        if self.has_selection() {
            self.bump_selection_revision();
        }
    }
}
