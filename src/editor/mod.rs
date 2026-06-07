pub mod buffer;
pub mod cursor;
pub mod editorconfig;
pub mod git_gutter;
pub mod history;
pub mod perf;
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
    /// Vertical scroll in wrapped visual rows. Used only when soft word wrap is
    /// active; `scroll_line` remains the logical-line scroll for classic mode.
    pub wrap_scroll_row: usize,
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
    pub git_gutter: Option<git_gutter::GitGutter>,
    /// Markdown source/preview state for markdown buffers.
    pub markdown_mode: MarkdownViewMode,
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
            wrap_scroll_row: 0,
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
            git_gutter: None,
            markdown_mode: MarkdownViewMode::Source,
        }
    }
}

impl Clone for BufferView {
    fn clone(&self) -> Self {
        Self {
            cursor: self.cursor.clone(),
            wrap_scroll_row: self.wrap_scroll_row,
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
            git_gutter: None, // Git gutter reloaded on open
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

    match (&view.tree, view.pending_tree_edit) {
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

        let mut buf = Buffer::from_file(&path)?;
        // `.editorconfig` cascade overrides the auto-detected indent style
        // and records on-save policies. Applied once at open time; if the
        // user edits an upstream `.editorconfig` while the file is open they
        // need to reopen to pick up changes (consistent with the spec's
        // intent — these are file-load settings).
        let settings = editorconfig::resolve_for(&path);
        buf.apply_editorconfig(&settings);
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

    /// Close the buffer at the given index. Returns true if closed.
    pub fn close(&mut self, idx: usize) -> bool {
        if idx >= self.buffers.len() {
            return false;
        }
        let closing_active = self.active == idx;
        self.buffers.remove(idx);
        self.views.remove(idx);
        self.buffer_ids.remove(idx);
        if self.buffers.is_empty() {
            self.active = 0;
        } else if closing_active {
            self.active = idx.min(self.buffers.len() - 1);
        } else if idx < self.active {
            self.active -= 1;
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
        if !perf::syntax_enabled(line_count) {
            disable_syntax_for_large_view(&mut self.views[self.active]);
            return;
        }
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
        let current_line_count = self
            .buffers
            .get(view_index)
            .map(Buffer::line_count)
            .unwrap_or_default();
        if !perf::syntax_enabled(current_line_count) || !perf::syntax_enabled(result.line_count) {
            if let Some(view) = self.views.get_mut(view_index) {
                disable_syntax_for_large_view(view);
            }
            return;
        }
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

}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}

fn disable_syntax_for_large_view(view: &mut BufferView) {
    view.parse_pending = false;
    view.pending_tree_edit = None;
    view.tree = None;
    view.last_parse_used_incremental = false;
    view.folded_ranges.clear();
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
mod tests;
