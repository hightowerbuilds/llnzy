pub mod buffer;
pub mod cursor;
pub mod file_watcher;
pub mod git_gutter;
pub mod history;
pub mod keymap;
pub mod perf;
pub mod project_search;
pub mod search;
pub mod snippet;
pub mod syntax;

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

use buffer::Buffer;
use cursor::EditorCursor;
use syntax::{FoldRange, SyntaxEngine};
use tree_sitter::Tree;

use crate::keybindings::VimMode;

/// Per-buffer view state (cursor position, scroll offsets, syntax tree).
pub struct BufferView {
    pub cursor: EditorCursor,
    pub scroll_line: usize,
    pub scroll_col: usize,
    /// Smooth scroll target (None = already at destination).
    pub scroll_target: Option<f32>,
    /// Smooth cursor display position (lerped toward actual cursor pos).
    pub cursor_display_x: f32,
    pub cursor_display_y: f32,
    /// Whether cursor display position has been initialized.
    pub cursor_display_init: bool,
    /// The language ID detected for this buffer (e.g. "rust", "python").
    pub lang_id: Option<&'static str>,
    /// The tree-sitter parse tree, if available.
    pub tree: Option<Tree>,
    /// Whether the tree needs re-parsing (set after edits).
    pub tree_dirty: bool,
    parse_pending: bool,
    parse_generation: u64,
    pub folded_ranges: Vec<FoldRange>,
    pub pending_key_chord: Option<EditorKeyChord>,
    pub git_gutter: Option<git_gutter::GitGutter>,
    /// Vim mode state. `Some(mode)` when Vim keybinding preset is active;
    /// `None` when using VS Code or Emacs presets.
    pub vim_mode: Option<VimMode>,
    /// Pending Vim command buffer for multi-key sequences (e.g. "dd", "gg", "yy").
    pub vim_pending: Option<char>,
    /// Markdown source/preview state for markdown buffers.
    pub markdown_mode: MarkdownViewMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorKeyChord {
    CmdK,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkdownViewMode {
    Source,
    Preview,
    Split,
}

impl MarkdownViewMode {
    pub fn cycle(self) -> Self {
        match self {
            Self::Source => Self::Preview,
            Self::Preview => Self::Split,
            Self::Split => Self::Source,
        }
    }
}

impl Default for BufferView {
    fn default() -> Self {
        Self {
            cursor: EditorCursor::new(),
            scroll_line: 0,
            scroll_col: 0,
            scroll_target: None,
            cursor_display_x: 0.0,
            cursor_display_y: 0.0,
            cursor_display_init: false,
            lang_id: None,
            tree: None,
            tree_dirty: false,
            parse_pending: false,
            parse_generation: 0,
            folded_ranges: Vec::new(),
            pending_key_chord: None,
            git_gutter: None,
            vim_mode: None,
            vim_pending: None,
            markdown_mode: MarkdownViewMode::Source,
        }
    }
}

impl Clone for BufferView {
    fn clone(&self) -> Self {
        Self {
            cursor: self.cursor.clone(),
            scroll_line: self.scroll_line,
            scroll_col: self.scroll_col,
            scroll_target: self.scroll_target,
            cursor_display_x: self.cursor_display_x,
            cursor_display_y: self.cursor_display_y,
            cursor_display_init: self.cursor_display_init,
            lang_id: self.lang_id,
            tree: None, // Trees aren't cheaply cloneable; will re-parse
            tree_dirty: true,
            parse_pending: false,
            parse_generation: self.parse_generation,
            folded_ranges: self.folded_ranges.clone(),
            pending_key_chord: self.pending_key_chord,
            git_gutter: None, // Git gutter reloaded on open
            vim_mode: self.vim_mode,
            vim_pending: self.vim_pending,
            markdown_mode: self.markdown_mode,
        }
    }
}

struct ParseResult {
    view_index: usize,
    generation: u64,
    path: Option<PathBuf>,
    lang_id: &'static str,
    tree: Option<Tree>,
    line_count: usize,
}

/// Top-level editor state managing open buffers.
pub struct EditorState {
    pub buffers: Vec<Buffer>,
    pub views: Vec<BufferView>,
    pub active: usize,
    pub syntax: SyntaxEngine,
    parse_tx: Sender<ParseResult>,
    parse_rx: Receiver<ParseResult>,
}

impl EditorState {
    pub fn new() -> Self {
        let (parse_tx, parse_rx) = mpsc::channel();
        Self {
            buffers: Vec::new(),
            views: Vec::new(),
            active: 0,
            syntax: SyntaxEngine::new(),
            parse_tx,
            parse_rx,
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
        let tree_dirty = lang_id.is_some();
        let git_gutter = git_gutter::GitGutter::load(&path);

        self.buffers.push(buf);
        self.views.push(BufferView {
            lang_id,
            tree: None,
            tree_dirty,
            git_gutter,
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

    pub fn active_parse_pending(&self) -> bool {
        self.views
            .get(self.active)
            .is_some_and(|view| view.parse_pending)
    }

    /// Re-parse the active buffer's syntax tree if it's dirty.
    pub fn reparse_active(&mut self) {
        self.poll_parse_results();
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
        if view.parse_pending {
            return;
        }

        let source = self.buffers[self.active].text();
        let line_count = self.buffers[self.active].line_count();
        let path = self.buffers[self.active].path().map(PathBuf::from);
        view.parse_generation = view.parse_generation.wrapping_add(1);
        let generation = view.parse_generation;
        let view_index = self.active;
        let tx = self.parse_tx.clone();
        view.parse_pending = true;
        view.tree_dirty = false;

        let spawn_result = thread::Builder::new()
            .name("llnzy-tree-sitter-parse".to_string())
            .spawn(move || {
                let mut syntax = SyntaxEngine::new();
                let tree = syntax.parse(lang_id, &source);
                let _ = tx.send(ParseResult {
                    view_index,
                    generation,
                    path,
                    lang_id,
                    tree,
                    line_count,
                });
            });
        if let Err(err) = spawn_result {
            log::warn!("Failed to spawn tree-sitter parser thread: {err}");
            if let Some(view) = self.views.get_mut(view_index) {
                view.parse_pending = false;
                view.tree_dirty = true;
            }
        }
    }

    fn poll_parse_results(&mut self) {
        loop {
            match self.parse_rx.try_recv() {
                Ok(result) => self.apply_parse_result(result),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
    }

    fn apply_parse_result(&mut self, result: ParseResult) {
        let Some(view) = self.views.get_mut(result.view_index) else {
            return;
        };
        view.parse_pending = false;
        if view.parse_generation != result.generation
            || view.tree_dirty
            || view.lang_id != Some(result.lang_id)
        {
            return;
        }
        let buffer_path = self
            .buffers
            .get(result.view_index)
            .and_then(|buffer| buffer.path().map(PathBuf::from));
        if buffer_path != result.path {
            return;
        }

        view.tree = result.tree;
        view.folded_ranges.retain(|range| {
            range.start_line < range.end_line && range.end_line < result.line_count
        });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_defers_tree_sitter_parse_to_background() {
        let path =
            std::env::temp_dir().join(format!("llnzy_async_parse_{}_{}.rs", std::process::id(), 1));
        std::fs::write(&path, "fn main() {\n    println!(\"hi\");\n}\n").unwrap();

        let mut editor = EditorState::new();
        let idx = editor.open(path.clone()).unwrap();
        assert_eq!(idx, 0);
        assert!(editor.views[0].tree.is_none());
        assert!(editor.views[0].tree_dirty);

        editor.reparse_active();
        for _ in 0..100 {
            editor.reparse_active();
            if editor.views[0].tree.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        assert!(editor.views[0].tree.is_some());
        let _ = std::fs::remove_file(path);
    }
}
