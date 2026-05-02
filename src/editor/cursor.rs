use unicode_segmentation::UnicodeSegmentation;

use super::buffer::{Buffer, Position};

/// A single extra cursor position with optional selection anchor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CursorRange {
    pub pos: Position,
    pub anchor: Option<Position>,
}

/// A cursor in the editor with optional selection anchor.
#[derive(Clone, Debug)]
pub struct EditorCursor {
    /// Current cursor position.
    pub pos: Position,
    /// When holding Shift or dragging, the anchor is the start of the selection.
    /// The selection range is [min(anchor, pos), max(anchor, pos)).
    pub anchor: Option<Position>,
    /// Desired column when moving vertically — preserved across short lines.
    pub desired_col: Option<usize>,
    /// Additional cursors for multi-cursor editing.
    pub extra_cursors: Vec<CursorRange>,
}

impl EditorCursor {
    pub fn new() -> Self {
        Self {
            pos: Position::new(0, 0),
            anchor: None,
            desired_col: None,
            extra_cursors: Vec::new(),
        }
    }

    pub fn at(line: usize, col: usize) -> Self {
        Self {
            pos: Position::new(line, col),
            anchor: None,
            desired_col: None,
            extra_cursors: Vec::new(),
        }
    }

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

    /// Move the cursor, optionally extending the selection.
    fn move_to(&mut self, pos: Position, extend: bool) {
        if extend {
            self.start_selection();
        } else {
            self.clear_selection();
        }
        self.pos = pos;
    }

    // ── Movement ──

    /// Move right by one grapheme cluster.
    pub fn move_right(&mut self, buf: &Buffer, extend: bool) {
        self.desired_col = None;
        let line = buf.line(self.pos.line);
        let graphemes: Vec<&str> = line.graphemes(true).collect();

        // Count chars up to current col to find grapheme index
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
            // Wrap to next line
            self.move_to(Position::new(self.pos.line + 1, 0), extend);
        }
    }

    /// Move left by one grapheme cluster.
    pub fn move_left(&mut self, buf: &Buffer, extend: bool) {
        self.desired_col = None;
        if self.pos.col > 0 {
            let line = buf.line(self.pos.line);
            let graphemes: Vec<&str> = line.graphemes(true).collect();

            // Find the grapheme before current col
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
            // Wrap to end of previous line
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

    /// Move right by one word boundary.
    pub fn move_word_right(&mut self, buf: &Buffer, extend: bool) {
        self.desired_col = None;
        let line = buf.line(self.pos.line);
        let chars: Vec<char> = line.chars().collect();
        let col = self.pos.col;

        if col >= chars.len() {
            // At end of line — move to start of next line
            if self.pos.line + 1 < buf.line_count() {
                self.move_to(Position::new(self.pos.line + 1, 0), extend);
            }
            return;
        }

        // Skip current word chars, then skip whitespace
        let mut i = col;
        let start_kind = char_kind(chars[i]);

        // Move past current word
        while i < chars.len() && char_kind(chars[i]) == start_kind {
            i += 1;
        }
        // Skip whitespace
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }

        self.move_to(Position::new(self.pos.line, i), extend);
    }

    /// Move left by one word boundary.
    pub fn move_word_left(&mut self, buf: &Buffer, extend: bool) {
        self.desired_col = None;
        if self.pos.col == 0 {
            // At start of line — move to end of previous line
            if self.pos.line > 0 {
                let prev_len = buf.line_len(self.pos.line - 1);
                self.move_to(Position::new(self.pos.line - 1, prev_len), extend);
            }
            return;
        }

        let line = buf.line(self.pos.line);
        let chars: Vec<char> = line.chars().collect();
        let mut i = self.pos.col;

        // Skip whitespace going backward
        while i > 0 && chars[i - 1].is_whitespace() {
            i -= 1;
        }

        if i == 0 {
            self.move_to(Position::new(self.pos.line, 0), extend);
            return;
        }

        // Move past the word
        let target_kind = char_kind(chars[i - 1]);
        while i > 0 && char_kind(chars[i - 1]) == target_kind {
            i -= 1;
        }

        self.move_to(Position::new(self.pos.line, i), extend);
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
        // Get word under cursor
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
        // Find the furthest cursor position to search after
        let mut search_after = self.pos;
        for extra in &self.extra_cursors {
            if extra.pos > search_after {
                search_after = extra.pos;
            }
        }
        let text = buf.text();
        let search_char_idx = buf.pos_to_char(search_after);
        let search_byte_idx = char_to_byte_idx(&text, search_char_idx);
        // Search from after the last cursor, wrapping around
        if let Some(found) = text[search_byte_idx..].find(needle) {
            let abs_byte = search_byte_idx + found;
            let abs_char = byte_to_char_idx(&text, abs_byte);
            let found_pos = buf.char_to_pos(abs_char);
            let found_end = buf.char_to_pos(abs_char + needle.chars().count());
            // Don't add a duplicate
            if found_end != self.pos && !self.extra_cursors.iter().any(|c| c.pos == found_end) {
                self.extra_cursors.push(CursorRange {
                    pos: found_end,
                    anchor: Some(found_pos),
                });
                return true;
            }
        }
        // Wrap around from the beginning
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

impl Default for EditorCursor {
    fn default() -> Self {
        Self::new()
    }
}

/// Classify a character for word movement.
#[derive(PartialEq, Eq)]
enum CharKind {
    Word,
    Punctuation,
    Whitespace,
}

fn char_kind(c: char) -> CharKind {
    if c.is_alphanumeric() || c == '_' {
        CharKind::Word
    } else if c.is_whitespace() {
        CharKind::Whitespace
    } else {
        CharKind::Punctuation
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

#[cfg(test)]
mod tests {
    use super::*;

    fn buf_with(text: &str) -> Buffer {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), text);
        buf
    }

    #[test]
    fn new_cursor_at_origin() {
        let c = EditorCursor::new();
        assert_eq!(c.pos, Position::new(0, 0));
        assert!(!c.has_selection());
    }

    // ── Right movement ──

    #[test]
    fn move_right_within_line() {
        let buf = buf_with("hello");
        let mut c = EditorCursor::new();
        c.move_right(&buf, false);
        assert_eq!(c.pos, Position::new(0, 1));
    }

    #[test]
    fn move_right_wraps_to_next_line() {
        let buf = buf_with("ab\ncd");
        let mut c = EditorCursor::at(0, 2);
        c.move_right(&buf, false);
        assert_eq!(c.pos, Position::new(1, 0));
    }

    #[test]
    fn move_right_at_end_of_buffer_stays() {
        let buf = buf_with("ab");
        let mut c = EditorCursor::at(0, 2);
        c.move_right(&buf, false);
        assert_eq!(c.pos, Position::new(0, 2));
    }

    // ── Left movement ──

    #[test]
    fn move_left_within_line() {
        let buf = buf_with("hello");
        let mut c = EditorCursor::at(0, 3);
        c.move_left(&buf, false);
        assert_eq!(c.pos, Position::new(0, 2));
    }

    #[test]
    fn move_left_wraps_to_prev_line() {
        let buf = buf_with("ab\ncd");
        let mut c = EditorCursor::at(1, 0);
        c.move_left(&buf, false);
        assert_eq!(c.pos, Position::new(0, 2));
    }

    #[test]
    fn move_left_at_start_stays() {
        let buf = buf_with("hello");
        let mut c = EditorCursor::new();
        c.move_left(&buf, false);
        assert_eq!(c.pos, Position::new(0, 0));
    }

    // ── Vertical movement ──

    #[test]
    fn move_down_preserves_desired_col() {
        let buf = buf_with("long line here\nhi\nlong again here");
        let mut c = EditorCursor::at(0, 10);
        c.move_down(&buf, false);
        assert_eq!(c.pos, Position::new(1, 2)); // "hi" only has 2 chars
        c.move_down(&buf, false);
        assert_eq!(c.pos, Position::new(2, 10)); // desired_col restored
    }

    #[test]
    fn move_up_at_first_line_goes_to_start() {
        let buf = buf_with("hello");
        let mut c = EditorCursor::at(0, 3);
        c.move_up(&buf, false);
        assert_eq!(c.pos, Position::new(0, 0));
    }

    // ── Word movement ──

    #[test]
    fn move_word_right() {
        let buf = buf_with("hello world_test foo");
        let mut c = EditorCursor::new();
        c.move_word_right(&buf, false);
        assert_eq!(c.pos.col, 6); // past "hello "
        c.move_word_right(&buf, false);
        assert_eq!(c.pos.col, 17); // past "world_test "
    }

    #[test]
    fn move_word_left() {
        let buf = buf_with("hello world");
        let mut c = EditorCursor::at(0, 11);
        c.move_word_left(&buf, false);
        assert_eq!(c.pos.col, 6);
        c.move_word_left(&buf, false);
        assert_eq!(c.pos.col, 0);
    }

    // ── Home/End ──

    #[test]
    fn move_home_to_first_non_ws() {
        let buf = buf_with("    indented");
        let mut c = EditorCursor::at(0, 8);
        c.move_home(&buf, false);
        assert_eq!(c.pos.col, 4); // first non-whitespace
        c.move_home(&buf, false);
        assert_eq!(c.pos.col, 0); // second press goes to col 0
    }

    #[test]
    fn move_end() {
        let buf = buf_with("hello");
        let mut c = EditorCursor::new();
        c.move_end(&buf, false);
        assert_eq!(c.pos.col, 5);
    }

    // ── Selection ──

    #[test]
    fn shift_right_creates_selection() {
        let buf = buf_with("hello");
        let mut c = EditorCursor::new();
        c.move_right(&buf, true);
        c.move_right(&buf, true);
        assert!(c.has_selection());
        let (start, end) = c.selection().unwrap();
        assert_eq!(start, Position::new(0, 0));
        assert_eq!(end, Position::new(0, 2));
    }

    #[test]
    fn move_without_shift_clears_selection() {
        let buf = buf_with("hello");
        let mut c = EditorCursor::new();
        c.move_right(&buf, true);
        c.move_right(&buf, true);
        assert!(c.has_selection());
        c.move_right(&buf, false);
        assert!(!c.has_selection());
    }

    #[test]
    fn select_word() {
        let buf = buf_with("hello world");
        let mut c = EditorCursor::at(0, 2); // inside "hello"
        c.select_word(&buf);
        let (start, end) = c.selection().unwrap();
        assert_eq!(start, Position::new(0, 0));
        assert_eq!(end, Position::new(0, 5));
    }

    #[test]
    fn select_line() {
        let buf = buf_with("hello\nworld");
        let mut c = EditorCursor::at(0, 2);
        c.select_line(&buf);
        let (start, end) = c.selection().unwrap();
        assert_eq!(start, Position::new(0, 0));
        assert_eq!(end, Position::new(1, 0)); // start of next line
    }

    #[test]
    fn select_all() {
        let buf = buf_with("hello\nworld");
        let mut c = EditorCursor::new();
        c.select_all(&buf);
        let (start, end) = c.selection().unwrap();
        assert_eq!(start, Position::new(0, 0));
        assert_eq!(end, Position::new(1, 5));
    }

    #[test]
    fn go_to_line() {
        let buf = buf_with("a\nb\nc\nd");
        let mut c = EditorCursor::new();
        c.go_to_line(3, &buf); // 1-indexed
        assert_eq!(c.pos, Position::new(2, 0));
    }

    #[test]
    fn clamp_out_of_bounds() {
        let buf = buf_with("hi");
        let mut c = EditorCursor::at(10, 50);
        c.clamp(&buf);
        assert_eq!(c.pos.line, 0);
        assert_eq!(c.pos.col, 2);
    }

    // ── Page movement ──

    #[test]
    fn page_down() {
        let text = (0..50)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let buf = buf_with(&text);
        let mut c = EditorCursor::new();
        c.move_page_down(&buf, 20, false);
        assert_eq!(c.pos.line, 20);
    }

    #[test]
    fn page_up_clamps_to_zero() {
        let buf = buf_with("a\nb\nc");
        let mut c = EditorCursor::at(1, 0);
        c.move_page_up(&buf, 20, false);
        assert_eq!(c.pos.line, 0);
    }

    #[test]
    fn empty_buffer_movements_stay_at_origin() {
        let buf = Buffer::empty();
        let mut c = EditorCursor::new();

        c.move_right(&buf, false);
        c.move_left(&buf, false);
        c.move_up(&buf, false);
        c.move_down(&buf, false);
        c.move_home(&buf, false);
        c.move_end(&buf, false);
        c.move_page_up(&buf, 100, false);
        c.move_page_down(&buf, 100, false);
        c.move_to_start(false);
        c.move_to_end(&buf, false);
        c.clamp(&buf);

        assert_eq!(c.pos, Position::new(0, 0));
        assert!(!c.has_selection());
    }

    #[test]
    fn vertical_movement_restores_column_on_long_lines() {
        let buf = buf_with("01234567890123456789\nx\n01234567890123456789");
        let mut c = EditorCursor::at(0, 18);

        c.move_down(&buf, false);
        assert_eq!(c.pos, Position::new(1, 1));
        c.move_down(&buf, false);
        assert_eq!(c.pos, Position::new(2, 18));
    }

    #[test]
    fn document_start_and_end_extend_selection() {
        let buf = buf_with("abc\ndef");
        let mut c = EditorCursor::at(0, 2);

        c.move_to_end(&buf, true);
        assert_eq!(
            c.selection(),
            Some((Position::new(0, 2), Position::new(1, 3)))
        );

        c.move_to_start(true);
        assert_eq!(
            c.selection(),
            Some((Position::new(0, 0), Position::new(0, 2)))
        );

        c.move_to_end(&buf, false);
        assert_eq!(c.pos, Position::new(1, 3));
        assert!(!c.has_selection());
    }

    #[test]
    fn page_down_saturates_and_clamps_to_document_end() {
        let buf = buf_with("a\nbb\nccc");
        let mut c = EditorCursor::at(0, 10);

        c.move_page_down(&buf, usize::MAX, false);

        assert_eq!(c.pos, Position::new(2, 3));
        assert_eq!(c.desired_col, Some(10));
    }

    #[test]
    fn page_movement_extends_selection_from_anchor() {
        let buf = buf_with("aa\nbb\ncc\ndd");
        let mut c = EditorCursor::at(1, 1);

        c.move_page_down(&buf, 2, true);

        assert_eq!(c.pos, Position::new(3, 1));
        assert_eq!(
            c.selection(),
            Some((Position::new(1, 1), Position::new(3, 1)))
        );
    }

    #[test]
    fn clamp_clamps_anchors_and_dedups_extra_cursors() {
        let buf = buf_with("hi\nx");
        let mut c = EditorCursor::at(5, 5);
        c.anchor = Some(Position::new(9, 9));
        c.extra_cursors = vec![
            CursorRange {
                pos: Position::new(1, 50),
                anchor: Some(Position::new(10, 10)),
            },
            CursorRange {
                pos: Position::new(0, 20),
                anchor: None,
            },
            CursorRange {
                pos: Position::new(0, 2),
                anchor: None,
            },
        ];

        c.clamp(&buf);

        assert_eq!(c.pos, Position::new(1, 1));
        assert_eq!(c.anchor, Some(Position::new(1, 1)));
        assert_eq!(
            c.extra_cursors,
            vec![CursorRange {
                pos: Position::new(0, 2),
                anchor: None,
            }]
        );
    }

    #[test]
    fn occurrences_use_character_positions_and_avoid_primary_duplicate() {
        let buf = buf_with("éx éx");
        let mut c = EditorCursor::new();

        c.select_all_occurrences(&buf, "éx");
        assert_eq!(
            c.selection(),
            Some((Position::new(0, 0), Position::new(0, 2)))
        );
        assert_eq!(
            c.extra_cursors,
            vec![CursorRange {
                pos: Position::new(0, 5),
                anchor: Some(Position::new(0, 3)),
            }]
        );

        assert!(!c.add_next_occurrence(&buf, "éx"));
        assert_eq!(c.extra_cursors.len(), 1);
    }
}
