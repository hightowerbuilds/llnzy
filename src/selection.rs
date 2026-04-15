use crate::terminal::Terminal;

pub struct Selection {
    /// Anchor point (where mouse was first pressed)
    anchor: Option<(usize, usize)>,
    /// Current end point (where mouse is / was released)
    end: Option<(usize, usize)>,
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

    /// Get ordered (start, end) — start is always before end in reading order.
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

    /// Select the entire visible grid.
    pub fn select_all(&mut self, rows: usize, cols: usize) {
        self.anchor = Some((0, 0));
        self.end = Some((rows.saturating_sub(1), cols.saturating_sub(1)));
    }

    /// Extract the selected text from the terminal grid.
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

            // Trim trailing whitespace from each line
            let trimmed = line.trim_end();
            result.push_str(trimmed);

            if row < end.0 {
                result.push('\n');
            }
        }

        result
    }

    /// Generate highlight rectangles: (x, y, w, h, color) in pixel coordinates.
    pub fn rects(
        &self,
        cell_w: f32,
        cell_h: f32,
        cols: usize,
    ) -> Vec<(f32, f32, f32, f32, [f32; 4])> {
        let Some((start, end)) = self.range() else {
            return Vec::new();
        };
        let color = [0.30, 0.50, 0.80, 0.35];
        let mut rects = Vec::new();

        for row in start.0..=end.0 {
            let col_start = if row == start.0 { start.1 } else { 0 };
            let col_end = if row == end.0 {
                end.1
            } else {
                cols.saturating_sub(1)
            };

            let x = col_start as f32 * cell_w;
            let y = row as f32 * cell_h;
            let w = (col_end - col_start + 1) as f32 * cell_w;

            rects.push((x, y, w, cell_h, color));
        }

        rects
    }
}
