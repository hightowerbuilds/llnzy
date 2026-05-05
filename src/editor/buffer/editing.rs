use crate::editor::history::EditOp;

use super::indent::leading_whitespace_len;
use super::model::content_hash;
use super::{Buffer, BufferEdit, Position};

impl Buffer {
    /// Insert text at a position. Records undo history.
    pub fn insert(&mut self, pos: Position, text: &str) {
        if text.is_empty() {
            return;
        }
        let char_idx = self.pos_to_char(pos);
        let end_pos = self.compute_end_pos(pos, text);

        self.history.push(EditOp {
            start: pos,
            end_before: pos,
            end_after: end_pos,
            old_text: String::new(),
            new_text: text.to_string(),
        });

        self.rope.insert(char_idx, text);
        self.last_edit = Some(BufferEdit {
            start: pos,
            old_end: pos,
            new_end: end_pos,
            new_text: text.to_string(),
        });
        self.modified = content_hash(&self.rope) != self.saved_hash;
    }

    /// Insert a single character at a position. Coalesces with prior insert for undo.
    pub fn insert_char(&mut self, pos: Position, ch: char) {
        let char_idx = self.pos_to_char(pos);
        let s = ch.to_string();
        let end_pos = self.compute_end_pos(pos, &s);

        let coalesced = self.history.try_coalesce_insert(pos, &s, end_pos);
        if !coalesced {
            self.history.push(EditOp {
                start: pos,
                end_before: pos,
                end_after: end_pos,
                old_text: String::new(),
                new_text: s,
            });
        }

        self.rope.insert_char(char_idx, ch);
        self.last_edit = Some(BufferEdit {
            start: pos,
            old_end: pos,
            new_end: end_pos,
            new_text: ch.to_string(),
        });
        self.modified = content_hash(&self.rope) != self.saved_hash;
    }

    /// Delete the range [start, end). Records undo history.
    pub fn delete(&mut self, start: Position, end: Position) {
        if start == end {
            return;
        }
        let (start, end) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        let start_idx = self.pos_to_char(start);
        let end_idx = self.pos_to_char(end);

        if start_idx == end_idx {
            return;
        }

        let old_text: String = self.rope.slice(start_idx..end_idx).chars().collect();

        self.history.push(EditOp {
            start,
            end_before: end,
            end_after: start,
            old_text,
            new_text: String::new(),
        });

        self.rope.remove(start_idx..end_idx);
        self.last_edit = Some(BufferEdit {
            start,
            old_end: end,
            new_end: start,
            new_text: String::new(),
        });
        self.modified = content_hash(&self.rope) != self.saved_hash;
    }

    /// Replace a range with new text. Records undo history.
    pub fn replace(&mut self, start: Position, end: Position, text: &str) {
        let (start, end) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        let start_idx = self.pos_to_char(start);
        let end_idx = self.pos_to_char(end);

        let old_text: String = self.rope.slice(start_idx..end_idx).chars().collect();
        let end_after = self.compute_end_pos(start, text);

        self.history.push(EditOp {
            start,
            end_before: end,
            end_after,
            old_text,
            new_text: text.to_string(),
        });

        if start_idx < end_idx {
            self.rope.remove(start_idx..end_idx);
        }
        if !text.is_empty() {
            self.rope.insert(start_idx, text);
        }
        self.last_edit = Some(BufferEdit {
            start,
            old_end: end,
            new_end: end_after,
            new_text: text.to_string(),
        });
        self.modified = content_hash(&self.rope) != self.saved_hash;
    }

    /// Delete an entire line (including its newline). Returns the cursor position after.
    pub fn delete_line(&mut self, line_idx: usize) -> Position {
        if line_idx >= self.line_count() {
            return Position::new(line_idx, 0);
        }
        let start = Position::new(line_idx, 0);
        let end = if line_idx + 1 < self.line_count() {
            Position::new(line_idx + 1, 0)
        } else if line_idx > 0 {
            // Last line: also delete the preceding newline.
            let prev_len = self.line_len(line_idx - 1);
            let s = Position::new(line_idx - 1, prev_len);
            self.delete(s, Position::new(line_idx, self.line_len(line_idx)));
            return Position::new(line_idx - 1, prev_len.min(self.line_len(line_idx - 1)));
        } else {
            // Only line in buffer: clear it.
            Position::new(0, self.line_len(0))
        };
        self.delete(start, end);
        let new_line = line_idx.min(self.line_count().saturating_sub(1));
        Position::new(new_line, 0)
    }

    /// Get the full text of a line including its newline (for move/duplicate).
    fn line_with_newline(&self, line_idx: usize) -> String {
        if line_idx >= self.rope.len_lines() {
            return String::new();
        }
        let line = self.rope.line(line_idx);
        line.as_str().unwrap_or("").to_string()
    }

    /// Move a line up by one position. Returns new cursor position.
    pub fn move_line_up(&mut self, line_idx: usize) -> Option<Position> {
        if line_idx == 0 || line_idx >= self.line_count() {
            return None;
        }
        let this_line = self.line_with_newline(line_idx);
        let prev_line = self.line_with_newline(line_idx - 1);

        // Delete both lines and reinsert in swapped order.
        let start = Position::new(line_idx - 1, 0);
        let end = if line_idx + 1 < self.line_count() {
            Position::new(line_idx + 1, 0)
        } else {
            Position::new(line_idx, self.line_len(line_idx))
        };

        let mut swapped = this_line;
        if !swapped.ends_with('\n') && !prev_line.is_empty() {
            swapped.push('\n');
        }
        let prev_trimmed = prev_line.trim_end_matches('\n');
        swapped.push_str(prev_trimmed);
        if line_idx + 1 < self.line_count() {
            // Preserve trailing newline from original last line of the pair.
            if !swapped.ends_with('\n') {
                swapped.push('\n');
            }
        }

        self.replace(start, end, &swapped);
        Some(Position::new(line_idx - 1, 0))
    }

    /// Move a line down by one position. Returns new cursor position.
    pub fn move_line_down(&mut self, line_idx: usize) -> Option<Position> {
        if line_idx + 1 >= self.line_count() {
            return None;
        }
        // Moving line N down is the same as moving line N+1 up.
        self.move_line_up(line_idx + 1)?;
        Some(Position::new(line_idx + 1, 0))
    }

    /// Duplicate a line below. Returns cursor position on the new line.
    pub fn duplicate_line(&mut self, line_idx: usize) -> Position {
        if line_idx >= self.line_count() {
            return Position::new(line_idx, 0);
        }
        let line_text = self.line(line_idx).to_string();
        // Insert a newline at the end of this line, then the duplicated text.
        let insert_pos = if line_idx + 1 < self.line_count() {
            Position::new(line_idx + 1, 0)
        } else {
            Position::new(line_idx, self.line_len(line_idx))
        };

        let insert_text = if line_idx + 1 < self.line_count() {
            format!("{}\n", line_text)
        } else {
            format!("\n{}", line_text)
        };

        self.insert(insert_pos, &insert_text);
        Position::new(line_idx + 1, 0)
    }

    /// Get text in a range (for copy/cut).
    pub fn text_range(&self, start: Position, end: Position) -> String {
        let (start, end) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        let start_idx = self.pos_to_char(start);
        let end_idx = self.pos_to_char(end);
        if start_idx >= end_idx {
            return String::new();
        }
        self.rope.slice(start_idx..end_idx).chars().collect()
    }

    /// Get the full text of a line (with newline, for cut/copy line).
    pub fn line_text_for_copy(&self, line_idx: usize) -> String {
        let text = self.line(line_idx).to_string();
        format!("{}\n", text)
    }

    /// Toggle a line comment prefix across a range of lines.
    pub fn toggle_line_comments(&mut self, start_line: usize, end_line: usize, prefix: &str) {
        if prefix.is_empty() || self.line_count() == 0 {
            return;
        }
        let end_line = end_line.min(self.line_count().saturating_sub(1));
        if start_line > end_line {
            return;
        }

        let mut any_content = false;
        let mut all_commented = true;
        for line_idx in start_line..=end_line {
            let line = self.line(line_idx);
            if line.trim().is_empty() {
                continue;
            }
            any_content = true;
            let indent_len = leading_whitespace_len(line);
            let after_indent = &line[indent_len..];
            if !after_indent.starts_with(prefix) {
                all_commented = false;
                break;
            }
        }

        if !any_content {
            return;
        }

        for line_idx in (start_line..=end_line).rev() {
            let line = self.line(line_idx);
            if line.trim().is_empty() {
                continue;
            }
            let indent_len = leading_whitespace_len(line);
            if all_commented {
                let after_prefix = indent_len + prefix.len();
                let remove_end = if line[after_prefix..].starts_with(' ') {
                    after_prefix + 1
                } else {
                    after_prefix
                };
                self.delete(
                    Position::new(line_idx, indent_len),
                    Position::new(line_idx, remove_end),
                );
            } else {
                self.insert(Position::new(line_idx, indent_len), &format!("{prefix} "));
            }
        }
    }

    /// Toggle a block comment around a range.
    pub fn toggle_block_comment(
        &mut self,
        start: Position,
        end: Position,
        open: &str,
        close: &str,
    ) -> (Position, Position) {
        if open.is_empty() || close.is_empty() {
            return (start, end);
        }
        let (start, end) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        if start == end {
            return (start, end);
        }

        let selected = self.text_range(start, end);
        if selected.starts_with(open) && selected.ends_with(close) {
            let close_start =
                Position::new(end.line, end.col.saturating_sub(close.chars().count()));
            self.delete(close_start, end);
            self.delete(
                start,
                Position::new(start.line, start.col + open.chars().count()),
            );
            let end_col = if start.line == end.line {
                end.col
                    .saturating_sub(open.chars().count())
                    .saturating_sub(close.chars().count())
            } else {
                end.col.saturating_sub(close.chars().count())
            };
            let new_end = Position::new(end.line, end_col);
            (start, new_end)
        } else {
            self.insert(end, close);
            self.insert(start, open);
            let new_start = Position::new(start.line, start.col + open.chars().count());
            let end_col = if start.line == end.line {
                end.col + open.chars().count()
            } else {
                end.col
            };
            let new_end = Position::new(end.line, end_col);
            (new_start, new_end)
        }
    }
}
