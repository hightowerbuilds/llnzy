use super::model::EditorCursor;
use super::word::char_kind;
use crate::editor::buffer::{Buffer, Position};

impl EditorCursor {
    /// Whether there is an active selection.
    pub fn has_selection(&self) -> bool {
        self.anchor.is_some_and(|a| a != self.pos)
    }

    /// Get the ordered selection range, if any.
    pub fn selection(&self) -> Option<(Position, Position)> {
        let anchor = self.anchor?;
        if anchor == self.pos {
            return None;
        }
        if anchor <= self.pos {
            Some((anchor, self.pos))
        } else {
            Some((self.pos, anchor))
        }
    }

    /// Clear the selection anchor.
    pub fn clear_selection(&mut self) {
        self.anchor = None;
    }

    /// Start or extend a selection. If extending is true and there's already
    /// an anchor, keep it. Otherwise set the anchor to the current position.
    pub fn start_selection(&mut self) {
        if self.anchor.is_none() {
            self.anchor = Some(self.pos);
        }
    }

    /// Select the entire word at the cursor position.
    pub fn select_word(&mut self, buf: &Buffer) {
        let line = buf.line(self.pos.line);
        let chars: Vec<char> = line.chars().collect();
        if chars.is_empty() || self.pos.col >= chars.len() {
            return;
        }

        let kind = char_kind(chars[self.pos.col]);
        let mut start = self.pos.col;
        let mut end = self.pos.col;

        while start > 0 && char_kind(chars[start - 1]) == kind {
            start -= 1;
        }
        while end < chars.len() && char_kind(chars[end]) == kind {
            end += 1;
        }

        self.anchor = Some(Position::new(self.pos.line, start));
        self.pos = Position::new(self.pos.line, end);
        self.desired_col = None;
    }

    /// Select the entire line at the cursor position.
    pub fn select_line(&mut self, buf: &Buffer) {
        let line_end = buf.line_len(self.pos.line);
        self.anchor = Some(Position::new(self.pos.line, 0));
        if self.pos.line + 1 < buf.line_count() {
            self.pos = Position::new(self.pos.line + 1, 0);
        } else {
            self.pos = Position::new(self.pos.line, line_end);
        }
        self.desired_col = None;
    }

    /// Select the entire buffer.
    pub fn select_all(&mut self, buf: &Buffer) {
        let last_line = buf.line_count().saturating_sub(1);
        let last_col = buf.line_len(last_line);
        self.anchor = Some(Position::new(0, 0));
        self.pos = Position::new(last_line, last_col);
        self.desired_col = None;
    }
}
