pub mod buffer;
pub mod cursor;
pub mod history;
pub mod keymap;
pub mod perf;
pub mod syntax;

use std::path::PathBuf;

use buffer::Buffer;
use cursor::EditorCursor;
use syntax::SyntaxEngine;
use tree_sitter::Tree;

/// Per-buffer view state (cursor position, scroll offsets, syntax tree).
pub struct BufferView {
    pub cursor: EditorCursor,
    pub scroll_line: usize,
    pub scroll_col: usize,
    /// The language ID detected for this buffer (e.g. "rust", "python").
    pub lang_id: Option<&'static str>,
    /// The tree-sitter parse tree, if available.
    pub tree: Option<Tree>,
    /// Whether the tree needs re-parsing (set after edits).
    pub tree_dirty: bool,
}

impl Default for BufferView {
    fn default() -> Self {
        Self {
            cursor: EditorCursor::new(),
            scroll_line: 0,
            scroll_col: 0,
            lang_id: None,
            tree: None,
            tree_dirty: false,
        }
    }
}

impl Clone for BufferView {
    fn clone(&self) -> Self {
        Self {
            cursor: self.cursor.clone(),
            scroll_line: self.scroll_line,
            scroll_col: self.scroll_col,
            lang_id: self.lang_id,
            tree: None, // Trees aren't cheaply cloneable; will re-parse
            tree_dirty: true,
        }
    }
}

/// Top-level editor state managing open buffers.
pub struct EditorState {
    pub buffers: Vec<Buffer>,
    pub views: Vec<BufferView>,
    pub active: usize,
    pub syntax: SyntaxEngine,
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            buffers: Vec::new(),
            views: Vec::new(),
            active: 0,
            syntax: SyntaxEngine::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.buffers.is_empty()
    }

    /// Open a file into a new buffer, or switch to it if already open.
    pub fn open(&mut self, path: PathBuf) -> Result<usize, String> {
        if let Some(idx) = self.buffers.iter().position(|b| b.path() == Some(&path)) {
            self.active = idx;
            return Ok(idx);
        }

        let buf = Buffer::from_file(&path)?;
        let lang_id = self.syntax.detect_language(&path);

        // Parse the initial tree
        let tree = lang_id.and_then(|id| {
            let source = buf.text();
            self.syntax.parse(id, &source)
        });

        self.buffers.push(buf);
        self.views.push(BufferView {
            lang_id,
            tree,
            tree_dirty: false,
            ..Default::default()
        });
        let idx = self.buffers.len() - 1;
        self.active = idx;
        Ok(idx)
    }

    /// Switch to the buffer at the given index.
    pub fn switch_to(&mut self, idx: usize) {
        if idx < self.buffers.len() {
            self.active = idx;
        }
    }

    /// Close the buffer at the given index. Returns true if closed.
    pub fn close(&mut self, idx: usize) -> bool {
        if idx >= self.buffers.len() {
            return false;
        }
        self.buffers.remove(idx);
        self.views.remove(idx);
        if self.buffers.is_empty() {
            self.active = 0;
        } else if self.active >= self.buffers.len() {
            self.active = self.buffers.len() - 1;
        }
        true
    }

    /// Get the active buffer and its view.
    pub fn active_buf_view(&mut self) -> Option<(&mut Buffer, &mut BufferView)> {
        if self.active < self.buffers.len() {
            Some((&mut self.buffers[self.active], &mut self.views[self.active]))
        } else {
            None
        }
    }

    /// Re-parse the active buffer's syntax tree if it's dirty.
    pub fn reparse_active(&mut self) {
        if self.active >= self.buffers.len() {
            return;
        }
        // Skip re-parse for very large files
        if !perf::syntax_enabled(self.buffers[self.active].line_count()) {
            return;
        }
        let view = &mut self.views[self.active];
        if !view.tree_dirty {
            return;
        }
        let Some(lang_id) = view.lang_id else { return };
        let source = self.buffers[self.active].text();
        view.tree = if let Some(old_tree) = &view.tree {
            self.syntax.reparse(lang_id, &source, old_tree)
        } else {
            self.syntax.parse(lang_id, &source)
        };
        view.tree_dirty = false;
    }

    /// Tab titles for rendering: (name, is_active, is_modified).
    pub fn tab_info(&self) -> Vec<(&str, bool, bool)> {
        self.buffers
            .iter()
            .enumerate()
            .map(|(i, buf)| (buf.file_name(), i == self.active, buf.is_modified()))
            .collect()
    }
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}
