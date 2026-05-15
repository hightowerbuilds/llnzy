use std::path::{Path, PathBuf};

use ropey::Rope;

use crate::editor::history::UndoHistory;

use super::{BufferKind, IndentStyle, LineEnding};

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BufferEdit {
    pub start: Position,
    pub old_end: Position,
    pub new_end: Position,
    pub new_text: String,
}

/// A text buffer backed by a rope for efficient editing.
pub struct Buffer {
    pub(super) rope: Rope,
    pub(super) path: Option<PathBuf>,
    pub(super) line_ending: LineEnding,
    pub(super) modified: bool,
    /// Content hash at last save, for detecting external changes.
    pub(super) saved_hash: u64,
    pub(super) history: UndoHistory,
    pub(super) last_edit: Option<BufferEdit>,
    /// Indent style detected or configured for this buffer.
    pub indent_style: IndentStyle,
    /// What kind of content this buffer holds. Drives tree-sitter, LSP,
    /// rendering, and font decisions outside the buffer itself.
    pub(super) kind: BufferKind,
    /// Save-time policy sourced from `.editorconfig`. `None` means "no
    /// opinion, leave the existing behavior alone".
    ///
    /// TODO: Wire these into the save path:
    ///   - `insert_final_newline = Some(true)` should append `\n` if missing.
    ///   - `insert_final_newline = Some(false)` should strip a trailing `\n`.
    ///   - `trim_trailing_whitespace = Some(true)` should strip trailing
    ///     `[ \t]+` from each line on save.
    ///   - `eol_override` should override `line_ending` for the next save.
    ///   - `charset_override` requires real encoding plumbing (the buffer
    ///     currently assumes UTF-8 via `fs::read_to_string`); recorded but
    ///     not applied.
    ///
    /// All four fields are intentionally inert today — see `Buffer::save`.
    pub insert_final_newline: Option<bool>,
    pub trim_trailing_whitespace: Option<bool>,
    pub eol_override: Option<crate::editor::editorconfig::EndOfLine>,
    pub charset_override: Option<crate::editor::editorconfig::Charset>,
}

pub(super) fn content_hash(rope: &Rope) -> u64 {
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
        Self::empty_with_kind(BufferKind::Code)
    }

    /// Create an empty prose buffer with no associated file. Prose buffers
    /// turn off code-oriented behaviors (tree-sitter, LSP, gutter/minimap)
    /// at downstream consumers and default to a prose font and word wrap.
    pub fn empty_prose() -> Self {
        Self::empty_with_kind(BufferKind::Prose)
    }

    fn empty_with_kind(kind: BufferKind) -> Self {
        let rope = Rope::new();
        let hash = content_hash(&rope);
        Self {
            rope,
            path: None,
            line_ending: LineEnding::Lf,
            modified: false,
            saved_hash: hash,
            history: UndoHistory::new(),
            last_edit: None,
            indent_style: IndentStyle::default(),
            kind,
            insert_final_newline: None,
            trim_trailing_whitespace: None,
            eol_override: None,
            charset_override: None,
        }
    }

    pub fn kind(&self) -> BufferKind {
        self.kind
    }

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

    pub fn take_last_edit(&mut self) -> Option<BufferEdit> {
        self.last_edit.take()
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

    /// Compute the position after inserting `text` at `start`.
    pub fn compute_end_pos_pub(&self, start: Position, text: &str) -> Position {
        self.compute_end_pos(start, text)
    }

    pub(super) fn compute_end_pos(&self, start: Position, text: &str) -> Position {
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
}
