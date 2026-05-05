use super::model::{CursorRange, EditorCursor};
use super::word::{char_kind, CharKind};
use crate::editor::buffer::{Buffer, Position};

impl EditorCursor {
    /// Clamp the cursor position to valid bounds within the buffer.
    pub fn clamp(&mut self, buf: &Buffer) {
        let max_line = buf.line_count().saturating_sub(1);
        self.pos.line = self.pos.line.min(max_line);
        self.pos.col = self.pos.col.min(buf.line_len(self.pos.line));
        if let Some(ref mut anchor) = self.anchor {
            anchor.line = anchor.line.min(max_line);
            anchor.col = anchor.col.min(buf.line_len(anchor.line));
        }
        for extra in &mut self.extra_cursors {
            extra.pos.line = extra.pos.line.min(max_line);
            extra.pos.col = extra.pos.col.min(buf.line_len(extra.pos.line));
            if let Some(ref mut anchor) = extra.anchor {
                anchor.line = anchor.line.min(max_line);
                anchor.col = anchor.col.min(buf.line_len(anchor.line));
            }
        }
        let mut seen = Vec::with_capacity(self.extra_cursors.len() + 1);
        seen.push(self.pos);
        self.extra_cursors.retain(|extra| {
            if seen.contains(&extra.pos) {
                false
            } else {
                seen.push(extra.pos);
                true
            }
        });
    }

    /// Clear all extra cursors.
    pub fn clear_extra_cursors(&mut self) {
        self.extra_cursors.clear();
    }

    /// Get all cursor positions (primary + extras), sorted in reverse document order
    /// for safe editing (edits from bottom to top preserve positions).
    pub fn all_positions_reverse(&self) -> Vec<(Position, Option<Position>)> {
        let mut positions: Vec<(Position, Option<Position>)> =
            Vec::with_capacity(1 + self.extra_cursors.len());
        positions.push((self.pos, self.anchor));
        for extra in &self.extra_cursors {
            positions.push((extra.pos, extra.anchor));
        }
        positions.sort_by(|a, b| b.0.cmp(&a.0));
        positions.dedup_by(|a, b| a.0 == b.0);
        positions
    }

    /// Get the selected text for the primary cursor, or the word under the cursor.
    pub fn word_or_selection_text<'a>(&self, buf: &'a Buffer) -> Option<String> {
        if let Some((start, end)) = self.selection() {
            let text = buf.text_range(start, end);
            if !text.is_empty() {
                return Some(text);
            }
        }

        let line = buf.line(self.pos.line);
        let chars: Vec<char> = line.chars().collect();
        if chars.is_empty() || self.pos.col >= chars.len() {
            return None;
        }
        let kind = char_kind(chars[self.pos.col]);
        if kind == CharKind::Whitespace {
            return None;
        }
        let mut start = self.pos.col;
        let mut end = self.pos.col;
        while start > 0 && char_kind(chars[start - 1]) == kind {
            start -= 1;
        }
        while end < chars.len() && char_kind(chars[end]) == kind {
            end += 1;
        }
        let word: String = chars[start..end].iter().collect();
        if word.is_empty() {
            None
        } else {
            Some(word)
        }
    }

    /// Add a cursor at the next occurrence of `needle` after the last cursor position.
    /// Returns true if a new cursor was added.
    pub fn add_next_occurrence(&mut self, buf: &Buffer, needle: &str) -> bool {
        if needle.is_empty() {
            return false;
        }

        let mut search_after = self.pos;
        for extra in &self.extra_cursors {
            if extra.pos > search_after {
                search_after = extra.pos;
            }
        }
        let text = buf.text();
        let search_char_idx = buf.pos_to_char(search_after);
        let search_byte_idx = char_to_byte_idx(&text, search_char_idx);

        if let Some(found) = text[search_byte_idx..].find(needle) {
            let abs_byte = search_byte_idx + found;
            let abs_char = byte_to_char_idx(&text, abs_byte);
            let found_pos = buf.char_to_pos(abs_char);
            let found_end = buf.char_to_pos(abs_char + needle.chars().count());
            if found_end != self.pos && !self.extra_cursors.iter().any(|c| c.pos == found_end) {
                self.extra_cursors.push(CursorRange {
                    pos: found_end,
                    anchor: Some(found_pos),
                });
                return true;
            }
        }

        if let Some(found) = text[..search_byte_idx].find(needle) {
            let found_char = byte_to_char_idx(&text, found);
            let found_pos = buf.char_to_pos(found_char);
            let found_end = buf.char_to_pos(found_char + needle.chars().count());
            if found_end != self.pos && !self.extra_cursors.iter().any(|c| c.pos == found_end) {
                self.extra_cursors.push(CursorRange {
                    pos: found_end,
                    anchor: Some(found_pos),
                });
                return true;
            }
        }
        false
    }

    /// Select all occurrences of `needle` and place cursors at each.
    pub fn select_all_occurrences(&mut self, buf: &Buffer, needle: &str) {
        if needle.is_empty() {
            return;
        }
        let text = buf.text();
        self.extra_cursors.clear();
        let mut first = true;
        let mut start_byte = 0;
        while let Some(found) = text[start_byte..].find(needle) {
            let abs_byte = start_byte + found;
            let abs_char = byte_to_char_idx(&text, abs_byte);
            let found_pos = buf.char_to_pos(abs_char);
            let found_end = buf.char_to_pos(abs_char + needle.chars().count());
            if first {
                self.anchor = Some(found_pos);
                self.pos = found_end;
                first = false;
            } else {
                self.extra_cursors.push(CursorRange {
                    pos: found_end,
                    anchor: Some(found_pos),
                });
            }
            start_byte = abs_byte + needle.len().max(1);
        }
    }
}

fn char_to_byte_idx(text: &str, char_idx: usize) -> usize {
    text.char_indices()
        .nth(char_idx)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len())
}

fn byte_to_char_idx(text: &str, byte_idx: usize) -> usize {
    text[..byte_idx].chars().count()
}
