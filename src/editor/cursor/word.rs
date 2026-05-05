use super::model::EditorCursor;
use crate::editor::buffer::{Buffer, Position};

/// Classify a character for word movement.
#[derive(PartialEq, Eq)]
pub(in crate::editor::cursor) enum CharKind {
    Word,
    Punctuation,
    Whitespace,
}

pub(in crate::editor::cursor) fn char_kind(c: char) -> CharKind {
    if c.is_alphanumeric() || c == '_' {
        CharKind::Word
    } else if c.is_whitespace() {
        CharKind::Whitespace
    } else {
        CharKind::Punctuation
    }
}

impl EditorCursor {
    /// Move right by one word boundary.
    pub fn move_word_right(&mut self, buf: &Buffer, extend: bool) {
        self.desired_col = None;
        let line = buf.line(self.pos.line);
        let chars: Vec<char> = line.chars().collect();
        let col = self.pos.col;

        if col >= chars.len() {
            if self.pos.line + 1 < buf.line_count() {
                self.move_to(Position::new(self.pos.line + 1, 0), extend);
            }
            return;
        }

        let mut i = col;
        let start_kind = char_kind(chars[i]);

        while i < chars.len() && char_kind(chars[i]) == start_kind {
            i += 1;
        }
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }

        self.move_to(Position::new(self.pos.line, i), extend);
    }

    /// Move left by one word boundary.
    pub fn move_word_left(&mut self, buf: &Buffer, extend: bool) {
        self.desired_col = None;
        if self.pos.col == 0 {
            if self.pos.line > 0 {
                let prev_len = buf.line_len(self.pos.line - 1);
                self.move_to(Position::new(self.pos.line - 1, prev_len), extend);
            }
            return;
        }

        let line = buf.line(self.pos.line);
        let chars: Vec<char> = line.chars().collect();
        let mut i = self.pos.col;

        while i > 0 && chars[i - 1].is_whitespace() {
            i -= 1;
        }

        if i == 0 {
            self.move_to(Position::new(self.pos.line, 0), extend);
            return;
        }

        let target_kind = char_kind(chars[i - 1]);
        while i > 0 && char_kind(chars[i - 1]) == target_kind {
            i -= 1;
        }

        self.move_to(Position::new(self.pos.line, i), extend);
    }
}
