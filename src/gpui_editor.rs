use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use gpui::prelude::*;
use gpui::{
    actions, div, font, px, relative, rgb, rgba, size, App, Application, Bounds, ClipboardItem,
    Context, CursorStyle, Element, ElementId, ElementInputHandler, Entity, EntityInputHandler,
    FocusHandle, Focusable, GlobalElementId, KeyBinding, KeyDownEvent, LayoutId, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Render, ScrollWheelEvent, ShapedLine,
    SharedString, Style, TextRun, UTF16Selection, Window, WindowBounds, WindowOptions,
};
use tokio::sync::oneshot;

use crate::config::{Config, CursorStyle as ConfigCursorStyle, EditorConfig};
use crate::editor::buffer::{Buffer, BufferEdit, Position};
use crate::editor::perf;
use crate::editor::search::EditorSearch;
use crate::editor::syntax::{group_color, HighlightGroup, HighlightSpan};
use crate::editor::{BufferId, BufferView, EditorState};
use crate::lsp::{
    CodeAction, CompletionItem, DiagSeverity, FileDiagnostic, FormatEdit,
    IncrementalDocumentChange, LspEnsureStatus, LspManager, ReferenceLocation, SignatureInfo,
    SymbolInfo, WorkspaceEdits,
};
use crate::path_utils::{path_contains, same_path};
use crate::stacker::utf16::{char_index_to_utf16_index, utf16_index_to_char_index};

actions!(
    editor_gpui,
    [
        Backspace,
        Delete,
        Enter,
        Tab,
        ShiftTab,
        Left,
        Right,
        Up,
        Down,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        WordLeft,
        WordRight,
        SelectWordLeft,
        SelectWordRight,
        Home,
        End,
        SelectHome,
        SelectEnd,
        LineStart,
        LineEnd,
        SelectLineStart,
        SelectLineEnd,
        DocumentStart,
        DocumentEnd,
        SelectDocumentStart,
        SelectDocumentEnd,
        PageUp,
        PageDown,
        SelectPageUp,
        SelectPageDown,
        SelectAll,
        SelectWord,
        SelectLine,
        DeleteWordBackward,
        DeleteWordForward,
        DeleteToLineStart,
        DeleteToLineEnd,
        DeleteLine,
        DuplicateLineOrSelection,
        MoveLineUp,
        MoveLineDown,
        ToggleLineComment,
        Paste,
        Cut,
        Copy,
        Save,
        Undo,
        Redo,
        Find,
        FindNext,
        FindPrevious,
        CloseFind,
        GoToLine,
        Quit
    ]
);

pub fn run_editor_prototype() {
    Application::new().run(|cx: &mut App| {
        bind_editor_keys(cx);
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);

        let bounds = Bounds::centered(None, size(px(1120.0), px(760.0)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_, cx| cx.new(EditorPrototype::standalone),
            )
            .unwrap();
        window
            .update(cx, |view, window, cx| {
                window.focus(&view.focus_handle(cx));
            })
            .unwrap();
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.activate(true);
    });
}

pub(crate) fn bind_editor_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, None),
        KeyBinding::new("delete", Delete, None),
        KeyBinding::new("enter", Enter, None),
        KeyBinding::new("tab", Tab, None),
        KeyBinding::new("shift-tab", ShiftTab, None),
        KeyBinding::new("left", Left, None),
        KeyBinding::new("right", Right, None),
        KeyBinding::new("up", Up, None),
        KeyBinding::new("down", Down, None),
        KeyBinding::new("shift-left", SelectLeft, None),
        KeyBinding::new("shift-right", SelectRight, None),
        KeyBinding::new("shift-up", SelectUp, None),
        KeyBinding::new("shift-down", SelectDown, None),
        KeyBinding::new("alt-left", WordLeft, None),
        KeyBinding::new("alt-right", WordRight, None),
        KeyBinding::new("shift-alt-left", SelectWordLeft, None),
        KeyBinding::new("shift-alt-right", SelectWordRight, None),
        KeyBinding::new("home", Home, None),
        KeyBinding::new("end", End, None),
        KeyBinding::new("shift-home", SelectHome, None),
        KeyBinding::new("shift-end", SelectEnd, None),
        KeyBinding::new("cmd-left", LineStart, None),
        KeyBinding::new("cmd-right", LineEnd, None),
        KeyBinding::new("shift-cmd-left", SelectLineStart, None),
        KeyBinding::new("shift-cmd-right", SelectLineEnd, None),
        KeyBinding::new("ctrl-a", LineStart, None),
        KeyBinding::new("ctrl-e", LineEnd, None),
        KeyBinding::new("cmd-up", DocumentStart, None),
        KeyBinding::new("cmd-down", DocumentEnd, None),
        KeyBinding::new("shift-cmd-up", SelectDocumentStart, None),
        KeyBinding::new("shift-cmd-down", SelectDocumentEnd, None),
        KeyBinding::new("pageup", PageUp, None),
        KeyBinding::new("pagedown", PageDown, None),
        KeyBinding::new("shift-pageup", SelectPageUp, None),
        KeyBinding::new("shift-pagedown", SelectPageDown, None),
        KeyBinding::new("alt-backspace", DeleteWordBackward, None),
        KeyBinding::new("alt-delete", DeleteWordForward, None),
        KeyBinding::new("cmd-backspace", DeleteToLineStart, None),
        KeyBinding::new("cmd-delete", DeleteToLineEnd, None),
        KeyBinding::new("ctrl-k", DeleteToLineEnd, None),
        KeyBinding::new("cmd-a", SelectAll, None),
        KeyBinding::new("cmd-d", SelectWord, None),
        KeyBinding::new("cmd-l", SelectLine, None),
        KeyBinding::new("cmd-shift-k", DeleteLine, None),
        KeyBinding::new("cmd-shift-d", DuplicateLineOrSelection, None),
        KeyBinding::new("alt-up", MoveLineUp, None),
        KeyBinding::new("alt-down", MoveLineDown, None),
        KeyBinding::new("cmd-/", ToggleLineComment, None),
        KeyBinding::new("cmd-v", Paste, None),
        KeyBinding::new("cmd-c", Copy, None),
        KeyBinding::new("cmd-x", Cut, None),
        KeyBinding::new("cmd-s", Save, None),
        KeyBinding::new("cmd-z", Undo, None),
        KeyBinding::new("cmd-shift-z", Redo, None),
        KeyBinding::new("cmd-f", Find, None),
        KeyBinding::new("cmd-g", FindNext, None),
        KeyBinding::new("shift-cmd-g", FindPrevious, None),
        KeyBinding::new("ctrl-g", GoToLine, None),
        KeyBinding::new("escape", CloseFind, None),
    ]);
}

fn start_cursor_blink_task(cx: &mut Context<EditorPrototype>) {
    cx.spawn(
        |editor: gpui::WeakEntity<EditorPrototype>, cx: &mut gpui::AsyncApp| {
            let mut cx = cx.clone();
            async move {
                loop {
                    cx.background_executor().timer(CURSOR_BLINK_TICK).await;
                    let Ok(()) = editor.update(&mut cx, |editor, cx| {
                        if editor.refresh_cursor_blink() {
                            cx.notify();
                        }
                    }) else {
                        break;
                    };
                }
            }
        },
    )
    .detach();
}

fn start_lsp_poll_task(cx: &mut Context<EditorPrototype>) {
    cx.spawn(
        |editor: gpui::WeakEntity<EditorPrototype>, cx: &mut gpui::AsyncApp| {
            let mut cx = cx.clone();
            async move {
                loop {
                    cx.background_executor().timer(LSP_POLL_TICK).await;
                    let Ok(()) = editor.update(&mut cx, |editor, cx| {
                        if editor.poll_lsp(cx) {
                            cx.notify();
                        }
                    }) else {
                        break;
                    };
                }
            }
        },
    )
    .detach();
}

pub(crate) struct EditorPrototype {
    focus_handle: FocusHandle,
    editor: EditorState,
    lsp: LspManager,
    lsp_pending: GpuiLspPending,
    lsp_pending_changes: HashMap<BufferId, GpuiPendingLspChange>,
    lsp_panel: Option<GpuiLspPanel>,
    lsp_snapshot_key: String,
    appearance_config: EditorAppearanceConfig,
    editor_search: EditorSearch,
    search_input_target: EditorSearchInputTarget,
    go_to_line_active: bool,
    go_to_line_input: String,
    rename_active: bool,
    rename_input: String,
    recently_closed_paths: Vec<PathBuf>,
    last_seen_disk_text: HashMap<PathBuf, String>,
    external_change: Option<ExternalFileChange>,
    load_error: Option<String>,
    sample_text: String,
    sample_scroll_line: usize,
    status_message: Option<String>,
    last_text_bounds: Option<Bounds<gpui::Pixels>>,
    last_text_layout: Option<EditorMeasuredLayout>,
    is_selecting: bool,
    cursor_blink_anchor: Instant,
    cursor_blink_visible: bool,
    show_chrome: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditorCommand {
    Move { motion: EditorMotion, extend: bool },
    Delete(EditorDeleteTarget),
    Enter,
    Indent { outdent: bool },
    Select(EditorSelectTarget),
    DuplicateLineOrSelection,
    MoveLine(EditorLineMove),
    DeleteLine,
    ToggleLineComment,
    Copy,
    Cut,
    Paste,
    Save,
    Undo,
    Redo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditorMotion {
    Left,
    Right,
    Up,
    Down,
    WordLeft,
    WordRight,
    SmartLineStart,
    LineStart,
    LineEnd,
    DocumentStart,
    DocumentEnd,
    PageUp,
    PageDown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditorDeleteTarget {
    BackwardChar,
    ForwardChar,
    BackwardWord,
    ForwardWord,
    ToLineStart,
    ToLineEnd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditorSelectTarget {
    All,
    Word,
    Line,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditorLineMove {
    Up,
    Down,
}

#[derive(Clone)]
struct EditorMeasuredLayout {
    scroll_col: usize,
    lines: Vec<EditorMeasuredLine>,
}

#[derive(Clone)]
struct EditorMeasuredLine {
    source_line: usize,
    visible_text: String,
    shaped: ShapedLine,
}

#[derive(Default)]
struct GpuiLspPending {
    hover: Option<GpuiPendingLspRequest<Option<String>>>,
    completion: Option<GpuiPendingLspRequest<Vec<CompletionItem>>>,
    definition: Option<GpuiPendingLspRequest<Option<(PathBuf, u32, u32)>>>,
    signature_help: Option<GpuiPendingLspRequest<Option<SignatureInfo>>>,
    references: Option<GpuiPendingLspRequest<Vec<ReferenceLocation>>>,
    format: Option<GpuiPendingLspRequest<Vec<FormatEdit>>>,
    code_actions: Option<GpuiPendingLspRequest<Vec<CodeAction>>>,
    document_symbols: Option<GpuiPendingLspRequest<Vec<SymbolInfo>>>,
    rename: Option<GpuiPendingLspRequest<WorkspaceEdits>>,
}

struct GpuiPendingLspRequest<T> {
    buffer_id: BufferId,
    rx: oneshot::Receiver<T>,
}

impl<T> GpuiPendingLspRequest<T> {
    fn new(buffer_id: BufferId, rx: oneshot::Receiver<T>) -> Self {
        Self { buffer_id, rx }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GpuiPendingLspChange {
    queued_at: Instant,
    kind: GpuiPendingLspChangeKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum GpuiPendingLspChangeKind {
    Incremental {
        start: Position,
        old_end: Position,
        new_text: String,
    },
    Full,
}

#[derive(Clone)]
struct GpuiLspPanel {
    title: String,
    items: Vec<GpuiLspPanelItem>,
    selected: usize,
    anchor: Option<GpuiLspPanelAnchor>,
}

#[derive(Clone, Copy)]
struct GpuiLspPanelAnchor {
    top: gpui::Pixels,
    left: gpui::Pixels,
}

#[derive(Clone)]
struct GpuiLspPanelItem {
    label: String,
    action: GpuiLspPanelAction,
}

impl GpuiLspPanelItem {
    fn plain(label: String) -> Self {
        Self {
            label,
            action: GpuiLspPanelAction::None,
        }
    }
}

#[derive(Clone)]
enum GpuiLspPanelAction {
    None,
    Complete { text: String },
    GoTo { path: PathBuf, line: u32, col: u32 },
    ApplyWorkspaceEdit { edits: WorkspaceEdits },
}

#[derive(Clone)]
struct EditorAppearanceConfig {
    terminal_font_size: f32,
    font_family: Option<String>,
    foreground: [u8; 3],
    background: [u8; 3],
    cursor: [u8; 3],
    selection: [u8; 3],
    selection_alpha: f32,
    cursor_style: ConfigCursorStyle,
    editor: EditorConfig,
}

impl EditorAppearanceConfig {
    fn from_config(config: &Config) -> Self {
        Self {
            terminal_font_size: config.font_size,
            font_family: config.font_family.clone(),
            foreground: config.colors.foreground,
            background: config.colors.background,
            cursor: config.colors.cursor,
            selection: config.colors.selection,
            selection_alpha: config.colors.selection_alpha,
            cursor_style: config.cursor_style,
            editor: config.editor.clone(),
        }
    }

    fn for_language(&self, lang_id: Option<&str>) -> EditorAppearance {
        let effective = self.editor.effective_for(lang_id, self.terminal_font_size);
        let font_size = effective.font_size.clamp(8.0, 40.0);
        let line_height = (font_size * effective.line_height).clamp(font_size + 2.0, 72.0);

        EditorAppearance {
            font_family: self
                .font_family
                .clone()
                .unwrap_or_else(|| "Berkeley Mono".to_string()),
            font_size: px(font_size),
            line_height: px(line_height),
            char_width: px((font_size * 0.6).max(4.0)),
            line_number_width: if effective.show_line_numbers {
                px(72.0)
            } else {
                px(0.0)
            },
            vertical_padding: EDITOR_VERTICAL_PADDING,
            foreground: self.foreground,
            background: self.background,
            gutter_background: mix_rgb(self.background, self.foreground, 0.04),
            active_line_background: mix_rgb(self.background, self.foreground, 0.08),
            active_gutter_background: mix_rgb(self.background, self.foreground, 0.11),
            selected_line_background: mix_rgb(self.background, self.selection, 0.18),
            selected_gutter_background: mix_rgb(self.background, self.selection, 0.25),
            muted: mix_rgb(self.background, self.foreground, 0.68),
            dim: mix_rgb(self.background, self.foreground, 0.45),
            cursor: self.cursor,
            selection: self.selection,
            selection_alpha: self.selection_alpha.clamp(0.05, 1.0),
            cursor_style: self.cursor_style,
            show_line_numbers: effective.show_line_numbers,
            highlight_current_line: effective.highlight_current_line,
            visible_whitespace: effective.visible_whitespace,
            rulers: effective.rulers,
        }
    }
}

impl Default for EditorAppearanceConfig {
    fn default() -> Self {
        Self::from_config(&Config::default())
    }
}

#[derive(Clone)]
struct EditorAppearance {
    font_family: String,
    font_size: gpui::Pixels,
    line_height: gpui::Pixels,
    char_width: gpui::Pixels,
    line_number_width: gpui::Pixels,
    vertical_padding: gpui::Pixels,
    foreground: [u8; 3],
    background: [u8; 3],
    gutter_background: [u8; 3],
    active_line_background: [u8; 3],
    active_gutter_background: [u8; 3],
    selected_line_background: [u8; 3],
    selected_gutter_background: [u8; 3],
    muted: [u8; 3],
    dim: [u8; 3],
    cursor: [u8; 3],
    selection: [u8; 3],
    selection_alpha: f32,
    cursor_style: ConfigCursorStyle,
    show_line_numbers: bool,
    highlight_current_line: bool,
    visible_whitespace: bool,
    rulers: Vec<usize>,
}

impl EditorAppearance {
    fn foreground_color(&self) -> gpui::Rgba {
        rgb(rgb_u32(self.foreground))
    }

    fn background_color(&self) -> gpui::Rgba {
        rgb(rgb_u32(self.background))
    }

    fn gutter_color(&self) -> gpui::Rgba {
        rgb(rgb_u32(self.gutter_background))
    }

    fn active_line_color(&self) -> gpui::Rgba {
        rgb(rgb_u32(self.active_line_background))
    }

    fn active_gutter_color(&self) -> gpui::Rgba {
        rgb(rgb_u32(self.active_gutter_background))
    }

    fn selected_line_color(&self) -> gpui::Rgba {
        rgb(rgb_u32(self.selected_line_background))
    }

    fn selected_gutter_color(&self) -> gpui::Rgba {
        rgb(rgb_u32(self.selected_gutter_background))
    }

    fn muted_color(&self) -> gpui::Rgba {
        rgb(rgb_u32(self.muted))
    }

    fn dim_color(&self) -> gpui::Rgba {
        rgb(rgb_u32(self.dim))
    }

    fn cursor_color(&self) -> gpui::Rgba {
        rgba(rgba_u32(self.cursor, 1.0))
    }

    fn selection_color(&self) -> gpui::Rgba {
        rgba(rgba_u32(self.selection, self.selection_alpha))
    }

    fn ruler_color(&self) -> gpui::Rgba {
        rgba(rgba_u32(self.foreground, 0.12))
    }
}

#[derive(Clone)]
struct EditorBufferTabSnapshot {
    index: usize,
    title: String,
    subtitle: Option<String>,
    dirty: bool,
    active: bool,
}

#[derive(Clone)]
struct EditorSearchLineMatch {
    start_col: usize,
    end_col: usize,
    focused: bool,
}

#[derive(Clone)]
struct EditorDiagnosticSnapshot {
    line: usize,
    col: usize,
    end_line: usize,
    end_col: usize,
    severity: DiagSeverity,
    message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct EditorDiagnosticLineRange {
    start_col: usize,
    end_col: usize,
    severity: DiagSeverity,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct WorkspaceApplySummary {
    edits: usize,
    opened_files: usize,
    written_files: usize,
    failed_files: usize,
}

impl WorkspaceApplySummary {
    fn status(self, verb: &str) -> String {
        let file_total = self.opened_files + self.written_files;
        let mut status = format!("{verb} {} edit(s) across {file_total} file(s)", self.edits);
        if self.failed_files > 0 {
            status.push_str(&format!("; {} file(s) failed", self.failed_files));
        }
        status
    }
}

#[derive(Clone)]
struct ExternalFileChange {
    buffer_id: BufferId,
    path: PathBuf,
}

#[derive(Clone)]
struct ExternalFileChangeSnapshot {
    file_name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct EditorLineSelection {
    start_col: usize,
    end_col: usize,
    includes_line_break: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditorSearchDirection {
    Next,
    Previous,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditorSearchInputTarget {
    Query,
    Replacement,
}

#[cfg_attr(not(feature = "gpui-workspace"), allow(dead_code))]
fn move_sources_affect_path(moved_sources: &[(PathBuf, bool)], path: &Path) -> bool {
    moved_sources.iter().any(|(source, is_dir)| {
        if *is_dir {
            same_path(path, source) || path_contains(source, path)
        } else {
            same_path(path, source)
        }
    })
}

#[cfg_attr(not(feature = "gpui-workspace"), allow(dead_code))]
fn remap_path_after_move(path: &Path, moved: &[(PathBuf, PathBuf, bool)]) -> Option<PathBuf> {
    for (source, destination, is_dir) in moved {
        if *is_dir {
            if same_path(path, source) {
                return Some(destination.clone());
            }
            if path_contains(source, path) {
                let relative = path.strip_prefix(source).ok()?;
                return Some(destination.join(relative));
            }
        } else if same_path(path, source) {
            return Some(destination.clone());
        }
    }
    None
}

impl EditorPrototype {
    #[cfg_attr(not(feature = "gpui-workspace"), allow(dead_code))]
    pub(crate) fn new(cx: &mut Context<Self>) -> Self {
        Self::with_chrome(cx, false)
    }

    pub(crate) fn standalone(cx: &mut Context<Self>) -> Self {
        Self::with_chrome(cx, true)
    }

    #[cfg_attr(not(feature = "gpui-workspace"), allow(dead_code))]
    pub(crate) fn open_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        match self.editor.open(path.clone()) {
            Ok(buffer_id) => {
                refresh_active_syntax(&mut self.editor);
                self.editor_search.mark_dirty();
                self.remember_disk_text_for_path(&path);
                self.clear_external_change_for_path(&path);
                self.open_buffer_with_lsp(buffer_id);
                self.load_error = None;
                self.status_message = Some(format!("Opened {}", path.display()));
            }
            Err(err) => {
                self.load_error = Some(format!("{}: {err}", path.display()));
                self.status_message = Some("Open failed".to_string());
            }
        }
        cx.notify();
    }

    #[cfg_attr(not(feature = "gpui-workspace"), allow(dead_code))]
    pub(crate) fn active_path(&self) -> Option<PathBuf> {
        self.editor
            .active_buffer_view()
            .and_then(|(_, buffer, _)| buffer.path().map(PathBuf::from))
    }

    #[cfg_attr(not(feature = "gpui-workspace"), allow(dead_code))]
    pub(crate) fn open_find_from_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_find(window, cx);
    }

    #[cfg_attr(not(feature = "gpui-workspace"), allow(dead_code))]
    pub(crate) fn modified_open_path_for_move(
        &self,
        moved_sources: &[(PathBuf, bool)],
    ) -> Option<String> {
        self.editor.buffers.iter().find_map(|buffer| {
            let path = buffer.path()?;
            if !buffer.is_modified() || !move_sources_affect_path(moved_sources, path) {
                return None;
            }
            Some(format!(
                "Save or close {} before moving it.",
                buffer.file_name()
            ))
        })
    }

    #[cfg_attr(not(feature = "gpui-workspace"), allow(dead_code))]
    pub(crate) fn remap_moved_paths(
        &mut self,
        moved: &[(PathBuf, PathBuf, bool)],
        cx: &mut Context<Self>,
    ) {
        let mut remapped_active_path = None;
        for (idx, buffer) in self.editor.buffers.iter_mut().enumerate() {
            let Some(path) = buffer.path().map(PathBuf::from) else {
                continue;
            };
            let Some(new_path) = remap_path_after_move(&path, moved) else {
                continue;
            };

            let lang_id = self.editor.views.get(idx).and_then(|view| view.lang_id);
            let text = buffer.text();
            buffer.set_path(new_path.clone());
            if let Some(view) = self.editor.views.get_mut(idx) {
                view.tree_dirty = true;
                view.git_gutter = crate::editor::git_gutter::GitGutter::load(&new_path);
            }
            if let Some(lang_id) = lang_id {
                self.lsp.did_move(&path, &new_path, lang_id, &text);
            }
            if idx == self.editor.active {
                remapped_active_path = Some(new_path);
            }
        }

        if let Some(path) = remapped_active_path {
            self.status_message = Some(format!("Moved {}", path.display()));
        }
        self.rebuild_last_seen_disk_text();
        cx.notify();
    }

    pub(crate) fn save_active_buffer(&mut self, cx: &mut Context<Self>) {
        let active = self.editor.active;
        let Some(buffer) = self.editor.buffers.get_mut(active) else {
            self.status_message = Some("No active buffer to save".to_string());
            cx.notify();
            return;
        };

        let save_result = buffer.save();
        let path = buffer.path().map(PathBuf::from);
        let label = path
            .as_deref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| buffer.file_name().to_string());

        match save_result {
            Ok(()) => {
                let active_id = self.editor.buffer_id(active);
                if let Some(path) = path {
                    self.remember_disk_text_for_path(&path);
                    self.clear_external_change_for_path(&path);
                }
                if let Some(active_id) = active_id {
                    self.send_lsp_save_for_buffer_id(active_id);
                }
                self.status_message = Some(format!("Saved {label}"));
            }
            Err(err) => {
                self.status_message = Some(format!("Save failed: {err}"));
            }
        }
        cx.notify();
    }

    #[cfg_attr(not(feature = "gpui-workspace"), allow(dead_code))]
    pub(crate) fn copy_selection_to_clipboard(&mut self, cx: &mut Context<Self>) {
        self.copy_selection_or_line(cx);
    }

    pub(crate) fn paste_from_clipboard(&mut self, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.replace_selection_or_range(cx, None, &text);
        }
    }

    #[cfg_attr(not(feature = "gpui-workspace"), allow(dead_code))]
    pub(crate) fn select_all_text(&mut self, cx: &mut Context<Self>) {
        let visible_cols = self.visible_col_limit();
        if let Some((buffer, view)) = self.active_buffer_and_view() {
            view.cursor.select_all(buffer);
            reveal_cursor(view, buffer.line_count(), visible_cols);
            cx.notify();
        }
    }

    fn activate_buffer_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.editor.buffers.len() {
            return;
        }
        self.editor.switch_to(index);
        refresh_active_syntax(&mut self.editor);
        self.editor_search.mark_dirty();
        self.status_message = self
            .editor
            .buffers
            .get(index)
            .map(|buffer| format!("Focused {}", buffer.file_name()));
        cx.notify();
    }

    fn close_buffer_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(buffer) = self.editor.buffers.get(index) else {
            return;
        };
        if buffer.is_modified() {
            self.status_message = Some(format!("Save {} before closing it.", buffer.file_name()));
            cx.notify();
            return;
        }

        let active_id = self.editor.active_buffer_id();
        let closing_id = self.editor.buffer_id(index);
        let label = buffer.file_name().to_string();
        let closed_path = buffer.path().map(PathBuf::from);
        self.send_lsp_close_for_index(index);
        if self.editor.close(index) {
            if let Some(path) = closed_path.as_deref() {
                self.remember_recently_closed_path(path);
                self.clear_external_change_for_path(path);
            }
            if closing_id != active_id {
                if let Some(active_id) = active_id {
                    self.editor.switch_to_id(active_id);
                }
            }
            refresh_active_syntax(&mut self.editor);
            self.editor_search.mark_dirty();
            self.status_message = Some(format!("Closed {label}"));
            cx.notify();
        }
    }

    fn close_other_buffer_tabs(&mut self, cx: &mut Context<Self>) {
        let Some(active_id) = self.editor.active_buffer_id() else {
            self.status_message = Some("No active buffer".to_string());
            cx.notify();
            return;
        };

        let closing_ids = closable_other_buffer_ids(&self.editor, active_id);
        if closing_ids.is_empty() {
            let dirty_others = self
                .editor
                .buffers
                .iter()
                .zip(self.editor.buffer_ids.iter())
                .filter(|(buffer, id)| **id != active_id && buffer.is_modified())
                .count();
            self.status_message = if dirty_others > 0 {
                Some(format!(
                    "Save {dirty_others} modified buffer(s) before closing."
                ))
            } else {
                Some("No other buffers to close".to_string())
            };
            cx.notify();
            return;
        }

        let closed = self.close_buffer_ids(&closing_ids);
        self.editor.switch_to_id(active_id);
        refresh_active_syntax(&mut self.editor);
        self.editor_search.mark_dirty();
        self.status_message = Some(format!("Closed {closed} other buffer(s)"));
        cx.notify();
    }

    fn close_saved_buffer_tabs(&mut self, cx: &mut Context<Self>) {
        let closing_ids = closable_saved_buffer_ids(&self.editor);
        if closing_ids.is_empty() {
            self.status_message = Some("No saved buffers to close".to_string());
            cx.notify();
            return;
        }

        let active_id = self.editor.active_buffer_id();
        let closed = self.close_buffer_ids(&closing_ids);
        if let Some(active_id) = active_id {
            self.editor.switch_to_id(active_id);
        }
        refresh_active_syntax(&mut self.editor);
        self.editor_search.mark_dirty();
        self.status_message = Some(format!("Closed {closed} saved buffer(s)"));
        cx.notify();
    }

    fn reopen_recent_buffer_tab(&mut self, cx: &mut Context<Self>) {
        let open_paths = self.open_buffer_paths();
        let Some(path) = pop_reopen_candidate(&mut self.recently_closed_paths, &open_paths) else {
            self.status_message = Some("No recently closed file to reopen".to_string());
            cx.notify();
            return;
        };

        match self.editor.open(path.clone()) {
            Ok(buffer_id) => {
                refresh_active_syntax(&mut self.editor);
                self.editor_search.mark_dirty();
                self.remember_disk_text_for_path(&path);
                self.clear_external_change_for_path(&path);
                self.open_buffer_with_lsp(buffer_id);
                self.load_error = None;
                self.status_message = Some(format!("Reopened {}", path.display()));
            }
            Err(err) => {
                self.load_error = Some(format!("{}: {err}", path.display()));
                self.status_message = Some("Reopen failed".to_string());
            }
        }
        cx.notify();
    }

    fn check_active_external_change(&mut self, cx: &mut Context<Self>) {
        let Some((buffer_id, path, modified)) =
            self.editor
                .active_buffer_view()
                .and_then(|(buffer_id, buffer, _)| {
                    Some((
                        buffer_id,
                        buffer.path().map(PathBuf::from)?,
                        buffer.is_modified(),
                    ))
                })
        else {
            self.status_message = Some("No file-backed buffer to check".to_string());
            cx.notify();
            return;
        };

        let disk_text = match read_normalized_file_text(&path) {
            Ok(text) => text,
            Err(err) => {
                self.status_message = Some(format!("External check failed: {err}"));
                cx.notify();
                return;
            }
        };

        let Some(last_seen) = self.last_seen_disk_text.get(&path) else {
            self.last_seen_disk_text.insert(path.clone(), disk_text);
            self.status_message = Some(format!("Tracking {}", path.display()));
            cx.notify();
            return;
        };

        if *last_seen == disk_text {
            self.clear_external_change_for_path(&path);
            self.status_message = Some("No external changes".to_string());
            cx.notify();
            return;
        }

        if modified {
            self.external_change = Some(ExternalFileChange { buffer_id, path });
            self.status_message = Some("File changed on disk. Reload or keep local.".to_string());
            cx.notify();
            return;
        }

        self.reload_buffer_id_from_disk(buffer_id, cx);
    }

    fn reload_external_change(&mut self, cx: &mut Context<Self>) {
        let Some(change) = self.external_change.clone() else {
            self.reload_active_buffer_from_disk(cx);
            return;
        };
        self.reload_buffer_id_from_disk(change.buffer_id, cx);
    }

    fn keep_local_external_change(&mut self, cx: &mut Context<Self>) {
        let Some(change) = self.external_change.take() else {
            self.status_message = Some("No external change pending".to_string());
            cx.notify();
            return;
        };

        match read_normalized_file_text(&change.path) {
            Ok(text) => {
                self.last_seen_disk_text.insert(change.path.clone(), text);
                self.status_message = Some(format!("Keeping local {}", change.path.display()));
            }
            Err(err) => {
                self.status_message = Some(format!("Keep local failed: {err}"));
            }
        }
        cx.notify();
    }

    fn reload_active_buffer_from_disk(&mut self, cx: &mut Context<Self>) {
        let Some(buffer_id) = self.editor.active_buffer_id() else {
            self.status_message = Some("No active buffer to reload".to_string());
            cx.notify();
            return;
        };
        self.reload_buffer_id_from_disk(buffer_id, cx);
    }

    fn reload_buffer_id_from_disk(&mut self, buffer_id: BufferId, cx: &mut Context<Self>) {
        let Some(index) = self.editor.index_for_id(buffer_id) else {
            self.status_message = Some("Buffer is no longer open".to_string());
            cx.notify();
            return;
        };
        let Some(path) = self.editor.buffers[index].path().map(PathBuf::from) else {
            self.status_message = Some("No file-backed buffer to reload".to_string());
            cx.notify();
            return;
        };

        match self.reload_buffer_from_disk(index, &path) {
            Ok(()) => {
                self.editor.switch_to_id(buffer_id);
                self.clear_external_change_for_path(&path);
                refresh_active_syntax(&mut self.editor);
                self.editor_search.mark_dirty();
                self.open_buffer_with_lsp(buffer_id);
                self.status_message = Some(format!("Reloaded {}", path.display()));
            }
            Err(err) => {
                self.status_message = Some(format!("Reload failed: {err}"));
            }
        }
        cx.notify();
    }

    fn reload_buffer_from_disk(&mut self, index: usize, path: &Path) -> Result<(), String> {
        let buffer = Buffer::from_file(path)?;
        let lang_id = self.editor.syntax.detect_language(path);
        self.editor.buffers[index] = buffer;
        if let Some(view) = self.editor.views.get_mut(index) {
            view.lang_id = lang_id;
            view.tree = None;
            view.tree_dirty = lang_id.is_some();
            view.folded_ranges.clear();
            view.git_gutter = crate::editor::git_gutter::GitGutter::load(path);
            view.cursor.clamp(&self.editor.buffers[index]);
        }
        self.remember_disk_text_for_path(path);
        Ok(())
    }

    fn close_buffer_ids(&mut self, buffer_ids: &[BufferId]) -> usize {
        let mut closed = 0;
        for buffer_id in buffer_ids {
            let Some(index) = self.editor.index_for_id(*buffer_id) else {
                continue;
            };
            let path = self.editor.buffers[index].path().map(PathBuf::from);
            self.send_lsp_close_for_index(index);
            if self.editor.close(index) {
                closed += 1;
                if let Some(path) = path.as_deref() {
                    self.remember_recently_closed_path(path);
                    self.clear_external_change_for_path(path);
                }
            }
        }
        closed
    }

    fn remember_recently_closed_path(&mut self, path: &Path) {
        remember_recently_closed_path(&mut self.recently_closed_paths, path.to_path_buf());
    }

    fn remember_disk_text_for_path(&mut self, path: &Path) {
        if let Ok(text) = read_normalized_file_text(path) {
            self.last_seen_disk_text.insert(path.to_path_buf(), text);
        }
    }

    fn rebuild_last_seen_disk_text(&mut self) {
        self.last_seen_disk_text.clear();
        let paths = self.open_buffer_paths();
        for path in paths {
            self.remember_disk_text_for_path(&path);
        }
    }

    fn clear_external_change_for_path(&mut self, path: &Path) {
        if self
            .external_change
            .as_ref()
            .is_some_and(|change| same_path(&change.path, path))
        {
            self.external_change = None;
        }
    }

    fn open_buffer_paths(&self) -> HashSet<PathBuf> {
        self.editor
            .buffers
            .iter()
            .filter_map(|buffer| buffer.path().map(PathBuf::from))
            .collect()
    }

    pub(crate) fn undo_edit(&mut self, cx: &mut Context<Self>) {
        self.edit_active(cx, |buffer, view| {
            if let Some(pos) = buffer.undo() {
                view.cursor.pos = pos;
                view.cursor.clear_selection();
                view.cursor.desired_col = None;
            }
        });
    }

    pub(crate) fn redo_edit(&mut self, cx: &mut Context<Self>) {
        self.edit_active(cx, |buffer, view| {
            if let Some(pos) = buffer.redo() {
                view.cursor.pos = pos;
                view.cursor.clear_selection();
                view.cursor.desired_col = None;
            }
        });
    }

    fn with_chrome(cx: &mut Context<Self>, show_chrome: bool) -> Self {
        start_cursor_blink_task(cx);
        start_lsp_poll_task(cx);
        let mut editor = EditorState::new();
        let mut load_error = None;
        let mut last_seen_disk_text = HashMap::new();

        if let Some(path) = initial_path() {
            if let Err(err) = editor.open(path.clone()) {
                load_error = Some(format!("{}: {err}", path.display()));
            } else {
                refresh_active_syntax(&mut editor);
                if let Ok(text) = read_normalized_file_text(&path) {
                    last_seen_disk_text.insert(path, text);
                }
            }
        } else {
            load_error = Some("No readable file found for GPUI editor prototype".to_string());
        }

        let mut this = Self {
            focus_handle: cx.focus_handle(),
            editor,
            lsp: LspManager::new_without_event_proxy(),
            lsp_pending: GpuiLspPending::default(),
            lsp_pending_changes: HashMap::new(),
            lsp_panel: None,
            lsp_snapshot_key: String::new(),
            appearance_config: EditorAppearanceConfig::default(),
            editor_search: EditorSearch::default(),
            search_input_target: EditorSearchInputTarget::Query,
            go_to_line_active: false,
            go_to_line_input: String::new(),
            rename_active: false,
            rename_input: String::new(),
            recently_closed_paths: Vec::new(),
            last_seen_disk_text,
            external_change: None,
            load_error,
            sample_text: sample_text(),
            sample_scroll_line: 0,
            status_message: None,
            last_text_bounds: None,
            last_text_layout: None,
            is_selecting: false,
            cursor_blink_anchor: Instant::now(),
            cursor_blink_visible: true,
            show_chrome,
        };
        this.open_all_file_backed_buffers_with_lsp();
        this
    }

    pub(crate) fn set_appearance_config(&mut self, config: Config, cx: &mut Context<Self>) {
        self.appearance_config = EditorAppearanceConfig::from_config(&config);
        self.last_text_layout = None;
        cx.notify();
    }

    fn open_all_file_backed_buffers_with_lsp(&mut self) {
        let buffer_ids = self.editor.buffer_ids.clone();
        for buffer_id in buffer_ids {
            self.open_buffer_with_lsp(buffer_id);
        }
    }

    fn active_lsp_context(&self) -> Option<(BufferId, PathBuf, &'static str, Position, usize)> {
        let (buffer_id, buffer, view) = self.editor.active_buffer_view()?;
        Some((
            buffer_id,
            buffer.path().map(PathBuf::from)?,
            view.lang_id?,
            view.cursor.pos,
            buffer.line_count(),
        ))
    }

    fn open_buffer_with_lsp(&mut self, buffer_id: BufferId) -> Option<LspEnsureStatus> {
        let index = self.editor.index_for_id(buffer_id)?;
        let buffer = self.editor.buffers.get(index)?;
        let view = self.editor.views.get(index)?;
        let path = buffer.path()?.to_path_buf();
        let lang_id = view.lang_id?;
        if !perf::live_lsp_enabled(buffer.line_count()) {
            return None;
        }
        if let Some(root) = LspManager::detect_root(&path) {
            self.lsp.set_root(root);
        }
        let text = buffer.text();
        let status = self.lsp.open_document(&path, lang_id, &text);
        self.lsp_pending_changes.remove(&buffer_id);
        Some(status)
    }

    fn queue_lsp_change_for_buffer_id(&mut self, buffer_id: BufferId, edit: Option<BufferEdit>) {
        if !self
            .lsp_buffer_context(buffer_id)
            .is_some_and(|(_, _, line_count)| perf::live_lsp_enabled(line_count))
        {
            return;
        }

        let existing = self
            .lsp_pending_changes
            .get(&buffer_id)
            .map(|change| &change.kind);
        let kind = next_lsp_change_kind(existing, edit);
        self.lsp_pending_changes.insert(
            buffer_id,
            GpuiPendingLspChange {
                queued_at: Instant::now(),
                kind,
            },
        );
    }

    fn flush_due_lsp_changes(&mut self) -> bool {
        if self.lsp_pending_changes.is_empty() {
            return false;
        }

        let now = Instant::now();
        let due = self
            .lsp_pending_changes
            .iter()
            .filter_map(|(buffer_id, change)| {
                (now.duration_since(change.queued_at)
                    >= Duration::from_millis(perf::LSP_DEBOUNCE_MS))
                .then_some(*buffer_id)
            })
            .collect::<Vec<_>>();
        if due.is_empty() {
            return false;
        }

        for buffer_id in due {
            self.flush_lsp_change_for_buffer_id(buffer_id);
        }
        false
    }

    fn flush_lsp_change_for_buffer_id(&mut self, buffer_id: BufferId) {
        let Some(change) = self.lsp_pending_changes.remove(&buffer_id) else {
            return;
        };
        self.send_lsp_change_for_buffer_id(buffer_id, Some(change.kind));
    }

    fn lsp_buffer_context(&self, buffer_id: BufferId) -> Option<(PathBuf, &'static str, usize)> {
        let Some(index) = self.editor.index_for_id(buffer_id) else {
            return None;
        };
        let Some(buffer) = self.editor.buffers.get(index) else {
            return None;
        };
        let Some(path) = buffer.path().map(PathBuf::from) else {
            return None;
        };
        let Some(lang_id) = self.editor.views.get(index).and_then(|view| view.lang_id) else {
            return None;
        };
        Some((path, lang_id, buffer.line_count()))
    }

    fn send_lsp_change_for_buffer_id(
        &mut self,
        buffer_id: BufferId,
        kind: Option<GpuiPendingLspChangeKind>,
    ) {
        let Some(index) = self.editor.index_for_id(buffer_id) else {
            return;
        };
        let Some(buffer) = self.editor.buffers.get(index) else {
            return;
        };
        if !perf::live_lsp_enabled(buffer.line_count()) {
            return;
        }
        let Some(path) = buffer.path().map(PathBuf::from) else {
            return;
        };
        let Some(lang_id) = self.editor.views.get(index).and_then(|view| view.lang_id) else {
            return;
        };

        if let Some(GpuiPendingLspChangeKind::Incremental {
            start,
            old_end,
            new_text,
        }) = kind
        {
            self.lsp.did_change_incremental(IncrementalDocumentChange {
                path: &path,
                lang_id,
                start_line: start.line as u32,
                start_col: start.col as u32,
                end_line: old_end.line as u32,
                end_col: old_end.col as u32,
                new_text: &new_text,
            });
            return;
        }

        let text = buffer.text();
        self.lsp.did_change(&path, lang_id, &text);
    }

    fn send_lsp_save_for_buffer_id(&mut self, buffer_id: BufferId) {
        self.flush_lsp_change_for_buffer_id(buffer_id);
        let Some(index) = self.editor.index_for_id(buffer_id) else {
            return;
        };
        let Some(buffer) = self.editor.buffers.get(index) else {
            return;
        };
        let Some(path) = buffer.path().map(PathBuf::from) else {
            return;
        };
        let Some(lang_id) = self.editor.views.get(index).and_then(|view| view.lang_id) else {
            return;
        };
        let text = buffer.text();
        self.lsp.did_save(&path, lang_id, &text);
    }

    fn send_lsp_close_for_index(&mut self, index: usize) {
        if let Some(buffer_id) = self.editor.buffer_id(index) {
            self.flush_lsp_change_for_buffer_id(buffer_id);
        }
        let Some(buffer) = self.editor.buffers.get(index) else {
            return;
        };
        let Some(path) = buffer.path().map(PathBuf::from) else {
            return;
        };
        let Some(lang_id) = self.editor.views.get(index).and_then(|view| view.lang_id) else {
            return;
        };
        self.lsp.did_close(&path, lang_id);
    }

    fn poll_lsp(&mut self, _cx: &mut Context<Self>) -> bool {
        self.flush_due_lsp_changes();
        self.lsp.drain_server_messages();
        let mut changed = false;

        if let Some((buffer_id, result)) = poll_lsp_request(&mut self.lsp_pending.hover) {
            changed = true;
            if self.request_targets_active_buffer(buffer_id) {
                match result {
                    Ok(Some(text)) => {
                        self.lsp_panel = Some(lsp_panel(
                            "Hover",
                            plain_lsp_panel_items(panel_lines(text, 8)),
                        ));
                        self.status_message = Some("Hover ready".to_string());
                    }
                    Ok(None) => self.status_message = Some("No hover information".to_string()),
                    Err(()) => self.status_message = Some("Hover request closed".to_string()),
                }
            }
        }

        if let Some((buffer_id, result)) = poll_lsp_request(&mut self.lsp_pending.completion) {
            changed = true;
            if self.request_targets_active_buffer(buffer_id) {
                match result {
                    Ok(items) if items.is_empty() => {
                        self.status_message = Some("No completions".to_string());
                    }
                    Ok(items) => {
                        let count = items.len();
                        self.lsp_panel = Some(lsp_panel(
                            format!("Completions ({count})"),
                            completion_panel_items(items, 14),
                        ));
                        self.status_message = Some(format!("{count} completion(s)"));
                    }
                    Err(()) => self.status_message = Some("Completion request closed".to_string()),
                }
            }
        }

        if let Some((buffer_id, result)) = poll_lsp_request(&mut self.lsp_pending.definition) {
            changed = true;
            if self.request_targets_active_buffer(buffer_id) {
                match result {
                    Ok(Some((path, line, col))) => self.apply_lsp_definition(path, line, col),
                    Ok(None) => self.status_message = Some("No definition found".to_string()),
                    Err(()) => self.status_message = Some("Definition request closed".to_string()),
                }
            }
        }

        if let Some((buffer_id, result)) = poll_lsp_request(&mut self.lsp_pending.signature_help) {
            changed = true;
            if self.request_targets_active_buffer(buffer_id) {
                match result {
                    Ok(Some(signature)) => {
                        self.lsp_panel = Some(lsp_panel(
                            "Signature",
                            plain_lsp_panel_items(signature_panel_items(signature)),
                        ));
                    }
                    Ok(None) => self.status_message = Some("No signature help".to_string()),
                    Err(()) => self.status_message = Some("Signature request closed".to_string()),
                }
            }
        }

        if let Some((buffer_id, result)) = poll_lsp_request(&mut self.lsp_pending.references) {
            changed = true;
            if self.request_targets_active_buffer(buffer_id) {
                match result {
                    Ok(references) if references.is_empty() => {
                        self.status_message = Some("No references found".to_string());
                    }
                    Ok(references) => {
                        let count = references.len();
                        self.lsp_panel = Some(lsp_panel(
                            format!("References ({count})"),
                            references_panel_items(references, 14),
                        ));
                    }
                    Err(()) => self.status_message = Some("References request closed".to_string()),
                }
            }
        }

        if let Some((buffer_id, result)) = poll_lsp_request(&mut self.lsp_pending.format) {
            changed = true;
            if self.editor.index_for_id(buffer_id).is_some() {
                match result {
                    Ok(edits) => {
                        let applied = self.apply_lsp_format_edits(buffer_id, edits);
                        self.status_message = if applied == 0 {
                            Some("No formatting changes".to_string())
                        } else {
                            Some("Formatted".to_string())
                        };
                    }
                    Err(()) => self.status_message = Some("Format request closed".to_string()),
                }
            }
        }

        if let Some((buffer_id, result)) = poll_lsp_request(&mut self.lsp_pending.code_actions) {
            changed = true;
            if self.request_targets_active_buffer(buffer_id) {
                match result {
                    Ok(actions) if actions.is_empty() => {
                        self.status_message = Some("No code actions".to_string());
                    }
                    Ok(actions) => {
                        let count = actions.len();
                        self.lsp_panel = Some(lsp_panel(
                            format!("Code Actions ({count})"),
                            code_action_panel_items(actions, 14),
                        ));
                    }
                    Err(()) => self.status_message = Some("Code actions closed".to_string()),
                }
            }
        }

        if let Some((buffer_id, result)) = poll_lsp_request(&mut self.lsp_pending.document_symbols)
        {
            changed = true;
            if self.request_targets_active_buffer(buffer_id) {
                match result {
                    Ok(symbols) if symbols.is_empty() => {
                        self.status_message = Some("No symbols found".to_string());
                    }
                    Ok(symbols) => {
                        let count = symbols.len();
                        let path = self
                            .editor
                            .active_buffer_view()
                            .and_then(|(_, buffer, _)| buffer.path().map(PathBuf::from));
                        self.lsp_panel = Some(lsp_panel(
                            format!("Symbols ({count})"),
                            symbols_panel_items(symbols, path, 14),
                        ));
                    }
                    Err(()) => self.status_message = Some("Symbols request closed".to_string()),
                }
            }
        }

        if let Some((buffer_id, result)) = poll_lsp_request(&mut self.lsp_pending.rename) {
            changed = true;
            if self.request_targets_active_buffer(buffer_id) {
                match result {
                    Ok(edits) => {
                        let summary = self.apply_lsp_workspace_edits(edits);
                        self.status_message = Some(summary.status("Renamed"));
                    }
                    Err(()) => self.status_message = Some("Rename request closed".to_string()),
                }
            }
        }

        let key = self.active_lsp_snapshot_key();
        if self.lsp_snapshot_key != key {
            self.lsp_snapshot_key = key;
            changed = true;
        }

        changed
    }

    fn request_targets_active_buffer(&self, buffer_id: BufferId) -> bool {
        self.editor.active_buffer_id() == Some(buffer_id)
    }

    fn active_lsp_snapshot_key(&self) -> String {
        let status = self.active_lsp_status();
        let diagnostics = self.active_diagnostics_snapshot();
        format!(
            "{status}|{}|{}",
            diagnostics.len(),
            diagnostics
                .iter()
                .take(6)
                .map(|diagnostic| format!(
                    "{}:{}:{:?}:{}",
                    diagnostic.line, diagnostic.col, diagnostic.severity, diagnostic.message
                ))
                .collect::<Vec<_>>()
                .join("|")
        )
    }

    fn active_lsp_status(&self) -> String {
        let Some((_, _, view)) = self.editor.active_buffer_view() else {
            return String::new();
        };
        let Some(lang_id) = view.lang_id else {
            return "LSP: plain text".to_string();
        };
        let status = self.lsp.server_status(lang_id);
        if status.is_empty() {
            String::new()
        } else {
            format!("LSP: {status}")
        }
    }

    fn active_diagnostics_snapshot(&self) -> Vec<EditorDiagnosticSnapshot> {
        let Some((_, buffer, _)) = self.editor.active_buffer_view() else {
            return Vec::new();
        };
        let Some(path) = buffer.path() else {
            return Vec::new();
        };
        self.lsp
            .get_diagnostics(path)
            .iter()
            .map(diagnostic_snapshot)
            .collect()
    }

    fn request_lsp_hover(&mut self, cx: &mut Context<Self>) {
        let Some((buffer_id, path, lang_id, pos, line_count)) = self.active_lsp_context() else {
            self.status_message = Some("No language server for this buffer".to_string());
            cx.notify();
            return;
        };
        if !perf::live_lsp_enabled(line_count) {
            self.status_message = Some("LSP disabled for this large file".to_string());
            cx.notify();
            return;
        }
        self.open_buffer_with_lsp(buffer_id);
        if let Some(rx) = self
            .lsp
            .hover_async(&path, lang_id, pos.line as u32, pos.col as u32)
        {
            self.lsp_pending.hover = Some(GpuiPendingLspRequest::new(buffer_id, rx));
            self.status_message = Some("Loading hover...".to_string());
        } else {
            self.status_message = Some("LSP is starting or unavailable".to_string());
        }
        cx.notify();
    }

    fn request_lsp_completion(&mut self, cx: &mut Context<Self>) {
        let Some((buffer_id, path, lang_id, pos, line_count)) = self.active_lsp_context() else {
            self.status_message = Some("No language server for this buffer".to_string());
            cx.notify();
            return;
        };
        if !perf::live_lsp_enabled(line_count) {
            self.status_message = Some("LSP disabled for this large file".to_string());
            cx.notify();
            return;
        }
        self.open_buffer_with_lsp(buffer_id);
        if let Some(rx) = self
            .lsp
            .completion_async(&path, lang_id, pos.line as u32, pos.col as u32)
        {
            self.lsp_pending.completion = Some(GpuiPendingLspRequest::new(buffer_id, rx));
            self.status_message = Some("Loading completions...".to_string());
        } else {
            self.status_message = Some("LSP is starting or unavailable".to_string());
        }
        cx.notify();
    }

    fn request_lsp_definition(&mut self, cx: &mut Context<Self>) {
        let Some((buffer_id, path, lang_id, pos, line_count)) = self.active_lsp_context() else {
            self.status_message = Some("No language server for this buffer".to_string());
            cx.notify();
            return;
        };
        if !perf::live_lsp_enabled(line_count) {
            self.status_message = Some("LSP disabled for this large file".to_string());
            cx.notify();
            return;
        }
        self.open_buffer_with_lsp(buffer_id);
        if let Some(rx) = self
            .lsp
            .definition_async(&path, lang_id, pos.line as u32, pos.col as u32)
        {
            self.lsp_pending.definition = Some(GpuiPendingLspRequest::new(buffer_id, rx));
            self.status_message = Some("Finding definition...".to_string());
        } else {
            self.status_message = Some("LSP is starting or unavailable".to_string());
        }
        cx.notify();
    }

    fn request_lsp_references(&mut self, cx: &mut Context<Self>) {
        let Some((buffer_id, path, lang_id, pos, line_count)) = self.active_lsp_context() else {
            self.status_message = Some("No language server for this buffer".to_string());
            cx.notify();
            return;
        };
        if !perf::live_lsp_enabled(line_count) {
            self.status_message = Some("LSP disabled for this large file".to_string());
            cx.notify();
            return;
        }
        self.open_buffer_with_lsp(buffer_id);
        if let Some(rx) = self
            .lsp
            .references_async(&path, lang_id, pos.line as u32, pos.col as u32)
        {
            self.lsp_pending.references = Some(GpuiPendingLspRequest::new(buffer_id, rx));
            self.status_message = Some("Finding references...".to_string());
        } else {
            self.status_message = Some("LSP is starting or unavailable".to_string());
        }
        cx.notify();
    }

    fn request_lsp_signature_help(&mut self, cx: &mut Context<Self>) {
        let Some((buffer_id, path, lang_id, pos, line_count)) = self.active_lsp_context() else {
            self.status_message = Some("No language server for this buffer".to_string());
            cx.notify();
            return;
        };
        if !perf::live_lsp_enabled(line_count) {
            self.status_message = Some("LSP disabled for this large file".to_string());
            cx.notify();
            return;
        }
        self.open_buffer_with_lsp(buffer_id);
        if let Some(rx) =
            self.lsp
                .signature_help_async(&path, lang_id, pos.line as u32, pos.col as u32)
        {
            self.lsp_pending.signature_help = Some(GpuiPendingLspRequest::new(buffer_id, rx));
            self.status_message = Some("Loading signature help...".to_string());
        } else {
            self.status_message = Some("LSP is starting or unavailable".to_string());
        }
        cx.notify();
    }

    fn request_lsp_format(&mut self, cx: &mut Context<Self>) {
        let Some((buffer_id, path, lang_id, _, line_count)) = self.active_lsp_context() else {
            self.status_message = Some("No language server for this buffer".to_string());
            cx.notify();
            return;
        };
        if !perf::live_lsp_enabled(line_count) {
            self.status_message = Some("LSP disabled for this large file".to_string());
            cx.notify();
            return;
        }
        self.open_buffer_with_lsp(buffer_id);
        let selection = self
            .editor
            .active_buffer_view()
            .and_then(|(_, _, view)| view.cursor.selection());
        let rx = if let Some((start, end)) = selection {
            self.lsp.range_format_async(
                &path,
                lang_id,
                start.line as u32,
                start.col as u32,
                end.line as u32,
                end.col as u32,
            )
        } else {
            self.lsp.format_async(&path, lang_id)
        };
        if let Some(rx) = rx {
            self.lsp_pending.format = Some(GpuiPendingLspRequest::new(buffer_id, rx));
            self.status_message = Some(if selection.is_some() {
                "Formatting selection...".to_string()
            } else {
                "Formatting document...".to_string()
            });
        } else {
            self.status_message = Some("LSP is starting or unavailable".to_string());
        }
        cx.notify();
    }

    fn request_lsp_code_actions(&mut self, cx: &mut Context<Self>) {
        let Some((buffer_id, path, lang_id, pos, line_count)) = self.active_lsp_context() else {
            self.status_message = Some("No language server for this buffer".to_string());
            cx.notify();
            return;
        };
        if !perf::live_lsp_enabled(line_count) {
            self.status_message = Some("LSP disabled for this large file".to_string());
            cx.notify();
            return;
        }
        let (start, end) = self
            .editor
            .active_buffer_view()
            .and_then(|(_, _, view)| view.cursor.selection())
            .unwrap_or((pos, pos));
        self.open_buffer_with_lsp(buffer_id);
        if let Some(rx) = self.lsp.code_actions_async(
            &path,
            lang_id,
            start.line as u32,
            start.col as u32,
            end.line as u32,
            end.col as u32,
        ) {
            self.lsp_pending.code_actions = Some(GpuiPendingLspRequest::new(buffer_id, rx));
            self.status_message = Some("Loading code actions...".to_string());
        } else {
            self.status_message = Some("LSP is starting or unavailable".to_string());
        }
        cx.notify();
    }

    fn request_lsp_symbols(&mut self, cx: &mut Context<Self>) {
        let Some((buffer_id, path, lang_id, _, line_count)) = self.active_lsp_context() else {
            self.status_message = Some("No language server for this buffer".to_string());
            cx.notify();
            return;
        };
        if !perf::live_lsp_enabled(line_count) {
            self.status_message = Some("LSP disabled for this large file".to_string());
            cx.notify();
            return;
        }
        self.open_buffer_with_lsp(buffer_id);
        if let Some(rx) = self.lsp.document_symbols_async(&path, lang_id) {
            self.lsp_pending.document_symbols = Some(GpuiPendingLspRequest::new(buffer_id, rx));
            self.status_message = Some("Loading symbols...".to_string());
        } else {
            self.status_message = Some("LSP is starting or unavailable".to_string());
        }
        cx.notify();
    }

    fn open_lsp_rename(&mut self, cx: &mut Context<Self>) {
        let seed = self
            .editor
            .active_buffer_view()
            .and_then(|(_, buffer, view)| view.cursor.word_or_selection_text(buffer))
            .unwrap_or_default();
        self.editor_search.close();
        self.search_input_target = EditorSearchInputTarget::Query;
        self.go_to_line_active = false;
        self.go_to_line_input.clear();
        self.lsp_panel = None;
        self.rename_active = true;
        self.rename_input = seed;
        cx.notify();
    }

    fn close_lsp_rename(&mut self, cx: &mut Context<Self>) {
        if self.rename_active {
            self.rename_active = false;
            self.rename_input.clear();
            cx.notify();
        }
    }

    fn push_lsp_rename_text(&mut self, text: &str, cx: &mut Context<Self>) {
        for ch in text.chars().filter(|ch| !ch.is_control()) {
            if self.rename_input.chars().count() < 160 {
                self.rename_input.push(ch);
            }
        }
        cx.notify();
    }

    fn pop_lsp_rename_text(&mut self, cx: &mut Context<Self>) {
        self.rename_input.pop();
        cx.notify();
    }

    fn submit_lsp_rename(&mut self, cx: &mut Context<Self>) {
        let new_name = self.rename_input.trim().to_string();
        if new_name.is_empty() {
            self.status_message = Some("Enter a new symbol name".to_string());
            cx.notify();
            return;
        }
        self.rename_active = false;
        self.rename_input.clear();
        self.request_lsp_rename(new_name, cx);
    }

    fn request_lsp_rename(&mut self, new_name: String, cx: &mut Context<Self>) {
        let Some((buffer_id, path, lang_id, pos, line_count)) = self.active_lsp_context() else {
            self.status_message = Some("No language server for this buffer".to_string());
            cx.notify();
            return;
        };
        if !perf::live_lsp_enabled(line_count) {
            self.status_message = Some("LSP disabled for this large file".to_string());
            cx.notify();
            return;
        }
        self.open_buffer_with_lsp(buffer_id);
        if let Some(rx) =
            self.lsp
                .rename_async(&path, lang_id, pos.line as u32, pos.col as u32, &new_name)
        {
            self.lsp_pending.rename = Some(GpuiPendingLspRequest::new(buffer_id, rx));
            self.status_message = Some(format!("Renaming to {new_name}..."));
        } else {
            self.status_message = Some("LSP is starting or unavailable".to_string());
        }
        cx.notify();
    }

    fn close_lsp_panel(&mut self, cx: &mut Context<Self>) {
        self.lsp_panel = None;
        cx.notify();
    }

    fn close_lsp_panel_without_notify(&mut self) -> bool {
        self.lsp_panel.take().is_some()
    }

    fn move_lsp_panel_selection(&mut self, delta: isize, cx: &mut Context<Self>) -> bool {
        let Some(panel) = self.lsp_panel.as_mut() else {
            return false;
        };
        if panel.items.is_empty() {
            return false;
        }
        let len = panel.items.len() as isize;
        let selected = (panel.selected as isize + delta).rem_euclid(len) as usize;
        panel.selected = selected;
        cx.notify();
        true
    }

    fn accept_lsp_panel_selection(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(index) = self.lsp_panel.as_ref().map(|panel| panel.selected) else {
            return false;
        };
        self.activate_lsp_panel_item(index, cx);
        true
    }

    fn activate_lsp_panel_item(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(action) = self
            .lsp_panel
            .as_ref()
            .and_then(|panel| panel.items.get(index))
            .map(|item| item.action.clone())
        else {
            return;
        };

        match action {
            GpuiLspPanelAction::None => {}
            GpuiLspPanelAction::Complete { text } => {
                self.replace_selection_or_range(cx, None, &text);
                self.status_message = Some("Completion inserted".to_string());
            }
            GpuiLspPanelAction::GoTo { path, line, col } => {
                self.apply_lsp_definition(path, line, col);
            }
            GpuiLspPanelAction::ApplyWorkspaceEdit { edits } => {
                let summary = self.apply_lsp_workspace_edits(edits);
                self.status_message = Some(summary.status("Applied"));
            }
        }

        self.lsp_panel = None;
        cx.notify();
    }

    fn apply_lsp_definition(&mut self, path: PathBuf, line: u32, col: u32) {
        match self.editor.open(path.clone()) {
            Ok(buffer_id) => {
                refresh_active_syntax(&mut self.editor);
                self.open_buffer_with_lsp(buffer_id);
                if let Some(index) = self.editor.index_for_id(buffer_id) {
                    let line = line as usize;
                    let col = col as usize;
                    let visible_cols = self.visible_col_limit();
                    if let Some(buffer) = self.editor.buffers.get(index) {
                        let line_count = buffer.line_count();
                        let target_line = line.min(line_count.saturating_sub(1));
                        let target =
                            Position::new(target_line, col.min(buffer.line_len(target_line)));
                        if let Some(view) = self.editor.views.get_mut(index) {
                            view.cursor.pos = target;
                            view.cursor.clear_selection();
                            view.cursor.desired_col = None;
                            reveal_cursor(view, line_count, visible_cols);
                        }
                    }
                }
                self.editor_search.mark_dirty();
                self.status_message = Some(format!("Opened definition {}", path.display()));
            }
            Err(err) => {
                self.status_message = Some(format!("Definition open failed: {err}"));
            }
        }
    }

    fn apply_lsp_format_edits(&mut self, buffer_id: BufferId, edits: Vec<FormatEdit>) -> usize {
        let Some(index) = self.editor.index_for_id(buffer_id) else {
            return 0;
        };
        if edits.is_empty() {
            return 0;
        }
        let mut sorted = edits;
        sorted.sort_by(|a, b| {
            b.start_line
                .cmp(&a.start_line)
                .then(b.start_col.cmp(&a.start_col))
        });
        let mut applied = 0;
        if let Some(buffer) = self.editor.buffers.get_mut(index) {
            for edit in sorted {
                let start = Position::new(edit.start_line as usize, edit.start_col as usize);
                let end = Position::new(edit.end_line as usize, edit.end_col as usize);
                buffer.replace(start, end, &edit.new_text);
                applied += 1;
            }
        }
        if applied > 0 {
            if let Some(view) = self.editor.views.get_mut(index) {
                view.tree_dirty = true;
            }
            self.editor_search.mark_dirty();
            refresh_active_syntax(&mut self.editor);
            self.send_lsp_change_for_buffer_id(buffer_id, Some(GpuiPendingLspChangeKind::Full));
        }
        applied
    }

    fn apply_lsp_workspace_edits(&mut self, file_edits: WorkspaceEdits) -> WorkspaceApplySummary {
        let mut summary = WorkspaceApplySummary::default();
        for (path, edits) in file_edits {
            let Some(index) = self
                .editor
                .buffers
                .iter()
                .position(|buffer| buffer.path() == Some(path.as_path()))
            else {
                match apply_format_edits_to_file(&path, &edits) {
                    Ok(applied) => {
                        summary.edits += applied;
                        summary.written_files += 1;
                        self.remember_disk_text_for_path(&path);
                        self.clear_external_change_for_path(&path);
                    }
                    Err(err) => {
                        log::warn!("workspace edit failed for {}: {err}", path.display());
                        summary.failed_files += 1;
                    }
                }
                continue;
            };
            let Some(buffer_id) = self.editor.buffer_id(index) else {
                continue;
            };
            let applied = self.apply_lsp_format_edits(buffer_id, edits);
            summary.edits += applied;
            summary.opened_files += 1;
        }
        summary
    }

    fn snapshot(&mut self, is_focused: bool) -> EditorSnapshot {
        refresh_active_syntax(&mut self.editor);
        if self.editor_search.active {
            let active = self.editor.active;
            if let Some(buffer) = self.editor.buffers.get(active) {
                self.editor_search.update_if_dirty(buffer);
            }
        }
        let buffer_tabs = buffer_tab_snapshots(&self.editor);
        let search_active = self.editor_search.active;
        let search_query = self.editor_search.query.clone();
        let search_replacement = self.editor_search.replacement.clone();
        let search_replace_mode = self.editor_search.replace_mode;
        let search_input_target = self.search_input_target;
        let search_status = self.editor_search.status();
        let go_to_line_active = self.go_to_line_active;
        let go_to_line_input = self.go_to_line_input.clone();
        let rename_active = self.rename_active;
        let rename_input = self.rename_input.clone();
        let cursor_visible = is_focused && (self.is_selecting || self.cursor_blink_visible);
        let can_reopen_recent = !self.recently_closed_paths.is_empty();
        let lsp_status = self.active_lsp_status();
        let diagnostics = self.active_diagnostics_snapshot();
        let external_change =
            self.external_change
                .as_ref()
                .map(|change| ExternalFileChangeSnapshot {
                    file_name: change
                        .path
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| change.path.display().to_string()),
                });
        if let Some((buffer_id, buffer, view)) = self.editor.active_buffer_view() {
            let appearance = self.appearance_config.for_language(view.lang_id);
            let line_count = buffer.line_count();
            let degraded_notice =
                perf::LargeFileDegradation::for_line_count(line_count).status_label();
            let cursor_diagnostic_message = diagnostic_at_position(&diagnostics, view.cursor.pos)
                .map(|diagnostic| format!("{:?}: {}", diagnostic.severity, diagnostic.message));
            let lsp_panel = self.lsp_panel.clone().map(|mut panel| {
                panel.anchor = lsp_panel_anchor(
                    view.cursor.pos,
                    view.scroll_line,
                    view.scroll_col,
                    &appearance,
                );
                panel
            });
            let visible_start = view.scroll_line.min(line_count.saturating_sub(1));
            let visible_end = line_count.min(visible_start + VISIBLE_LINE_LIMIT);
            let highlight_spans = match (view.lang_id, view.tree.as_ref()) {
                (Some(lang_id), Some(tree)) => self.editor.syntax.highlights_for_range(
                    lang_id,
                    tree,
                    buffer.text().as_bytes(),
                    visible_start,
                    visible_end,
                ),
                _ => vec![Vec::new(); visible_end.saturating_sub(visible_start)],
            };
            let lines = (visible_start..visible_end)
                .enumerate()
                .map(|(idx, line)| EditorLineSnapshot {
                    number: line + 1,
                    text: buffer.line(line).to_string(),
                    highlights: highlight_spans.get(idx).cloned().unwrap_or_default(),
                    search_matches: search_matches_for_line(&self.editor_search, line),
                    diagnostic: diagnostic_for_line(&diagnostics, line),
                })
                .collect();

            return EditorSnapshot {
                title: buffer
                    .path()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| buffer.file_name().to_string()),
                subtitle: format!(
                    "buffer #{}, {} lines, {} chars{}",
                    buffer_id.raw(),
                    line_count,
                    buffer.len_chars(),
                    if buffer.is_modified() {
                        ", modified"
                    } else {
                        ""
                    }
                ),
                language: language_label(view),
                lines,
                first_line_number: visible_start + 1,
                cursor: Some(view.cursor.pos),
                cursor_visible,
                selection: view.cursor.selection(),
                scroll_col: view.scroll_col,
                total_lines: line_count,
                load_error: self.load_error.clone(),
                status_message: self.status_message.clone(),
                buffer_tabs,
                search_active,
                search_query,
                search_replacement,
                search_replace_mode,
                search_input_target,
                search_status,
                go_to_line_active,
                go_to_line_input,
                rename_active,
                rename_input,
                can_reopen_recent,
                external_change,
                lsp_status,
                diagnostics,
                lsp_panel,
                degraded_notice,
                cursor_diagnostic_message,
                sample: false,
                appearance,
            };
        }

        let appearance = self.appearance_config.for_language(None);
        let lsp_panel = self.lsp_panel.clone();
        // TODO(gpui-editor): once the editor model exposes a public buffer
        // constructor from text, seed EditorState with this sample instead of
        // keeping a display-only fallback.
        EditorSnapshot {
            title: "Sample GPUI editor buffer".to_string(),
            subtitle: "display-only fallback, EditorState has no opened file".to_string(),
            language: "plain text".to_string(),
            lines: self
                .sample_text
                .lines()
                .skip(self.sample_scroll_line)
                .take(VISIBLE_LINE_LIMIT)
                .enumerate()
                .map(|(idx, line)| EditorLineSnapshot {
                    number: self.sample_scroll_line + idx + 1,
                    text: line.to_string(),
                    highlights: Vec::new(),
                    search_matches: Vec::new(),
                    diagnostic: None,
                })
                .collect(),
            first_line_number: self.sample_scroll_line + 1,
            cursor: None,
            cursor_visible: false,
            selection: None,
            scroll_col: 0,
            total_lines: self.sample_text.lines().count().max(1),
            load_error: self.load_error.clone(),
            status_message: self.status_message.clone(),
            buffer_tabs,
            search_active,
            search_query,
            search_replacement,
            search_replace_mode,
            search_input_target,
            search_status,
            go_to_line_active,
            go_to_line_input,
            rename_active,
            rename_input,
            can_reopen_recent,
            external_change,
            lsp_status,
            diagnostics,
            lsp_panel,
            degraded_notice: None,
            cursor_diagnostic_message: None,
            sample: true,
            appearance,
        }
    }

    fn active_appearance(&self) -> EditorAppearance {
        let lang_id = self
            .editor
            .views
            .get(self.editor.active)
            .and_then(|view| view.lang_id);
        self.appearance_config.for_language(lang_id)
    }

    fn active_buffer_and_view(
        &mut self,
    ) -> Option<(&crate::editor::buffer::Buffer, &mut BufferView)> {
        let active = self.editor.active;
        if active >= self.editor.buffers.len() || active >= self.editor.views.len() {
            return None;
        }
        Some((&self.editor.buffers[active], &mut self.editor.views[active]))
    }

    fn active_buffer_and_view_mut(
        &mut self,
    ) -> Option<(&mut crate::editor::buffer::Buffer, &mut BufferView)> {
        let active = self.editor.active;
        if active >= self.editor.buffers.len() || active >= self.editor.views.len() {
            return None;
        }
        Some((
            &mut self.editor.buffers[active],
            &mut self.editor.views[active],
        ))
    }

    fn edit_active(
        &mut self,
        cx: &mut Context<Self>,
        edit: impl FnOnce(&mut crate::editor::buffer::Buffer, &mut BufferView),
    ) {
        let visible_cols = self.visible_col_limit();
        let active = self.editor.active;
        if active >= self.editor.buffers.len() || active >= self.editor.views.len() {
            return;
        }

        let buffer_id = self.editor.buffer_id(active);
        let old_source = self.editor.buffers[active].text();
        let mut changed = false;
        {
            let buffer = &mut self.editor.buffers[active];
            let view = &mut self.editor.views[active];
            edit(buffer, view);
            view.cursor.clamp(buffer);
            reveal_cursor(view, buffer.line_count(), visible_cols);
        }

        let buffer_edit = self.editor.buffers[active].take_last_edit();
        if let Some(buffer_edit) = buffer_edit.as_ref() {
            self.editor.record_active_incremental_edit(
                &old_source,
                buffer_edit.start,
                buffer_edit.old_end,
                &buffer_edit.new_text,
            );
            self.editor_search.mark_dirty();
            changed = true;
        }
        refresh_active_syntax(&mut self.editor);
        if changed {
            if let Some(buffer_id) = buffer_id {
                self.queue_lsp_change_for_buffer_id(buffer_id, buffer_edit);
            }
            self.lsp_panel = None;
        }
        self.wake_cursor_blink();
        cx.notify();
    }

    fn visible_col_limit(&self) -> usize {
        let appearance = self.active_appearance();
        self.last_text_bounds
            .map(|bounds| visible_col_limit_for_bounds(bounds, &appearance))
            .unwrap_or(DEFAULT_VISIBLE_COL_LIMIT)
    }

    fn wake_cursor_blink(&mut self) {
        self.cursor_blink_anchor = Instant::now();
        self.cursor_blink_visible = true;
    }

    fn refresh_cursor_blink(&mut self) -> bool {
        let visible = cursor_visible_for_elapsed(self.cursor_blink_anchor.elapsed());
        if self.cursor_blink_visible == visible {
            return false;
        }
        self.cursor_blink_visible = visible;
        true
    }

    fn dispatch_editor_command(&mut self, command: EditorCommand, cx: &mut Context<Self>) {
        match command {
            EditorCommand::Move { motion, extend } => self.move_cursor(motion, extend, cx),
            EditorCommand::Delete(target) => self.delete_target(target, cx),
            EditorCommand::Enter => self.insert_newline(cx),
            EditorCommand::Indent { outdent } => self.indent_or_outdent(outdent, cx),
            EditorCommand::Select(target) => self.select_target(target, cx),
            EditorCommand::DuplicateLineOrSelection => self.duplicate_line_or_selection(cx),
            EditorCommand::MoveLine(direction) => self.move_active_line(direction, cx),
            EditorCommand::DeleteLine => self.delete_selected_lines(cx),
            EditorCommand::ToggleLineComment => self.toggle_line_comment(cx),
            EditorCommand::Copy => self.copy_selection_or_line(cx),
            EditorCommand::Cut => self.cut_selection_or_line(cx),
            EditorCommand::Paste => self.paste_from_clipboard(cx),
            EditorCommand::Save => self.save_active_buffer(cx),
            EditorCommand::Undo => self.undo_edit(cx),
            EditorCommand::Redo => self.redo_edit(cx),
        }
    }

    fn move_cursor(&mut self, motion: EditorMotion, extend: bool, cx: &mut Context<Self>) {
        let visible_cols = self.visible_col_limit();
        let moved = if let Some((buffer, view)) = self.active_buffer_and_view() {
            match motion {
                EditorMotion::Left => view.cursor.move_left(buffer, extend),
                EditorMotion::Right => view.cursor.move_right(buffer, extend),
                EditorMotion::Up => view.cursor.move_up(buffer, extend),
                EditorMotion::Down => view.cursor.move_down(buffer, extend),
                EditorMotion::WordLeft => view.cursor.move_word_left(buffer, extend),
                EditorMotion::WordRight => view.cursor.move_word_right(buffer, extend),
                EditorMotion::SmartLineStart => view.cursor.move_home(buffer, extend),
                EditorMotion::LineStart => {
                    set_cursor_position(view, Position::new(view.cursor.pos.line, 0), extend);
                }
                EditorMotion::LineEnd => {
                    let line = view.cursor.pos.line;
                    set_cursor_position(view, Position::new(line, buffer.line_len(line)), extend);
                }
                EditorMotion::DocumentStart => view.cursor.move_to_start(extend),
                EditorMotion::DocumentEnd => view.cursor.move_to_end(buffer, extend),
                EditorMotion::PageUp => {
                    view.cursor.move_page_up(buffer, VISIBLE_LINE_LIMIT, extend)
                }
                EditorMotion::PageDown => {
                    view.cursor
                        .move_page_down(buffer, VISIBLE_LINE_LIMIT, extend);
                }
            }
            view.cursor.clamp(buffer);
            reveal_cursor(view, buffer.line_count(), visible_cols);
            true
        } else {
            false
        };

        if moved {
            self.wake_cursor_blink();
            cx.notify();
        }
    }

    fn delete_target(&mut self, target: EditorDeleteTarget, cx: &mut Context<Self>) {
        self.edit_active(cx, |buffer, view| {
            let Some((start, end)) = deletion_range(buffer, view, target) else {
                return;
            };
            buffer.delete(start, end);
            view.cursor.pos = start;
            view.cursor.clear_selection();
            view.cursor.desired_col = None;
        });
    }

    fn insert_newline(&mut self, cx: &mut Context<Self>) {
        self.edit_active(cx, |buffer, view| {
            if let Some((start, end)) = view.cursor.selection() {
                buffer.delete(start, end);
                view.cursor.pos = start;
                view.cursor.clear_selection();
            }

            let indent = buffer.line_indent(view.cursor.pos.line).to_string();
            let line_before = buffer.line(view.cursor.pos.line);
            let cursor_byte = byte_index_for_char_col(line_before, view.cursor.pos.col);
            let before_cursor = &line_before[..cursor_byte];
            let extra = if before_cursor.trim_end().ends_with('{')
                || before_cursor.trim_end().ends_with('(')
                || before_cursor.trim_end().ends_with('[')
            {
                buffer.indent_style.as_str()
            } else {
                ""
            };
            let text = format!("\n{indent}{extra}");
            let new_pos = buffer.compute_end_pos_pub(view.cursor.pos, &text);
            buffer.insert(view.cursor.pos, &text);
            view.cursor.pos = new_pos;
            view.cursor.desired_col = None;
        });
    }

    fn indent_or_outdent(&mut self, outdent: bool, cx: &mut Context<Self>) {
        self.edit_active(cx, |buffer, view| {
            if let Some((start, _end)) = view.cursor.selection() {
                let (start_line, end_line) = selected_line_range(buffer, view);
                let replacement = if outdent {
                    dedented_lines_replacement(buffer, start_line, end_line)
                } else {
                    indented_lines_replacement(buffer, start_line, end_line)
                };
                buffer.replace(
                    Position::new(start_line, 0),
                    Position::new(end_line, buffer.line_len(end_line)),
                    &replacement,
                );
                if outdent {
                    view.cursor.anchor = Some(Position::new(
                        start.line,
                        start.col.saturating_sub(buffer.indent_style.width()),
                    ));
                } else {
                    view.cursor.anchor = Some(Position::new(
                        start.line,
                        start.col + buffer.indent_style.width(),
                    ));
                }
                view.cursor.pos = Position::new(end_line, buffer.line_len(end_line));
            } else if outdent {
                let line = view.cursor.pos.line;
                let original_col = view.cursor.pos.col;
                let replacement = dedented_lines_replacement(buffer, line, line);
                buffer.replace(
                    Position::new(line, 0),
                    Position::new(line, buffer.line_len(line)),
                    &replacement,
                );
                view.cursor.pos.col = original_col
                    .saturating_sub(buffer.indent_style.width())
                    .min(buffer.line_len(line));
            } else {
                let indent = buffer.indent_style.as_str().to_string();
                let new_pos = buffer.compute_end_pos_pub(view.cursor.pos, &indent);
                buffer.insert(view.cursor.pos, &indent);
                view.cursor.pos = new_pos;
            }
            view.cursor.desired_col = None;
        });
    }

    fn select_target(&mut self, target: EditorSelectTarget, cx: &mut Context<Self>) {
        let visible_cols = self.visible_col_limit();
        let selected = if let Some((buffer, view)) = self.active_buffer_and_view() {
            match target {
                EditorSelectTarget::All => view.cursor.select_all(buffer),
                EditorSelectTarget::Word => view.cursor.select_word(buffer),
                EditorSelectTarget::Line => view.cursor.select_line(buffer),
            }
            reveal_cursor(view, buffer.line_count(), visible_cols);
            true
        } else {
            false
        };

        if selected {
            self.wake_cursor_blink();
            cx.notify();
        }
    }

    fn duplicate_line_or_selection(&mut self, cx: &mut Context<Self>) {
        self.edit_active(cx, |buffer, view| {
            if let Some((start, end)) = view.cursor.selection() {
                let text = buffer.text_range(start, end);
                if text.is_empty() {
                    return;
                }
                let new_end = buffer.compute_end_pos_pub(end, &text);
                buffer.insert(end, &text);
                view.cursor.anchor = Some(end);
                view.cursor.pos = new_end;
            } else {
                let line = view.cursor.pos.line;
                let col = view.cursor.pos.col;
                let new_pos = buffer.duplicate_line(line);
                view.cursor.pos =
                    Position::new(new_pos.line, col.min(buffer.line_len(new_pos.line)));
                view.cursor.clear_selection();
            }
            view.cursor.desired_col = None;
        });
    }

    fn move_active_line(&mut self, direction: EditorLineMove, cx: &mut Context<Self>) {
        self.edit_active(cx, |buffer, view| {
            let line = view.cursor.pos.line;
            let col = view.cursor.pos.col;
            let moved = match direction {
                EditorLineMove::Up => buffer.move_line_up(line),
                EditorLineMove::Down => buffer.move_line_down(line),
            };
            if let Some(pos) = moved {
                view.cursor.pos = Position::new(pos.line, col.min(buffer.line_len(pos.line)));
                view.cursor.clear_selection();
                view.cursor.desired_col = None;
            }
        });
    }

    fn delete_selected_lines(&mut self, cx: &mut Context<Self>) {
        self.edit_active(cx, |buffer, view| {
            let (start_line, end_line) = selected_line_range(buffer, view);
            view.cursor.pos = delete_lines_as_command(buffer, start_line, end_line);
            view.cursor.clear_selection();
            view.cursor.desired_col = None;
        });
    }

    fn toggle_line_comment(&mut self, cx: &mut Context<Self>) {
        let style = self
            .editor
            .active_buffer_view()
            .map(|(_, buffer, view)| comment_style(view.lang_id, buffer.path()));
        let Some(style) = style else {
            self.status_message = Some("No active buffer to comment".to_string());
            cx.notify();
            return;
        };
        let Some(prefix) = style.line else {
            self.status_message = Some("No line comment syntax for this file".to_string());
            cx.notify();
            return;
        };

        self.edit_active(cx, |buffer, view| {
            let (start_line, end_line) = selected_line_range(buffer, view);
            let had_selection = view.cursor.selection().is_some();
            if toggle_line_comments_as_command(buffer, start_line, end_line, prefix) {
                if had_selection {
                    view.cursor.anchor = Some(Position::new(start_line, 0));
                    view.cursor.pos = Position::new(end_line, buffer.line_len(end_line));
                }
                view.cursor.desired_col = None;
            } else if had_selection {
                view.cursor.anchor = Some(Position::new(start_line, 0));
                view.cursor.pos = Position::new(end_line, buffer.line_len(end_line));
            }
        });
    }

    fn copy_selection_or_line(&mut self, cx: &mut Context<Self>) {
        if let Some(text) = self.selected_text_or_current_line() {
            if !text.is_empty() {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
            }
        }
    }

    fn cut_selection_or_line(&mut self, cx: &mut Context<Self>) {
        let Some(text) = self.selected_text_or_current_line() else {
            return;
        };
        if text.is_empty() {
            return;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.edit_active(cx, |buffer, view| {
            if let Some((start, end)) = view.cursor.selection() {
                buffer.delete(start, end);
                view.cursor.pos = start;
            } else {
                view.cursor.pos = buffer.delete_line(view.cursor.pos.line);
            }
            view.cursor.clear_selection();
            view.cursor.desired_col = None;
        });
    }

    fn selected_text_or_current_line(&mut self) -> Option<String> {
        let (_, buffer, view) = self.editor.active_buffer_view()?;
        if let Some((start, end)) = view.cursor.selection() {
            let text = buffer.text_range(start, end);
            if !text.is_empty() {
                return Some(text);
            }
        }
        Some(buffer.line_text_for_copy(view.cursor.pos.line))
    }

    fn open_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle);
        self.close_go_to_line(cx);
        self.close_lsp_rename(cx);
        if let Some(seed) = self.find_query_seed_from_selection() {
            self.editor_search.query = seed;
        }
        self.editor_search.open_find();
        self.search_input_target = EditorSearchInputTarget::Query;
        self.editor_search.mark_dirty();
        self.refresh_search_matches_for_active_buffer();
        if let Some((_, _, view)) = self.editor.active_buffer_view() {
            self.editor_search.focus_nearest(view.cursor.pos);
        }
        cx.notify();
    }

    fn open_go_to_line(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle);
        if self.editor_search.active {
            self.editor_search.close();
            self.search_input_target = EditorSearchInputTarget::Query;
        }
        self.close_lsp_rename(cx);
        self.go_to_line_active = true;
        self.go_to_line_input.clear();
        self.wake_cursor_blink();
        cx.notify();
    }

    fn close_go_to_line(&mut self, cx: &mut Context<Self>) {
        if self.go_to_line_active {
            self.go_to_line_active = false;
            self.go_to_line_input.clear();
            cx.notify();
        }
    }

    fn close_find(&mut self, cx: &mut Context<Self>) {
        if self.editor_search.active {
            self.editor_search.close();
            self.search_input_target = EditorSearchInputTarget::Query;
            cx.notify();
        }
    }

    fn toggle_replace_mode(&mut self, cx: &mut Context<Self>) {
        self.editor_search.replace_mode = !self.editor_search.replace_mode;
        self.search_input_target = if self.editor_search.replace_mode {
            EditorSearchInputTarget::Replacement
        } else {
            EditorSearchInputTarget::Query
        };
        cx.notify();
    }

    fn push_go_to_line_text(&mut self, text: &str, cx: &mut Context<Self>) {
        for ch in text.chars().filter(|ch| ch.is_ascii_digit()) {
            if self.go_to_line_input.len() < 8 {
                self.go_to_line_input.push(ch);
            }
        }
        cx.notify();
    }

    fn pop_go_to_line_text(&mut self, cx: &mut Context<Self>) {
        self.go_to_line_input.pop();
        cx.notify();
    }

    fn submit_go_to_line(&mut self, cx: &mut Context<Self>) {
        let total_lines = self
            .editor
            .active_buffer_view()
            .map(|(_, buffer, _)| buffer.line_count())
            .unwrap_or(0);
        let Some(line) = parse_go_to_line(&self.go_to_line_input, total_lines) else {
            self.status_message = Some("Enter a line number".to_string());
            cx.notify();
            return;
        };

        let visible_cols = self.visible_col_limit();
        let moved = if let Some((buffer, view)) = self.active_buffer_and_view() {
            view.cursor.pos = Position::new(line, 0);
            view.cursor.clear_selection();
            view.cursor.desired_col = None;
            reveal_cursor(view, buffer.line_count(), visible_cols);
            true
        } else {
            false
        };

        if moved {
            self.go_to_line_active = false;
            self.go_to_line_input.clear();
            self.status_message = Some(format!("Moved to line {}", line + 1));
            self.wake_cursor_blink();
            cx.notify();
        }
    }

    fn set_search_input_target(&mut self, target: EditorSearchInputTarget, cx: &mut Context<Self>) {
        if target == EditorSearchInputTarget::Replacement {
            self.editor_search.replace_mode = true;
        }
        self.search_input_target = target;
        cx.notify();
    }

    fn refresh_search_matches_for_active_buffer(&mut self) {
        let active = self.editor.active;
        if let Some(buffer) = self.editor.buffers.get(active) {
            self.editor_search.update_matches(buffer);
        }
    }

    fn find_query_seed_from_selection(&self) -> Option<String> {
        let (_, buffer, view) = self.editor.active_buffer_view()?;
        let (start, end) = view.cursor.selection()?;
        let text = buffer.text_range(start, end);
        (!text.is_empty() && !text.contains('\n') && text.chars().count() <= 160).then_some(text)
    }

    fn update_find_query(&mut self, query: String, cx: &mut Context<Self>) {
        self.editor_search.query = query;
        self.editor_search.mark_dirty();
        self.refresh_search_matches_for_active_buffer();
        if let Some((_, _, view)) = self.editor.active_buffer_view() {
            self.editor_search.focus_nearest(view.cursor.pos);
        }
        cx.notify();
    }

    fn update_replacement_text(&mut self, replacement: String, cx: &mut Context<Self>) {
        self.editor_search.replacement = replacement;
        cx.notify();
    }

    fn push_search_text(&mut self, text: &str, cx: &mut Context<Self>) {
        match self.search_input_target {
            EditorSearchInputTarget::Query => {
                let mut query = self.editor_search.query.clone();
                if query.chars().count() < 200 {
                    query.push_str(text);
                    self.update_find_query(query, cx);
                }
            }
            EditorSearchInputTarget::Replacement => {
                let mut replacement = self.editor_search.replacement.clone();
                if replacement.chars().count() < 200 {
                    replacement.push_str(text);
                    self.update_replacement_text(replacement, cx);
                }
            }
        }
    }

    fn pop_search_text(&mut self, cx: &mut Context<Self>) {
        match self.search_input_target {
            EditorSearchInputTarget::Query => {
                let mut query = self.editor_search.query.clone();
                query.pop();
                self.update_find_query(query, cx);
            }
            EditorSearchInputTarget::Replacement => {
                let mut replacement = self.editor_search.replacement.clone();
                replacement.pop();
                self.update_replacement_text(replacement, cx);
            }
        }
    }

    fn toggle_search_input_target(&mut self, cx: &mut Context<Self>) {
        if !self.editor_search.replace_mode {
            self.editor_search.replace_mode = true;
            self.search_input_target = EditorSearchInputTarget::Replacement;
        } else {
            self.search_input_target = match self.search_input_target {
                EditorSearchInputTarget::Query => EditorSearchInputTarget::Replacement,
                EditorSearchInputTarget::Replacement => EditorSearchInputTarget::Query,
            };
        }
        cx.notify();
    }

    fn move_search_focus(&mut self, direction: EditorSearchDirection, cx: &mut Context<Self>) {
        if !self.editor_search.active {
            self.editor_search.open_find();
        }
        if self.editor_search.query.is_empty() {
            cx.notify();
            return;
        }
        self.refresh_search_matches_for_active_buffer();
        match direction {
            EditorSearchDirection::Next => {
                self.editor_search.next_match();
            }
            EditorSearchDirection::Previous => {
                self.editor_search.previous_match();
            }
        }
        self.select_focused_search_match(cx);
    }

    fn select_focused_search_match(&mut self, cx: &mut Context<Self>) {
        let Some(search_match) = self.editor_search.focused_match().copied() else {
            cx.notify();
            return;
        };
        let visible_cols = self.visible_col_limit();
        if let Some((buffer, view)) = self.active_buffer_and_view() {
            view.cursor.anchor = Some(search_match.start);
            view.cursor.pos = search_match.end;
            view.cursor.desired_col = None;
            reveal_cursor(view, buffer.line_count(), visible_cols);
        }
        cx.notify();
    }

    fn replace_focused_search_match(&mut self, cx: &mut Context<Self>) {
        if self.editor_search.query.is_empty() {
            return;
        }
        self.refresh_search_matches_for_active_buffer();
        let Some(search_match) = self.editor_search.focused_match().copied() else {
            cx.notify();
            return;
        };
        let replacement = self.editor_search.replacement.clone();
        let replacement_chars = replacement.chars().count();
        let cursor = Position::new(
            search_match.start.line,
            search_match.start.col + replacement_chars,
        );
        self.edit_active(cx, |buffer, view| {
            buffer.replace(search_match.start, search_match.end, &replacement);
            view.cursor.clear_selection();
            view.cursor.pos = cursor;
            view.cursor.desired_col = None;
        });
        self.refresh_search_matches_for_active_buffer();
        self.editor_search.focus_nearest(cursor);
        self.select_focused_search_match(cx);
    }

    fn replace_all_search_matches(&mut self, cx: &mut Context<Self>) {
        if self.editor_search.query.is_empty() {
            return;
        }
        self.refresh_search_matches_for_active_buffer();
        let matches = self.editor_search.matches.clone();
        if matches.is_empty() {
            cx.notify();
            return;
        }
        let replacement = self.editor_search.replacement.clone();
        let count = matches.len();
        self.edit_active(cx, |buffer, view| {
            let mut text = buffer.text();
            for search_match in matches.iter().rev() {
                let start_char = buffer.pos_to_char(search_match.start);
                let end_char = buffer.pos_to_char(search_match.end);
                let start_byte = byte_index_for_char_col(&text, start_char);
                let end_byte = byte_index_for_char_col(&text, end_char);
                text.replace_range(start_byte..end_byte, &replacement);
            }
            let end = Position::new(
                buffer.line_count().saturating_sub(1),
                buffer.line_len(buffer.line_count().saturating_sub(1)),
            );
            buffer.replace(Position::new(0, 0), end, &text);
            view.cursor.clear_selection();
            view.cursor.pos = Position::new(0, 0);
            view.cursor.desired_col = None;
        });
        self.editor_search.mark_dirty();
        self.refresh_search_matches_for_active_buffer();
        self.status_message = Some(format!("Replaced {count} matches"));
        cx.notify();
    }

    fn replace_selection_or_range(
        &mut self,
        cx: &mut Context<Self>,
        range: Option<(Position, Position)>,
        text: &str,
    ) {
        if range.is_none() && text.is_empty() {
            return;
        }
        self.edit_active(cx, |buffer, view| {
            let (start, end) = range
                .or_else(|| view.cursor.selection())
                .unwrap_or((view.cursor.pos, view.cursor.pos));
            let new_pos = buffer.compute_end_pos_pub(start, text);
            buffer.replace(start, end, text);
            view.cursor.pos = new_pos;
            view.cursor.clear_selection();
            view.cursor.desired_col = None;
        });
    }

    fn position_for_utf16(&self, utf16: usize) -> Option<Position> {
        let (.., buffer, _) = self.editor.active_buffer_view()?;
        let char_index = utf16_index_to_char_index(&buffer.text(), utf16);
        Some(buffer.char_to_pos(char_index))
    }

    fn utf16_for_position(&self, position: Position) -> Option<usize> {
        let (.., buffer, _) = self.editor.active_buffer_view()?;
        Some(char_index_to_utf16_index(
            &buffer.text(),
            buffer.pos_to_char(position),
        ))
    }

    fn scroll_by_lines(&mut self, delta: isize, cx: &mut Context<Self>) {
        if self.scroll_active_by_lines_without_notify(delta) {
            cx.notify();
        }
    }

    fn move_left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::Left,
                extend: false,
            },
            cx,
        );
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::Left,
                extend: true,
            },
            cx,
        );
    }

    fn move_right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::Right,
                extend: false,
            },
            cx,
        );
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::Right,
                extend: true,
            },
            cx,
        );
    }

    fn move_up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        if self.move_lsp_panel_selection(-1, cx) {
            return;
        }
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::Up,
                extend: false,
            },
            cx,
        );
    }

    fn select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::Up,
                extend: true,
            },
            cx,
        );
    }

    fn move_down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        if self.move_lsp_panel_selection(1, cx) {
            return;
        }
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::Down,
                extend: false,
            },
            cx,
        );
    }

    fn select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::Down,
                extend: true,
            },
            cx,
        );
    }

    fn move_word_left(&mut self, _: &WordLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::WordLeft,
                extend: false,
            },
            cx,
        );
    }

    fn select_word_left(&mut self, _: &SelectWordLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::WordLeft,
                extend: true,
            },
            cx,
        );
    }

    fn move_word_right(&mut self, _: &WordRight, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::WordRight,
                extend: false,
            },
            cx,
        );
    }

    fn select_word_right(&mut self, _: &SelectWordRight, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::WordRight,
                extend: true,
            },
            cx,
        );
    }

    fn move_home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::SmartLineStart,
                extend: false,
            },
            cx,
        );
    }

    fn select_home(&mut self, _: &SelectHome, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::SmartLineStart,
                extend: true,
            },
            cx,
        );
    }

    fn move_end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::LineEnd,
                extend: false,
            },
            cx,
        );
    }

    fn select_end(&mut self, _: &SelectEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::LineEnd,
                extend: true,
            },
            cx,
        );
    }

    fn move_line_start(&mut self, _: &LineStart, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::LineStart,
                extend: false,
            },
            cx,
        );
    }

    fn select_line_start(&mut self, _: &SelectLineStart, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::LineStart,
                extend: true,
            },
            cx,
        );
    }

    fn move_line_end(&mut self, _: &LineEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::LineEnd,
                extend: false,
            },
            cx,
        );
    }

    fn select_line_end(&mut self, _: &SelectLineEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::LineEnd,
                extend: true,
            },
            cx,
        );
    }

    fn move_document_start(&mut self, _: &DocumentStart, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::DocumentStart,
                extend: false,
            },
            cx,
        );
    }

    fn select_document_start(
        &mut self,
        _: &SelectDocumentStart,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::DocumentStart,
                extend: true,
            },
            cx,
        );
    }

    fn move_document_end(&mut self, _: &DocumentEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::DocumentEnd,
                extend: false,
            },
            cx,
        );
    }

    fn select_document_end(
        &mut self,
        _: &SelectDocumentEnd,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::DocumentEnd,
                extend: true,
            },
            cx,
        );
    }

    fn page_up(&mut self, _: &PageUp, _: &mut Window, cx: &mut Context<Self>) {
        self.page_up_impl(false, cx);
    }

    fn page_down(&mut self, _: &PageDown, _: &mut Window, cx: &mut Context<Self>) {
        self.page_down_impl(false, cx);
    }

    fn select_page_up(&mut self, _: &SelectPageUp, _: &mut Window, cx: &mut Context<Self>) {
        self.page_up_impl(true, cx);
    }

    fn select_page_down(&mut self, _: &SelectPageDown, _: &mut Window, cx: &mut Context<Self>) {
        self.page_down_impl(true, cx);
    }

    fn page_up_impl(&mut self, extend: bool, cx: &mut Context<Self>) {
        if self.editor.is_empty() {
            self.scroll_by_lines(-(VISIBLE_LINE_LIMIT as isize), cx);
            return;
        }
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::PageUp,
                extend,
            },
            cx,
        );
    }

    fn page_down_impl(&mut self, extend: bool, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::PageDown,
                extend,
            },
            cx,
        );
        if self.editor.is_empty() {
            self.scroll_by_lines(VISIBLE_LINE_LIMIT as isize, cx);
        }
    }

    fn backspace(&mut self, _: &Backspace, _: &mut Window, cx: &mut Context<Self>) {
        if self.rename_active {
            self.pop_lsp_rename_text(cx);
            return;
        }
        if self.editor_search.active {
            self.pop_search_text(cx);
            return;
        }
        self.dispatch_editor_command(EditorCommand::Delete(EditorDeleteTarget::BackwardChar), cx);
    }

    fn delete(&mut self, _: &Delete, _: &mut Window, cx: &mut Context<Self>) {
        if self.rename_active {
            self.pop_lsp_rename_text(cx);
            return;
        }
        if self.editor_search.active {
            return;
        }
        self.dispatch_editor_command(EditorCommand::Delete(EditorDeleteTarget::ForwardChar), cx);
    }

    fn delete_word_backward(
        &mut self,
        _: &DeleteWordBackward,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dispatch_editor_command(EditorCommand::Delete(EditorDeleteTarget::BackwardWord), cx);
    }

    fn delete_word_forward(
        &mut self,
        _: &DeleteWordForward,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dispatch_editor_command(EditorCommand::Delete(EditorDeleteTarget::ForwardWord), cx);
    }

    fn delete_to_line_start(
        &mut self,
        _: &DeleteToLineStart,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dispatch_editor_command(EditorCommand::Delete(EditorDeleteTarget::ToLineStart), cx);
    }

    fn delete_to_line_end(&mut self, _: &DeleteToLineEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(EditorCommand::Delete(EditorDeleteTarget::ToLineEnd), cx);
    }

    fn enter(&mut self, _: &Enter, _: &mut Window, cx: &mut Context<Self>) {
        if self.accept_lsp_panel_selection(cx) {
            return;
        }
        if self.rename_active {
            self.submit_lsp_rename(cx);
            return;
        }
        if self.go_to_line_active {
            self.submit_go_to_line(cx);
            return;
        }
        if self.editor_search.active {
            self.move_search_focus(EditorSearchDirection::Next, cx);
            return;
        }
        self.dispatch_editor_command(EditorCommand::Enter, cx);
    }

    fn tab(&mut self, _: &Tab, _: &mut Window, cx: &mut Context<Self>) {
        if self.accept_lsp_panel_selection(cx) {
            return;
        }
        if self.rename_active {
            return;
        }
        if self.go_to_line_active {
            return;
        }
        if self.editor_search.active {
            self.toggle_search_input_target(cx);
            return;
        }
        self.dispatch_editor_command(EditorCommand::Indent { outdent: false }, cx);
    }

    fn shift_tab(&mut self, _: &ShiftTab, _: &mut Window, cx: &mut Context<Self>) {
        if self.move_lsp_panel_selection(-1, cx) {
            return;
        }
        if self.rename_active {
            return;
        }
        if self.go_to_line_active {
            return;
        }
        if self.editor_search.active {
            self.toggle_search_input_target(cx);
            return;
        }
        self.dispatch_editor_command(EditorCommand::Indent { outdent: true }, cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        if self.go_to_line_active {
            return;
        }
        if self.editor_search.active {
            return;
        }
        self.dispatch_editor_command(EditorCommand::Select(EditorSelectTarget::All), cx);
    }

    fn select_word(&mut self, _: &SelectWord, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(EditorCommand::Select(EditorSelectTarget::Word), cx);
    }

    fn select_line(&mut self, _: &SelectLine, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(EditorCommand::Select(EditorSelectTarget::Line), cx);
    }

    fn delete_line(&mut self, _: &DeleteLine, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(EditorCommand::DeleteLine, cx);
    }

    fn duplicate_line_or_selection_action(
        &mut self,
        _: &DuplicateLineOrSelection,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dispatch_editor_command(EditorCommand::DuplicateLineOrSelection, cx);
    }

    fn move_line_up(&mut self, _: &MoveLineUp, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(EditorCommand::MoveLine(EditorLineMove::Up), cx);
    }

    fn move_line_down(&mut self, _: &MoveLineDown, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(EditorCommand::MoveLine(EditorLineMove::Down), cx);
    }

    fn toggle_line_comment_action(
        &mut self,
        _: &ToggleLineComment,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dispatch_editor_command(EditorCommand::ToggleLineComment, cx);
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(EditorCommand::Copy, cx);
    }

    fn cut(&mut self, _: &Cut, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(EditorCommand::Cut, cx);
    }

    fn paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
        if self.rename_active {
            if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                self.push_lsp_rename_text(&text.replace('\n', "").replace('\r', ""), cx);
            }
            return;
        }
        if self.editor_search.active {
            if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                self.push_search_text(&text.replace('\n', " "), cx);
            }
            return;
        }
        self.dispatch_editor_command(EditorCommand::Paste, cx);
    }

    fn save(&mut self, _: &Save, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(EditorCommand::Save, cx);
    }

    fn undo(&mut self, _: &Undo, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(EditorCommand::Undo, cx);
    }

    fn redo(&mut self, _: &Redo, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(EditorCommand::Redo, cx);
    }

    fn find(&mut self, _: &Find, window: &mut Window, cx: &mut Context<Self>) {
        self.open_find(window, cx);
    }

    fn find_next(&mut self, _: &FindNext, _: &mut Window, cx: &mut Context<Self>) {
        self.move_search_focus(EditorSearchDirection::Next, cx);
    }

    fn find_previous(&mut self, _: &FindPrevious, _: &mut Window, cx: &mut Context<Self>) {
        self.move_search_focus(EditorSearchDirection::Previous, cx);
    }

    fn close_find_action(&mut self, _: &CloseFind, _: &mut Window, cx: &mut Context<Self>) {
        self.close_go_to_line(cx);
        self.close_lsp_rename(cx);
        self.close_lsp_panel(cx);
        self.close_find(cx);
    }

    fn go_to_line(&mut self, _: &GoToLine, window: &mut Window, cx: &mut Context<Self>) {
        self.open_go_to_line(window, cx);
    }

    fn on_editor_key_down(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.lsp_panel.is_some() {
            match event.keystroke.key.as_str() {
                "escape" => {
                    cx.stop_propagation();
                    self.close_lsp_panel(cx);
                    return;
                }
                "enter" | "tab" => {
                    cx.stop_propagation();
                    self.accept_lsp_panel_selection(cx);
                    return;
                }
                "up" => {
                    cx.stop_propagation();
                    self.move_lsp_panel_selection(-1, cx);
                    return;
                }
                "down" => {
                    cx.stop_propagation();
                    self.move_lsp_panel_selection(1, cx);
                    return;
                }
                _ => {}
            }
        }

        if self.rename_active {
            let modifiers = event.keystroke.modifiers;
            if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                return;
            }

            match event.keystroke.key.as_str() {
                "escape" => {
                    cx.stop_propagation();
                    self.close_lsp_rename(cx);
                }
                "enter" => {
                    cx.stop_propagation();
                    self.submit_lsp_rename(cx);
                }
                "backspace" | "delete" => {
                    cx.stop_propagation();
                    self.pop_lsp_rename_text(cx);
                }
                _ => {
                    let Some(text) = event.keystroke.key_char.as_deref() else {
                        return;
                    };
                    if text
                        .chars()
                        .any(|ch| ch.is_control() || ch == '\n' || ch == '\r')
                    {
                        return;
                    }
                    cx.stop_propagation();
                    self.push_lsp_rename_text(text, cx);
                }
            }
            return;
        }

        if self.go_to_line_active {
            let modifiers = event.keystroke.modifiers;
            if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                return;
            }

            match event.keystroke.key.as_str() {
                "escape" => {
                    cx.stop_propagation();
                    self.close_go_to_line(cx);
                }
                "enter" => {
                    cx.stop_propagation();
                    self.submit_go_to_line(cx);
                }
                "backspace" | "delete" => {
                    cx.stop_propagation();
                    self.pop_go_to_line_text(cx);
                }
                _ => {
                    let Some(text) = event.keystroke.key_char.as_deref() else {
                        return;
                    };
                    cx.stop_propagation();
                    if !text.chars().all(|ch| ch.is_ascii_digit()) {
                        return;
                    }
                    self.push_go_to_line_text(text, cx);
                }
            }
            return;
        }

        if !self.editor_search.active {
            return;
        }

        let modifiers = event.keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
            return;
        }

        match event.keystroke.key.as_str() {
            "escape" => {
                cx.stop_propagation();
                self.close_find(cx);
            }
            "enter" => {
                cx.stop_propagation();
                if self.search_input_target == EditorSearchInputTarget::Replacement {
                    self.replace_focused_search_match(cx);
                } else {
                    self.move_search_focus(EditorSearchDirection::Next, cx);
                }
            }
            "tab" => {
                cx.stop_propagation();
                self.toggle_search_input_target(cx);
            }
            "backspace" => {
                cx.stop_propagation();
                self.pop_search_text(cx);
            }
            _ => {
                let Some(text) = event.keystroke.key_char.as_deref() else {
                    return;
                };
                if text
                    .chars()
                    .any(|ch| ch.is_control() || ch == '\n' || ch == '\r')
                {
                    return;
                }
                cx.stop_propagation();
                self.push_search_text(text, cx);
            }
        }
    }

    fn on_editor_scroll(
        &mut self,
        event: &ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let appearance = self.active_appearance();
        let pixel_delta = event.delta.pixel_delta(appearance.line_height);
        let lines = (pixel_delta.y / appearance.line_height).round() as isize;
        let columns = (pixel_delta.x / appearance.char_width).round() as isize;
        let mut changed = false;
        if lines != 0 {
            changed |= self.scroll_active_by_lines_without_notify(-lines);
        }
        if columns != 0 {
            changed |= self.scroll_active_by_columns_without_notify(columns);
        }
        if changed {
            cx.notify();
        }
    }

    fn on_editor_mouse_down_at_point(
        &mut self,
        event: &MouseDownEvent,
        point: gpui::Point<gpui::Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.button != MouseButton::Left {
            return;
        }
        window.focus(&self.focus_handle);
        let closed_panel = self.close_lsp_panel_without_notify();
        if let Some(position) = self.position_for_point(point) {
            if event.click_count >= 3 {
                let visible_cols = self.visible_col_limit();
                if let Some((buffer, view)) = self.active_buffer_and_view() {
                    view.cursor.pos = position;
                    view.cursor.select_line(buffer);
                    view.cursor.desired_col = None;
                    reveal_cursor(view, buffer.line_count(), visible_cols);
                }
                self.is_selecting = false;
                self.wake_cursor_blink();
                cx.notify();
                return;
            }

            if event.click_count == 2 {
                let visible_cols = self.visible_col_limit();
                if let Some((buffer, view)) = self.active_buffer_and_view() {
                    view.cursor.pos = position;
                    view.cursor.select_word(buffer);
                    view.cursor.desired_col = None;
                    reveal_cursor(view, buffer.line_count(), visible_cols);
                }
                self.is_selecting = false;
                self.wake_cursor_blink();
                cx.notify();
                return;
            }

            self.is_selecting = true;
            if event.modifiers.shift {
                if let Some((_, view)) = self.active_buffer_and_view_mut() {
                    view.cursor.start_selection();
                    view.cursor.pos = position;
                    view.cursor.desired_col = None;
                }
            } else if let Some((_, view)) = self.active_buffer_and_view_mut() {
                view.cursor.pos = position;
                view.cursor.clear_selection();
                view.cursor.desired_col = None;
            }
            self.wake_cursor_blink();
            cx.notify();
        } else {
            self.is_selecting = false;
            if closed_panel {
                cx.notify();
            }
        }
    }

    fn on_editor_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.on_editor_mouse_down_at_point(event, event.position, window, cx);
    }

    fn on_editor_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.on_editor_mouse_move_at_point(event, event.position, cx);
    }

    fn on_editor_mouse_move_at_point(
        &mut self,
        event: &MouseMoveEvent,
        point: gpui::Point<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) {
        if !self.is_selecting {
            return;
        }
        if !event.dragging() {
            self.is_selecting = false;
            return;
        }

        if let Some((position, scrolled)) = self.drag_position_for_point(point) {
            let visible_cols = self.visible_col_limit();
            let changed = if let Some((buffer, view)) = self.active_buffer_and_view() {
                let changed = view.cursor.pos != position || scrolled;
                view.cursor.start_selection();
                view.cursor.pos = position;
                view.cursor.desired_col = None;
                reveal_cursor(view, buffer.line_count(), visible_cols);
                changed
            } else {
                false
            };
            if changed {
                self.wake_cursor_blink();
                cx.notify();
            }
        }
    }

    fn on_editor_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.is_selecting = false;
        cx.notify();
    }

    fn position_for_point(&self, point: gpui::Point<gpui::Pixels>) -> Option<Position> {
        let bounds = self.last_text_bounds?;
        let local_point = local_editor_point(bounds, point)?;
        self.position_for_local_point(local_point, true)
    }

    fn drag_position_for_point(
        &mut self,
        point: gpui::Point<gpui::Pixels>,
    ) -> Option<(Position, bool)> {
        let bounds = self.last_text_bounds?;
        let local_point = raw_local_editor_point(bounds, point);
        let appearance = self.active_appearance();
        let vertical_delta =
            drag_scroll_delta_for_local_y(local_point.y, bounds.size.height, &appearance);
        let horizontal_delta =
            drag_scroll_delta_for_local_x(local_point.x, bounds.size.width, &appearance);
        let scrolled = self.scroll_active_by_lines_without_notify(vertical_delta)
            | self.scroll_active_by_columns_without_notify(horizontal_delta);
        let clamped_point = clamp_local_editor_point(bounds, local_point);
        self.position_for_local_point(clamped_point, !scrolled)
            .map(|position| (position, scrolled))
    }

    fn position_for_local_point(
        &self,
        local_point: gpui::Point<gpui::Pixels>,
        use_measured_layout: bool,
    ) -> Option<Position> {
        let (_, buffer, view) = self.editor.active_buffer_view()?;
        let appearance = self.active_appearance();
        if use_measured_layout {
            if let Some(layout) = self.last_text_layout.as_ref() {
                if let Some(position) =
                    measured_position_for_point(layout, buffer, local_point, &appearance)
                {
                    return Some(position);
                }
            }
        }

        let row = editor_row_for_local_y(local_point.y, &appearance)
            .min(VISIBLE_LINE_LIMIT.saturating_sub(1));
        let line = (view.scroll_line + row).min(buffer.line_count().saturating_sub(1));
        let col = if local_point.x <= appearance.line_number_width {
            0
        } else {
            let visible_col = ((local_point.x - appearance.line_number_width)
                / appearance.char_width)
                .round()
                .max(0.0) as usize;
            view.scroll_col.saturating_add(visible_col)
        };
        Some(Position::new(line, col.min(buffer.line_len(line))))
    }

    fn scroll_active_by_lines_without_notify(&mut self, delta: isize) -> bool {
        if delta == 0 {
            return false;
        }
        if let Some((buffer, view)) = self.active_buffer_and_view() {
            let old_scroll_line = view.scroll_line;
            scroll_view_by_lines(view, buffer.line_count(), delta);
            view.scroll_line != old_scroll_line
        } else {
            let old_scroll_line = self.sample_scroll_line;
            self.sample_scroll_line = scroll_line_by_delta(
                self.sample_scroll_line,
                self.sample_text.lines().count().max(1),
                delta,
            );
            self.sample_scroll_line != old_scroll_line
        }
    }

    fn scroll_active_by_columns_without_notify(&mut self, delta: isize) -> bool {
        if delta == 0 {
            return false;
        }
        let visible_cols = self.visible_col_limit();
        let Some((buffer, view)) = self.active_buffer_and_view() else {
            return false;
        };
        let old_scroll_col = view.scroll_col;
        scroll_view_by_columns(view, buffer, visible_cols, delta);
        view.scroll_col != old_scroll_col
    }
}

impl Render for EditorPrototype {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = self.snapshot(self.focus_handle.is_focused(window));
        let content = div().flex_1().flex().flex_col();
        let content = if self.show_chrome {
            content.child(editor_header(&snapshot))
        } else {
            content
        }
        .child(editor_file_tabs(&snapshot, cx))
        .child(editor_body(&snapshot, cx.entity(), cx))
        .child(status_bar(&snapshot));

        let root = div()
            .size_full()
            .flex()
            .flex_col()
            .bg(snapshot.appearance.background_color())
            .text_color(snapshot.appearance.foreground_color())
            .font_family("Inter")
            .key_context("EditorPrototype")
            .track_focus(&self.focus_handle(cx))
            .on_key_down(cx.listener(Self::on_editor_key_down))
            .on_action(cx.listener(Self::move_left))
            .on_action(cx.listener(Self::move_right))
            .on_action(cx.listener(Self::move_up))
            .on_action(cx.listener(Self::move_down))
            .on_action(cx.listener(Self::move_word_left))
            .on_action(cx.listener(Self::move_word_right))
            .on_action(cx.listener(Self::select_word_left))
            .on_action(cx.listener(Self::select_word_right))
            .on_action(cx.listener(Self::move_home))
            .on_action(cx.listener(Self::move_end))
            .on_action(cx.listener(Self::move_line_start))
            .on_action(cx.listener(Self::move_line_end))
            .on_action(cx.listener(Self::select_line_start))
            .on_action(cx.listener(Self::select_line_end))
            .on_action(cx.listener(Self::move_document_start))
            .on_action(cx.listener(Self::move_document_end))
            .on_action(cx.listener(Self::select_document_start))
            .on_action(cx.listener(Self::select_document_end))
            .on_action(cx.listener(Self::page_up))
            .on_action(cx.listener(Self::page_down))
            .on_action(cx.listener(Self::select_page_up))
            .on_action(cx.listener(Self::select_page_down))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_up))
            .on_action(cx.listener(Self::select_down))
            .on_action(cx.listener(Self::select_home))
            .on_action(cx.listener(Self::select_end))
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::delete_word_backward))
            .on_action(cx.listener(Self::delete_word_forward))
            .on_action(cx.listener(Self::delete_to_line_start))
            .on_action(cx.listener(Self::delete_to_line_end))
            .on_action(cx.listener(Self::enter))
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::shift_tab))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::select_word))
            .on_action(cx.listener(Self::select_line))
            .on_action(cx.listener(Self::delete_line))
            .on_action(cx.listener(Self::duplicate_line_or_selection_action))
            .on_action(cx.listener(Self::move_line_up))
            .on_action(cx.listener(Self::move_line_down))
            .on_action(cx.listener(Self::toggle_line_comment_action))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::save))
            .on_action(cx.listener(Self::undo))
            .on_action(cx.listener(Self::redo))
            .on_action(cx.listener(Self::find))
            .on_action(cx.listener(Self::find_next))
            .on_action(cx.listener(Self::find_previous))
            .on_action(cx.listener(Self::go_to_line))
            .on_action(cx.listener(Self::close_find_action));

        if self.show_chrome {
            root.child(header()).child(content)
        } else {
            root.child(content)
        }
    }
}

impl Focusable for EditorPrototype {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EntityInputHandler for EditorPrototype {
    fn text_for_range(
        &mut self,
        range_utf16: std::ops::Range<usize>,
        actual_range: &mut Option<std::ops::Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let start = self.position_for_utf16(range_utf16.start)?;
        let end = self.position_for_utf16(range_utf16.end)?;
        let (.., buffer, _) = self.editor.active_buffer_view()?;
        actual_range.replace(
            self.utf16_for_position(start).unwrap_or(range_utf16.start)
                ..self.utf16_for_position(end).unwrap_or(range_utf16.end),
        );
        Some(buffer.text_range(start, end))
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let (_, buffer, view) = self.editor.active_buffer_view()?;
        let (start, end, reversed) = if let Some((start, end)) = view.cursor.selection() {
            (
                start,
                end,
                view.cursor
                    .anchor
                    .is_some_and(|anchor| anchor > view.cursor.pos),
            )
        } else {
            (view.cursor.pos, view.cursor.pos, false)
        };
        Some(UTF16Selection {
            range: char_index_to_utf16_index(&buffer.text(), buffer.pos_to_char(start))
                ..char_index_to_utf16_index(&buffer.text(), buffer.pos_to_char(end)),
            reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<std::ops::Range<usize>> {
        None
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {}

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<std::ops::Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16.and_then(|range| {
            Some((
                self.position_for_utf16(range.start)?,
                self.position_for_utf16(range.end)?,
            ))
        });
        self.replace_selection_or_range(cx, range, new_text);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<std::ops::Range<usize>>,
        new_text: &str,
        _new_selected_range_utf16: Option<std::ops::Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.replace_text_in_range(range_utf16, new_text, window, cx);
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: std::ops::Range<usize>,
        bounds: Bounds<gpui::Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<gpui::Pixels>> {
        let start = self.position_for_utf16(range_utf16.start)?;
        let appearance = self.active_appearance();
        Some(bounds_for_position(
            self.editor.active_buffer_view()?.2,
            start,
            bounds,
            self.last_text_layout.as_ref(),
            &appearance,
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<gpui::Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let position = self.position_for_point(point)?;
        self.utf16_for_position(position)
    }
}

struct EditorInputElement {
    input: Entity<EditorPrototype>,
}

impl IntoElement for EditorInputElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for EditorInputElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        style.size.height = relative(1.0).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<gpui::Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let layout = {
            let input = self.input.read(cx);
            let appearance = input.active_appearance();
            build_measured_layout(&input.editor, &appearance, window)
        };
        self.input.update(cx, |input, _cx| {
            input.last_text_bounds = Some(bounds);
            input.last_text_layout = layout;
        });
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<gpui::Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        self.input.update(cx, |input, _cx| {
            input.last_text_bounds = Some(bounds);
        });
    }
}

fn header() -> impl IntoElement {
    div()
        .h(px(44.0))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px_4()
        .border_b_1()
        .border_color(rgb(EDITOR_BORDER))
        .bg(rgb(EDITOR_CHROME_BG))
        .child(
            div()
                .font_weight(gpui::FontWeight::BOLD)
                .text_size(px(15.0))
                .child("LLNZY GPUI Editor Prototype"),
        )
        .child(
            div()
                .text_size(px(12.0))
                .text_color(rgb(EDITOR_MUTED_FG))
                .child("EditorState-backed text input prototype"),
        )
}

fn editor_header(snapshot: &EditorSnapshot) -> impl IntoElement {
    div()
        .h(px(48.0))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px_4()
        .border_b_1()
        .border_color(rgb(EDITOR_BORDER))
        .bg(rgb(EDITOR_CHROME_BG))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_size(px(13.0)).child(snapshot.title.clone()))
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(rgb(EDITOR_MUTED_FG))
                        .child(snapshot.subtitle.clone()),
                ),
        )
        .child(
            div()
                .rounded_sm()
                .border_1()
                .border_color(rgb(EDITOR_BORDER))
                .px_2()
                .py_1()
                .text_size(px(11.0))
                .text_color(rgb(EDITOR_TEXT_FG))
                .child(snapshot.language.clone()),
        )
}

fn editor_file_tabs(snapshot: &EditorSnapshot, cx: &mut Context<EditorPrototype>) -> gpui::Div {
    let mut bar = div();
    if snapshot.buffer_tabs.is_empty() && !snapshot.can_reopen_recent {
        return bar;
    }

    bar = bar
        .h(px(36.0))
        .w_full()
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .border_b_1()
        .border_color(rgb(EDITOR_BORDER))
        .bg(rgb(0x15151c))
        .overflow_hidden();

    for tab in snapshot.buffer_tabs.iter().cloned() {
        bar = bar.child(editor_file_tab(tab, cx));
    }
    bar.child(div().flex_1())
        .when(!snapshot.buffer_tabs.is_empty(), |bar| {
            bar.child(editor_tab_action_button("Check Disk", cx, |editor, cx| {
                editor.check_active_external_change(cx);
            }))
            .child(editor_tab_action_button("Hover", cx, |editor, cx| {
                editor.request_lsp_hover(cx);
            }))
            .child(editor_tab_action_button("Complete", cx, |editor, cx| {
                editor.request_lsp_completion(cx);
            }))
            .child(editor_tab_action_button("Def", cx, |editor, cx| {
                editor.request_lsp_definition(cx);
            }))
            .child(editor_tab_action_button("Refs", cx, |editor, cx| {
                editor.request_lsp_references(cx);
            }))
            .child(editor_tab_action_button("Sig", cx, |editor, cx| {
                editor.request_lsp_signature_help(cx);
            }))
            .child(editor_tab_action_button("Rename", cx, |editor, cx| {
                editor.open_lsp_rename(cx);
            }))
            .child(editor_tab_action_button("Actions", cx, |editor, cx| {
                editor.request_lsp_code_actions(cx);
            }))
            .child(editor_tab_action_button("Format", cx, |editor, cx| {
                editor.request_lsp_format(cx);
            }))
            .child(editor_tab_action_button("Symbols", cx, |editor, cx| {
                editor.request_lsp_symbols(cx);
            }))
            .child(editor_tab_action_button(
                "Close Others",
                cx,
                |editor, cx| {
                    editor.close_other_buffer_tabs(cx);
                },
            ))
            .child(editor_tab_action_button("Close Saved", cx, |editor, cx| {
                editor.close_saved_buffer_tabs(cx);
            }))
        })
        .when(snapshot.can_reopen_recent, |bar| {
            bar.child(editor_tab_action_button("Reopen", cx, |editor, cx| {
                editor.reopen_recent_buffer_tab(cx);
            }))
        })
}

fn editor_file_tab(
    tab: EditorBufferTabSnapshot,
    cx: &mut Context<EditorPrototype>,
) -> impl IntoElement {
    let index = tab.index;
    let close_index = tab.index;
    let active = tab.active;
    let label = if tab.dirty {
        format!("* {}", tab.title)
    } else {
        tab.title.clone()
    };
    let subtitle = tab.subtitle.clone();

    div()
        .h(px(28.0))
        .max_w(px(210.0))
        .min_w(px(104.0))
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .rounded_sm()
        .border_1()
        .border_color(rgb(if active { 0x3d5f7a } else { EDITOR_BORDER }))
        .bg(rgb(if active { 0x202735 } else { 0x111117 }))
        .text_color(rgb(if active {
            EDITOR_TEXT_FG
        } else {
            EDITOR_MUTED_FG
        }))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
                this.activate_buffer_tab(index, cx);
            }),
        )
        .child(
            div()
                .flex_1()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_size(px(12.0))
                .child(label),
        )
        .when_some(subtitle, |tab, subtitle| {
            tab.child(
                div()
                    .max_w(px(72.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_size(px(10.0))
                    .text_color(rgb(EDITOR_DIM_FG))
                    .child(subtitle),
            )
        })
        .child(
            div()
                .w(px(16.0))
                .h(px(16.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded_sm()
                .text_size(px(11.0))
                .text_color(rgb(if active { 0xcbd5e1 } else { 0x6f7482 }))
                .hover(|style| style.bg(rgb(0x303845)))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                        cx.stop_propagation();
                        this.close_buffer_tab(close_index, cx);
                    }),
                )
                .child("x"),
        )
}

fn editor_tab_action_button(
    label: &'static str,
    cx: &mut Context<EditorPrototype>,
    handler: fn(&mut EditorPrototype, &mut Context<EditorPrototype>),
) -> impl IntoElement {
    div()
        .h(px(24.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x3c4658))
        .bg(rgb(0x171923))
        .px_2()
        .text_size(px(11.0))
        .text_color(rgb(EDITOR_MUTED_FG))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0x242b38)))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
                handler(this, cx);
            }),
        )
        .child(label)
}

fn editor_body(
    snapshot: &EditorSnapshot,
    input: Entity<EditorPrototype>,
    cx: &mut Context<EditorPrototype>,
) -> impl IntoElement {
    let lines = snapshot
        .lines
        .iter()
        .fold(div().flex().flex_col().py_3(), |column, line| {
            column.child(editor_line(
                line.number,
                &line.text,
                &line.highlights,
                &line.search_matches,
                line.diagnostic.as_ref(),
                snapshot
                    .cursor
                    .filter(|cursor| cursor.line + 1 == line.number),
                snapshot.cursor_visible,
                snapshot.selection,
                snapshot.scroll_col,
                &snapshot.appearance,
            ))
        });
    let ruler_layer = editor_ruler_layer(&snapshot.appearance, snapshot.scroll_col);
    let input_layer = div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .cursor(CursorStyle::IBeam)
        .child(EditorInputElement {
            input: input.clone(),
        });

    div()
        .relative()
        .flex_1()
        .w_full()
        .overflow_hidden()
        .bg(snapshot.appearance.background_color())
        .cursor(CursorStyle::IBeam)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(EditorPrototype::on_editor_mouse_down),
        )
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(EditorPrototype::on_editor_mouse_up),
        )
        .on_mouse_up_out(
            MouseButton::Left,
            cx.listener(EditorPrototype::on_editor_mouse_up),
        )
        .on_mouse_move(cx.listener(EditorPrototype::on_editor_mouse_move))
        .on_scroll_wheel(cx.listener(EditorPrototype::on_editor_scroll))
        .child(lines)
        .child(ruler_layer)
        .child(input_layer)
        .when(snapshot.search_active, |body| {
            body.child(find_overlay(snapshot, cx))
        })
        .when(snapshot.go_to_line_active, |body| {
            body.child(go_to_line_overlay(snapshot, cx))
        })
        .when(snapshot.rename_active, |body| {
            body.child(rename_symbol_overlay(snapshot, cx))
        })
        .when_some(snapshot.external_change.clone(), |body, change| {
            body.child(external_change_overlay(change, cx))
        })
        .when_some(snapshot.degraded_notice.clone(), |body, notice| {
            body.child(degraded_mode_overlay(notice))
        })
        .when_some(snapshot.lsp_panel.clone(), |body, panel| {
            body.child(language_panel_overlay(panel, cx))
        })
}

fn editor_ruler_layer(appearance: &EditorAppearance, scroll_col: usize) -> gpui::Div {
    if appearance.rulers.is_empty() {
        return div();
    }

    let mut layer = div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .overflow_hidden();

    for ruler in appearance.rulers.iter().copied() {
        if ruler < scroll_col {
            continue;
        }
        let x = appearance.line_number_width
            + appearance.char_width * ruler.saturating_sub(scroll_col) as f32;
        layer = layer.child(
            div()
                .absolute()
                .top_0()
                .left(x)
                .w(px(1.0))
                .h_full()
                .bg(appearance.ruler_color()),
        );
    }

    layer
}

fn language_panel_overlay(
    panel: GpuiLspPanel,
    cx: &mut Context<EditorPrototype>,
) -> impl IntoElement {
    let selected = panel.selected;
    let anchor = panel.anchor.unwrap_or(GpuiLspPanelAnchor {
        top: px(44.0),
        left: px(92.0),
    });
    let items = panel.items.into_iter().take(14).enumerate().fold(
        div().flex().flex_col().gap_1(),
        |column, (index, item)| {
            let is_actionable = !matches!(&item.action, GpuiLspPanelAction::None);
            let is_selected = index == selected;
            column.child(
                div()
                    .w_full()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_size(px(11.0))
                    .text_color(rgb(EDITOR_TEXT_FG))
                    .px_1()
                    .py_1()
                    .rounded_sm()
                    .bg(if is_selected {
                        rgb(0x2d374a)
                    } else {
                        rgba(0x00000000)
                    })
                    .when(is_actionable, |row| {
                        row.cursor(CursorStyle::PointingHand)
                            .hover(|style| style.bg(rgb(0x2d374a)))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |editor, _: &MouseDownEvent, _window, cx| {
                                    cx.stop_propagation();
                                    editor.activate_lsp_panel_item(index, cx);
                                }),
                            )
                    })
                    .child(item.label),
            )
        },
    );

    div()
        .absolute()
        .top(anchor.top)
        .left(anchor.left)
        .w(px(420.0))
        .max_h(px(360.0))
        .overflow_hidden()
        .flex()
        .flex_col()
        .gap_2()
        .px_2()
        .py_2()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x3d4a5f))
        .bg(rgb(0x202432))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_this, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
            }),
        )
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_size(px(12.0))
                        .text_color(rgb(EDITOR_TEXT_FG))
                        .child(panel.title),
                )
                .child(find_button("x", cx, |editor, cx| {
                    editor.close_lsp_panel(cx);
                })),
        )
        .child(items)
}

fn degraded_mode_overlay(notice: String) -> impl IntoElement {
    div()
        .absolute()
        .top(px(46.0))
        .left(px(84.0))
        .h(px(30.0))
        .max_w(px(420.0))
        .flex()
        .items_center()
        .px_2()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x665f3a))
        .bg(rgb(0x252313))
        .text_size(px(11.0))
        .text_color(rgb(0xe1d58a))
        .overflow_hidden()
        .whitespace_nowrap()
        .child(notice)
}

fn external_change_overlay(
    change: ExternalFileChangeSnapshot,
    cx: &mut Context<EditorPrototype>,
) -> impl IntoElement {
    div()
        .absolute()
        .top(px(8.0))
        .left(px(84.0))
        .h(px(36.0))
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x765b2f))
        .bg(rgb(0x2a2419))
        .text_size(px(12.0))
        .text_color(rgb(EDITOR_TEXT_FG))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_this, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
            }),
        )
        .child(
            div()
                .max_w(px(280.0))
                .overflow_hidden()
                .whitespace_nowrap()
                .child(format!("{} changed on disk", change.file_name)),
        )
        .child(find_button("Reload", cx, |editor, cx| {
            editor.reload_external_change(cx);
        }))
        .child(find_button("Keep Local", cx, |editor, cx| {
            editor.keep_local_external_change(cx);
        }))
}

fn find_overlay(snapshot: &EditorSnapshot, cx: &mut Context<EditorPrototype>) -> impl IntoElement {
    let height = if snapshot.search_replace_mode {
        px(66.0)
    } else {
        px(34.0)
    };
    div()
        .absolute()
        .top(px(8.0))
        .right(px(12.0))
        .h(height)
        .w(px(470.0))
        .flex()
        .flex_col()
        .justify_center()
        .gap_1()
        .px_2()
        .py_1()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x3d4a5f))
        .bg(rgb(0x202432))
        .text_size(px(12.0))
        .text_color(rgb(EDITOR_TEXT_FG))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_this, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
            }),
        )
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .gap_1()
                .child(find_text_field(
                    "Find",
                    snapshot.search_query.clone(),
                    snapshot.search_input_target == EditorSearchInputTarget::Query,
                    EditorSearchInputTarget::Query,
                    cx,
                ))
                .child(
                    div()
                        .w(px(54.0))
                        .text_align(gpui::TextAlign::Center)
                        .text_color(rgb(EDITOR_MUTED_FG))
                        .child(snapshot.search_status.clone()),
                )
                .child(find_button("Prev", cx, |editor, cx| {
                    editor.move_search_focus(EditorSearchDirection::Previous, cx);
                }))
                .child(find_button("Next", cx, |editor, cx| {
                    editor.move_search_focus(EditorSearchDirection::Next, cx);
                }))
                .child(find_button("Repl", cx, |editor, cx| {
                    editor.toggle_replace_mode(cx);
                }))
                .child(find_button("x", cx, |editor, cx| {
                    editor.close_find(cx);
                })),
        )
        .when(snapshot.search_replace_mode, |overlay| {
            overlay.child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(find_text_field(
                        "Replace",
                        snapshot.search_replacement.clone(),
                        snapshot.search_input_target == EditorSearchInputTarget::Replacement,
                        EditorSearchInputTarget::Replacement,
                        cx,
                    ))
                    .child(find_button("Replace", cx, |editor, cx| {
                        editor.replace_focused_search_match(cx);
                    }))
                    .child(find_button("All", cx, |editor, cx| {
                        editor.replace_all_search_matches(cx);
                    })),
            )
        })
}

fn go_to_line_overlay(
    snapshot: &EditorSnapshot,
    cx: &mut Context<EditorPrototype>,
) -> impl IntoElement {
    div()
        .absolute()
        .top(px(8.0))
        .right(px(12.0))
        .h(px(34.0))
        .w(px(300.0))
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .py_1()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x3d4a5f))
        .bg(rgb(0x202432))
        .text_size(px(12.0))
        .text_color(rgb(EDITOR_TEXT_FG))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_this, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
            }),
        )
        .child(go_to_line_text_field(
            snapshot.go_to_line_input.clone(),
            snapshot.total_lines,
        ))
        .child(find_button("Go", cx, |editor, cx| {
            editor.submit_go_to_line(cx);
        }))
        .child(find_button("x", cx, |editor, cx| {
            editor.close_go_to_line(cx);
        }))
}

fn rename_symbol_overlay(
    snapshot: &EditorSnapshot,
    cx: &mut Context<EditorPrototype>,
) -> impl IntoElement {
    div()
        .absolute()
        .top(px(8.0))
        .right(px(12.0))
        .h(px(34.0))
        .w(px(360.0))
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .py_1()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x3d4a5f))
        .bg(rgb(0x202432))
        .text_size(px(12.0))
        .text_color(rgb(EDITOR_TEXT_FG))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_this, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
            }),
        )
        .child(rename_symbol_text_field(snapshot.rename_input.clone()))
        .child(find_button("Rename", cx, |editor, cx| {
            editor.submit_lsp_rename(cx);
        }))
        .child(find_button("x", cx, |editor, cx| {
            editor.close_lsp_rename(cx);
        }))
}

fn rename_symbol_text_field(input: String) -> impl IntoElement {
    let empty = input.is_empty();
    let label = if empty {
        "New symbol name".to_string()
    } else {
        input
    };

    div()
        .flex_1()
        .h(px(24.0))
        .flex()
        .items_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x78a6d8))
        .bg(rgb(0x141d2b))
        .px_2()
        .overflow_hidden()
        .whitespace_nowrap()
        .text_color(rgb(if empty { EDITOR_DIM_FG } else { EDITOR_TEXT_FG }))
        .cursor(CursorStyle::IBeam)
        .child(label)
}

fn go_to_line_text_field(input: String, total_lines: usize) -> impl IntoElement {
    let empty = input.is_empty();
    let label = if empty {
        format!("Line 1-{total_lines}")
    } else {
        input
    };

    div()
        .flex_1()
        .h(px(24.0))
        .flex()
        .items_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x78a6d8))
        .bg(rgb(0x141d2b))
        .px_2()
        .overflow_hidden()
        .whitespace_nowrap()
        .text_color(rgb(if empty { EDITOR_DIM_FG } else { EDITOR_TEXT_FG }))
        .cursor(CursorStyle::IBeam)
        .child(label)
}

fn find_text_field(
    placeholder: &'static str,
    text: String,
    active: bool,
    target: EditorSearchInputTarget,
    cx: &mut Context<EditorPrototype>,
) -> impl IntoElement {
    let empty = text.is_empty();
    div()
        .flex_1()
        .h(px(24.0))
        .flex()
        .items_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(if active { 0x78a6d8 } else { 0x48546a }))
        .bg(rgb(if active { 0x141d2b } else { 0x111722 }))
        .px_2()
        .overflow_hidden()
        .whitespace_nowrap()
        .text_color(rgb(if empty { EDITOR_DIM_FG } else { EDITOR_TEXT_FG }))
        .cursor(CursorStyle::IBeam)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |editor, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
                editor.set_search_input_target(target, cx);
            }),
        )
        .child(if empty { placeholder.to_string() } else { text })
}

fn find_button(
    label: &'static str,
    cx: &mut Context<EditorPrototype>,
    handler: fn(&mut EditorPrototype, &mut Context<EditorPrototype>),
) -> impl IntoElement {
    div()
        .h(px(24.0))
        .min_w(px(28.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x48546a))
        .bg(rgb(0x151b26))
        .px_2()
        .text_size(px(11.0))
        .text_color(rgb(EDITOR_TEXT_FG))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0x273247)))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
                handler(this, cx);
            }),
        )
        .child(label)
}

fn editor_line(
    number: usize,
    text: &str,
    highlights: &[HighlightSpan],
    search_matches: &[EditorSearchLineMatch],
    diagnostic: Option<&EditorDiagnosticSnapshot>,
    cursor: Option<Position>,
    cursor_visible: bool,
    selection: Option<(Position, Position)>,
    scroll_col: usize,
    appearance: &EditorAppearance,
) -> gpui::Div {
    let line = number.saturating_sub(1);
    let visible_text = display_visible_text(text, scroll_col, appearance.visible_whitespace);
    let diagnostic_range = diagnostic.and_then(|diagnostic| {
        diagnostic_line_range(
            diagnostic,
            line,
            text.chars().count(),
            scroll_col,
            visible_text.chars().count().max(1),
        )
    });
    let visible_cursor = cursor.and_then(|cursor| {
        (cursor.col >= scroll_col).then_some(Position::new(line, cursor.col - scroll_col))
    });
    let line_selection =
        selection.and_then(|(start, end)| selection_for_line(start, end, line, text));
    let active_line = cursor.is_some() && appearance.highlight_current_line;
    let selected = line_selection.is_some();
    let trailing_selection = line_selection.is_some_and(|selection| {
        selection.includes_line_break
            && selection.end_col >= scroll_col + visible_text.chars().count()
    });
    let text_cell = if let Some(cursor) = visible_cursor {
        let (before, after) = split_chars(&visible_text, cursor.col);
        div()
            .flex_1()
            .overflow_hidden()
            .relative()
            .flex()
            .items_center()
            .child(styled_text_segments(
                &before,
                highlights,
                line_selection,
                search_matches,
                scroll_col,
                appearance,
            ))
            .child(editor_caret(cursor_visible, appearance))
            .child(styled_text_segments(
                &after,
                highlights,
                line_selection,
                search_matches,
                scroll_col + cursor.col,
                appearance,
            ))
            .when(trailing_selection, |cell| {
                cell.child(selection_trailing_block(appearance))
            })
    } else {
        div()
            .flex_1()
            .overflow_hidden()
            .relative()
            .flex()
            .items_center()
            .child(styled_text_segments(
                &visible_text,
                highlights,
                line_selection,
                search_matches,
                scroll_col,
                appearance,
            ))
            .when(trailing_selection, |cell| {
                cell.child(selection_trailing_block(appearance))
            })
    };
    let text_cell = text_cell.when_some(diagnostic_range, |cell, range| {
        cell.child(diagnostic_inline_underline(range, appearance))
    });

    let row_bg = if selected {
        appearance.selected_line_color()
    } else if active_line {
        appearance.active_line_color()
    } else {
        appearance.background_color()
    };
    let mut row = div()
        .h(appearance.line_height)
        .w_full()
        .flex()
        .items_center()
        .font_family(appearance.font_family.clone())
        .text_size(appearance.font_size)
        .bg(row_bg);

    if appearance.show_line_numbers {
        row = row.child(
            div()
                .w(appearance.line_number_width)
                .h_full()
                .flex()
                .items_center()
                .justify_end()
                .gap_2()
                .pr_3()
                .bg(if selected {
                    appearance.selected_gutter_color()
                } else if active_line {
                    appearance.active_gutter_color()
                } else {
                    appearance.gutter_color()
                })
                .text_align(gpui::TextAlign::Right)
                .text_color(if selected {
                    appearance.foreground_color()
                } else {
                    appearance.dim_color()
                })
                .when_some(diagnostic, |gutter, diagnostic| {
                    gutter.child(diagnostic_marker(diagnostic.severity))
                })
                .child(number.to_string()),
        );
    } else if let Some(diagnostic) = diagnostic {
        row = row.child(
            div()
                .w(px(14.0))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .child(diagnostic_marker(diagnostic.severity)),
        );
    }

    row.child(text_cell)
}

fn status_bar(snapshot: &EditorSnapshot) -> impl IntoElement {
    let left = if let Some(cursor) = snapshot.cursor {
        format!("Ln {}, Col {}", cursor.line + 1, cursor.col + 1)
    } else if snapshot.sample {
        "sample fallback".to_string()
    } else {
        "EditorState active buffer".to_string()
    };
    let diagnostics = diagnostic_status(&snapshot.diagnostics);
    let lsp = [snapshot.lsp_status.clone(), diagnostics]
        .into_iter()
        .filter(|status| !status.is_empty())
        .collect::<Vec<_>>()
        .join(" | ");
    let right = snapshot
        .status_message
        .clone()
        .or_else(|| snapshot.load_error.clone())
        .or_else(|| snapshot.cursor_diagnostic_message.clone())
        .unwrap_or_else(|| "Ready".to_string());

    div()
        .h(px(24.0))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px_3()
        .border_t_1()
        .border_color(rgb(EDITOR_BORDER))
        .bg(rgb(EDITOR_CHROME_BG))
        .text_size(px(11.0))
        .text_color(snapshot.appearance.muted_color())
        .child(format!(
            "{left} | {}-{} / {}{}",
            snapshot.first_line_number,
            snapshot
                .first_line_number
                .saturating_add(snapshot.lines.len().saturating_sub(1)),
            snapshot.total_lines,
            if lsp.is_empty() {
                String::new()
            } else {
                format!(" | {lsp}")
            }
        ))
        .child(right)
}

fn diagnostic_marker(severity: DiagSeverity) -> impl IntoElement {
    div()
        .w(px(7.0))
        .h(px(7.0))
        .rounded_sm()
        .bg(diagnostic_color(severity))
}

fn diagnostic_inline_underline(
    range: EditorDiagnosticLineRange,
    appearance: &EditorAppearance,
) -> impl IntoElement {
    let start = appearance.char_width * range.start_col as f32;
    let width = appearance.char_width * range.end_col.saturating_sub(range.start_col).max(1) as f32;
    div()
        .absolute()
        .top(appearance.line_height - px(4.0))
        .left(start)
        .w(width)
        .h(px(2.0))
        .rounded_sm()
        .bg(diagnostic_color(range.severity))
}

fn diagnostic_color(severity: DiagSeverity) -> gpui::Rgba {
    match severity {
        DiagSeverity::Error => rgb(0xff6b6b),
        DiagSeverity::Warning => rgb(0xf2c94c),
        DiagSeverity::Info => rgb(0x6cb6ff),
        DiagSeverity::Hint => rgb(0x9aa4b2),
    }
}

fn initial_path() -> Option<PathBuf> {
    env::args()
        .nth(1)
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .or_else(|| readable_repo_file("src/main.rs"))
        .or_else(|| readable_repo_file("Cargo.toml"))
}

fn readable_repo_file(path: impl AsRef<Path>) -> Option<PathBuf> {
    let path = path.as_ref();
    path.is_file().then(|| path.to_path_buf())
}

fn language_label(view: &BufferView) -> String {
    view.lang_id.unwrap_or("plain text").to_string()
}

fn refresh_active_syntax(editor: &mut EditorState) {
    let active = editor.active;
    if active >= editor.buffers.len() || active >= editor.views.len() {
        return;
    }
    if !crate::editor::perf::syntax_enabled(editor.buffers[active].line_count()) {
        editor.reparse_active();
        if let Some(view) = editor.views.get_mut(active) {
            view.tree = None;
            view.folded_ranges.clear();
        }
        return;
    }

    let Some(lang_id) = editor.views[active].lang_id else {
        return;
    };
    if !editor.views[active].tree_dirty && editor.views[active].tree.is_some() {
        return;
    }
    let source = editor.buffers[active].text();
    editor.views[active].tree = editor.syntax.parse(lang_id, &source);
    editor.views[active].tree_dirty = false;
    editor.views[active].folded_ranges.clear();
}

fn buffer_tab_snapshots(editor: &EditorState) -> Vec<EditorBufferTabSnapshot> {
    editor
        .buffers
        .iter()
        .enumerate()
        .map(|(index, buffer)| {
            let title = buffer
                .path()
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| buffer.file_name().to_string());
            let subtitle = buffer
                .path()
                .and_then(|path| path.parent())
                .and_then(|parent| parent.file_name())
                .map(|name| name.to_string_lossy().into_owned());
            EditorBufferTabSnapshot {
                index,
                title,
                subtitle,
                dirty: buffer.is_modified(),
                active: index == editor.active,
            }
        })
        .collect()
}

fn closable_other_buffer_ids(editor: &EditorState, active_id: BufferId) -> Vec<BufferId> {
    editor
        .buffers
        .iter()
        .zip(editor.buffer_ids.iter().copied())
        .filter_map(|(buffer, id)| (id != active_id && !buffer.is_modified()).then_some(id))
        .collect()
}

fn closable_saved_buffer_ids(editor: &EditorState) -> Vec<BufferId> {
    editor
        .buffers
        .iter()
        .zip(editor.buffer_ids.iter().copied())
        .filter_map(|(buffer, id)| (!buffer.is_modified()).then_some(id))
        .collect()
}

fn remember_recently_closed_path(recently_closed_paths: &mut Vec<PathBuf>, path: PathBuf) {
    recently_closed_paths.retain(|candidate| !same_path(candidate, &path));
    recently_closed_paths.push(path);
    if recently_closed_paths.len() > RECENTLY_CLOSED_LIMIT {
        let overflow = recently_closed_paths.len() - RECENTLY_CLOSED_LIMIT;
        recently_closed_paths.drain(0..overflow);
    }
}

fn pop_reopen_candidate(
    recently_closed_paths: &mut Vec<PathBuf>,
    open_paths: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    while let Some(path) = recently_closed_paths.pop() {
        if !path.is_file() {
            continue;
        }
        if open_paths
            .iter()
            .any(|open_path| same_path(open_path, &path))
        {
            continue;
        }
        return Some(path);
    }
    None
}

fn read_normalized_file_text(path: &Path) -> Result<String, String> {
    let text = fs::read_to_string(path).map_err(|err| format!("Cannot read file: {err}"))?;
    Ok(text.replace("\r\n", "\n"))
}

fn search_matches_for_line(search: &EditorSearch, line: usize) -> Vec<EditorSearchLineMatch> {
    if !search.active || search.query.is_empty() {
        return Vec::new();
    }

    search
        .matches
        .iter()
        .enumerate()
        .filter_map(|(idx, search_match)| {
            (search_match.start.line == line && search_match.end.line == line).then_some(
                EditorSearchLineMatch {
                    start_col: search_match.start.col,
                    end_col: search_match.end.col,
                    focused: idx == search.focus,
                },
            )
        })
        .collect()
}

fn sample_text() -> String {
    [
        "LLNZY GPUI editor prototype",
        "",
        "This surface now accepts GPUI text input for opened files.",
        "The production editor model is present, but no file could be opened.",
        "",
        "Next step: move syntax highlighting and richer selection painting over.",
    ]
    .join("\n")
}

fn set_cursor_position(view: &mut BufferView, position: Position, extend: bool) {
    if extend {
        if view.cursor.anchor.is_none() {
            view.cursor.anchor = Some(view.cursor.pos);
        }
    } else {
        view.cursor.clear_selection();
    }
    view.cursor.pos = position;
    view.cursor.desired_col = None;
}

fn deletion_range(
    buffer: &Buffer,
    view: &BufferView,
    target: EditorDeleteTarget,
) -> Option<(Position, Position)> {
    if let Some(selection) = view.cursor.selection() {
        return Some(selection);
    }

    match target {
        EditorDeleteTarget::BackwardChar => movement_range(buffer, view, |cursor, buffer| {
            cursor.move_left(buffer, true);
        }),
        EditorDeleteTarget::ForwardChar => movement_range(buffer, view, |cursor, buffer| {
            cursor.move_right(buffer, true);
        }),
        EditorDeleteTarget::BackwardWord => movement_range(buffer, view, |cursor, buffer| {
            cursor.move_word_left(buffer, true);
        }),
        EditorDeleteTarget::ForwardWord => movement_range(buffer, view, |cursor, buffer| {
            cursor.move_word_right(buffer, true);
        }),
        EditorDeleteTarget::ToLineStart => {
            let pos = view.cursor.pos;
            if pos.col > 0 {
                Some((Position::new(pos.line, 0), pos))
            } else if pos.line > 0 {
                let previous_end = Position::new(pos.line - 1, buffer.line_len(pos.line - 1));
                Some((previous_end, pos))
            } else {
                None
            }
        }
        EditorDeleteTarget::ToLineEnd => {
            let pos = view.cursor.pos;
            let line_end = Position::new(pos.line, buffer.line_len(pos.line));
            if pos < line_end {
                Some((pos, line_end))
            } else if pos.line + 1 < buffer.line_count() {
                Some((pos, Position::new(pos.line + 1, 0)))
            } else {
                None
            }
        }
    }
}

fn movement_range(
    buffer: &Buffer,
    view: &BufferView,
    move_cursor: impl FnOnce(&mut crate::editor::cursor::EditorCursor, &Buffer),
) -> Option<(Position, Position)> {
    let mut cursor = view.cursor.clone();
    move_cursor(&mut cursor, buffer);
    cursor.selection()
}

fn build_measured_layout(
    editor: &EditorState,
    appearance: &EditorAppearance,
    window: &mut Window,
) -> Option<EditorMeasuredLayout> {
    let (_, buffer, view) = editor.active_buffer_view()?;
    let line_count = buffer.line_count();
    let first_line = view.scroll_line.min(line_count.saturating_sub(1));
    let visible_end = line_count.min(first_line + VISIBLE_LINE_LIMIT);
    let editor_font = font(appearance.font_family.clone());
    let lines = (first_line..visible_end)
        .map(|line_idx| {
            let visible_text = skip_chars(buffer.line(line_idx), view.scroll_col);
            let display_text = SharedString::from(visible_text.clone());
            let run = TextRun {
                len: display_text.len(),
                font: editor_font.clone(),
                color: appearance.foreground_color().into(),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let shaped =
                window
                    .text_system()
                    .shape_line(display_text, appearance.font_size, &[run], None);
            EditorMeasuredLine {
                source_line: line_idx,
                visible_text,
                shaped,
            }
        })
        .collect();

    Some(EditorMeasuredLayout {
        scroll_col: view.scroll_col,
        lines,
    })
}

fn measured_position_for_point(
    layout: &EditorMeasuredLayout,
    buffer: &Buffer,
    local_point: gpui::Point<Pixels>,
    appearance: &EditorAppearance,
) -> Option<Position> {
    let row = editor_row_for_local_y(local_point.y, appearance);
    let measured_line = layout.lines.get(row).or_else(|| layout.lines.last())?;
    let text_x = (local_point.x - appearance.line_number_width).max(px(0.0));
    let byte_index = measured_line.shaped.closest_index_for_x(text_x);
    let visible_col = char_count_for_byte_index(&measured_line.visible_text, byte_index);
    let col = layout.scroll_col.saturating_add(visible_col);
    let line = measured_line
        .source_line
        .min(buffer.line_count().saturating_sub(1));
    Some(Position::new(line, col.min(buffer.line_len(line))))
}

fn selected_line_range(buffer: &Buffer, view: &BufferView) -> (usize, usize) {
    if let Some((start, end)) = view.cursor.selection() {
        let mut end_line = end.line;
        if end.col == 0 && end.line > start.line {
            end_line -= 1;
        }
        return (
            start.line.min(buffer.line_count().saturating_sub(1)),
            end_line.min(buffer.line_count().saturating_sub(1)),
        );
    }

    let line = view
        .cursor
        .pos
        .line
        .min(buffer.line_count().saturating_sub(1));
    (line, line)
}

fn byte_index_for_char_col(text: &str, col: usize) -> usize {
    text.char_indices()
        .map(|(byte, _)| byte)
        .nth(col)
        .unwrap_or(text.len())
}

fn char_count_for_byte_index(text: &str, byte_index: usize) -> usize {
    let mut byte = byte_index.min(text.len());
    while byte > 0 && !text.is_char_boundary(byte) {
        byte -= 1;
    }
    text[..byte].chars().count()
}

fn editor_row_for_local_y(y: Pixels, appearance: &EditorAppearance) -> usize {
    ((y - appearance.vertical_padding) / appearance.line_height)
        .floor()
        .max(0.0) as usize
}

fn indented_lines_replacement(buffer: &Buffer, start_line: usize, end_line: usize) -> String {
    let indent = buffer.indent_style.as_str();
    (start_line..=end_line)
        .map(|line_idx| format!("{indent}{}", buffer.line(line_idx)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn dedented_lines_replacement(buffer: &Buffer, start_line: usize, end_line: usize) -> String {
    (start_line..=end_line)
        .map(|line_idx| {
            let line = buffer.line(line_idx);
            let remove_count = dedent_char_count(line, buffer.indent_style.width());
            line.chars().skip(remove_count).collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn dedent_char_count(line: &str, width: usize) -> usize {
    if line.starts_with('\t') {
        return 1;
    }
    line.chars().take_while(|ch| *ch == ' ').count().min(width)
}

fn delete_lines_as_command(buffer: &mut Buffer, start_line: usize, end_line: usize) -> Position {
    if buffer.line_count() == 0 {
        return Position::new(0, 0);
    }

    let start_line = start_line.min(buffer.line_count().saturating_sub(1));
    let end_line = end_line.min(buffer.line_count().saturating_sub(1));
    if start_line > end_line {
        return Position::new(start_line, 0);
    }

    if start_line == 0 && end_line + 1 >= buffer.line_count() {
        buffer.replace(
            Position::new(0, 0),
            Position::new(end_line, buffer.line_len(end_line)),
            "",
        );
        return Position::new(0, 0);
    }

    if end_line + 1 < buffer.line_count() {
        buffer.replace(
            Position::new(start_line, 0),
            Position::new(end_line + 1, 0),
            "",
        );
        return Position::new(start_line.min(buffer.line_count().saturating_sub(1)), 0);
    }

    let previous_line = start_line.saturating_sub(1);
    let previous_len = buffer.line_len(previous_line);
    buffer.replace(
        Position::new(previous_line, previous_len),
        Position::new(end_line, buffer.line_len(end_line)),
        "",
    );
    Position::new(previous_line, 0)
}

#[derive(Clone, Copy)]
struct CommentStyle {
    line: Option<&'static str>,
}

fn comment_style(lang_id: Option<&'static str>, path: Option<&Path>) -> CommentStyle {
    let ext = path
        .and_then(|p| p.extension())
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    let lang = lang_id.or(match ext.as_deref() {
        Some("rs") => Some("rust"),
        Some("js" | "mjs" | "cjs" | "jsx") => Some("javascript"),
        Some("ts" | "mts" | "cts") => Some("typescript"),
        Some("tsx") => Some("tsx"),
        Some("py" | "pyi") => Some("python"),
        Some("rb") => Some("ruby"),
        Some("go") => Some("go"),
        Some("c" | "h") => Some("c"),
        Some("cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx") => Some("cpp"),
        Some("java") => Some("java"),
        Some("kt" | "kts") => Some("kotlin"),
        Some("swift") => Some("swift"),
        Some("sql") => Some("sql"),
        Some("lua") => Some("lua"),
        Some("html" | "htm") => Some("html"),
        Some("css" | "scss") => Some("css"),
        Some("sh" | "bash" | "zsh") => Some("bash"),
        Some("toml") => Some("toml"),
        _ => None,
    });

    match lang {
        Some(
            "rust" | "javascript" | "typescript" | "tsx" | "go" | "c" | "cpp" | "java" | "kotlin"
            | "swift",
        ) => CommentStyle { line: Some("//") },
        Some("python" | "ruby" | "bash" | "toml") => CommentStyle { line: Some("#") },
        Some("sql" | "lua") => CommentStyle { line: Some("--") },
        Some("html" | "css") => CommentStyle { line: None },
        _ => CommentStyle { line: Some("//") },
    }
}

fn toggle_line_comments_as_command(
    buffer: &mut Buffer,
    start_line: usize,
    end_line: usize,
    prefix: &str,
) -> bool {
    if prefix.is_empty() || buffer.line_count() == 0 {
        return false;
    }
    let end_line = end_line.min(buffer.line_count().saturating_sub(1));
    if start_line > end_line {
        return false;
    }

    let mut any_content = false;
    let mut all_commented = true;
    for line_idx in start_line..=end_line {
        let line = buffer.line(line_idx);
        if line.trim().is_empty() {
            continue;
        }
        any_content = true;
        let indent = line_indent(line);
        if !line[indent.len()..].starts_with(prefix) {
            all_commented = false;
            break;
        }
    }
    if !any_content {
        return false;
    }

    let replacement = (start_line..=end_line)
        .map(|line_idx| {
            let line = buffer.line(line_idx);
            if line.trim().is_empty() {
                return line.to_string();
            }

            let indent = line_indent(line);
            let after_indent = &line[indent.len()..];
            if all_commented {
                let after_prefix = &after_indent[prefix.len()..];
                let after_prefix = after_prefix.strip_prefix(' ').unwrap_or(after_prefix);
                format!("{indent}{after_prefix}")
            } else {
                format!("{indent}{prefix} {after_indent}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let start = Position::new(start_line, 0);
    let end = Position::new(end_line, buffer.line_len(end_line));
    if buffer.text_range(start, end) == replacement {
        return false;
    }
    buffer.replace(start, end, &replacement);
    true
}

fn line_indent(line: &str) -> &str {
    let trimmed = line.trim_start_matches([' ', '\t']);
    &line[..line.len() - trimmed.len()]
}

struct EditorSnapshot {
    title: String,
    subtitle: String,
    language: String,
    lines: Vec<EditorLineSnapshot>,
    buffer_tabs: Vec<EditorBufferTabSnapshot>,
    first_line_number: usize,
    cursor: Option<Position>,
    cursor_visible: bool,
    selection: Option<(Position, Position)>,
    scroll_col: usize,
    total_lines: usize,
    load_error: Option<String>,
    status_message: Option<String>,
    sample: bool,
    search_active: bool,
    search_query: String,
    search_replacement: String,
    search_replace_mode: bool,
    search_input_target: EditorSearchInputTarget,
    search_status: String,
    go_to_line_active: bool,
    go_to_line_input: String,
    rename_active: bool,
    rename_input: String,
    can_reopen_recent: bool,
    external_change: Option<ExternalFileChangeSnapshot>,
    lsp_status: String,
    diagnostics: Vec<EditorDiagnosticSnapshot>,
    lsp_panel: Option<GpuiLspPanel>,
    degraded_notice: Option<String>,
    cursor_diagnostic_message: Option<String>,
    appearance: EditorAppearance,
}

struct EditorLineSnapshot {
    number: usize,
    text: String,
    highlights: Vec<HighlightSpan>,
    search_matches: Vec<EditorSearchLineMatch>,
    diagnostic: Option<EditorDiagnosticSnapshot>,
}

const EDITOR_CHROME_BG: u32 = 0x242424;
const EDITOR_BORDER: u32 = 0x34343c;
const EDITOR_TEXT_FG: u32 = 0xe8e8ee;
const EDITOR_MUTED_FG: u32 = 0x9a9aa7;
const EDITOR_DIM_FG: u32 = 0x70707d;
const VISIBLE_LINE_LIMIT: usize = 32;
const EDITOR_VERTICAL_PADDING: gpui::Pixels = px(12.0);
const DEFAULT_VISIBLE_COL_LIMIT: usize = 96;
const RECENTLY_CLOSED_LIMIT: usize = 16;
const CURSOR_BLINK_INTERVAL: Duration = Duration::from_millis(530);
const CURSOR_BLINK_TICK: Duration = Duration::from_millis(80);
const LSP_POLL_TICK: Duration = Duration::from_millis(220);

fn rgb_u32(color: [u8; 3]) -> u32 {
    ((color[0] as u32) << 16) | ((color[1] as u32) << 8) | color[2] as u32
}

fn rgba_u32(color: [u8; 3], alpha: f32) -> u32 {
    (rgb_u32(color) << 8) | color_channel(alpha) as u32
}

fn color_channel(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn mix_rgb(base: [u8; 3], overlay: [u8; 3], amount: f32) -> [u8; 3] {
    let amount = amount.clamp(0.0, 1.0);
    [
        (base[0] as f32 + (overlay[0] as f32 - base[0] as f32) * amount).round() as u8,
        (base[1] as f32 + (overlay[1] as f32 - base[1] as f32) * amount).round() as u8,
        (base[2] as f32 + (overlay[2] as f32 - base[2] as f32) * amount).round() as u8,
    ]
}

fn poll_lsp_request<T>(
    slot: &mut Option<GpuiPendingLspRequest<T>>,
) -> Option<(BufferId, Result<T, ()>)> {
    let mut request = slot.take()?;
    match request.rx.try_recv() {
        Ok(value) => Some((request.buffer_id, Ok(value))),
        Err(oneshot::error::TryRecvError::Empty) => {
            *slot = Some(request);
            None
        }
        Err(oneshot::error::TryRecvError::Closed) => Some((request.buffer_id, Err(()))),
    }
}

fn lsp_panel(title: impl Into<String>, items: Vec<GpuiLspPanelItem>) -> GpuiLspPanel {
    let selected = items
        .iter()
        .position(|item| !matches!(item.action, GpuiLspPanelAction::None))
        .unwrap_or(0);
    GpuiLspPanel {
        title: title.into(),
        items,
        selected,
        anchor: None,
    }
}

fn lsp_panel_anchor(
    cursor: Position,
    scroll_line: usize,
    scroll_col: usize,
    appearance: &EditorAppearance,
) -> Option<GpuiLspPanelAnchor> {
    if cursor.line < scroll_line || cursor.col < scroll_col {
        return None;
    }
    Some(GpuiLspPanelAnchor {
        top: appearance.vertical_padding
            + appearance.line_height * cursor.line.saturating_sub(scroll_line) as f32
            + appearance.line_height,
        left: appearance.line_number_width
            + appearance.char_width * cursor.col.saturating_sub(scroll_col) as f32,
    })
}

fn next_lsp_change_kind(
    existing: Option<&GpuiPendingLspChangeKind>,
    edit: Option<BufferEdit>,
) -> GpuiPendingLspChangeKind {
    match (existing, edit) {
        (None, Some(edit)) => GpuiPendingLspChangeKind::Incremental {
            start: edit.start,
            old_end: edit.old_end,
            new_text: edit.new_text,
        },
        (None, None) => GpuiPendingLspChangeKind::Full,
        (Some(_), _) => GpuiPendingLspChangeKind::Full,
    }
}

fn diagnostic_snapshot(diagnostic: &FileDiagnostic) -> EditorDiagnosticSnapshot {
    EditorDiagnosticSnapshot {
        line: diagnostic.line as usize,
        col: diagnostic.col as usize,
        end_line: diagnostic.end_line as usize,
        end_col: diagnostic.end_col as usize,
        severity: diagnostic.severity,
        message: diagnostic.message.clone(),
    }
}

fn diagnostic_for_line(
    diagnostics: &[EditorDiagnosticSnapshot],
    line: usize,
) -> Option<EditorDiagnosticSnapshot> {
    diagnostics
        .iter()
        .filter(|diagnostic| line >= diagnostic.line && line <= diagnostic.end_line)
        .min_by_key(|diagnostic| diagnostic_severity_rank(diagnostic.severity))
        .cloned()
}

fn diagnostic_at_position(
    diagnostics: &[EditorDiagnosticSnapshot],
    position: Position,
) -> Option<EditorDiagnosticSnapshot> {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic_contains_position(diagnostic, position))
        .min_by_key(|diagnostic| diagnostic_severity_rank(diagnostic.severity))
        .cloned()
}

fn diagnostic_contains_position(diagnostic: &EditorDiagnosticSnapshot, position: Position) -> bool {
    let start = Position::new(diagnostic.line, diagnostic.col);
    let end = Position::new(diagnostic.end_line, diagnostic.end_col);
    position >= start && (position < end || start == end)
}

fn diagnostic_line_range(
    diagnostic: &EditorDiagnosticSnapshot,
    line: usize,
    line_len: usize,
    scroll_col: usize,
    visible_cols: usize,
) -> Option<EditorDiagnosticLineRange> {
    if line < diagnostic.line || line > diagnostic.end_line {
        return None;
    }

    let mut start = if line == diagnostic.line {
        diagnostic.col
    } else {
        0
    }
    .min(line_len);
    let mut end = if line == diagnostic.end_line {
        diagnostic.end_col
    } else {
        line_len
    }
    .min(line_len.max(start + 1));

    if end <= start {
        end = start + 1;
    }

    let visible_start = scroll_col;
    let visible_end = scroll_col + visible_cols.max(1);
    if end <= visible_start || start >= visible_end {
        return None;
    }

    start = start.max(visible_start) - visible_start;
    end = end.min(visible_end) - visible_start;
    (end > start).then_some(EditorDiagnosticLineRange {
        start_col: start,
        end_col: end,
        severity: diagnostic.severity,
    })
}

fn diagnostic_severity_rank(severity: DiagSeverity) -> u8 {
    match severity {
        DiagSeverity::Error => 0,
        DiagSeverity::Warning => 1,
        DiagSeverity::Info => 2,
        DiagSeverity::Hint => 3,
    }
}

fn diagnostic_status(diagnostics: &[EditorDiagnosticSnapshot]) -> String {
    if diagnostics.is_empty() {
        return String::new();
    }

    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == DiagSeverity::Error)
        .count();
    let warnings = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == DiagSeverity::Warning)
        .count();
    match (errors, warnings) {
        (0, 0) => format!("{} diagnostic(s)", diagnostics.len()),
        (0, warnings) => format!("{warnings} warning(s)"),
        (errors, 0) => format!("{errors} error(s)"),
        (errors, warnings) => format!("{errors} error(s), {warnings} warning(s)"),
    }
}

fn panel_lines(text: String, limit: usize) -> Vec<String> {
    let lines = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(limit)
        .map(|line| truncate_panel_text(line.trim().to_string(), 140))
        .collect::<Vec<_>>();
    if lines.is_empty() {
        vec![truncate_panel_text(text, 140)]
    } else {
        lines
    }
}

fn plain_lsp_panel_items(items: Vec<String>) -> Vec<GpuiLspPanelItem> {
    items.into_iter().map(GpuiLspPanelItem::plain).collect()
}

fn completion_panel_items(items: Vec<CompletionItem>, limit: usize) -> Vec<GpuiLspPanelItem> {
    items
        .into_iter()
        .take(limit)
        .map(|item| {
            let insert_text = completion_insert_text(&item);
            let mut parts = vec![item.label.clone()];
            if let Some(kind) = item.kind {
                parts.push(format!("{kind:?}"));
            }
            if let Some(detail) = item.detail.as_ref().filter(|detail| !detail.is_empty()) {
                parts.push(detail.clone());
            }
            let label = truncate_panel_text(parts.join("  "), 140);
            GpuiLspPanelItem {
                label,
                action: GpuiLspPanelAction::Complete { text: insert_text },
            }
        })
        .collect()
}

fn completion_insert_text(item: &CompletionItem) -> String {
    let text = item
        .insert_text
        .as_deref()
        .filter(|text| !text.is_empty())
        .unwrap_or(&item.label);
    sanitize_lsp_insert_text(text)
}

fn sanitize_lsp_insert_text(text: &str) -> String {
    let mut output = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '$' {
            output.push(ch);
            continue;
        }

        match chars.peek().copied() {
            Some('{') => {
                chars.next();
                let mut placeholder = String::new();
                let mut saw_colon = false;
                for inner in chars.by_ref() {
                    if inner == '}' {
                        break;
                    }
                    if saw_colon {
                        placeholder.push(inner);
                    } else if inner == ':' {
                        saw_colon = true;
                    }
                }
                if saw_colon {
                    output.push_str(&placeholder);
                }
            }
            Some(next) if next.is_ascii_digit() => {
                while chars.peek().is_some_and(|next| next.is_ascii_digit()) {
                    chars.next();
                }
            }
            _ => output.push(ch),
        }
    }
    output
}

fn references_panel_items(
    references: Vec<ReferenceLocation>,
    limit: usize,
) -> Vec<GpuiLspPanelItem> {
    references
        .into_iter()
        .take(limit)
        .map(|reference| {
            let file = reference
                .path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| reference.path.display().to_string());
            GpuiLspPanelItem {
                label: truncate_panel_text(
                    format!(
                        "{}:{}:{}  {}",
                        file,
                        reference.line + 1,
                        reference.col + 1,
                        reference.context.trim()
                    ),
                    140,
                ),
                action: GpuiLspPanelAction::GoTo {
                    path: reference.path,
                    line: reference.line,
                    col: reference.col,
                },
            }
        })
        .collect()
}

fn symbols_panel_items(
    symbols: Vec<SymbolInfo>,
    path: Option<PathBuf>,
    limit: usize,
) -> Vec<GpuiLspPanelItem> {
    symbols
        .into_iter()
        .take(limit)
        .map(|symbol| {
            let action = path
                .clone()
                .map(|path| GpuiLspPanelAction::GoTo {
                    path,
                    line: symbol.line,
                    col: symbol.col,
                })
                .unwrap_or(GpuiLspPanelAction::None);
            GpuiLspPanelItem {
                label: truncate_panel_text(
                    format!(
                        "{}  {}:{}  {}",
                        symbol.kind,
                        symbol.line + 1,
                        symbol.col + 1,
                        symbol.name
                    ),
                    140,
                ),
                action,
            }
        })
        .collect()
}

fn signature_panel_items(signature: SignatureInfo) -> Vec<String> {
    let mut items = vec![truncate_panel_text(signature.label, 140)];
    if let Some(parameter) = signature.parameters.get(signature.active_parameter) {
        items.push(truncate_panel_text(format!("active: {parameter}"), 140));
    }
    items
}

fn code_action_panel_items(actions: Vec<CodeAction>, limit: usize) -> Vec<GpuiLspPanelItem> {
    actions
        .into_iter()
        .take(limit)
        .map(|action| {
            let action_kind = if action.edits.is_empty() {
                GpuiLspPanelAction::None
            } else {
                GpuiLspPanelAction::ApplyWorkspaceEdit {
                    edits: action.edits,
                }
            };
            GpuiLspPanelItem {
                label: truncate_panel_text(action.title, 120),
                action: action_kind,
            }
        })
        .collect()
}

fn apply_format_edits_to_file(path: &Path, edits: &[FormatEdit]) -> Result<usize, String> {
    if edits.is_empty() {
        return Ok(0);
    }
    let text =
        fs::read_to_string(path).map_err(|err| format!("read {} failed: {err}", path.display()))?;
    let (new_text, applied) = apply_format_edits_to_text(&text, edits)?;
    if applied > 0 {
        fs::write(path, new_text)
            .map_err(|err| format!("write {} failed: {err}", path.display()))?;
    }
    Ok(applied)
}

fn apply_format_edits_to_text(text: &str, edits: &[FormatEdit]) -> Result<(String, usize), String> {
    let mut output = text.to_string();
    let mut sorted = edits.to_vec();
    sorted.sort_by(|a, b| {
        b.start_line
            .cmp(&a.start_line)
            .then(b.start_col.cmp(&a.start_col))
    });

    let mut applied = 0;
    for edit in sorted {
        let start =
            text_position_to_byte_index(&output, edit.start_line as usize, edit.start_col as usize)
                .ok_or_else(|| "edit start is out of bounds".to_string())?;
        let end =
            text_position_to_byte_index(&output, edit.end_line as usize, edit.end_col as usize)
                .ok_or_else(|| "edit end is out of bounds".to_string())?;
        if start > end {
            return Err("edit start is after edit end".to_string());
        }
        output.replace_range(start..end, &edit.new_text);
        applied += 1;
    }
    Ok((output, applied))
}

fn text_position_to_byte_index(text: &str, line: usize, col: usize) -> Option<usize> {
    let mut current_line = 0;
    let mut line_start = 0;
    for segment in text.split_inclusive('\n') {
        let line_without_newline = segment.trim_end_matches('\n').trim_end_matches('\r');
        if current_line == line {
            return Some(line_start + byte_index_for_char_col(line_without_newline, col));
        }
        line_start += segment.len();
        current_line += 1;
    }

    if current_line == line {
        let tail = &text[line_start..];
        return Some(line_start + byte_index_for_char_col(tail, col));
    }
    None
}

fn truncate_panel_text(text: String, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text;
    }
    let mut truncated = text
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    truncated.push_str("...");
    truncated
}

fn visible_col_limit_for_bounds(
    bounds: Bounds<gpui::Pixels>,
    appearance: &EditorAppearance,
) -> usize {
    ((bounds.size.width - appearance.line_number_width) / appearance.char_width)
        .floor()
        .max(1.0) as usize
}

fn local_editor_point(
    bounds: Bounds<gpui::Pixels>,
    point: gpui::Point<gpui::Pixels>,
) -> Option<gpui::Point<gpui::Pixels>> {
    if let Some(local) = bounds.localize(&point) {
        return Some(local);
    }

    let zero = px(0.0);
    (point.x >= zero
        && point.y >= zero
        && point.x <= bounds.size.width
        && point.y <= bounds.size.height)
        .then_some(point)
}

fn raw_local_editor_point(
    bounds: Bounds<gpui::Pixels>,
    point: gpui::Point<gpui::Pixels>,
) -> gpui::Point<gpui::Pixels> {
    bounds
        .localize(&point)
        .unwrap_or_else(|| gpui::point(point.x - bounds.left(), point.y - bounds.top()))
}

fn clamp_local_editor_point(
    bounds: Bounds<gpui::Pixels>,
    point: gpui::Point<gpui::Pixels>,
) -> gpui::Point<gpui::Pixels> {
    gpui::point(
        clamp_pixels(point.x, px(0.0), bounds.size.width),
        clamp_pixels(point.y, px(0.0), bounds.size.height),
    )
}

fn clamp_pixels(value: gpui::Pixels, min: gpui::Pixels, max: gpui::Pixels) -> gpui::Pixels {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

fn drag_scroll_delta_for_local_y(
    y: gpui::Pixels,
    height: gpui::Pixels,
    appearance: &EditorAppearance,
) -> isize {
    let top_threshold = appearance.vertical_padding;
    let bottom_threshold = (height - appearance.vertical_padding).max(top_threshold);
    if y < top_threshold {
        -(1 + ((top_threshold - y) / appearance.line_height)
            .floor()
            .min(5.0) as isize)
    } else if y > bottom_threshold {
        1 + ((y - bottom_threshold) / appearance.line_height)
            .floor()
            .min(5.0) as isize
    } else {
        0
    }
}

fn drag_scroll_delta_for_local_x(
    x: gpui::Pixels,
    width: gpui::Pixels,
    appearance: &EditorAppearance,
) -> isize {
    let text_left = appearance.line_number_width;
    let right_threshold = (width - appearance.char_width * 2.0).max(text_left);
    if x < text_left {
        -(1 + ((text_left - x) / appearance.char_width).floor().min(12.0) as isize)
    } else if x > right_threshold {
        1 + ((x - right_threshold) / appearance.char_width)
            .floor()
            .min(12.0) as isize
    } else {
        0
    }
}

fn cursor_visible_for_elapsed(elapsed: Duration) -> bool {
    let phase = elapsed.as_millis() / CURSOR_BLINK_INTERVAL.as_millis().max(1);
    phase % 2 == 0
}

fn reveal_cursor(view: &mut BufferView, line_count: usize, visible_cols: usize) {
    let cursor_line = view.cursor.pos.line.min(line_count.saturating_sub(1));
    if cursor_line < view.scroll_line {
        view.scroll_line = cursor_line;
    } else if cursor_line >= view.scroll_line + VISIBLE_LINE_LIMIT {
        view.scroll_line = cursor_line.saturating_sub(VISIBLE_LINE_LIMIT - 1);
    }

    let visible_cols = visible_cols.max(1);
    let cursor_col = view.cursor.pos.col;
    if cursor_col < view.scroll_col {
        view.scroll_col = cursor_col;
    } else if cursor_col >= view.scroll_col.saturating_add(visible_cols) {
        view.scroll_col = cursor_col.saturating_sub(visible_cols.saturating_sub(1));
    }
}

fn scroll_view_by_lines(view: &mut BufferView, line_count: usize, delta: isize) {
    view.scroll_line = scroll_line_by_delta(view.scroll_line, line_count, delta);
}

fn scroll_view_by_columns(
    view: &mut BufferView,
    buffer: &Buffer,
    visible_cols: usize,
    delta: isize,
) {
    let max_line_len = max_buffer_line_len(buffer);
    view.scroll_col = scroll_col_by_delta(view.scroll_col, max_line_len, visible_cols, delta);
}

fn scroll_line_by_delta(current: usize, line_count: usize, delta: isize) -> usize {
    let max_scroll = line_count.saturating_sub(VISIBLE_LINE_LIMIT);
    if delta < 0 {
        current.saturating_sub(delta.unsigned_abs()).min(max_scroll)
    } else {
        current.saturating_add(delta as usize).min(max_scroll)
    }
}

fn scroll_col_by_delta(
    current: usize,
    max_line_len: usize,
    visible_cols: usize,
    delta: isize,
) -> usize {
    let max_scroll = max_line_len.saturating_sub(visible_cols.max(1).saturating_sub(1));
    if delta < 0 {
        current.saturating_sub(delta.unsigned_abs()).min(max_scroll)
    } else {
        current.saturating_add(delta as usize).min(max_scroll)
    }
}

fn max_buffer_line_len(buffer: &Buffer) -> usize {
    (0..buffer.line_count())
        .map(|line| buffer.line_len(line))
        .max()
        .unwrap_or(0)
}

fn parse_go_to_line(input: &str, total_lines: usize) -> Option<usize> {
    if total_lines == 0 {
        return None;
    }
    let requested = input.trim().parse::<usize>().ok()?;
    if requested == 0 {
        return None;
    }
    Some(
        requested
            .saturating_sub(1)
            .min(total_lines.saturating_sub(1)),
    )
}

fn split_chars(text: &str, char_index: usize) -> (String, String) {
    let byte = text
        .char_indices()
        .map(|(byte, _)| byte)
        .nth(char_index)
        .unwrap_or(text.len());
    (text[..byte].to_string(), text[byte..].to_string())
}

fn skip_chars(text: &str, char_count: usize) -> String {
    let byte = text
        .char_indices()
        .map(|(byte, _)| byte)
        .nth(char_count)
        .unwrap_or(text.len());
    text[byte..].to_string()
}

fn display_visible_text(text: &str, scroll_col: usize, visible_whitespace: bool) -> String {
    let visible = skip_chars(text, scroll_col);
    if !visible_whitespace {
        return visible;
    }

    visible
        .chars()
        .map(|ch| match ch {
            ' ' => '·',
            '\t' => '→',
            _ => ch,
        })
        .collect()
}

fn styled_text_segments(
    text: &str,
    highlights: &[HighlightSpan],
    selection: Option<EditorLineSelection>,
    search_matches: &[EditorSearchLineMatch],
    col_offset: usize,
    appearance: &EditorAppearance,
) -> impl IntoElement {
    let mut row = div()
        .flex()
        .items_center()
        .text_color(appearance.foreground_color());
    if text.is_empty() {
        return row;
    }

    let mut segment_start = 0;
    let mut current_col = col_offset;
    let mut current_style = text_style_for_col(
        current_col,
        highlights,
        selection,
        search_matches,
        appearance,
    );

    for (byte, _) in text.char_indices().skip(1) {
        let next_col = current_col + 1;
        let next_style =
            text_style_for_col(next_col, highlights, selection, search_matches, appearance);
        if next_style != current_style {
            row = row.child(styled_text_chunk(
                &text[segment_start..byte],
                current_style,
                appearance,
            ));
            segment_start = byte;
            current_style = next_style;
        }
        current_col = next_col;
    }

    row.child(styled_text_chunk(
        &text[segment_start..],
        current_style,
        appearance,
    ))
}

fn styled_text_chunk(
    text: &str,
    style: TextChunkStyle,
    appearance: &EditorAppearance,
) -> impl IntoElement {
    let mut chunk = div()
        .h(appearance.line_height)
        .flex()
        .items_center()
        .text_color(style.color);
    if style.selected {
        chunk = chunk.bg(appearance.selection_color());
    } else if style.search_focused {
        chunk = chunk.bg(rgb(0x72521c));
    } else if style.search_match {
        chunk = chunk.bg(rgb(0x3f3518));
    }
    chunk.child(text.to_string())
}

fn selection_trailing_block(appearance: &EditorAppearance) -> impl IntoElement {
    div()
        .w(appearance.char_width)
        .h(appearance.line_height)
        .bg(appearance.selection_color())
}

fn editor_caret(visible: bool, appearance: &EditorAppearance) -> impl IntoElement {
    let color = if visible {
        appearance.cursor_color()
    } else {
        rgba(0x00000000)
    };

    match appearance.cursor_style {
        ConfigCursorStyle::Block => div()
            .w(appearance.char_width)
            .h(appearance.line_height)
            .bg(color),
        ConfigCursorStyle::Underline => div()
            .w(appearance.char_width)
            .h(appearance.line_height)
            .flex()
            .items_end()
            .child(div().w_full().h(px(2.0)).bg(color)),
        ConfigCursorStyle::Beam => div()
            .w(px(2.0))
            .h((appearance.line_height - px(4.0)).max(px(8.0)))
            .bg(color),
    }
}

fn text_style_for_col(
    col: usize,
    highlights: &[HighlightSpan],
    selection: Option<EditorLineSelection>,
    search_matches: &[EditorSearchLineMatch],
    appearance: &EditorAppearance,
) -> TextChunkStyle {
    let group = highlights
        .iter()
        .find(|span| col >= span.col_start && col < span.col_end)
        .map(|span| span.group);
    let search_match = search_matches
        .iter()
        .find(|search_match| col >= search_match.start_col && col < search_match.end_col);
    TextChunkStyle {
        color: highlight_color(group, appearance),
        selected: selection
            .is_some_and(|selection| col >= selection.start_col && col < selection.end_col),
        search_match: search_match.is_some(),
        search_focused: search_match.is_some_and(|search_match| search_match.focused),
    }
}

fn highlight_color(group: Option<HighlightGroup>, appearance: &EditorAppearance) -> gpui::Rgba {
    let [red, green, blue] = group.map(group_color).unwrap_or(appearance.foreground);
    rgb(((red as u32) << 16) | ((green as u32) << 8) | blue as u32)
}

#[derive(Clone, Copy, PartialEq)]
struct TextChunkStyle {
    color: gpui::Rgba,
    selected: bool,
    search_match: bool,
    search_focused: bool,
}

fn selection_for_line(
    start: Position,
    end: Position,
    line: usize,
    text: &str,
) -> Option<EditorLineSelection> {
    if line < start.line || line > end.line {
        return None;
    }

    let line_len = text.chars().count();
    let start_col = if line == start.line { start.col } else { 0 };
    let end_col = if line == end.line { end.col } else { line_len };
    let includes_line_break = line < end.line;
    (start_col < end_col || includes_line_break).then_some(EditorLineSelection {
        start_col: start_col.min(line_len),
        end_col: end_col.min(line_len),
        includes_line_break,
    })
}

fn bounds_for_position(
    view: &BufferView,
    position: Position,
    bounds: Bounds<gpui::Pixels>,
    measured: Option<&EditorMeasuredLayout>,
    appearance: &EditorAppearance,
) -> Bounds<gpui::Pixels> {
    if let Some(line) = measured.and_then(|layout| {
        layout
            .lines
            .iter()
            .find(|line| line.source_line == position.line)
    }) {
        let visible_col = position.col.saturating_sub(view.scroll_col);
        let byte_index = byte_index_for_char_col(&line.visible_text, visible_col);
        let x = line.shaped.x_for_index(byte_index.min(line.shaped.len()));
        return Bounds::new(
            gpui::point(
                bounds.left() + appearance.line_number_width + x,
                bounds.top()
                    + appearance.vertical_padding
                    + appearance.line_height
                        * position.line.saturating_sub(view.scroll_line) as f32,
            ),
            size(px(2.0), appearance.line_height),
        );
    }

    let visible_col = position.col.saturating_sub(view.scroll_col);
    Bounds::new(
        gpui::point(
            bounds.left()
                + appearance.line_number_width
                + appearance.char_width * visible_col as f32,
            bounds.top()
                + appearance.vertical_padding
                + appearance.line_height * position.line.saturating_sub(view.scroll_line) as f32,
        ),
        size(px(2.0), appearance.line_height),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_selection_includes_empty_intermediate_line() {
        let selection = selection_for_line(Position::new(0, 3), Position::new(2, 4), 1, "")
            .expect("empty line between selection endpoints should still be painted");

        assert_eq!(
            selection,
            EditorLineSelection {
                start_col: 0,
                end_col: 0,
                includes_line_break: true,
            }
        );
    }

    #[test]
    fn line_selection_marks_newline_after_selected_line() {
        let selection = selection_for_line(Position::new(0, 2), Position::new(1, 0), 0, "alpha")
            .expect("selection ending at next line start should paint the newline edge");

        assert_eq!(
            selection,
            EditorLineSelection {
                start_col: 2,
                end_col: 5,
                includes_line_break: true,
            }
        );
        assert!(selection_for_line(Position::new(0, 2), Position::new(1, 0), 1, "beta").is_none());
    }

    #[test]
    fn drag_scroll_delta_uses_edges_only() {
        let appearance = test_appearance();
        assert_eq!(
            drag_scroll_delta_for_local_y(px(20.0), px(300.0), &appearance),
            0
        );
        assert!(drag_scroll_delta_for_local_y(px(-1.0), px(300.0), &appearance) < 0);
        assert!(drag_scroll_delta_for_local_y(px(301.0), px(300.0), &appearance) > 0);
    }

    #[test]
    fn horizontal_drag_scroll_delta_uses_text_edges() {
        let appearance = test_appearance();
        assert_eq!(
            drag_scroll_delta_for_local_x(px(90.0), px(500.0), &appearance),
            0
        );
        assert!(drag_scroll_delta_for_local_x(px(60.0), px(500.0), &appearance) < 0);
        assert!(drag_scroll_delta_for_local_x(px(500.0), px(500.0), &appearance) > 0);
    }

    #[test]
    fn scroll_col_delta_clamps_to_longest_line() {
        assert_eq!(scroll_col_by_delta(0, 140, 80, 25), 25);
        assert_eq!(scroll_col_by_delta(25, 140, 80, 100), 61);
        assert_eq!(scroll_col_by_delta(25, 140, 80, -100), 0);
        assert_eq!(scroll_col_by_delta(25, 40, 80, 100), 0);
    }

    #[test]
    fn parse_go_to_line_returns_zero_indexed_clamped_line() {
        assert_eq!(parse_go_to_line("1", 10), Some(0));
        assert_eq!(parse_go_to_line("10", 10), Some(9));
        assert_eq!(parse_go_to_line("999", 10), Some(9));
        assert_eq!(parse_go_to_line("0", 10), None);
        assert_eq!(parse_go_to_line("", 10), None);
        assert_eq!(parse_go_to_line("2", 0), None);
    }

    #[test]
    fn cursor_blink_is_visible_at_start_of_cycle() {
        assert!(cursor_visible_for_elapsed(Duration::from_millis(0)));
        assert!(cursor_visible_for_elapsed(Duration::from_millis(529)));
        assert!(!cursor_visible_for_elapsed(Duration::from_millis(530)));
        assert!(cursor_visible_for_elapsed(Duration::from_millis(1060)));
    }

    #[test]
    fn editor_appearance_uses_editor_specific_overrides() {
        let mut config = Config::default();
        config.font_size = 18.0;
        config.editor.font_size = Some(15.0);
        config.editor.line_height = 1.5;
        config.editor.show_line_numbers = false;
        config.editor.highlight_current_line = false;
        config.cursor_style = ConfigCursorStyle::Underline;
        config.colors.foreground = [10, 20, 30];
        config.colors.background = [1, 2, 3];

        let appearance = EditorAppearanceConfig::from_config(&config).for_language(None);

        assert_eq!(appearance.font_size, px(15.0));
        assert_eq!(appearance.line_height, px(22.5));
        assert_eq!(appearance.line_number_width, px(0.0));
        assert!(!appearance.show_line_numbers);
        assert!(!appearance.highlight_current_line);
        assert_eq!(appearance.cursor_style, ConfigCursorStyle::Underline);
        assert_eq!(appearance.foreground, [10, 20, 30]);
        assert_eq!(appearance.background, [1, 2, 3]);
    }

    #[test]
    fn refresh_active_syntax_skips_large_files() {
        let path = temp_rust_file("gpui-large-syntax.rs", "fn main() {}\n");
        let mut editor = EditorState::new();
        let buffer_id = editor.open(path.clone()).unwrap();
        let idx = editor.index_for_id(buffer_id).unwrap();
        let source = editor.buffers[idx].text();
        editor.views[idx].tree = editor.syntax.parse("rust", &source);
        assert!(editor.views[idx].tree.is_some());

        let extra_lines = "let value = 1;\n".repeat(crate::editor::perf::SYNTAX_LINE_LIMIT + 1);
        editor.buffers[idx].insert(Position::new(1, 0), &extra_lines);
        editor.views[idx].tree_dirty = true;

        refresh_active_syntax(&mut editor);

        assert!(editor.views[idx].tree.is_none());
        assert!(editor.views[idx].folded_ranges.is_empty());
        assert!(editor.views[idx].tree_dirty);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn recently_closed_paths_are_deduped_and_capped() {
        let mut recent = Vec::new();
        for idx in 0..(RECENTLY_CLOSED_LIMIT + 2) {
            remember_recently_closed_path(&mut recent, PathBuf::from(format!("/tmp/file-{idx}")));
        }
        remember_recently_closed_path(&mut recent, PathBuf::from("/tmp/file-4"));

        assert_eq!(recent.len(), RECENTLY_CLOSED_LIMIT);
        assert_eq!(recent.last(), Some(&PathBuf::from("/tmp/file-4")));
        assert_eq!(
            recent
                .iter()
                .filter(|path| **path == PathBuf::from("/tmp/file-4"))
                .count(),
            1
        );
    }

    #[test]
    fn reopen_candidate_skips_open_and_missing_paths() {
        let dir = test_temp_dir("gpui-reopen-candidate");
        let open = dir.join("open.txt");
        let missing = dir.join("missing.txt");
        let closed = dir.join("closed.txt");
        fs::write(&open, "open").unwrap();
        fs::write(&closed, "closed").unwrap();

        let mut recent = vec![closed.clone(), missing, open.clone()];
        let open_paths = HashSet::from([open]);

        assert_eq!(pop_reopen_candidate(&mut recent, &open_paths), Some(closed));
        assert!(recent.is_empty());
    }

    #[test]
    fn lifecycle_close_helpers_skip_modified_buffers() {
        let dir = test_temp_dir("gpui-close-helpers");
        let clean = dir.join("clean.txt");
        let dirty = dir.join("dirty.txt");
        let active = dir.join("active.txt");
        fs::write(&clean, "clean").unwrap();
        fs::write(&dirty, "dirty").unwrap();
        fs::write(&active, "active").unwrap();

        let mut editor = EditorState::new();
        let clean_id = editor.open(clean).unwrap();
        let dirty_id = editor.open(dirty).unwrap();
        let active_id = editor.open(active).unwrap();
        let dirty_index = editor.index_for_id(dirty_id).unwrap();
        editor.buffers[dirty_index].insert(Position::new(0, 0), "changed ");

        assert_eq!(
            closable_other_buffer_ids(&editor, active_id),
            vec![clean_id]
        );
        assert_eq!(
            closable_saved_buffer_ids(&editor),
            vec![clean_id, active_id]
        );
    }

    #[test]
    fn diagnostic_for_line_prefers_highest_severity_on_that_line() {
        let diagnostics = vec![
            EditorDiagnosticSnapshot {
                line: 3,
                col: 8,
                end_line: 3,
                end_col: 15,
                severity: DiagSeverity::Warning,
                message: "warning".into(),
            },
            EditorDiagnosticSnapshot {
                line: 3,
                col: 2,
                end_line: 3,
                end_col: 6,
                severity: DiagSeverity::Error,
                message: "error".into(),
            },
            EditorDiagnosticSnapshot {
                line: 4,
                col: 0,
                end_line: 4,
                end_col: 4,
                severity: DiagSeverity::Hint,
                message: "hint".into(),
            },
        ];

        let diagnostic = diagnostic_for_line(&diagnostics, 3).unwrap();
        assert_eq!(diagnostic.severity, DiagSeverity::Error);
        assert_eq!(diagnostic.message, "error");
        assert!(diagnostic_for_line(&diagnostics, 9).is_none());
    }

    #[test]
    fn diagnostic_line_range_clips_to_visible_columns() {
        let diagnostic = EditorDiagnosticSnapshot {
            line: 2,
            col: 4,
            end_line: 2,
            end_col: 12,
            severity: DiagSeverity::Warning,
            message: "range".into(),
        };

        assert_eq!(
            diagnostic_line_range(&diagnostic, 2, 20, 0, 80),
            Some(EditorDiagnosticLineRange {
                start_col: 4,
                end_col: 12,
                severity: DiagSeverity::Warning,
            })
        );
        assert_eq!(
            diagnostic_line_range(&diagnostic, 2, 20, 8, 4),
            Some(EditorDiagnosticLineRange {
                start_col: 0,
                end_col: 4,
                severity: DiagSeverity::Warning,
            })
        );
        assert!(diagnostic_line_range(&diagnostic, 2, 20, 13, 4).is_none());
    }

    #[test]
    fn diagnostic_for_line_includes_multiline_diagnostics() {
        let diagnostics = vec![EditorDiagnosticSnapshot {
            line: 1,
            col: 5,
            end_line: 3,
            end_col: 2,
            severity: DiagSeverity::Info,
            message: "multi".into(),
        }];

        assert!(diagnostic_for_line(&diagnostics, 1).is_some());
        assert!(diagnostic_for_line(&diagnostics, 2).is_some());
        assert!(diagnostic_for_line(&diagnostics, 3).is_some());
        assert!(diagnostic_for_line(&diagnostics, 4).is_none());
    }

    #[test]
    fn diagnostic_at_position_prefers_highest_severity() {
        let diagnostics = vec![
            EditorDiagnosticSnapshot {
                line: 1,
                col: 0,
                end_line: 1,
                end_col: 10,
                severity: DiagSeverity::Info,
                message: "info".into(),
            },
            EditorDiagnosticSnapshot {
                line: 1,
                col: 2,
                end_line: 1,
                end_col: 4,
                severity: DiagSeverity::Error,
                message: "error".into(),
            },
        ];

        let diagnostic = diagnostic_at_position(&diagnostics, Position::new(1, 3)).unwrap();
        assert_eq!(diagnostic.severity, DiagSeverity::Error);
        assert!(diagnostic_at_position(&diagnostics, Position::new(1, 10)).is_none());
    }

    #[test]
    fn lsp_change_kind_uses_incremental_until_edits_coalesce() {
        let edit = BufferEdit {
            start: Position::new(1, 2),
            old_end: Position::new(1, 2),
            new_end: Position::new(1, 5),
            new_text: "abc".into(),
        };

        assert_eq!(
            next_lsp_change_kind(None, Some(edit.clone())),
            GpuiPendingLspChangeKind::Incremental {
                start: Position::new(1, 2),
                old_end: Position::new(1, 2),
                new_text: "abc".into(),
            }
        );
        assert_eq!(
            next_lsp_change_kind(
                Some(&GpuiPendingLspChangeKind::Incremental {
                    start: edit.start,
                    old_end: edit.old_end,
                    new_text: edit.new_text.clone(),
                }),
                Some(edit)
            ),
            GpuiPendingLspChangeKind::Full
        );
    }

    #[test]
    fn reveal_cursor_does_not_jump_when_cursor_is_visible() {
        let mut view = BufferView::default();
        view.scroll_line = 10;
        view.scroll_col = 20;
        view.cursor.pos = Position::new(15, 30);

        reveal_cursor(&mut view, 100, 80);

        assert_eq!(view.scroll_line, 10);
        assert_eq!(view.scroll_col, 20);
    }

    #[test]
    fn reveal_cursor_scrolls_minimally_to_cursor() {
        let mut view = BufferView::default();
        view.scroll_line = 10;
        view.scroll_col = 20;
        view.cursor.pos = Position::new(100, 150);

        reveal_cursor(&mut view, 200, 80);

        assert_eq!(
            view.scroll_line,
            100usize.saturating_sub(VISIBLE_LINE_LIMIT - 1)
        );
        assert_eq!(view.scroll_col, 71);
    }

    #[test]
    fn lsp_panel_helpers_keep_items_compact() {
        let truncated = truncate_panel_text("abcdefghijklmnopqrstuvwxyz".to_string(), 12);
        assert_eq!(truncated, "abcdefghi...");

        let completions = completion_panel_items(
            vec![
                CompletionItem {
                    label: "render".into(),
                    detail: Some("fn()".into()),
                    insert_text: Some("render()".into()),
                    kind: None,
                },
                CompletionItem {
                    label: "ignored".into(),
                    detail: None,
                    insert_text: None,
                    kind: None,
                },
            ],
            1,
        );
        assert_eq!(
            completions
                .iter()
                .map(|item| item.label.as_str())
                .collect::<Vec<_>>(),
            vec!["render  fn()"]
        );
        assert!(matches!(
            &completions[0].action,
            GpuiLspPanelAction::Complete { text } if text == "render()"
        ));

        let references = references_panel_items(
            vec![ReferenceLocation {
                path: PathBuf::from("/tmp/app.rs"),
                line: 1,
                col: 2,
                context: "  render();  ".into(),
            }],
            1,
        );
        assert_eq!(
            references
                .iter()
                .map(|item| item.label.as_str())
                .collect::<Vec<_>>(),
            vec!["app.rs:2:3  render();"]
        );
        assert!(matches!(
            &references[0].action,
            GpuiLspPanelAction::GoTo { path, line: 1, col: 2 } if path == &PathBuf::from("/tmp/app.rs")
        ));

        let code_actions = code_action_panel_items(
            vec![CodeAction {
                title: "Apply fix".into(),
                edits: vec![(
                    PathBuf::from("/tmp/app.rs"),
                    vec![FormatEdit {
                        start_line: 0,
                        start_col: 0,
                        end_line: 0,
                        end_col: 0,
                        new_text: "fixed".into(),
                    }],
                )],
            }],
            1,
        );
        assert_eq!(code_actions[0].label, "Apply fix");
        assert!(matches!(
            &code_actions[0].action,
            GpuiLspPanelAction::ApplyWorkspaceEdit { edits } if edits.len() == 1
        ));
    }

    #[test]
    fn lsp_panel_selects_first_actionable_item() {
        let panel = lsp_panel(
            "Mixed",
            vec![
                GpuiLspPanelItem::plain("Header".into()),
                GpuiLspPanelItem {
                    label: "Action".into(),
                    action: GpuiLspPanelAction::Complete {
                        text: "done".into(),
                    },
                },
            ],
        );

        assert_eq!(panel.selected, 1);
    }

    #[test]
    fn snippet_insert_text_is_sanitized_for_plain_insert() {
        assert_eq!(
            sanitize_lsp_insert_text("println!(\"${1:value}\");$0"),
            "println!(\"value\");"
        );
        assert_eq!(sanitize_lsp_insert_text("$1name"), "name");
    }

    #[test]
    fn format_edits_apply_to_unopened_file_text() {
        let edits = vec![FormatEdit {
            start_line: 0,
            start_col: 3,
            end_line: 1,
            end_col: 2,
            new_text: "X".into(),
        }];

        let (text, applied) = apply_format_edits_to_text("abc\ndef\n", &edits).unwrap();

        assert_eq!(applied, 1);
        assert_eq!(text, "abcXf\n");
    }

    #[test]
    fn text_position_to_byte_index_handles_trailing_empty_line() {
        assert_eq!(text_position_to_byte_index("abc\n", 1, 0), Some(4));
        assert_eq!(text_position_to_byte_index("", 0, 0), Some(0));
        assert_eq!(text_position_to_byte_index("abc", 2, 0), None);
    }

    fn test_temp_dir(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("{name}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn test_appearance() -> EditorAppearance {
        EditorAppearanceConfig::default().for_language(None)
    }

    fn temp_rust_file(name: &str, contents: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("{name}-{unique}"));
        fs::write(&path, contents).unwrap();
        path
    }
}
