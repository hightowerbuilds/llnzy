use unicode_segmentation::UnicodeSegmentation;

use super::model::EditorCursor;
use crate::editor::buffer::{Buffer, Position};

impl EditorCursor {
    /// Move right by one grapheme cluster.
    pub fn move_right(&mut self, buf: &Buffer, extend: bool) {
        self.desired_col = None;
        let line = buf.line(self.pos.line);
        let graphemes: Vec<&str> = line.graphemes(true).collect();

        // Count chars up to current col to find grapheme index.
        let mut char_count = 0;
        let mut grapheme_idx = 0;
        for (i, g) in graphemes.iter().enumerate() {
            if char_count >= self.pos.col {
                grapheme_idx = i;
                break;
            }
            char_count += g.chars().count();
            grapheme_idx = i + 1;
        }

        if grapheme_idx < graphemes.len() {
            let new_col = self.pos.col + graphemes[grapheme_idx].chars().count();
            self.move_to(Position::new(self.pos.line, new_col), extend);
        } else if self.pos.line + 1 < buf.line_count() {
            self.move_to(Position::new(self.pos.line + 1, 0), extend);
        }
    }

    /// Move left by one grapheme cluster.
    pub fn move_left(&mut self, buf: &Buffer, extend: bool) {
        self.desired_col = None;
        if self.pos.col > 0 {
            let line = buf.line(self.pos.line);
            let graphemes: Vec<&str> = line.graphemes(true).collect();

            // Find the grapheme before current col.
            let mut char_count = 0;
            let mut prev_char_count = 0;
            for g in &graphemes {
                prev_char_count = char_count;
                char_count += g.chars().count();
                if char_count >= self.pos.col {
                    break;
                }
            }
            self.move_to(Position::new(self.pos.line, prev_char_count), extend);
        } else if self.pos.line > 0 {
            let prev_len = buf.line_len(self.pos.line - 1);
            self.move_to(Position::new(self.pos.line - 1, prev_len), extend);
        }
    }

    /// Move up one line, preserving the desired column.
    pub fn move_up(&mut self, buf: &Buffer, extend: bool) {
        if self.pos.line == 0 {
            self.move_to(Position::new(0, 0), extend);
            return;
        }
        let target_col = self.desired_col.unwrap_or(self.pos.col);
        let new_line = self.pos.line - 1;
        let new_col = target_col.min(buf.line_len(new_line));
        self.move_to(Position::new(new_line, new_col), extend);
        self.desired_col = Some(target_col);
    }

    /// Move down one line, preserving the desired column.
    pub fn move_down(&mut self, buf: &Buffer, extend: bool) {
        if self.pos.line + 1 >= buf.line_count() {
            let end_col = buf.line_len(self.pos.line);
            self.move_to(Position::new(self.pos.line, end_col), extend);
            return;
        }
        let target_col = self.desired_col.unwrap_or(self.pos.col);
        let new_line = self.pos.line + 1;
        let new_col = target_col.min(buf.line_len(new_line));
        self.move_to(Position::new(new_line, new_col), extend);
        self.desired_col = Some(target_col);
    }

    /// Move to the beginning of the line (or to first non-whitespace on second press).
    pub fn move_home(&mut self, buf: &Buffer, extend: bool) {
        self.desired_col = None;
        let line = buf.line(self.pos.line);
        let first_non_ws = line.chars().position(|c| !c.is_whitespace()).unwrap_or(0);

        let new_col = if self.pos.col == first_non_ws || self.pos.col == 0 {
            if self.pos.col == first_non_ws {
                0
            } else {
                first_non_ws
            }
        } else {
            first_non_ws
        };
        self.move_to(Position::new(self.pos.line, new_col), extend);
    }

    /// Move to the end of the line.
    pub fn move_end(&mut self, buf: &Buffer, extend: bool) {
        self.desired_col = None;
        let end = buf.line_len(self.pos.line);
        self.move_to(Position::new(self.pos.line, end), extend);
    }

    /// Move to the beginning of the document.
    pub fn move_to_start(&mut self, extend: bool) {
        self.desired_col = None;
        self.move_to(Position::new(0, 0), extend);
    }

    /// Move to the end of the document.
    pub fn move_to_end(&mut self, buf: &Buffer, extend: bool) {
        self.desired_col = None;
        let last_line = buf.line_count().saturating_sub(1);
        let last_col = buf.line_len(last_line);
        self.move_to(Position::new(last_line, last_col), extend);
    }

    /// Move up by a page (n lines).
    pub fn move_page_up(&mut self, buf: &Buffer, page_lines: usize, extend: bool) {
        let target_col = self.desired_col.unwrap_or(self.pos.col);
        let new_line = self.pos.line.saturating_sub(page_lines);
        let new_col = target_col.min(buf.line_len(new_line));
        self.move_to(Position::new(new_line, new_col), extend);
        self.desired_col = Some(target_col);
    }

    /// Move down by a page (n lines).
    pub fn move_page_down(&mut self, buf: &Buffer, page_lines: usize, extend: bool) {
        let target_col = self.desired_col.unwrap_or(self.pos.col);
        let last_line = buf.line_count().saturating_sub(1);
        let new_line = self.pos.line.saturating_add(page_lines).min(last_line);
        let new_col = target_col.min(buf.line_len(new_line));
        self.move_to(Position::new(new_line, new_col), extend);
        self.desired_col = Some(target_col);
    }

    /// Move to a specific line number (1-indexed, for "go to line").
    pub fn go_to_line(&mut self, line_number: usize, buf: &Buffer) {
        let line = line_number
            .saturating_sub(1)
            .min(buf.line_count().saturating_sub(1));
        self.clear_selection();
        self.desired_col = None;
        self.pos = Position::new(line, 0);
    }
}
