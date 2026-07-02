use std::collections::HashMap;
#[cfg(test)]
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::config::{Config, CursorStyle as ConfigCursorStyle, EditorConfig};
use crate::editor::buffer::{Buffer, Position};
use crate::editor::perf;
use crate::editor::search::EditorSearch;
use crate::editor::syntax::{group_color_with_overrides, HighlightGroup, HighlightSpan};
use crate::editor::{BufferId, BufferView, EditorState, MarkdownViewMode};
use crate::lsp::{DiagSeverity, LspManager};
use crate::path_utils::{path_extension_matches, PREVIEW_IMAGE_EXTS};
use crate::stacker::utf16::{char_index_to_utf16_index, utf16_index_to_char_index};
use gpui::prelude::*;
use gpui::{
    actions, div, font, px, relative, rgb, rgba, size, App, Application, Bounds, Context,
    CursorStyle, Element, ElementId, ElementInputHandler, Entity, EntityInputHandler, FocusHandle,
    Focusable, GlobalElementId, KeyBinding, KeyDownEvent, LayoutId, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, Render, ScrollHandle, ScrollWheelEvent, ShapedLine,
    SharedString, Style, TextRun, UTF16Selection, Window, WindowBounds, WindowOptions,
};
use rustc_hash::FxHashMap;

mod commands;
mod file_state;
mod files;
mod input;
mod key_actions;
mod line_render;
mod lsp;
mod render;
mod search;

pub(crate) use key_actions::bind_editor_keys;

use commands::{
    byte_index_for_char_col, EditorCommand, EditorDeleteTarget, EditorLineMove, EditorMotion,
    EditorSelectTarget,
};
use file_state::read_normalized_file_text;
use files::initial_path;
use input::{
    reveal_cursor, visible_col_limit_for_bounds, visible_line_limit_for_bounds,
    wrapped_visual_rows, EditorInputElement, EditorMeasuredLayout, WrappedVisualRow,
};
use line_render::{skip_chars, EditorSearchLineMatch};
use lsp::*;
use search::{search_matches_for_line, EditorSearchDirection, EditorSearchInputTarget};

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
        let window = match cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(EditorPrototype::standalone),
        ) {
            Ok(window) => window,
            Err(error) => {
                log::error!("failed to open editor window: {error:?}");
                cx.quit();
                return;
            }
        };
        if let Err(error) = window.update(cx, |view, window, cx| {
            window.focus(&view.focus_handle(cx));
        }) {
            log::error!("failed to focus editor window: {error:?}");
            cx.quit();
            return;
        }
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.activate(true);
    });
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
    image_preview: Option<EditorImagePreview>,
    image_preview_active: bool,
    markdown_preview_scroll: ScrollHandle,
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
    scroll_line_remainder: f32,
    scroll_col_remainder: f32,
    status_message: Option<String>,
    last_text_bounds: Option<Bounds<gpui::Pixels>>,
    last_text_layout: Option<EditorMeasuredLayout>,
    /// Average glyph advance measured from the real shaped font, replacing
    /// the 0.6-em estimate in `EditorAppearance::char_width` once known.
    /// Keyed by font family + size so appearance changes re-measure.
    measured_char_width: Option<MeasuredCharWidth>,
    is_selecting: bool,
    cursor_blink_anchor: Instant,
    cursor_blink_visible: bool,
    /// Whether the most recent `snapshot()` saw the editor as focused.
    /// Used to skip cursor-blink updates and their accompanying `notify`
    /// cascade for blurred editors (the cursor is invisible when unfocused
    /// regardless of `cursor_blink_visible`, so toggling it produces a
    /// redundant render).
    last_render_was_focused: bool,
    show_chrome: bool,
    /// Region of the active buffer currently held as IME composition /
    /// dictation preview. `None` outside an active composition. Set by
    /// `replace_and_mark_text_in_range`, cleared by `unmark_text` or any
    /// non-IME mutation routed through `edit_active`.
    marked_range: Option<(Position, Position)>,
}

#[derive(Clone)]
struct EditorImagePreview {
    path: PathBuf,
    dimensions: Option<(u32, u32)>,
    file_size: Option<u64>,
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
    syntax_colors: Arc<FxHashMap<HighlightGroup, [u8; 3]>>,
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
            syntax_colors: Arc::new(config.syntax_colors.clone()),
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
            word_wrap: effective.word_wrap,
            rulers: effective.rulers,
            markdown_preview_style: self.editor.markdown_preview_style,
            syntax_colors: Arc::clone(&self.syntax_colors),
        }
    }
}

impl Default for EditorAppearanceConfig {
    fn default() -> Self {
        Self::from_config(&Config::default())
    }
}

/// A shaped-text measurement of the average glyph advance for one font
/// family + size combination.
#[derive(Clone, PartialEq)]
pub(super) struct MeasuredCharWidth {
    pub(super) font_family: String,
    pub(super) font_size: gpui::Pixels,
    pub(super) width: gpui::Pixels,
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
    word_wrap: bool,
    rulers: Vec<usize>,
    markdown_preview_style: crate::config::MarkdownPreviewStyle,
    syntax_colors: Arc<FxHashMap<HighlightGroup, [u8; 3]>>,
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

    /// Body ink softened toward the background — the newspaper two-tone
    /// convention (NYT: #363636 body under #121212 headlines) expressed
    /// relative to the active theme.
    fn preview_soft_foreground(&self) -> gpui::Rgba {
        rgb(rgb_u32(mix_rgb(self.foreground, self.background, 0.15)))
    }

    /// Hairline rule ink derived from the theme (stands in for the print
    /// #DFDFDF hairlines on white).
    fn preview_rule_color(&self) -> gpui::Rgba {
        rgb(rgb_u32(mix_rgb(self.background, self.foreground, 0.25)))
    }

    fn preview_code_background(&self) -> gpui::Rgba {
        rgb(rgb_u32(mix_rgb(self.background, self.foreground, 0.05)))
    }

    fn preview_code_border(&self) -> gpui::Rgba {
        rgb(rgb_u32(mix_rgb(self.background, self.foreground, 0.18)))
    }
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

#[derive(Clone)]
struct ExternalFileChange {
    buffer_id: BufferId,
    path: PathBuf,
}

#[derive(Clone)]
struct ExternalFileChangeSnapshot {
    file_name: String,
}

impl EditorPrototype {
    #[cfg(feature = "gpui-workspace")]
    pub(crate) fn new(cx: &mut Context<Self>) -> Self {
        Self::with_chrome(cx, false)
    }

    #[cfg(feature = "gpui-workspace")]
    pub(crate) fn file_tab(cx: &mut Context<Self>) -> Self {
        Self::with_chrome_and_initial_file(cx, false, false)
    }

    pub(crate) fn standalone(cx: &mut Context<Self>) -> Self {
        Self::with_chrome(cx, true)
    }

    #[cfg(feature = "gpui-workspace")]
    pub(crate) fn copy_selection_to_clipboard(&mut self, cx: &mut Context<Self>) {
        if self.image_preview_active {
            self.dispatch_editor_command(EditorCommand::Copy, cx);
            return;
        }
        self.copy_selection_or_line(cx);
    }

    #[cfg(feature = "gpui-workspace")]
    pub(crate) fn select_all_text(&mut self, cx: &mut Context<Self>) {
        if self.image_preview_active {
            self.status_message = Some("Image previews are read-only".to_string());
            cx.notify();
            return;
        }
        if self.go_to_line_active || self.editor_search.active || self.rename_active {
            self.status_message = Some("Close the active editor input first".to_string());
            cx.notify();
            return;
        }
        let visible_cols = self.visible_col_limit();
        let visible_lines = self.visible_line_limit();
        let word_wrap = self.active_appearance().word_wrap;
        if let Some((buffer, view)) = self.active_buffer_and_view() {
            view.cursor.select_all(buffer);
            reveal_cursor(view, buffer, visible_cols, visible_lines, word_wrap);
            cx.notify();
        } else {
            self.status_message = Some("No active buffer to select".to_string());
            cx.notify();
        }
    }

    #[cfg(feature = "gpui-workspace")]
    pub(crate) fn paste_from_workspace(&mut self, cx: &mut Context<Self>) {
        self.paste_from_clipboard_respecting_editor_overlay(cx);
    }

    pub(crate) fn cycle_markdown_preview_from_workspace(&mut self, cx: &mut Context<Self>) {
        self.cycle_markdown_preview(cx);
    }

    pub(crate) fn set_markdown_mode_from_workspace(
        &mut self,
        mode: MarkdownViewMode,
        cx: &mut Context<Self>,
    ) {
        self.set_markdown_mode(mode, cx);
    }

    pub(crate) fn check_active_external_change_from_workspace(&mut self, cx: &mut Context<Self>) {
        self.check_active_external_change(cx);
    }

    pub(crate) fn request_lsp_hover_from_workspace(&mut self, cx: &mut Context<Self>) {
        self.request_lsp_hover(cx);
    }

    pub(crate) fn request_lsp_completion_from_workspace(&mut self, cx: &mut Context<Self>) {
        self.request_lsp_completion(cx);
    }

    pub(crate) fn request_lsp_definition_from_workspace(&mut self, cx: &mut Context<Self>) {
        self.request_lsp_definition(cx);
    }

    pub(crate) fn request_lsp_references_from_workspace(&mut self, cx: &mut Context<Self>) {
        self.request_lsp_references(cx);
    }

    pub(crate) fn request_lsp_signature_help_from_workspace(&mut self, cx: &mut Context<Self>) {
        self.request_lsp_signature_help(cx);
    }

    pub(crate) fn open_lsp_rename_from_workspace(&mut self, cx: &mut Context<Self>) {
        self.open_lsp_rename(cx);
    }

    pub(crate) fn request_lsp_code_actions_from_workspace(&mut self, cx: &mut Context<Self>) {
        self.request_lsp_code_actions(cx);
    }

    pub(crate) fn request_lsp_format_from_workspace(&mut self, cx: &mut Context<Self>) {
        self.request_lsp_format(cx);
    }

    pub(crate) fn request_lsp_symbols_from_workspace(&mut self, cx: &mut Context<Self>) {
        self.request_lsp_symbols(cx);
    }

    pub(crate) fn close_other_buffer_tabs_from_workspace(&mut self, cx: &mut Context<Self>) {
        self.close_other_buffer_tabs(cx);
    }

    pub(crate) fn close_saved_buffer_tabs_from_workspace(&mut self, cx: &mut Context<Self>) {
        self.close_saved_buffer_tabs(cx);
    }

    pub(crate) fn reopen_recent_buffer_tab_from_workspace(&mut self, cx: &mut Context<Self>) {
        self.reopen_recent_buffer_tab(cx);
    }

    /// Move the active buffer's cursor to `(line, col)` (zero-indexed) and
    /// scroll it into view. Used by callers outside the editor — e.g. the
    /// error-log panel jumping to a logged source location — that have
    /// already opened the target file in this editor.
    pub(crate) fn navigate_to_position_from_workspace(
        &mut self,
        line: u32,
        col: u32,
        cx: &mut Context<Self>,
    ) {
        let visible_cols = self.visible_col_limit();
        let visible_lines = self.visible_line_limit();
        let word_wrap = self.active_appearance().word_wrap;
        if let Some((buffer, view)) = self.active_buffer_and_view() {
            let line = line as usize;
            let col = col as usize;
            let line_count = buffer.line_count();
            let target_line = line.min(line_count.saturating_sub(1));
            let target_col = col.min(buffer.line_len(target_line));
            view.cursor.pos = Position::new(target_line, target_col);
            view.cursor.clear_selection();
            view.cursor.desired_col = None;
            reveal_cursor(view, buffer, visible_cols, visible_lines, word_wrap);
            self.wake_cursor_blink();
            cx.notify();
        }
    }

    fn with_chrome(cx: &mut Context<Self>, show_chrome: bool) -> Self {
        Self::with_chrome_and_initial_file(cx, show_chrome, true)
    }

    fn with_chrome_and_initial_file(
        cx: &mut Context<Self>,
        show_chrome: bool,
        load_initial_file: bool,
    ) -> Self {
        start_cursor_blink_task(cx);
        start_lsp_poll_task(cx);
        let mut editor = EditorState::new();
        let mut last_seen_disk_text = HashMap::new();
        let load_error = open_initial_file_if_requested(
            &mut editor,
            &mut last_seen_disk_text,
            load_initial_file,
        );

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
            image_preview: None,
            image_preview_active: false,
            markdown_preview_scroll: ScrollHandle::new(),
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
            scroll_line_remainder: 0.0,
            scroll_col_remainder: 0.0,
            status_message: None,
            last_text_bounds: None,
            last_text_layout: None,
            measured_char_width: None,
            is_selecting: false,
            cursor_blink_anchor: Instant::now(),
            cursor_blink_visible: true,
            last_render_was_focused: false,
            show_chrome,
            marked_range: None,
        };
        this.open_all_file_backed_buffers_with_lsp();
        this
    }

    pub(crate) fn set_appearance_config(&mut self, config: Config, cx: &mut Context<Self>) {
        self.appearance_config = EditorAppearanceConfig::from_config(&config);
        self.last_text_layout = None;
        self.scroll_line_remainder = 0.0;
        self.scroll_col_remainder = 0.0;
        let visible_cols = self.visible_col_limit();
        let visible_lines = self.visible_line_limit();
        let appearance_config = self.appearance_config.clone();
        for (buffer, view) in self.editor.buffers.iter().zip(self.editor.views.iter_mut()) {
            if appearance_config.for_language(view.lang_id).word_wrap {
                reveal_cursor(view, buffer, visible_cols, visible_lines, true);
            } else {
                view.wrap_scroll_row = view.scroll_line;
            }
        }
        cx.notify();
    }

    fn snapshot(&mut self, is_focused: bool) -> EditorSnapshot {
        self.last_render_was_focused = is_focused;
        refresh_active_syntax(&mut self.editor);
        if self.editor_search.active {
            let active = self.editor.active;
            if let Some(buffer) = self.editor.buffers.get(active) {
                self.editor_search.update_if_dirty(buffer);
            }
        }
        let image_preview = self.image_preview.clone();
        let image_preview_active = self.image_preview_active && image_preview.is_some();
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
        if let Some(preview) = image_preview.filter(|_| image_preview_active) {
            let appearance = self.appearance_config.for_language(None);
            return EditorSnapshot {
                title: preview.path.display().to_string(),
                subtitle: image_subtitle(&preview),
                language: "image".to_string(),
                lines: Vec::new(),
                first_line_number: 0,
                cursor: None,
                cursor_visible: false,
                selection: None,
                scroll_col: 0,
                total_lines: 0,
                total_chars: 0,
                modified: false,
                load_error: self.load_error.clone(),
                status_message: self.status_message.clone(),
                image_preview: Some(EditorImagePreviewSnapshot {
                    path: preview.path,
                    dimensions: preview.dimensions,
                    file_size: preview.file_size,
                }),
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
                external_change: None,
                lsp_status: String::new(),
                diagnostics: Vec::new(),
                lsp_panel: None,
                degraded_notice: None,
                cursor_diagnostic_message: None,
                sample: false,
                markdown: false,
                markdown_mode: MarkdownViewMode::Source,
                markdown_preview_scroll: self.markdown_preview_scroll.clone(),
                markdown_preview: None,
                appearance,
            };
        }

        if let Some((buffer_id, buffer, view)) = self.editor.active_buffer_view() {
            let mut appearance = self.appearance_config.for_language(view.lang_id);
            self.apply_measured_char_width(&mut appearance);
            let markdown = buffer.path().is_some_and(is_markdown_path);
            let markdown_mode = if markdown {
                view.markdown_mode
            } else {
                MarkdownViewMode::Source
            };
            let want_markdown_preview = markdown && markdown_mode != MarkdownViewMode::Source;
            let want_highlights = view.lang_id.is_some() && view.tree.is_some();
            let buffer_text = (want_markdown_preview || want_highlights).then(|| buffer.text());
            let markdown_preview = want_markdown_preview
                .then(|| markdown_preview_blocks(buffer_text.as_deref().unwrap_or_default()));
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
            let visible_lines = self.visible_line_limit();
            let visible_cols = self.visible_col_limit().max(1);
            let fallback_visible_start = view.scroll_line.min(line_count.saturating_sub(1));
            let rows: Vec<WrappedVisualRow> = if appearance.word_wrap {
                wrapped_visual_rows(buffer, view.wrap_scroll_row, visible_lines, visible_cols)
            } else {
                let visible_end = line_count.min(fallback_visible_start + visible_lines);
                (fallback_visible_start..visible_end)
                    .map(|source_line| WrappedVisualRow {
                        source_line,
                        wrap_start_col: view.scroll_col,
                        wrap_cols: visible_cols,
                        show_line_number: true,
                    })
                    .collect()
            };
            let visible_start = rows
                .first()
                .map(|row| row.source_line)
                .unwrap_or(fallback_visible_start);
            let visible_end = rows
                .last()
                .map(|row| row.source_line + 1)
                .unwrap_or(visible_start);
            let highlight_spans = match (view.lang_id, view.tree.as_ref(), buffer_text.as_deref()) {
                (Some(lang_id), Some(tree), Some(text)) => self.editor.syntax.highlights_for_range(
                    lang_id,
                    tree,
                    text.as_bytes(),
                    visible_start,
                    visible_end,
                ),
                _ => vec![Vec::new(); visible_end.saturating_sub(visible_start)],
            };
            let lines = rows
                .into_iter()
                .map(|row| {
                    let highlight_idx = row.source_line.saturating_sub(visible_start);
                    EditorLineSnapshot {
                        number: row.source_line + 1,
                        text: buffer.line(row.source_line).to_string(),
                        wrap_start_col: row.wrap_start_col,
                        wrap_cols: appearance.word_wrap.then_some(row.wrap_cols),
                        show_line_number: row.show_line_number,
                        highlights: highlight_spans
                            .get(highlight_idx)
                            .cloned()
                            .unwrap_or_default(),
                        search_matches: search_matches_for_line(
                            &self.editor_search,
                            row.source_line,
                        ),
                        diagnostic: row
                            .show_line_number
                            .then(|| diagnostic_for_line(&diagnostics, row.source_line))
                            .flatten(),
                    }
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
                scroll_col: if appearance.word_wrap {
                    0
                } else {
                    view.scroll_col
                },
                total_lines: line_count,
                total_chars: buffer.len_chars(),
                modified: buffer.is_modified(),
                load_error: self.load_error.clone(),
                status_message: self.status_message.clone(),
                image_preview: None,
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
                external_change,
                lsp_status,
                diagnostics,
                lsp_panel,
                degraded_notice,
                cursor_diagnostic_message,
                sample: false,
                markdown,
                markdown_mode,
                markdown_preview_scroll: self.markdown_preview_scroll.clone(),
                markdown_preview,
                appearance,
            };
        }

        let appearance = self.appearance_config.for_language(None);
        let lsp_panel = self.lsp_panel.clone();
        // The editor model does not expose a public buffer constructor from
        // text, so the no-file fallback remains display-only.
        EditorSnapshot {
            title: "Sample GPUI editor buffer".to_string(),
            subtitle: "display-only fallback, EditorState has no opened file".to_string(),
            language: "plain text".to_string(),
            lines: self
                .sample_text
                .lines()
                .skip(self.sample_scroll_line)
                .take(self.visible_line_limit())
                .enumerate()
                .map(|(idx, line)| EditorLineSnapshot {
                    number: self.sample_scroll_line + idx + 1,
                    text: line.to_string(),
                    wrap_start_col: 0,
                    wrap_cols: None,
                    show_line_number: true,
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
            total_chars: self.sample_text.chars().count(),
            modified: false,
            load_error: self.load_error.clone(),
            status_message: self.status_message.clone(),
            image_preview: None,
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
            external_change,
            lsp_status,
            diagnostics,
            lsp_panel,
            degraded_notice: None,
            cursor_diagnostic_message: None,
            sample: true,
            markdown: false,
            markdown_mode: MarkdownViewMode::Source,
            markdown_preview_scroll: self.markdown_preview_scroll.clone(),
            markdown_preview: None,
            appearance,
        }
    }

    fn active_appearance(&self) -> EditorAppearance {
        let lang_id = self
            .editor
            .views
            .get(self.editor.active)
            .and_then(|view| view.lang_id);
        let mut appearance = self.appearance_config.for_language(lang_id);
        self.apply_measured_char_width(&mut appearance);
        appearance
    }

    fn apply_measured_char_width(&self, appearance: &mut EditorAppearance) {
        if let Some(measured) = &self.measured_char_width {
            if measured.font_family == appearance.font_family
                && measured.font_size == appearance.font_size
                && measured.width > px(0.0)
            {
                appearance.char_width = measured.width;
            }
        }
    }

    /// Store a freshly measured advance. Returns true when it changes the
    /// effective char width, so the caller can re-render/re-wrap.
    pub(super) fn set_measured_char_width(&mut self, measured: MeasuredCharWidth) -> bool {
        if self.measured_char_width.as_ref() == Some(&measured) {
            return false;
        }
        self.measured_char_width = Some(measured);
        true
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
        let visible_lines = self.visible_line_limit();
        let word_wrap = self.active_appearance().word_wrap;
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
            reveal_cursor(view, buffer, visible_cols, visible_lines, word_wrap);
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
            // Any mutation routed through edit_active invalidates positions
            // tracked by an in-flight IME composition. IME paths that intend
            // to keep the composition alive re-set marked_range after the
            // edit returns.
            self.marked_range = None;
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

    fn visible_line_limit(&self) -> usize {
        let appearance = self.active_appearance();
        self.last_text_bounds
            .map(|bounds| visible_line_limit_for_bounds(bounds, &appearance))
            .unwrap_or(VISIBLE_LINE_LIMIT)
    }

    fn wake_cursor_blink(&mut self) {
        self.cursor_blink_anchor = Instant::now();
        self.cursor_blink_visible = true;
    }

    fn paste_from_clipboard_respecting_editor_overlay(&mut self, cx: &mut Context<Self>) {
        if self.rename_active {
            if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                self.push_lsp_rename_text(&text.replace(['\n', '\r'], ""), cx);
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

    fn refresh_cursor_blink(&mut self) -> bool {
        if !self.last_render_was_focused {
            return false;
        }
        let visible = cursor_visible_for_elapsed(self.cursor_blink_anchor.elapsed());
        if self.cursor_blink_visible == visible {
            return false;
        }
        self.cursor_blink_visible = visible;
        true
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

    fn cycle_markdown_preview(&mut self, cx: &mut Context<Self>) {
        let Some(mode) = self.active_markdown_mode(cx) else {
            return;
        };
        self.set_markdown_mode(mode.cycle(), cx);
    }

    fn set_markdown_mode(&mut self, mode: MarkdownViewMode, cx: &mut Context<Self>) {
        let Some((buffer, view)) = self.active_buffer_and_view_mut() else {
            self.status_message = Some("Markdown preview is available for .md files".to_string());
            cx.notify();
            return;
        };
        if !buffer.path().is_some_and(is_markdown_path) {
            self.status_message = Some("Markdown preview is available for .md files".to_string());
            cx.notify();
            return;
        }
        view.markdown_mode = mode;
        self.status_message = Some(format!(
            "Markdown {}",
            markdown_mode_label(view.markdown_mode)
        ));
        if mode == MarkdownViewMode::Source {
            self.markdown_preview_scroll
                .set_offset(gpui::point(px(0.0), px(0.0)));
        }
        cx.notify();
    }

    fn active_markdown_mode(&mut self, cx: &mut Context<Self>) -> Option<MarkdownViewMode> {
        let Some((buffer, view)) = self.active_buffer_and_view_mut() else {
            self.status_message = Some("Markdown preview is available for .md files".to_string());
            cx.notify();
            return None;
        };
        if !buffer.path().is_some_and(is_markdown_path) {
            self.status_message = Some("Markdown preview is available for .md files".to_string());
            cx.notify();
            return None;
        }
        Some(view.markdown_mode)
    }
}

impl Focusable for EditorPrototype {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

fn language_label(view: &BufferView) -> String {
    view.lang_id.unwrap_or("plain text").to_string()
}

pub(super) fn is_markdown_path(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "md" | "markdown" | "mdown" | "mkd"
            )
        })
}

fn is_preview_image_path(path: &Path) -> bool {
    path_extension_matches(path, PREVIEW_IMAGE_EXTS)
}

fn image_subtitle(preview: &EditorImagePreview) -> String {
    let kind = preview
        .path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_uppercase())
        .unwrap_or_else(|| "Image".to_string());
    let dimensions = preview
        .dimensions
        .map(|(width, height)| format!("{width}x{height}"))
        .unwrap_or_else(|| "dimensions unavailable".to_string());
    let size = preview
        .file_size
        .map(format_file_size)
        .unwrap_or_else(|| "size unavailable".to_string());
    format!("{kind} image, {dimensions}, {size}")
}

fn format_file_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / KB)
    } else {
        format!("{:.1} MB", bytes as f64 / MB)
    }
}

fn markdown_mode_label(mode: MarkdownViewMode) -> &'static str {
    match mode {
        MarkdownViewMode::Source => "Source",
        MarkdownViewMode::Preview => "Preview",
        MarkdownViewMode::Split => "Split",
    }
}

fn markdown_preview_blocks(source: &str) -> Vec<MarkdownPreviewBlock> {
    let mut blocks = Vec::new();
    let mut paragraph = Vec::new();
    let mut code = Vec::new();
    let mut in_code = false;

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            flush_markdown_paragraph(&mut blocks, &mut paragraph);
            if in_code {
                blocks.push(MarkdownPreviewBlock {
                    kind: MarkdownPreviewBlockKind::Code,
                    text: code.join("\n"),
                });
                code.clear();
                in_code = false;
            } else {
                in_code = true;
            }
            continue;
        }

        if in_code {
            code.push(line.to_string());
            continue;
        }

        if trimmed.is_empty() {
            flush_markdown_paragraph(&mut blocks, &mut paragraph);
            continue;
        }

        if let Some((level, text)) = markdown_heading(trimmed) {
            flush_markdown_paragraph(&mut blocks, &mut paragraph);
            blocks.push(MarkdownPreviewBlock {
                kind: MarkdownPreviewBlockKind::Heading(level),
                text: strip_markdown_inline(text),
            });
        } else if let Some(text) = markdown_bullet(trimmed) {
            flush_markdown_paragraph(&mut blocks, &mut paragraph);
            blocks.push(MarkdownPreviewBlock {
                kind: MarkdownPreviewBlockKind::Bullet,
                text: strip_markdown_inline(text),
            });
        } else if let Some(text) = trimmed.strip_prefix("> ") {
            flush_markdown_paragraph(&mut blocks, &mut paragraph);
            blocks.push(MarkdownPreviewBlock {
                kind: MarkdownPreviewBlockKind::Quote,
                text: strip_markdown_inline(text),
            });
        } else {
            paragraph.push(strip_markdown_inline(trimmed));
        }
    }

    if in_code && !code.is_empty() {
        blocks.push(MarkdownPreviewBlock {
            kind: MarkdownPreviewBlockKind::Code,
            text: code.join("\n"),
        });
    }
    flush_markdown_paragraph(&mut blocks, &mut paragraph);

    if blocks.is_empty() {
        blocks.push(MarkdownPreviewBlock {
            kind: MarkdownPreviewBlockKind::Paragraph,
            text: "Empty markdown document".to_string(),
        });
    }

    blocks
}

fn flush_markdown_paragraph(blocks: &mut Vec<MarkdownPreviewBlock>, paragraph: &mut Vec<String>) {
    if paragraph.is_empty() {
        return;
    }
    blocks.push(MarkdownPreviewBlock {
        kind: MarkdownPreviewBlockKind::Paragraph,
        text: paragraph.join(" "),
    });
    paragraph.clear();
}

fn markdown_heading(line: &str) -> Option<(u8, &str)> {
    let level = line.chars().take_while(|ch| *ch == '#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let text = line.get(level..)?.trim_start();
    (!text.is_empty()).then_some((level as u8, text))
}

fn markdown_bullet(line: &str) -> Option<&str> {
    line.strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
        .or_else(|| line.strip_prefix("+ "))
}

fn strip_markdown_inline(text: &str) -> String {
    text.replace(['`', '*', '_'], "")
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

fn open_initial_file_if_requested(
    editor: &mut EditorState,
    last_seen_disk_text: &mut HashMap<PathBuf, String>,
    load_initial_file: bool,
) -> Option<String> {
    if !load_initial_file {
        return None;
    }

    if let Some(path) = initial_path() {
        if let Err(err) = editor.open(path.clone()) {
            Some(format!("{}: {err}", path.display()))
        } else {
            refresh_active_syntax(editor);
            if let Ok(text) = read_normalized_file_text(&path) {
                last_seen_disk_text.insert(path, text);
            }
            None
        }
    } else {
        Some("No readable file found for GPUI editor".to_string())
    }
}

fn sample_text() -> String {
    [
        "LLNZY GPUI editor",
        "",
        "This surface now accepts GPUI text input for opened files.",
        "The production editor model is present, but no file could be opened.",
        "",
        "Next step: move syntax highlighting and richer selection painting over.",
    ]
    .join("\n")
}

struct EditorSnapshot {
    title: String,
    subtitle: String,
    language: String,
    lines: Vec<EditorLineSnapshot>,
    image_preview: Option<EditorImagePreviewSnapshot>,
    first_line_number: usize,
    cursor: Option<Position>,
    cursor_visible: bool,
    selection: Option<(Position, Position)>,
    scroll_col: usize,
    total_lines: usize,
    total_chars: usize,
    modified: bool,
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
    external_change: Option<ExternalFileChangeSnapshot>,
    lsp_status: String,
    diagnostics: Vec<EditorDiagnosticSnapshot>,
    lsp_panel: Option<GpuiLspPanel>,
    degraded_notice: Option<String>,
    cursor_diagnostic_message: Option<String>,
    markdown: bool,
    markdown_mode: MarkdownViewMode,
    markdown_preview_scroll: ScrollHandle,
    markdown_preview: Option<Vec<MarkdownPreviewBlock>>,
    appearance: EditorAppearance,
}

#[derive(Clone)]
struct EditorImagePreviewSnapshot {
    path: PathBuf,
    dimensions: Option<(u32, u32)>,
    file_size: Option<u64>,
}

struct EditorLineSnapshot {
    number: usize,
    text: String,
    wrap_start_col: usize,
    wrap_cols: Option<usize>,
    show_line_number: bool,
    highlights: Vec<HighlightSpan>,
    search_matches: Vec<EditorSearchLineMatch>,
    diagnostic: Option<EditorDiagnosticSnapshot>,
}

#[derive(Clone)]
struct MarkdownPreviewBlock {
    kind: MarkdownPreviewBlockKind,
    text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MarkdownPreviewBlockKind {
    Heading(u8),
    Paragraph,
    Bullet,
    Code,
    Quote,
}

const EDITOR_CHROME_BG: u32 = 0x242424;
const EDITOR_BORDER: u32 = 0x34343c;
const EDITOR_TEXT_FG: u32 = 0xe8e8ee;
const EDITOR_MUTED_FG: u32 = 0x9a9aa7;
const EDITOR_DIM_FG: u32 = 0x70707d;
// Used before GPUI has reported real text bounds for the embedded editor.
// Keep this high enough that the first paint inside the workspace does not
// leave a short 30-line editor floating above an otherwise empty pane.
const VISIBLE_LINE_LIMIT: usize = 48;
const EDITOR_VERTICAL_PADDING: gpui::Pixels = px(12.0);
const DEFAULT_VISIBLE_COL_LIMIT: usize = 96;
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

fn cursor_visible_for_elapsed(elapsed: Duration) -> bool {
    let phase = elapsed.as_millis() / CURSOR_BLINK_INTERVAL.as_millis().max(1);
    phase.is_multiple_of(2)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        config
            .syntax_colors
            .insert(HighlightGroup::Keyword, [90, 80, 70]);

        let appearance = EditorAppearanceConfig::from_config(&config).for_language(None);

        assert_eq!(appearance.font_size, px(15.0));
        assert_eq!(appearance.line_height, px(22.5));
        assert_eq!(appearance.line_number_width, px(0.0));
        assert!(!appearance.show_line_numbers);
        assert!(!appearance.highlight_current_line);
        assert_eq!(appearance.cursor_style, ConfigCursorStyle::Underline);
        assert_eq!(appearance.foreground, [10, 20, 30]);
        assert_eq!(appearance.background, [1, 2, 3]);
        assert_eq!(
            appearance.syntax_colors.get(&HighlightGroup::Keyword),
            Some(&[90, 80, 70])
        );
    }

    #[test]
    fn file_tab_startup_skips_fallback_initial_file() {
        let mut editor = EditorState::new();
        let mut last_seen_disk_text = HashMap::new();

        let load_error =
            open_initial_file_if_requested(&mut editor, &mut last_seen_disk_text, false);

        assert!(load_error.is_none());
        assert!(editor.is_empty());
        assert!(last_seen_disk_text.is_empty());
    }

    #[test]
    fn markdown_preview_blocks_parse_common_blocks() {
        let blocks = markdown_preview_blocks("# Title\n\n- Item\n\n```rust\nfn main() {}\n```");

        assert_eq!(blocks.len(), 3);
        assert!(matches!(
            blocks[0].kind,
            MarkdownPreviewBlockKind::Heading(1)
        ));
        assert_eq!(blocks[0].text, "Title");
        assert_eq!(blocks[1].text, "Item");
        assert!(matches!(blocks[2].kind, MarkdownPreviewBlockKind::Code));
    }

    #[test]
    fn preview_image_detection_matches_repo_images() {
        assert!(is_preview_image_path(Path::new("assets/screenshot.PNG")));
        assert!(is_preview_image_path(Path::new("assets/photo.jpeg")));
        assert!(is_preview_image_path(Path::new("assets/icon.webp")));
        assert!(is_preview_image_path(Path::new("assets/diagram.svg")));
        assert!(!is_preview_image_path(Path::new("src/main.rs")));
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
