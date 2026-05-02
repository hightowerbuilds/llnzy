use crate::terminal::Terminal;

pub struct Selection {
    anchor: Option<(usize, usize)>,
    end: Option<(usize, usize)>,
}

impl Default for Selection {
    fn default() -> Self {
        Self::new()
    }
}

impl Selection {
    pub fn new() -> Self {
        Selection {
            anchor: None,
            end: None,
        }
    }

    pub fn start(&mut self, row: usize, col: usize) {
        self.anchor = Some((row, col));
        self.end = Some((row, col));
    }

    pub fn update(&mut self, row: usize, col: usize) {
        if self.anchor.is_some() {
            self.end = Some((row, col));
        }
    }

    pub fn clear(&mut self) {
        self.anchor = None;
        self.end = None;
    }

    pub fn is_active(&self) -> bool {
        if let (Some(a), Some(e)) = (self.anchor, self.end) {
            a != e
        } else {
            false
        }
    }

    pub fn range(&self) -> Option<((usize, usize), (usize, usize))> {
        let a = self.anchor?;
        let e = self.end?;
        if a == e {
            return None;
        }
        if a.0 < e.0 || (a.0 == e.0 && a.1 <= e.1) {
            Some((a, e))
        } else {
            Some((e, a))
        }
    }

    pub fn select_all(&mut self, rows: usize, cols: usize) {
        self.anchor = Some((0, 0));
        self.end = Some((rows.saturating_sub(1), cols.saturating_sub(1)));
    }

    /// Select the word at the given position.
    pub fn select_word(&mut self, row: usize, col: usize, terminal: &Terminal) {
        let (cols, _) = terminal.size();
        let is_word_char =
            |c: char| c.is_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '/';

        let ch = terminal.cell_char(row, col);
        if !is_word_char(ch) {
            // Select just the single non-word character
            self.anchor = Some((row, col));
            self.end = Some((row, col.saturating_add(1).min(cols.saturating_sub(1))));
            return;
        }

        // Expand left
        let mut start = col;
        while start > 0 && is_word_char(terminal.cell_char(row, start - 1)) {
            start -= 1;
        }

        // Expand right
        let mut end = col;
        while end + 1 < cols && is_word_char(terminal.cell_char(row, end + 1)) {
            end += 1;
        }

        self.anchor = Some((row, start));
        self.end = Some((row, end));
    }

    /// Select the entire line.
    pub fn select_line(&mut self, row: usize, cols: usize) {
        self.anchor = Some((row, 0));
        self.end = Some((row, cols.saturating_sub(1)));
    }

    /// Get the word under the given position (for URL detection, etc.)
    pub fn word_at(row: usize, col: usize, terminal: &Terminal) -> String {
        let (cols, _) = terminal.size();
        let is_url_char = |c: char| !c.is_whitespace() && c != '\0';

        let mut start = col;
        while start > 0 && is_url_char(terminal.cell_char(row, start - 1)) {
            start -= 1;
        }

        let mut end = col;
        while end + 1 < cols && is_url_char(terminal.cell_char(row, end + 1)) {
            end += 1;
        }

        (start..=end).map(|c| terminal.cell_char(row, c)).collect()
    }

    pub fn text(&self, terminal: &Terminal) -> String {
        let Some((start, end)) = self.range() else {
            return String::new();
        };
        let (cols, _) = terminal.size();
        let mut result = String::new();

        for row in start.0..=end.0 {
            let col_start = if row == start.0 { start.1 } else { 0 };
            let col_end = if row == end.0 {
                end.1
            } else {
                cols.saturating_sub(1)
            };

            let mut line = String::new();
            for col in col_start..=col_end {
                line.push(terminal.cell_char(row, col));
            }

            let trimmed = line.trim_end();
            result.push_str(trimmed);

            if row < end.0 {
                result.push('\n');
            }
        }

        result
    }

    pub fn rects(
        &self,
        cell_w: f32,
        cell_h: f32,
        cols: usize,
        sel_color: [u8; 3],
        sel_alpha: f32,
    ) -> Vec<(f32, f32, f32, f32, [f32; 4])> {
        let Some((start, end)) = self.range() else {
            return Vec::new();
        };
        if cols == 0 {
            return Vec::new();
        }
        let color = [
            sel_color[0] as f32 / 255.0,
            sel_color[1] as f32 / 255.0,
            sel_color[2] as f32 / 255.0,
            sel_alpha,
        ];
        let mut rects = Vec::new();

        for row in start.0..=end.0 {
            let max_col = cols.saturating_sub(1);
            let col_start = if row == start.0 {
                start.1.min(max_col)
            } else {
                0
            };
            let col_end = if row == end.0 {
                end.1.min(max_col)
            } else {
                max_col
            };
            if col_end < col_start {
                continue;
            }

            let x = col_start as f32 * cell_w;
            let y = row as f32 * cell_h;
            let w = (col_end - col_start + 1) as f32 * cell_w;

            rects.push((x, y, w, cell_h, color));
        }

        rects
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Basic lifecycle ──

    #[test]
    fn new_selection_is_inactive() {
        let sel = Selection::new();
        assert!(!sel.is_active());
        assert!(sel.range().is_none());
    }

    #[test]
    fn start_sets_same_anchor_and_end() {
        let mut sel = Selection::new();
        sel.start(5, 10);
        // Same position = not active
        assert!(!sel.is_active());
        assert!(sel.range().is_none());
    }

    #[test]
    fn start_and_update_makes_active() {
        let mut sel = Selection::new();
        sel.start(0, 0);
        sel.update(0, 5);
        assert!(sel.is_active());
    }

    #[test]
    fn clear_deactivates() {
        let mut sel = Selection::new();
        sel.start(0, 0);
        sel.update(0, 5);
        assert!(sel.is_active());
        sel.clear();
        assert!(!sel.is_active());
        assert!(sel.range().is_none());
    }

    // ── Range normalization ──

    #[test]
    fn range_forward_selection() {
        let mut sel = Selection::new();
        sel.start(0, 0);
        sel.update(0, 10);
        assert_eq!(sel.range(), Some(((0, 0), (0, 10))));
    }

    #[test]
    fn range_backward_selection_normalized() {
        let mut sel = Selection::new();
        sel.start(0, 10);
        sel.update(0, 0);
        // Should normalize to start <= end
        assert_eq!(sel.range(), Some(((0, 0), (0, 10))));
    }

    #[test]
    fn range_multiline_forward() {
        let mut sel = Selection::new();
        sel.start(2, 5);
        sel.update(5, 3);
        assert_eq!(sel.range(), Some(((2, 5), (5, 3))));
    }

    #[test]
    fn range_multiline_backward_normalized() {
        let mut sel = Selection::new();
        sel.start(5, 3);
        sel.update(2, 5);
        assert_eq!(sel.range(), Some(((2, 5), (5, 3))));
    }

    // ── select_all ──

    #[test]
    fn select_all_covers_grid() {
        let mut sel = Selection::new();
        sel.select_all(24, 80);
        assert!(sel.is_active());
        let range = sel.range().unwrap();
        assert_eq!(range.0, (0, 0));
        assert_eq!(range.1, (23, 79));
    }

    #[test]
    fn select_all_single_cell() {
        let mut sel = Selection::new();
        sel.select_all(1, 1);
        // (0,0) to (0,0) => same position => not active
        assert!(!sel.is_active());
    }

    // ── select_line ──

    #[test]
    fn select_line_covers_full_row() {
        let mut sel = Selection::new();
        sel.select_line(5, 80);
        assert!(sel.is_active());
        let range = sel.range().unwrap();
        assert_eq!(range.0, (5, 0));
        assert_eq!(range.1, (5, 79));
    }

    // ── Rect generation ──

    #[test]
    fn rects_empty_when_no_selection() {
        let sel = Selection::new();
        let rects = sel.rects(10.0, 20.0, 80, [255, 0, 0], 0.5);
        assert!(rects.is_empty());
    }

    #[test]
    fn rects_single_line_selection() {
        let mut sel = Selection::new();
        sel.start(0, 2);
        sel.update(0, 5);
        let rects = sel.rects(10.0, 20.0, 80, [255, 0, 0], 0.5);
        assert_eq!(rects.len(), 1);
        let (x, y, w, h, color) = rects[0];
        assert_eq!(x, 20.0); // col 2 * 10.0
        assert_eq!(y, 0.0); // row 0 * 20.0
        assert_eq!(w, 40.0); // 4 cells * 10.0
        assert_eq!(h, 20.0);
        assert_eq!(color[3], 0.5); // alpha
    }

    #[test]
    fn rects_multiline_selection() {
        let mut sel = Selection::new();
        sel.start(1, 5);
        sel.update(3, 10);
        let rects = sel.rects(10.0, 20.0, 80, [0, 0, 255], 0.35);
        assert_eq!(rects.len(), 3); // rows 1, 2, 3

        // Row 1: starts at col 5
        assert_eq!(rects[0].0, 50.0);
        // Row 2: full line, starts at col 0
        assert_eq!(rects[1].0, 0.0);
        // Row 3: ends at col 10
        assert_eq!(rects[2].0, 0.0);
    }

    #[test]
    fn rects_color_normalization() {
        let mut sel = Selection::new();
        sel.start(0, 0);
        sel.update(0, 5);
        let rects = sel.rects(10.0, 20.0, 80, [255, 128, 0], 0.4);
        let color = rects[0].4;
        assert!((color[0] - 1.0).abs() < 0.01); // 255/255
        assert!((color[1] - 0.502).abs() < 0.01); // 128/255
        assert!((color[2] - 0.0).abs() < 0.01); // 0/255
        assert_eq!(color[3], 0.4);
    }

    #[test]
    fn rects_clamps_selection_after_terminal_resize() {
        let mut sel = Selection::new();
        sel.start(0, 70);
        sel.update(2, 79);

        let rects = sel.rects(10.0, 20.0, 40, [255, 0, 0], 0.5);

        assert_eq!(rects.len(), 3);
        assert_eq!(rects[0].0, 390.0);
        assert_eq!(rects[0].2, 10.0);
        assert_eq!(rects[1].0, 0.0);
        assert_eq!(rects[1].2, 400.0);
        assert_eq!(rects[2].0, 0.0);
        assert_eq!(rects[2].2, 400.0);
    }

    // ── update without start does nothing ──

    #[test]
    fn update_without_start_stays_inactive() {
        let mut sel = Selection::new();
        sel.update(5, 10);
        assert!(!sel.is_active());
    }
}
