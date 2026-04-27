use std::fs;
use std::path::{Path, PathBuf};

use ropey::Rope;

use super::history::{EditOp, UndoHistory};

/// Detected line ending style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    CrLf,
}

impl LineEnding {
    pub fn as_str(self) -> &'static str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::CrLf => "\r\n",
        }
    }

    /// Detect the dominant line ending in a string.
    fn detect(text: &str) -> Self {
        let crlf = text.matches("\r\n").count();
        let lf = text.matches('\n').count().saturating_sub(crlf);
        if crlf > lf {
            LineEnding::CrLf
        } else {
            LineEnding::Lf
        }
    }
}

/// A position in a text document.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    pub line: usize,
    pub col: usize,
}

impl Position {
    pub fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

/// A text buffer backed by a rope for efficient editing.
pub struct Buffer {
    rope: Rope,
    path: Option<PathBuf>,
    line_ending: LineEnding,
    modified: bool,
    /// Content hash at last save, for detecting external changes.
    saved_hash: u64,
    history: UndoHistory,
    /// Indent style detected or configured for this buffer.
    pub indent_style: IndentStyle,
}

/// How this buffer indents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndentStyle {
    Tabs,
    Spaces(u8),
}

impl Default for IndentStyle {
    fn default() -> Self {
        IndentStyle::Spaces(4)
    }
}

impl IndentStyle {
    /// Detect indent style from file content.
    fn detect(text: &str) -> Self {
        let mut tab_lines = 0u32;
        let mut space_lines = 0u32;
        let mut space_widths = [0u32; 9]; // index 1..8

        for line in text.lines().take(200) {
            if line.starts_with('\t') {
                tab_lines += 1;
            } else if line.starts_with(' ') {
                space_lines += 1;
                let spaces = line.len() - line.trim_start_matches(' ').len();
                if (1..=8).contains(&spaces) {
                    space_widths[spaces] += 1;
                }
            }
        }

        if tab_lines > space_lines {
            IndentStyle::Tabs
        } else {
            // Find the most common space width
            let width = space_widths[1..=8]
                .iter()
                .enumerate()
                .max_by_key(|(_, &count)| count)
                .map(|(i, _)| i + 1)
                .unwrap_or(4) as u8;
            IndentStyle::Spaces(width)
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            IndentStyle::Tabs => "\t",
            IndentStyle::Spaces(1) => " ",
            IndentStyle::Spaces(2) => "  ",
            IndentStyle::Spaces(3) => "   ",
            IndentStyle::Spaces(4) => "    ",
            IndentStyle::Spaces(5) => "     ",
            IndentStyle::Spaces(6) => "      ",
            IndentStyle::Spaces(7) => "       ",
            IndentStyle::Spaces(8) => "        ",
            _ => "    ", // fallback
        }
    }

    pub fn width(self) -> usize {
        match self {
            IndentStyle::Tabs => 1,
            IndentStyle::Spaces(n) => n as usize,
        }
    }
}

fn content_hash(rope: &Rope) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    for chunk in rope.chunks() {
        chunk.hash(&mut hasher);
    }
    hasher.finish()
}

impl Buffer {
    /// Create an empty buffer with no associated file.
    pub fn empty() -> Self {
        let rope = Rope::new();
        let hash = content_hash(&rope);
        Self {
            rope,
            path: None,
            line_ending: LineEnding::Lf,
            modified: false,
            saved_hash: hash,
            history: UndoHistory::new(),
            indent_style: IndentStyle::default(),
        }
    }

    /// Load a buffer from a file on disk.
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path).map_err(|e| format!("Cannot read file: {e}"))?;
        let line_ending = LineEnding::detect(&text);
        let indent_style = IndentStyle::detect(&text);

        // Normalize to LF internally — we restore the original line ending on save.
        let normalized = text.replace("\r\n", "\n");
        let rope = Rope::from_str(&normalized);
        let hash = content_hash(&rope);

        Ok(Self {
            rope,
            path: Some(path.to_path_buf()),
            line_ending,
            modified: false,
            saved_hash: hash,
            history: UndoHistory::new(),
            indent_style,
        })
    }

    // ── Accessors ──

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.path = Some(path);
    }

    pub fn file_name(&self) -> &str {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("untitled")
    }

    pub fn is_modified(&self) -> bool {
        self.modified
    }

    pub fn line_ending(&self) -> LineEnding {
        self.line_ending
    }

    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    /// Get the text of a specific line (without trailing newline).
    pub fn line(&self, idx: usize) -> &str {
        if idx >= self.rope.len_lines() {
            return "";
        }
        let line = self.rope.line(idx);
        let s = line.as_str().unwrap_or("");
        s.trim_end_matches('\n').trim_end_matches('\r')
    }

    /// Get the length of a line in characters (without trailing newline).
    pub fn line_len(&self, idx: usize) -> usize {
        self.line(idx).chars().count()
    }

    /// Total character count.
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    /// Convert a (line, col) position to a character index in the rope.
    pub fn pos_to_char(&self, pos: Position) -> usize {
        if pos.line >= self.rope.len_lines() {
            return self.rope.len_chars();
        }
        let line_start = self.rope.line_to_char(pos.line);
        let line_len = self.line_len(pos.line);
        line_start + pos.col.min(line_len)
    }

    /// Convert a character index to a (line, col) position.
    pub fn char_to_pos(&self, char_idx: usize) -> Position {
        let idx = char_idx.min(self.rope.len_chars());
        let line = self.rope.char_to_line(idx);
        let line_start = self.rope.line_to_char(line);
        Position {
            line,
            col: idx - line_start,
        }
    }

    // ── Editing ──

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
        self.modified = content_hash(&self.rope) != self.saved_hash;
    }

    // ── Line operations ──

    /// Delete an entire line (including its newline). Returns the cursor position after.
    pub fn delete_line(&mut self, line_idx: usize) -> Position {
        if line_idx >= self.line_count() {
            return Position::new(line_idx, 0);
        }
        let start = Position::new(line_idx, 0);
        let end = if line_idx + 1 < self.line_count() {
            Position::new(line_idx + 1, 0)
        } else if line_idx > 0 {
            // Last line: also delete the preceding newline
            let prev_len = self.line_len(line_idx - 1);
            let s = Position::new(line_idx - 1, prev_len);
            self.delete(s, Position::new(line_idx, self.line_len(line_idx)));
            return Position::new(line_idx - 1, prev_len.min(self.line_len(line_idx - 1)));
        } else {
            // Only line in buffer: clear it
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

        // Delete both lines and reinsert in swapped order
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
            // Preserve trailing newline from original last line of the pair
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
        // Moving line N down is the same as moving line N+1 up
        self.move_line_up(line_idx + 1)?;
        Some(Position::new(line_idx + 1, 0))
    }

    /// Duplicate a line below. Returns cursor position on the new line.
    pub fn duplicate_line(&mut self, line_idx: usize) -> Position {
        if line_idx >= self.line_count() {
            return Position::new(line_idx, 0);
        }
        let line_text = self.line(line_idx).to_string();
        // Insert a newline at the end of this line, then the duplicated text
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

    /// Indent a range of lines by one level.
    pub fn indent_lines(&mut self, start_line: usize, end_line: usize) {
        let indent = self.indent_style.as_str().to_string();
        // Work backwards to keep positions stable
        for line_idx in (start_line..=end_line.min(self.line_count().saturating_sub(1))).rev() {
            let pos = Position::new(line_idx, 0);
            self.insert(pos, &indent);
        }
    }

    /// Dedent a range of lines by one level.
    pub fn dedent_lines(&mut self, start_line: usize, end_line: usize) {
        let width = self.indent_style.width();
        // Work backwards to keep positions stable
        for line_idx in (start_line..=end_line.min(self.line_count().saturating_sub(1))).rev() {
            let line = self.line(line_idx);
            let remove_count = if line.starts_with('\t') {
                1
            } else {
                let spaces = line.len() - line.trim_start_matches(' ').len();
                spaces.min(width)
            };
            if remove_count > 0 {
                self.delete(
                    Position::new(line_idx, 0),
                    Position::new(line_idx, remove_count),
                );
            }
        }
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

    // ── Undo / Redo ──

    /// Undo the last edit. Returns the cursor position to restore.
    pub fn undo(&mut self) -> Option<Position> {
        let op = self.history.undo()?;
        self.apply_inverse(&op);
        self.modified = content_hash(&self.rope) != self.saved_hash;
        Some(op.start)
    }

    /// Redo the last undone edit. Returns the cursor position to restore.
    pub fn redo(&mut self) -> Option<Position> {
        let op = self.history.redo()?;
        self.apply_forward(&op);
        self.modified = content_hash(&self.rope) != self.saved_hash;
        Some(op.end_after)
    }

    fn apply_inverse(&mut self, op: &EditOp) {
        let start_idx = self.pos_to_char(op.start);
        // Remove what was inserted
        if !op.new_text.is_empty() {
            let end_idx = start_idx + op.new_text.chars().count();
            let end_idx = end_idx.min(self.rope.len_chars());
            if start_idx < end_idx {
                self.rope.remove(start_idx..end_idx);
            }
        }
        // Re-insert what was deleted
        if !op.old_text.is_empty() {
            self.rope.insert(start_idx, &op.old_text);
        }
    }

    fn apply_forward(&mut self, op: &EditOp) {
        let start_idx = self.pos_to_char(op.start);
        // Remove what was there before
        if !op.old_text.is_empty() {
            let end_idx = start_idx + op.old_text.chars().count();
            let end_idx = end_idx.min(self.rope.len_chars());
            if start_idx < end_idx {
                self.rope.remove(start_idx..end_idx);
            }
        }
        // Insert the new text
        if !op.new_text.is_empty() {
            self.rope.insert(start_idx, &op.new_text);
        }
    }

    // ── Saving ──

    /// Save buffer to its associated file path.
    pub fn save(&mut self) -> Result<(), String> {
        let path = self.path.clone().ok_or("No file path set")?;
        self.save_to(&path)
    }

    /// Save buffer to the given path (also updates the buffer's path).
    pub fn save_to(&mut self, path: &Path) -> Result<(), String> {
        let mut content = String::with_capacity(self.rope.len_bytes());
        for chunk in self.rope.chunks() {
            content.push_str(chunk);
        }

        // Convert internal LF to the file's original line ending
        if self.line_ending == LineEnding::CrLf {
            content = content.replace('\n', "\r\n");
        }

        // Atomic write: write to temp file, then rename
        let dir = path.parent().ok_or("Invalid file path")?;
        let temp_name = format!(
            ".llnzy-save-{}.tmp",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let temp_path = dir.join(temp_name);

        fs::write(&temp_path, &content).map_err(|e| format!("Write failed: {e}"))?;
        fs::rename(&temp_path, path).map_err(|e| {
            let _ = fs::remove_file(&temp_path);
            format!("Rename failed: {e}")
        })?;

        self.path = Some(path.to_path_buf());
        self.saved_hash = content_hash(&self.rope);
        self.modified = false;
        self.history.mark_saved();
        Ok(())
    }

    // ── Helpers ──

    /// Compute the position after inserting `text` at `start`.
    /// Compute the position after inserting `text` at `start` (public for paste).
    pub fn compute_end_pos_pub(&self, start: Position, text: &str) -> Position {
        self.compute_end_pos(start, text)
    }

    fn compute_end_pos(&self, start: Position, text: &str) -> Position {
        let mut line = start.line;
        let mut col = start.col;
        for ch in text.chars() {
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }
        Position::new(line, col)
    }

    /// Get the full text (for small buffers / tests). Avoid on large files.
    pub fn text(&self) -> String {
        let mut s = String::with_capacity(self.rope.len_bytes());
        for chunk in self.rope.chunks() {
            s.push_str(chunk);
        }
        s
    }

    /// Get a character at a specific (line, col) position.
    pub fn char_at(&self, pos: Position) -> Option<char> {
        if pos.line >= self.rope.len_lines() {
            return None;
        }
        let line = self.line(pos.line);
        line.chars().nth(pos.col)
    }

    /// Find a matching bracket when the cursor is on or just after a bracket.
    pub fn matching_bracket(&self, cursor: Position) -> Option<(Position, Position)> {
        let (bracket_pos, bracket, char_idx) = self.adjacent_bracket(cursor)?;
        let (open, close, forward) = match bracket {
            '(' => ('(', ')', true),
            '[' => ('[', ']', true),
            '{' => ('{', '}', true),
            ')' => ('(', ')', false),
            ']' => ('[', ']', false),
            '}' => ('{', '}', false),
            _ => return None,
        };

        if forward {
            let mut depth = 0usize;
            for idx in char_idx..self.rope.len_chars() {
                let ch = self.rope.char(idx);
                if ch == open {
                    depth += 1;
                } else if ch == close {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Some((bracket_pos, self.char_to_pos(idx)));
                    }
                }
            }
        } else {
            let mut depth = 0usize;
            for idx in (0..=char_idx).rev() {
                let ch = self.rope.char(idx);
                if ch == close {
                    depth += 1;
                } else if ch == open {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Some((bracket_pos, self.char_to_pos(idx)));
                    }
                }
            }
        }

        None
    }

    fn adjacent_bracket(&self, cursor: Position) -> Option<(Position, char, usize)> {
        if let Some(ch @ ('(' | ')' | '[' | ']' | '{' | '}')) = self.char_at(cursor) {
            return Some((cursor, ch, self.pos_to_char(cursor)));
        }
        if cursor.col > 0 {
            let pos = Position::new(cursor.line, cursor.col - 1);
            if let Some(ch @ ('(' | ')' | '[' | ']' | '{' | '}')) = self.char_at(pos) {
                return Some((pos, ch, self.pos_to_char(pos)));
            }
        }
        None
    }

    /// Get the indentation string of a line.
    pub fn line_indent(&self, line_idx: usize) -> &str {
        let line = self.line(line_idx);
        let trimmed = line.trim_start_matches(|c: char| c == ' ' || c == '\t');
        &line[..line.len() - trimmed.len()]
    }
}

fn leading_whitespace_len(line: &str) -> usize {
    line.len()
        - line
            .trim_start_matches(|c: char| c == ' ' || c == '\t')
            .len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_buffer_has_one_line() {
        let buf = Buffer::empty();
        // Ropey considers empty text as 1 line
        assert_eq!(buf.line_count(), 1);
        assert_eq!(buf.line(0), "");
        assert!(!buf.is_modified());
    }

    #[test]
    fn insert_text() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "hello");
        assert_eq!(buf.line(0), "hello");
        assert!(buf.is_modified());
    }

    #[test]
    fn insert_multiline() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "line1\nline2\nline3");
        assert_eq!(buf.line_count(), 3);
        assert_eq!(buf.line(0), "line1");
        assert_eq!(buf.line(1), "line2");
        assert_eq!(buf.line(2), "line3");
    }

    #[test]
    fn insert_char_at_position() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "hllo");
        buf.insert_char(Position::new(0, 1), 'e');
        assert_eq!(buf.line(0), "hello");
    }

    #[test]
    fn delete_range() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "hello world");
        buf.delete(Position::new(0, 5), Position::new(0, 11));
        assert_eq!(buf.line(0), "hello");
    }

    #[test]
    fn delete_across_lines() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "hello\nworld");
        buf.delete(Position::new(0, 3), Position::new(1, 2));
        assert_eq!(buf.line(0), "helrld");
        assert_eq!(buf.line_count(), 1);
    }

    #[test]
    fn replace_range() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "hello world");
        buf.replace(Position::new(0, 6), Position::new(0, 11), "rust");
        assert_eq!(buf.line(0), "hello rust");
    }

    #[test]
    fn undo_insert() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "hello");
        assert_eq!(buf.line(0), "hello");

        let pos = buf.undo();
        assert!(pos.is_some());
        assert_eq!(buf.line(0), "");
        assert!(!buf.is_modified());
    }

    #[test]
    fn undo_delete() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "hello");
        buf.delete(Position::new(0, 0), Position::new(0, 5));
        assert_eq!(buf.text(), "");

        buf.undo(); // undo the delete
        assert_eq!(buf.line(0), "hello");
    }

    #[test]
    fn redo_after_undo() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "hello");
        buf.undo();
        assert_eq!(buf.text(), "");

        buf.redo();
        assert_eq!(buf.line(0), "hello");
    }

    #[test]
    fn undo_redo_multiple() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "a");
        buf.insert(Position::new(0, 1), "b");
        buf.insert(Position::new(0, 2), "c");
        assert_eq!(buf.line(0), "abc");

        buf.undo();
        assert_eq!(buf.line(0), "ab");
        buf.undo();
        assert_eq!(buf.line(0), "a");
        buf.redo();
        assert_eq!(buf.line(0), "ab");
    }

    #[test]
    fn pos_to_char_and_back() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "hello\nworld");

        assert_eq!(buf.pos_to_char(Position::new(0, 0)), 0);
        assert_eq!(buf.pos_to_char(Position::new(0, 5)), 5);
        assert_eq!(buf.pos_to_char(Position::new(1, 0)), 6);
        assert_eq!(buf.pos_to_char(Position::new(1, 3)), 9);

        assert_eq!(buf.char_to_pos(0), Position::new(0, 0));
        assert_eq!(buf.char_to_pos(6), Position::new(1, 0));
        assert_eq!(buf.char_to_pos(9), Position::new(1, 3));
    }

    #[test]
    fn line_ending_detection() {
        assert_eq!(LineEnding::detect("a\nb\nc\n"), LineEnding::Lf);
        assert_eq!(LineEnding::detect("a\r\nb\r\nc\r\n"), LineEnding::CrLf);
        assert_eq!(LineEnding::detect("a\r\nb\nc\n"), LineEnding::Lf); // more LF than CRLF
    }

    #[test]
    fn indent_style_detection() {
        assert_eq!(
            IndentStyle::detect("  a\n  b\n  c\n"),
            IndentStyle::Spaces(2)
        );
        assert_eq!(
            IndentStyle::detect("    a\n    b\n"),
            IndentStyle::Spaces(4)
        );
        assert_eq!(IndentStyle::detect("\ta\n\tb\n"), IndentStyle::Tabs);
    }

    #[test]
    fn file_name_untitled_when_no_path() {
        let buf = Buffer::empty();
        assert_eq!(buf.file_name(), "untitled");
    }

    #[test]
    fn char_at_position() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "hello");
        assert_eq!(buf.char_at(Position::new(0, 0)), Some('h'));
        assert_eq!(buf.char_at(Position::new(0, 4)), Some('o'));
        assert_eq!(buf.char_at(Position::new(0, 5)), None);
        assert_eq!(buf.char_at(Position::new(1, 0)), None);
    }

    #[test]
    fn line_indent_extraction() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "    indented\nnot indented\n\t\ttabs");
        assert_eq!(buf.line_indent(0), "    ");
        assert_eq!(buf.line_indent(1), "");
        assert_eq!(buf.line_indent(2), "\t\t");
    }

    #[test]
    fn save_and_reload_round_trip() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("llnzy-test-{}.txt", std::process::id()));

        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "hello\nworld");
        buf.save_to(&path).unwrap();
        assert!(!buf.is_modified());

        let loaded = Buffer::from_file(&path).unwrap();
        assert_eq!(loaded.line(0), "hello");
        assert_eq!(loaded.line(1), "world");
        assert!(!loaded.is_modified());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn crlf_preserved_on_save() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("llnzy-crlf-{}.txt", std::process::id()));

        // Write a CRLF file
        std::fs::write(&path, "line1\r\nline2\r\n").unwrap();

        let mut buf = Buffer::from_file(&path).unwrap();
        assert_eq!(buf.line_ending(), LineEnding::CrLf);
        assert_eq!(buf.line(0), "line1");

        buf.insert(Position::new(1, 5), "!");
        buf.save().unwrap();

        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(raw.contains("\r\n"), "CRLF should be preserved");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn delete_reversed_range() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "hello");
        // end before start should still work
        buf.delete(Position::new(0, 5), Position::new(0, 0));
        assert_eq!(buf.text(), "");
    }

    // ── Line operations ──

    #[test]
    fn delete_line_middle() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "aaa\nbbb\nccc");
        buf.delete_line(1);
        assert_eq!(buf.line(0), "aaa");
        assert_eq!(buf.line(1), "ccc");
        assert_eq!(buf.line_count(), 2);
    }

    #[test]
    fn delete_line_first() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "aaa\nbbb");
        buf.delete_line(0);
        assert_eq!(buf.line(0), "bbb");
        assert_eq!(buf.line_count(), 1);
    }

    #[test]
    fn delete_line_last() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "aaa\nbbb");
        buf.delete_line(1);
        assert_eq!(buf.line(0), "aaa");
        assert_eq!(buf.line_count(), 1);
    }

    #[test]
    fn delete_only_line() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "hello");
        buf.delete_line(0);
        assert_eq!(buf.text(), "");
    }

    #[test]
    fn duplicate_line() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "aaa\nbbb");
        buf.duplicate_line(0);
        assert_eq!(buf.line(0), "aaa");
        assert_eq!(buf.line(1), "aaa");
        assert_eq!(buf.line(2), "bbb");
        assert_eq!(buf.line_count(), 3);
    }

    #[test]
    fn move_line_up() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "aaa\nbbb\nccc");
        buf.move_line_up(1);
        assert_eq!(buf.line(0), "bbb");
        assert_eq!(buf.line(1), "aaa");
        assert_eq!(buf.line(2), "ccc");
    }

    #[test]
    fn move_line_down() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "aaa\nbbb\nccc");
        buf.move_line_down(0);
        assert_eq!(buf.line(0), "bbb");
        assert_eq!(buf.line(1), "aaa");
        assert_eq!(buf.line(2), "ccc");
    }

    #[test]
    fn move_line_up_at_top_returns_none() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "aaa\nbbb");
        assert!(buf.move_line_up(0).is_none());
    }

    #[test]
    fn move_line_down_at_bottom_returns_none() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "aaa\nbbb");
        assert!(buf.move_line_down(1).is_none());
    }

    #[test]
    fn text_range() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "hello world");
        assert_eq!(
            buf.text_range(Position::new(0, 0), Position::new(0, 5)),
            "hello"
        );
        assert_eq!(
            buf.text_range(Position::new(0, 6), Position::new(0, 11)),
            "world"
        );
    }

    #[test]
    fn indent_lines() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "a\nb\nc");
        buf.indent_lines(0, 2);
        assert_eq!(buf.line(0), "    a");
        assert_eq!(buf.line(1), "    b");
        assert_eq!(buf.line(2), "    c");
    }

    #[test]
    fn dedent_lines() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "    a\n    b\n    c");
        buf.dedent_lines(0, 2);
        assert_eq!(buf.line(0), "a");
        assert_eq!(buf.line(1), "b");
        assert_eq!(buf.line(2), "c");
    }

    #[test]
    fn dedent_partial() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "  a\n      b");
        buf.dedent_lines(0, 1);
        assert_eq!(buf.line(0), "a");
        assert_eq!(buf.line(1), "  b");
    }

    #[test]
    fn toggle_line_comments_adds_prefix_after_indent() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "fn main() {\n    println!();\n}");
        buf.toggle_line_comments(0, 2, "//");
        assert_eq!(buf.line(0), "// fn main() {");
        assert_eq!(buf.line(1), "    // println!();");
        assert_eq!(buf.line(2), "// }");
    }

    #[test]
    fn toggle_line_comments_removes_existing_prefix() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "// a\n    // b\n//c");
        buf.toggle_line_comments(0, 2, "//");
        assert_eq!(buf.line(0), "a");
        assert_eq!(buf.line(1), "    b");
        assert_eq!(buf.line(2), "c");
    }

    #[test]
    fn toggle_line_comments_ignores_blank_lines() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "a\n\n    b");
        buf.toggle_line_comments(0, 2, "#");
        assert_eq!(buf.line(0), "# a");
        assert_eq!(buf.line(1), "");
        assert_eq!(buf.line(2), "    # b");
    }

    #[test]
    fn toggle_block_comment_wraps_and_unwraps_selection() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "let value = 1;");
        let start = Position::new(0, 4);
        let end = Position::new(0, 9);
        let (start, end) = buf.toggle_block_comment(start, end, "/*", "*/");
        assert_eq!(buf.line(0), "let /*value*/ = 1;");

        buf.toggle_block_comment(
            Position::new(0, start.col - 2),
            Position::new(0, end.col + 2),
            "/*",
            "*/",
        );
        assert_eq!(buf.line(0), "let value = 1;");
    }

    #[test]
    fn matching_bracket_finds_pair_at_cursor() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "fn main() { call(1); }");
        assert_eq!(
            buf.matching_bracket(Position::new(0, 7)),
            Some((Position::new(0, 7), Position::new(0, 8)))
        );
        assert_eq!(
            buf.matching_bracket(Position::new(0, 10)),
            Some((Position::new(0, 10), Position::new(0, 21)))
        );
    }

    #[test]
    fn matching_bracket_handles_nested_pairs() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "outer(inner())");
        assert_eq!(
            buf.matching_bracket(Position::new(0, 5)),
            Some((Position::new(0, 5), Position::new(0, 13)))
        );
        assert_eq!(
            buf.matching_bracket(Position::new(0, 11)),
            Some((Position::new(0, 11), Position::new(0, 12)))
        );
    }

    #[test]
    fn matching_bracket_crosses_lines() {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "{\n    value\n}");
        assert_eq!(
            buf.matching_bracket(Position::new(0, 0)),
            Some((Position::new(0, 0), Position::new(2, 0)))
        );
    }
}
