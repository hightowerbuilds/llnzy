pub mod buffer;
pub mod cursor;
pub mod file_watcher;
pub mod git_gutter;
pub mod history;
pub mod keymap;
pub mod perf;
pub mod project_search;
pub mod recovery;
pub mod search;
pub mod snippet;
#[cfg(test)]
pub(crate) mod stress_fixtures;
pub mod syntax;

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

use buffer::Buffer;
use cursor::EditorCursor;
use syntax::{FoldRange, SyntaxEngine};
use tree_sitter::{InputEdit, Point, Tree};

use crate::keybindings::VimMode;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BufferId(u64);

impl BufferId {
    pub fn raw(self) -> u64 {
        self.0
    }
}

/// Per-buffer view state (cursor position, scroll offsets, syntax tree).
pub struct BufferView {
    pub cursor: EditorCursor,
    pub scroll_line: usize,
    pub scroll_col: usize,
    /// Smooth scroll target (None = already at destination).
    pub scroll_target: Option<f32>,
    /// Rendered cursor display position.
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
    pending_tree_edit: Option<InputEdit>,
    last_parse_used_incremental: bool,
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
            pending_tree_edit: None,
            last_parse_used_incremental: false,
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
            pending_tree_edit: None,
            last_parse_used_incremental: false,
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
    buffer_id: BufferId,
    generation: u64,
    path: Option<PathBuf>,
    lang_id: &'static str,
    tree: Option<Tree>,
    line_count: usize,
    used_incremental: bool,
}

enum SyntaxReparsePlan {
    Skip,
    Fresh {
        lang_id: &'static str,
    },
    Incremental {
        lang_id: &'static str,
        old_tree: Tree,
        edit: InputEdit,
    },
}

fn plan_syntax_reparse(line_count: usize, view: &BufferView) -> SyntaxReparsePlan {
    if !perf::syntax_enabled(line_count) || !view.tree_dirty || view.parse_pending {
        return SyntaxReparsePlan::Skip;
    }

    let Some(lang_id) = view.lang_id else {
        return SyntaxReparsePlan::Skip;
    };

    match (&view.tree, view.pending_tree_edit.clone()) {
        (Some(tree), Some(edit)) => SyntaxReparsePlan::Incremental {
            lang_id,
            old_tree: tree.clone(),
            edit,
        },
        _ => SyntaxReparsePlan::Fresh { lang_id },
    }
}

/// Top-level editor state managing open buffers.
pub struct EditorState {
    pub buffers: Vec<Buffer>,
    pub views: Vec<BufferView>,
    pub buffer_ids: Vec<BufferId>,
    pub active: usize,
    pub syntax: SyntaxEngine,
    next_buffer_id: u64,
    parse_tx: Sender<ParseResult>,
    parse_rx: Receiver<ParseResult>,
}

impl EditorState {
    pub fn new() -> Self {
        let (parse_tx, parse_rx) = mpsc::channel();
        Self {
            buffers: Vec::new(),
            views: Vec::new(),
            buffer_ids: Vec::new(),
            active: 0,
            syntax: SyntaxEngine::new(),
            next_buffer_id: 1,
            parse_tx,
            parse_rx,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.buffers.is_empty()
    }

    /// Open a file into a new buffer, or switch to it if already open.
    pub fn open(&mut self, path: PathBuf) -> Result<BufferId, String> {
        if let Some(idx) = self.buffers.iter().position(|b| b.path() == Some(&path)) {
            self.active = idx;
            return Ok(self.buffer_ids[idx]);
        }

        let buf = Buffer::from_file(&path)?;
        let lang_id = self.syntax.detect_language(&path);
        let tree_dirty = lang_id.is_some();
        let git_gutter = git_gutter::GitGutter::load(&path);
        let buffer_id = self.alloc_buffer_id();

        self.buffers.push(buf);
        self.views.push(BufferView {
            lang_id,
            tree: None,
            tree_dirty,
            git_gutter,
            ..Default::default()
        });
        self.buffer_ids.push(buffer_id);
        let idx = self.buffers.len() - 1;
        self.active = idx;
        Ok(buffer_id)
    }

    /// Switch to the buffer at the given index.
    pub fn switch_to(&mut self, idx: usize) {
        if idx < self.buffers.len() {
            self.active = idx;
        }
    }

    pub fn switch_to_id(&mut self, id: BufferId) -> bool {
        let Some(idx) = self.index_for_id(id) else {
            return false;
        };
        self.active = idx;
        true
    }

    pub fn active_buffer_id(&self) -> Option<BufferId> {
        self.buffer_ids.get(self.active).copied()
    }

    pub fn buffer_id(&self, idx: usize) -> Option<BufferId> {
        self.buffer_ids.get(idx).copied()
    }

    pub fn index_for_id(&self, id: BufferId) -> Option<usize> {
        self.buffer_ids
            .iter()
            .position(|candidate| *candidate == id)
    }

    pub fn buffer_for_id(&self, id: BufferId) -> Option<&Buffer> {
        self.index_for_id(id).and_then(|idx| self.buffers.get(idx))
    }

    pub fn buffer_for_id_mut(&mut self, id: BufferId) -> Option<&mut Buffer> {
        let idx = self.index_for_id(id)?;
        self.buffers.get_mut(idx)
    }

    pub fn id_for_path(&self, path: &std::path::Path) -> Option<BufferId> {
        self.buffers
            .iter()
            .position(|buffer| buffer.path() == Some(path))
            .and_then(|idx| self.buffer_ids.get(idx).copied())
    }

    pub fn update_path(&mut self, id: BufferId, new_path: PathBuf) -> bool {
        let Some(buffer) = self.buffer_for_id_mut(id) else {
            return false;
        };
        buffer.set_path(new_path);
        true
    }

    pub fn dirty_buffer_ids(&self) -> Vec<BufferId> {
        self.buffers
            .iter()
            .zip(self.buffer_ids.iter().copied())
            .filter_map(|(buffer, id)| buffer.is_modified().then_some(id))
            .collect()
    }

    pub fn buffer_view_for_id(&self, id: BufferId) -> Option<(&Buffer, &BufferView)> {
        let idx = self.index_for_id(id)?;
        Some((self.buffers.get(idx)?, self.views.get(idx)?))
    }

    /// Close the buffer at the given index. Returns true if closed.
    pub fn close(&mut self, idx: usize) -> bool {
        if idx >= self.buffers.len() {
            return false;
        }
        self.buffers.remove(idx);
        self.views.remove(idx);
        self.buffer_ids.remove(idx);
        if self.buffers.is_empty() {
            self.active = 0;
        } else if self.active >= self.buffers.len() {
            self.active = self.buffers.len() - 1;
        }
        true
    }

    pub fn close_id(&mut self, id: BufferId) -> bool {
        let Some(idx) = self.index_for_id(id) else {
            return false;
        };
        self.close(idx)
    }

    /// Get the active buffer and its view.
    pub fn active_buf_view(&mut self) -> Option<(&mut Buffer, &mut BufferView)> {
        if self.active < self.buffers.len() {
            Some((&mut self.buffers[self.active], &mut self.views[self.active]))
        } else {
            None
        }
    }

    pub fn active_buffer_view(&self) -> Option<(BufferId, &Buffer, &BufferView)> {
        let buffer_id = self.active_buffer_id()?;
        Some((
            buffer_id,
            self.buffers.get(self.active)?,
            self.views.get(self.active)?,
        ))
    }

    pub fn active_parse_pending(&self) -> bool {
        self.views
            .get(self.active)
            .is_some_and(|view| view.parse_pending)
    }

    pub fn record_active_incremental_edit(
        &mut self,
        old_source: &str,
        start: buffer::Position,
        old_end: buffer::Position,
        new_text: &str,
    ) -> bool {
        if self.active >= self.buffers.len() {
            return false;
        }
        let view = &mut self.views[self.active];
        if view.parse_pending || view.tree.is_none() || view.lang_id.is_none() {
            view.pending_tree_edit = None;
            return false;
        }

        view.pending_tree_edit = Some(input_edit_from_positions(
            old_source, start, old_end, new_text,
        ));
        view.tree_dirty = true;
        view.folded_ranges.clear();
        true
    }

    /// Re-parse the active buffer's syntax tree if it's dirty.
    pub fn reparse_active(&mut self) {
        self.poll_parse_results();
        if self.active >= self.buffers.len() {
            return;
        }
        let line_count = self.buffers[self.active].line_count();
        let (lang_id, old_tree, used_incremental) =
            match plan_syntax_reparse(line_count, &self.views[self.active]) {
                SyntaxReparsePlan::Skip => return,
                SyntaxReparsePlan::Fresh { lang_id } => (lang_id, None, false),
                SyntaxReparsePlan::Incremental {
                    lang_id,
                    mut old_tree,
                    edit,
                } => {
                    old_tree.edit(&edit);
                    (lang_id, Some(old_tree), true)
                }
            };

        let source = self.buffers[self.active].text();
        let path = self.buffers[self.active].path().map(PathBuf::from);
        let buffer_id = self.buffer_ids[self.active];
        let view = &mut self.views[self.active];
        view.parse_generation = view.parse_generation.wrapping_add(1);
        let generation = view.parse_generation;
        let view_index = self.active;
        let tx = self.parse_tx.clone();
        view.parse_pending = true;
        view.tree_dirty = false;
        view.pending_tree_edit = None;

        let spawn_result = thread::Builder::new()
            .name("llnzy-tree-sitter-parse".to_string())
            .spawn(move || {
                let mut syntax = SyntaxEngine::new();
                let tree = match old_tree {
                    Some(old_tree) => syntax.reparse(lang_id, &source, &old_tree),
                    None => syntax.parse(lang_id, &source),
                };
                let _ = tx.send(ParseResult {
                    buffer_id,
                    generation,
                    path,
                    lang_id,
                    tree,
                    line_count,
                    used_incremental,
                });
            });
        if let Err(err) = spawn_result {
            log::warn!("Failed to spawn tree-sitter parser thread: {err}");
            if let Some(view) = self.views.get_mut(view_index) {
                view.parse_pending = false;
                view.tree_dirty = true;
                view.pending_tree_edit = None;
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
        let Some(view_index) = self.index_for_id(result.buffer_id) else {
            return;
        };
        let Some(view) = self.views.get_mut(view_index) else {
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
            .get(view_index)
            .and_then(|buffer| buffer.path().map(PathBuf::from));
        if buffer_path != result.path {
            return;
        }

        view.tree = result.tree;
        view.last_parse_used_incremental = result.used_incremental;
        view.folded_ranges.retain(|range| {
            range.start_line < range.end_line && range.end_line < result.line_count
        });
    }

    fn alloc_buffer_id(&mut self) -> BufferId {
        let id = BufferId(self.next_buffer_id);
        self.next_buffer_id = self.next_buffer_id.saturating_add(1).max(1);
        id
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

fn input_edit_from_positions(
    old_source: &str,
    start: buffer::Position,
    old_end: buffer::Position,
    new_text: &str,
) -> InputEdit {
    let start_byte = byte_for_position(old_source, start);
    let old_end_byte = byte_for_position(old_source, old_end);
    let start_position = point_for_position(old_source, start);
    InputEdit {
        start_byte,
        old_end_byte,
        new_end_byte: start_byte + new_text.len(),
        start_position,
        old_end_position: point_for_position(old_source, old_end),
        new_end_position: point_after_insert(start_position, new_text),
    }
}

fn point_after_insert(start: Point, inserted: &str) -> Point {
    let mut row = start.row;
    let mut column = start.column;
    for chunk in inserted.split_inclusive('\n') {
        if chunk.ends_with('\n') {
            row += 1;
            column = 0;
        } else {
            column += chunk.len();
        }
    }
    Point { row, column }
}

fn point_for_position(source: &str, pos: buffer::Position) -> Point {
    let line_start = line_start_byte(source, pos.line);
    let byte = byte_for_position(source, pos);
    Point {
        row: pos.line,
        column: byte.saturating_sub(line_start),
    }
}

fn byte_for_position(source: &str, pos: buffer::Position) -> usize {
    let line_start = line_start_byte(source, pos.line);
    let line_end = source[line_start..]
        .find('\n')
        .map(|offset| line_start + offset)
        .unwrap_or(source.len());
    let line = &source[line_start..line_end];
    line.char_indices()
        .nth(pos.col)
        .map(|(offset, _)| line_start + offset)
        .unwrap_or(line_end)
}

fn line_start_byte(source: &str, line: usize) -> usize {
    if line == 0 {
        return 0;
    }
    let mut current_line = 0;
    for (idx, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            current_line += 1;
            if current_line == line {
                return idx + 1;
            }
        }
    }
    source.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str, contents: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "llnzy_editor_state_{}_{}",
            std::process::id(),
            name
        ));
        std::fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn open_defers_tree_sitter_parse_to_background() {
        let path = temp_file("async_parse.rs", "fn main() {\n    println!(\"hi\");\n}\n");

        let mut editor = EditorState::new();
        let buffer_id = editor.open(path.clone()).unwrap();
        assert_eq!(editor.index_for_id(buffer_id), Some(0));
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

    #[test]
    fn large_syntax_fixture_skips_tree_sitter_parse() {
        let dir = std::env::temp_dir().join(format!("llnzy-editor-large-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = stress_fixtures::write_rust_file(
            &dir,
            "large.rs",
            stress_fixtures::LARGE_SYNTAX_LINE_COUNT,
        );
        let mut editor = EditorState::new();
        let buffer_id = editor.open(path).unwrap();

        editor.reparse_active();

        let idx = editor.index_for_id(buffer_id).unwrap();
        assert!(!editor.active_parse_pending());
        assert!(editor.views[idx].tree_dirty);
        assert!(editor.views[idx].tree.is_none());

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn dirty_parsed_buffer_plans_incremental_reparse_with_previous_tree() {
        let source = "fn main() {\n    let value = 1;\n}\n";
        let path = temp_file("incremental_plan.rs", source);
        let mut editor = EditorState::new();
        let buffer_id = editor.open(path.clone()).unwrap();
        let idx = editor.index_for_id(buffer_id).unwrap();

        editor.views[idx].tree = editor.syntax.parse("rust", source);
        assert!(editor.views[idx].tree.is_some());

        let old_source = editor.buffers[idx].text();
        let start = crate::editor::buffer::Position::new(1, 16);
        editor.buffers[idx].insert(start, " + 1");
        editor.active = idx;
        assert!(editor.record_active_incremental_edit(&old_source, start, start, " + 1"));

        match plan_syntax_reparse(editor.buffers[idx].line_count(), &editor.views[idx]) {
            SyntaxReparsePlan::Incremental {
                lang_id, old_tree, ..
            } => {
                assert_eq!(lang_id, "rust");
                assert_eq!(old_tree.root_node().kind(), "source_file");
            }
            SyntaxReparsePlan::Fresh { .. } => {
                panic!("parsed dirty buffer should retain its previous tree for reparse")
            }
            SyntaxReparsePlan::Skip => panic!("small dirty Rust buffer should be parsed"),
        }

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn incremental_reparse_applies_pending_tree_edit() {
        let source = "fn main() {\n    let value = 1;\n}\n";
        let path = temp_file("incremental_reparse.rs", source);
        let mut editor = EditorState::new();
        let buffer_id = editor.open(path.clone()).unwrap();
        let idx = editor.index_for_id(buffer_id).unwrap();

        editor.reparse_active();
        for _ in 0..100 {
            editor.reparse_active();
            if editor.views[idx].tree.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(editor.views[idx].tree.is_some());

        let old_source = editor.buffers[idx].text();
        let start = crate::editor::buffer::Position::new(1, 17);
        editor.buffers[idx].insert(start, " + 1");
        editor.active = idx;
        assert!(editor.record_active_incremental_edit(&old_source, start, start, " + 1"));

        editor.reparse_active();
        for _ in 0..100 {
            editor.reparse_active();
            if !editor.active_parse_pending() && editor.views[idx].last_parse_used_incremental {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        assert!(editor.views[idx].tree.is_some());
        assert!(editor.views[idx].last_parse_used_incremental);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn input_edit_uses_byte_columns_for_unicode_text() {
        let source = "fn main() {\n    let café = 1;\n}\n";
        let start = crate::editor::buffer::Position::new(1, 8);
        let end = crate::editor::buffer::Position::new(1, 12);

        let edit = input_edit_from_positions(source, start, end, "value");

        assert_eq!(edit.start_position, Point { row: 1, column: 8 });
        assert_eq!(edit.old_end_position, Point { row: 1, column: 13 });
        assert_eq!(edit.new_end_position, Point { row: 1, column: 13 });
        assert_eq!(edit.new_end_byte, edit.start_byte + "value".len());
    }

    #[test]
    fn dirty_buffer_without_tree_plans_fresh_parse() {
        let mut view = BufferView {
            lang_id: Some("rust"),
            tree_dirty: true,
            tree: None,
            ..Default::default()
        };

        match plan_syntax_reparse(3, &view) {
            SyntaxReparsePlan::Fresh { lang_id } => assert_eq!(lang_id, "rust"),
            SyntaxReparsePlan::Incremental { .. } => {
                panic!("missing previous tree should fall back to a fresh parse")
            }
            SyntaxReparsePlan::Skip => panic!("dirty Rust buffer should not be skipped"),
        }

        view.parse_pending = true;
        assert!(matches!(
            plan_syntax_reparse(3, &view),
            SyntaxReparsePlan::Skip
        ));
    }

    #[test]
    fn large_dirty_buffer_skips_even_when_previous_tree_exists() {
        let source = "fn main() {}\n";
        let mut syntax = SyntaxEngine::new();
        let view = BufferView {
            lang_id: Some("rust"),
            tree_dirty: true,
            tree: syntax.parse("rust", source),
            ..Default::default()
        };

        assert!(matches!(
            plan_syntax_reparse(stress_fixtures::LARGE_SYNTAX_LINE_COUNT, &view),
            SyntaxReparsePlan::Skip
        ));
    }

    #[test]
    fn buffer_ids_remain_stable_when_indexes_shift() {
        let first = temp_file("first.txt", "first");
        let second = temp_file("second.txt", "second");

        let mut editor = EditorState::new();
        let first_id = editor.open(first.clone()).unwrap();
        let second_id = editor.open(second.clone()).unwrap();

        assert_ne!(first_id, second_id);
        assert_eq!(editor.index_for_id(first_id), Some(0));
        assert_eq!(editor.index_for_id(second_id), Some(1));

        assert!(editor.close_id(first_id));

        assert_eq!(editor.index_for_id(first_id), None);
        assert_eq!(editor.index_for_id(second_id), Some(0));
        assert_eq!(
            editor
                .buffer_for_id(second_id)
                .and_then(|buffer| buffer.path().map(PathBuf::from)),
            Some(second.clone())
        );

        let _ = std::fs::remove_file(first);
        let _ = std::fs::remove_file(second);
    }

    #[test]
    fn switch_to_id_selects_the_matching_buffer_after_index_shift() {
        let first = temp_file("switch_first.txt", "first");
        let second = temp_file("switch_second.txt", "second");

        let mut editor = EditorState::new();
        let first_id = editor.open(first.clone()).unwrap();
        let second_id = editor.open(second.clone()).unwrap();
        assert!(editor.close_id(first_id));

        assert!(editor.switch_to_id(second_id));
        assert_eq!(editor.active_buffer_id(), Some(second_id));
        assert_eq!(editor.active, 0);

        let _ = std::fs::remove_file(first);
        let _ = std::fs::remove_file(second);
    }

    #[test]
    fn active_buffer_view_returns_buffer_id_buffer_and_view() {
        let path = temp_file("active_buffer_view.rs", "fn main() {}\n");
        let mut editor = EditorState::new();
        let buffer_id = editor.open(path.clone()).unwrap();

        let (active_id, buffer, view) = editor.active_buffer_view().unwrap();

        assert_eq!(active_id, buffer_id);
        assert_eq!(buffer.path(), Some(path.as_path()));
        assert_eq!(view.lang_id, Some("rust"));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn registry_resolves_and_updates_paths_by_id() {
        let original = temp_file("registry_original.txt", "first");
        let renamed = std::env::temp_dir().join(format!(
            "llnzy_editor_state_{}_registry_renamed.txt",
            std::process::id()
        ));

        let mut editor = EditorState::new();
        let id = editor.open(original.clone()).unwrap();

        assert_eq!(editor.id_for_path(&original), Some(id));
        assert!(editor.update_path(id, renamed.clone()));
        assert_eq!(editor.id_for_path(&original), None);
        assert_eq!(editor.id_for_path(&renamed), Some(id));

        let _ = std::fs::remove_file(original);
        let _ = std::fs::remove_file(renamed);
    }

    #[test]
    fn dirty_buffer_ids_reports_modified_buffers_by_identity() {
        let clean = temp_file("dirty_clean.txt", "clean");
        let dirty = temp_file("dirty_modified.txt", "dirty");

        let mut editor = EditorState::new();
        let clean_id = editor.open(clean.clone()).unwrap();
        let dirty_id = editor.open(dirty.clone()).unwrap();
        editor
            .buffer_for_id_mut(dirty_id)
            .unwrap()
            .insert(crate::editor::buffer::Position::new(0, 5), "!");

        assert_eq!(editor.dirty_buffer_ids(), vec![dirty_id]);
        assert!(!editor.dirty_buffer_ids().contains(&clean_id));

        let _ = std::fs::remove_file(clean);
        let _ = std::fs::remove_file(dirty);
    }
}
