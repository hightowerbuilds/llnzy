use rustc_hash::FxHashMap;

use crate::config::EffectiveEditorConfig;
use crate::editor::buffer::{BufferEdit, IndentStyle};
use crate::editor::keymap::{handle_editor_keys, EditorKeymapContext, KeyAction};
use crate::editor::perf;
use crate::editor::search::EditorSearch;
use crate::editor::syntax::{HighlightGroup, SyntaxEngine};
use crate::editor::BufferView;
use crate::keybindings::KeybindingPreset;
use crate::lsp::{CodeLensInfo, CompletionItem, FileDiagnostic, InlayHintInfo, SignatureInfo};

use super::editor_cursor::render_primary_cursor;
use super::editor_folding::{
    apply_folding_actions, snap_cursor_to_visible_line, visible_doc_lines,
    visible_index_for_doc_line,
};
use super::editor_inline_overlays::render_inline_lsp_overlays;
use super::editor_input::handle_editor_pointer_input;
use super::editor_lines::render_editor_lines;
use super::editor_minimap::render_minimap;
use super::editor_paint::{render_bracket_match, render_diagnostics};
use super::editor_search_bar::render_search_bar;
use super::editor_selection::{
    render_extra_cursors, render_primary_selection, render_search_matches,
};
use super::editor_status::editor_status_text;
use super::editor_wrap::{compute_wrap_rows, wrap_row_for_cursor, WrapRow};

/// Result from rendering that the host needs to act on.
#[derive(Default)]
pub(crate) struct EditorFrameResult {
    pub key_action: KeyAction,
    pub buffer_edit: Option<BufferEdit>,
    /// Single galley + screen origin describing the painted prose layout.
    /// Set only when `prose_mode = true`. Consumers (currently the macOS
    /// `LlnzyStackerInputClient`) use this to translate UTF-16 character
    /// ranges to screen rectangles for `firstRectForCharacterRange:` and
    /// the inverse for `characterIndexForPoint:`. The galley is built
    /// with the same `FontId::monospace(font_size)`, `line_height`, and
    /// `wrap.max_width` as the visible per-line render, so glyph
    /// positions match within rounding.
    ///
    /// `None` in code mode and when the prose buffer is empty.
    pub input_anchor: Option<(std::sync::Arc<egui::Galley>, egui::Pos2)>,
}

pub(crate) struct TextEditorState<'a> {
    pub buf: &'a mut crate::editor::buffer::Buffer,
    pub view: &'a mut BufferView,
    pub status_msg: &'a mut Option<String>,
    pub clipboard_out: &'a mut Option<String>,
    pub clipboard_in: &'a mut Option<String>,
    pub editor_search: &'a mut EditorSearch,
}

pub(crate) struct TextEditorInput<'a> {
    pub syntax: &'a SyntaxEngine,
    pub editor_config: &'a EffectiveEditorConfig,
    pub syntax_colors: &'a FxHashMap<HighlightGroup, [u8; 3]>,
    pub diagnostics: Option<&'a [FileDiagnostic]>,
    pub hover_text: Option<&'a str>,
    pub completions: Option<(&'a [&'a CompletionItem], usize)>,
    pub signature_help: Option<&'a SignatureInfo>,
    pub inlay_hints: &'a [InlayHintInfo],
    pub code_lenses: &'a [CodeLensInfo],
    pub lsp_status: &'a str,
    pub keybinding_preset: KeybindingPreset,
    /// When true: suppress gutter, minimap, syntax highlighting, bracket
    /// match, search bar + match overlay, diagnostics, LSP inline overlays,
    /// status bar, and keyboard handling. Forces word-wrap on regardless of
    /// `editor_config.word_wrap`. The caller owns input via
    /// `NSTextInputClient`; the editor is the display surface only.
    pub prose_mode: bool,
}

/// Render the code editor for a text buffer.
pub(crate) fn render_text_editor(
    ui: &mut egui::Ui,
    state: TextEditorState<'_>,
    input: TextEditorInput<'_>,
) -> EditorFrameResult {
    let TextEditorState {
        buf,
        view,
        status_msg,
        clipboard_out,
        clipboard_in,
        editor_search,
    } = state;
    let TextEditorInput {
        syntax,
        editor_config,
        syntax_colors,
        diagnostics,
        hover_text,
        completions,
        signature_help,
        inlay_hints,
        code_lenses,
        lsp_status,
        keybinding_preset,
        prose_mode,
    } = input;
    let mut result = EditorFrameResult::default();

    let line_count = buf.line_count();
    let gutter_digits = ((line_count as f64).log10().floor() as usize + 1).max(2);
    let configured_indent = if editor_config.insert_spaces {
        IndentStyle::Spaces(editor_config.tab_size)
    } else {
        IndentStyle::Tabs
    };
    if buf.indent_style != configured_indent {
        buf.indent_style = configured_indent;
    }
    let editor_font_size = editor_config.font_size.clamp(8.0, 40.0);
    let char_width = (editor_font_size * 0.62).max(5.0);
    let line_height = (editor_font_size * editor_config.line_height.clamp(1.0, 2.2)).max(14.0);
    let gutter_width = compute_gutter_width(
        prose_mode,
        editor_config.show_line_numbers,
        gutter_digits,
        char_width,
    );
    let text_margin = 2.0;

    // Handle keyboard input (may modify buffer). Drop any stale edit record so
    // this frame only reports changes made by the key handler below.
    // In prose mode the caller owns input via NSTextInputClient; skip entirely.
    let _ = buf.take_last_edit();
    let cursor_before = view.cursor.pos;
    if !prose_mode {
        let ctx = ui.ctx().clone();
        let completion_active = completions.is_some();
        result.key_action = handle_editor_keys(EditorKeymapContext {
            ctx: &ctx,
            buf,
            view,
            status_msg,
            clipboard_out,
            clipboard_in,
            line_height,
            completion_active,
            keybinding_preset,
        });
        result.buffer_edit = buf.take_last_edit();
        if result.buffer_edit.is_some() {
            view.tree_dirty = true;
            view.folded_ranges.clear();
        }

        if result.key_action.add_cursor_next {
            if !view.cursor.has_selection() {
                view.cursor.select_word(buf);
            }
            if let Some(needle) = view.cursor.word_or_selection_text(buf) {
                view.cursor.add_next_occurrence(buf, &needle);
            }
        }
        if result.key_action.select_all_occurrences {
            if !view.cursor.has_selection() {
                view.cursor.select_word(buf);
            }
            if let Some(needle) = view.cursor.word_or_selection_text(buf) {
                view.cursor.select_all_occurrences(buf, &needle);
            }
        }
    }

    let cursor_moved = view.cursor.pos != cursor_before;

    view.cursor.clamp(buf);
    let foldable_ranges = view
        .tree
        .as_ref()
        .map(|tree| syntax.foldable_ranges(tree))
        .unwrap_or_default();
    apply_folding_actions(&result.key_action, view, &foldable_ranges, line_count);
    snap_cursor_to_visible_line(view, buf);
    let visible_doc_lines = visible_doc_lines(line_count, &view.folded_ranges);
    let visible_line_count = visible_doc_lines.len().max(1);

    // Word wrap: compute wrap rows for all visible doc lines.
    // Prose mode forces wrap on regardless of config — prompt panes are
    // narrow and a single long sentence must not horizontally scroll.
    let text_area_w_for_wrap = (ui.available_width() - gutter_width - text_margin).max(char_width);
    let wrap_cols = (text_area_w_for_wrap / char_width).max(10.0) as usize;
    let word_wrap = prose_mode || editor_config.word_wrap;
    let wrap_rows = if word_wrap {
        compute_wrap_rows(&visible_doc_lines, buf, wrap_cols)
    } else {
        Vec::new()
    };
    // In wrap mode, the total visual row count replaces visible_line_count for scroll
    let effective_line_count = if word_wrap {
        wrap_rows.len().max(1)
    } else {
        visible_line_count
    };

    // Vertical scroll -- smooth lerp toward target.
    let available_h = (ui.available_height() - 20.0).max(line_height);
    let visible_lines = (available_h / line_height).max(1.0) as usize;
    let max_scroll = effective_line_count.saturating_sub(1);
    view.scroll_line = view.scroll_line.min(max_scroll);
    if cursor_moved {
        let cursor_visible_line = if word_wrap {
            wrap_row_for_cursor(&wrap_rows, view.cursor.pos.line, view.cursor.pos.col)
        } else {
            visible_index_for_doc_line(&visible_doc_lines, view.cursor.pos.line)
        };
        if cursor_visible_line < view.scroll_line {
            view.scroll_target = Some(cursor_visible_line as f32);
        } else if cursor_visible_line >= view.scroll_line + visible_lines {
            view.scroll_target = Some(cursor_visible_line.saturating_sub(visible_lines - 1) as f32);
        }
    }
    // Animate smooth scroll toward target
    if let Some(target) = view.scroll_target {
        let target = target.clamp(0.0, max_scroll as f32);
        let current = view.scroll_line as f32;
        let diff = target - current;
        if diff.abs() < 0.5 {
            view.scroll_line = target.round() as usize;
            view.scroll_target = None;
        } else {
            let new_val = current + diff * 0.18;
            view.scroll_line = if diff > 0.0 {
                new_val.ceil() as usize
            } else {
                new_val.floor() as usize
            };
            view.scroll_line = view.scroll_line.min(max_scroll);
            ui.ctx().request_repaint();
        }
    }

    // Horizontal scroll (disabled in word-wrap mode)
    let text_area_w = (ui.available_width() - gutter_width - text_margin).max(char_width);
    let visible_cols = (text_area_w / char_width).max(1.0) as usize;
    if word_wrap {
        view.scroll_col = 0;
    } else {
        let margin_cols = 4;
        if view.cursor.pos.col < view.scroll_col {
            view.scroll_col = view.cursor.pos.col;
        } else if view.cursor.pos.col >= view.scroll_col + visible_cols.saturating_sub(margin_cols)
        {
            view.scroll_col = view
                .cursor
                .pos
                .col
                .saturating_sub(visible_cols.saturating_sub(margin_cols));
        }
    }

    // Compute the visible window of doc lines and (for wrap mode) the visible wrap rows
    let end_visible_line = if word_wrap {
        (view.scroll_line + visible_lines + 2).min(wrap_rows.len())
    } else {
        (view.scroll_line + visible_lines + 2).min(visible_doc_lines.len())
    };
    let visible_window: Vec<usize> = if word_wrap {
        let visible_wrap = &wrap_rows[view.scroll_line..end_visible_line];
        // Collect unique doc lines for syntax highlighting
        let mut doc_lines: Vec<usize> = visible_wrap.iter().map(|r| r.doc_line).collect();
        doc_lines.dedup();
        doc_lines
    } else {
        visible_doc_lines[view.scroll_line..end_visible_line].to_vec()
    };
    let visible_wrap_window: Vec<WrapRow> = if word_wrap {
        wrap_rows[view.scroll_line..end_visible_line].to_vec()
    } else {
        Vec::new()
    };
    let syntax_start_line = visible_window.first().copied().unwrap_or(0);
    let syntax_end_line = visible_window
        .last()
        .map(|line| line + 1)
        .unwrap_or(syntax_start_line);
    let h_offset = view.scroll_col as f32 * char_width;

    let status_text = editor_status_text(
        buf,
        view,
        editor_config,
        diagnostics,
        lsp_status,
        keybinding_preset,
    );

    // ── Find / Replace bar (suppressed in prose mode) ──
    let mut search_bar_h = 0.0;
    if !prose_mode && editor_search.active {
        search_bar_h = render_search_bar(ui, editor_search, buf, view);
    }
    let available_h = available_h - search_bar_h;

    // Main editor area — click + drag for cursor, scroll for navigation
    let (response, painter) = ui.allocate_painter(
        egui::Vec2::new(ui.available_width(), available_h),
        egui::Sense::click_and_drag(),
    );
    let rect = response.rect;

    let text_clip = handle_editor_pointer_input(
        ui,
        &response,
        &painter,
        buf,
        view,
        &foldable_ranges,
        &visible_doc_lines,
        &wrap_rows,
        rect,
        gutter_width,
        text_margin,
        h_offset,
        char_width,
        line_height,
        max_scroll,
        word_wrap,
    );

    render_primary_selection(
        &painter,
        text_clip,
        buf,
        view,
        &visible_window,
        &visible_wrap_window,
        rect,
        gutter_width,
        text_margin,
        char_width,
        line_height,
        h_offset,
        word_wrap,
    );
    if !prose_mode {
        render_search_matches(
            &painter,
            text_clip,
            buf,
            editor_search,
            &visible_window,
            rect,
            gutter_width,
            text_margin,
            char_width,
            line_height,
            h_offset,
        );
    }

    // Update git gutter. Invariant: in prose mode `view.git_gutter` is
    // always `None` (the prose `BufferView` is constructed with
    // `BufferView::default()` which leaves it unset), so this branch
    // skips itself. Belt-and-suspenders: a future change that attaches
    // a gutter to a prose view would silently start running here.
    if let Some(gutter) = &mut view.git_gutter {
        let current_lines: Vec<&str> = (0..line_count).map(|i| buf.line(i)).collect();
        gutter.update_if_needed(&current_lines);
    }

    // Syntax highlights for visible lines (disabled for very large files and in prose mode)
    let highlight_spans = if !prose_mode && perf::syntax_enabled(line_count) {
        let source_text = buf.text();
        match (view.lang_id, &view.tree) {
            (Some(lang_id), Some(tree)) => syntax.highlights_for_range(
                lang_id,
                tree,
                source_text.as_bytes(),
                syntax_start_line,
                syntax_end_line,
            ),
            _ => vec![Vec::new(); syntax_end_line.saturating_sub(syntax_start_line)],
        }
    } else {
        vec![Vec::new(); syntax_end_line.saturating_sub(syntax_start_line)]
    };

    let bracket_match = if prose_mode { None } else { buf.matching_bracket(view.cursor.pos) };
    render_editor_lines(
        &painter,
        text_clip,
        buf,
        view,
        editor_config,
        syntax_colors,
        inlay_hints,
        code_lenses,
        &highlight_spans,
        &foldable_ranges,
        &visible_window,
        &visible_wrap_window,
        rect,
        gutter_digits,
        gutter_width,
        text_margin,
        char_width,
        line_height,
        h_offset,
        syntax_start_line,
        editor_font_size,
        word_wrap,
    );

    render_bracket_match(
        &painter,
        text_clip,
        bracket_match,
        &visible_window,
        rect,
        gutter_width,
        text_margin,
        char_width,
        line_height,
        h_offset,
    );

    // Diagnostic underlines. Prose buffers never have diagnostics, but
    // guard explicitly so a future code path that hands non-empty
    // diagnostics into a prose render doesn't paint them over prompt text.
    if !prose_mode {
        render_diagnostics(
            &painter,
            text_clip,
            diagnostics,
            &visible_window,
            rect,
            gutter_width,
            text_margin,
            char_width,
            line_height,
            h_offset,
        );
    }

    render_primary_cursor(
        ui,
        &painter,
        text_clip,
        view,
        &visible_window,
        &visible_wrap_window,
        rect,
        gutter_width,
        text_margin,
        char_width,
        line_height,
        h_offset,
        word_wrap,
    );

    render_extra_cursors(
        ui,
        &painter,
        text_clip,
        buf,
        view,
        &visible_window,
        rect,
        gutter_width,
        text_margin,
        char_width,
        line_height,
        h_offset,
    );

    if !prose_mode {
        render_minimap(
            &response,
            &painter,
            buf,
            diagnostics,
            syntax_colors,
            &highlight_spans,
            &visible_doc_lines,
            rect,
            line_count,
            visible_line_count,
            visible_lines,
            max_scroll,
            syntax_start_line,
            syntax_end_line,
            view.scroll_line,
            &mut view.scroll_target,
        );

        render_inline_lsp_overlays(
            &painter,
            hover_text,
            completions,
            signature_help,
            view,
            &visible_window,
            rect,
            gutter_width,
            text_margin,
            char_width,
            line_height,
            h_offset,
        );

        // Status bar
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(status_text)
                    .size(12.0)
                    .color(egui::Color32::from_rgb(130, 130, 145))
                    .monospace(),
            );
        });
    }

    if prose_mode {
        result.input_anchor = compute_prose_input_anchor(
            ui,
            buf,
            rect,
            gutter_width,
            text_margin,
            text_area_w_for_wrap,
            editor_font_size,
            line_height,
        );
    }

    result
}

/// Build a single-galley layout describing the entire prose buffer so the
/// macOS `LlnzyStackerInputClient` can resolve UTF-16 character ranges to
/// screen rects (`firstRectForCharacterRange:`) and the inverse
/// (`characterIndexForPoint:`).
///
/// The galley is **not painted** — `render_editor_lines` already drew the
/// visible glyphs per-row above. This is geometry only, computed once per
/// frame in prose mode. We use the same `FontId::monospace(font_size)` the
/// visible render uses, set `line_height` explicitly so per-row Y matches
/// the editor view's own `(row * line_height)` math, and use the same wrap
/// width. For monospace + matched line-height, glyph positions agree
/// within sub-pixel rounding.
///
/// Returns `None` for empty buffers (no anchor to compute) so the input
/// client falls back to view-rect anchoring instead of pinning to (0, 0).
fn compute_prose_input_anchor(
    ui: &egui::Ui,
    buf: &crate::editor::buffer::Buffer,
    rect: egui::Rect,
    gutter_width: f32,
    text_margin: f32,
    wrap_max_width: f32,
    font_size: f32,
    line_height: f32,
) -> Option<(std::sync::Arc<egui::Galley>, egui::Pos2)> {
    if buf.is_empty() {
        return None;
    }
    let text = buf.text();
    let mut job = egui::text::LayoutJob::default();
    job.append(
        &text,
        0.0,
        egui::text::TextFormat {
            font_id: egui::FontId::monospace(font_size),
            color: egui::Color32::WHITE,
            line_height: Some(line_height),
            ..Default::default()
        },
    );
    job.wrap.max_width = wrap_max_width;
    let galley = ui.fonts(|f| f.layout_job(job));
    let origin = egui::pos2(rect.left() + gutter_width + text_margin, rect.top());
    Some((galley, origin))
}

/// Pure helper: gutter pixel width for a frame.
///
/// Prose mode collapses the gutter to zero (no line numbers, no fold
/// markers, no git markers). Code mode either shows numbered gutter
/// sized to the digit count, or a narrow icon-only gutter when line
/// numbers are off.
fn compute_gutter_width(
    prose_mode: bool,
    show_line_numbers: bool,
    gutter_digits: usize,
    char_width: f32,
) -> f32 {
    if prose_mode {
        0.0
    } else if show_line_numbers {
        (gutter_digits as f32 + 1.5) * char_width
    } else {
        (char_width * 2.4).max(22.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gutter_width_zero_in_prose_mode() {
        // Prose mode collapses the gutter to zero regardless of other inputs.
        assert_eq!(compute_gutter_width(true, true, 5, 10.0), 0.0);
        assert_eq!(compute_gutter_width(true, false, 5, 10.0), 0.0);
        assert_eq!(compute_gutter_width(true, true, 0, 0.0), 0.0);
    }

    #[test]
    fn gutter_width_uses_digits_when_line_numbers_on() {
        // 4 digits + 1.5 padding chars at 10px char width = 55.
        assert_eq!(compute_gutter_width(false, true, 4, 10.0), 55.0);
    }

    #[test]
    fn gutter_width_uses_icon_minimum_when_line_numbers_off() {
        // Without line numbers, gutter is `2.4 * char_width` floored at 22.
        assert_eq!(compute_gutter_width(false, false, 0, 10.0), 24.0);
        // Floor kicks in at small char widths.
        assert_eq!(compute_gutter_width(false, false, 0, 5.0), 22.0);
    }

    /// Sanity: a frame rendered with `prose_mode = true` returns the
    /// default `EditorFrameResult` (no key actions, no buffer edits)
    /// because keyboard handling is short-circuited.
    #[test]
    fn prose_mode_render_short_circuits_input() {
        use crate::config::EffectiveEditorConfig;
        use crate::editor::buffer::Buffer;
        use crate::editor::search::EditorSearch;
        use crate::editor::syntax::SyntaxEngine;
        use crate::editor::BufferView;
        use crate::keybindings::KeybindingPreset;
        use rustc_hash::FxHashMap;

        let mut buf = Buffer::empty_prose();
        buf.insert(crate::editor::buffer::Position::default(), "hello world");
        let mut view = BufferView::default();
        let syntax = SyntaxEngine::new();
        let editor_config = EffectiveEditorConfig {
            tab_size: 4,
            insert_spaces: true,
            rulers: Vec::new(),
            word_wrap: false,
            visible_whitespace: false,
            font_size: 14.0,
            line_height: 1.4,
            show_line_numbers: false,
            highlight_current_line: false,
        };
        let syntax_colors = FxHashMap::default();
        let mut status_msg = None;
        let mut clipboard_out = None;
        let mut clipboard_in = None;
        let mut editor_search = EditorSearch::default();

        let ctx = egui::Context::default();
        let mut frame_result_opt = None;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let result = render_text_editor(
                    ui,
                    TextEditorState {
                        buf: &mut buf,
                        view: &mut view,
                        status_msg: &mut status_msg,
                        clipboard_out: &mut clipboard_out,
                        clipboard_in: &mut clipboard_in,
                        editor_search: &mut editor_search,
                    },
                    TextEditorInput {
                        syntax: &syntax,
                        editor_config: &editor_config,
                        syntax_colors: &syntax_colors,
                        diagnostics: None,
                        hover_text: None,
                        completions: None,
                        signature_help: None,
                        inlay_hints: &[],
                        code_lenses: &[],
                        lsp_status: "",
                        keybinding_preset: KeybindingPreset::VsCode,
                        prose_mode: true,
                    },
                );
                frame_result_opt = Some(result);
            });
        });

        let result = frame_result_opt.expect("render produced a frame result");
        assert!(
            result.buffer_edit.is_none(),
            "prose mode must not mutate the buffer through the editor view"
        );
        // The KeyAction is `Default::default()` since no key handling ran.
        assert!(!result.key_action.add_cursor_next);
        assert!(!result.key_action.select_all_occurrences);
    }

    /// Phase D: prose-mode rendering produces an `input_anchor` whose
    /// galley contains the buffer text and whose origin sits inside the
    /// painter rect. Used by the macOS input client to anchor dictation.
    #[test]
    fn prose_mode_render_emits_input_anchor() {
        use crate::config::EffectiveEditorConfig;
        use crate::editor::buffer::Buffer;
        use crate::editor::search::EditorSearch;
        use crate::editor::syntax::SyntaxEngine;
        use crate::editor::BufferView;
        use crate::keybindings::KeybindingPreset;
        use rustc_hash::FxHashMap;

        let mut buf = Buffer::empty_prose();
        buf.insert(
            crate::editor::buffer::Position::default(),
            "first\nsecond line",
        );
        let mut view = BufferView::default();
        let syntax = SyntaxEngine::new();
        let editor_config = EffectiveEditorConfig {
            tab_size: 4,
            insert_spaces: true,
            rulers: Vec::new(),
            word_wrap: false,
            visible_whitespace: false,
            font_size: 14.0,
            line_height: 1.4,
            show_line_numbers: false,
            highlight_current_line: false,
        };
        let syntax_colors = FxHashMap::default();
        let mut status_msg = None;
        let mut clipboard_out = None;
        let mut clipboard_in = None;
        let mut editor_search = EditorSearch::default();

        let ctx = egui::Context::default();
        let mut anchor_opt = None;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let result = render_text_editor(
                    ui,
                    TextEditorState {
                        buf: &mut buf,
                        view: &mut view,
                        status_msg: &mut status_msg,
                        clipboard_out: &mut clipboard_out,
                        clipboard_in: &mut clipboard_in,
                        editor_search: &mut editor_search,
                    },
                    TextEditorInput {
                        syntax: &syntax,
                        editor_config: &editor_config,
                        syntax_colors: &syntax_colors,
                        diagnostics: None,
                        hover_text: None,
                        completions: None,
                        signature_help: None,
                        inlay_hints: &[],
                        code_lenses: &[],
                        lsp_status: "",
                        keybinding_preset: KeybindingPreset::VsCode,
                        prose_mode: true,
                    },
                );
                anchor_opt = result.input_anchor;
            });
        });

        let (galley, origin) = anchor_opt.expect("prose render must produce an anchor");
        // Galley non-empty: at least one row present and the buffer
        // text is reflected in `galley.job.text`.
        assert!(!galley.rows.is_empty(), "galley should have rows");
        assert!(galley.job.text.contains("second line"));
        // Origin sits at non-negative coordinates (real painter rect).
        assert!(origin.x >= 0.0);
        assert!(origin.y >= 0.0);

        // Char index 0 → top-left of galley; char index past first
        // newline lands on a later row.
        let p0 = galley.pos_from_ccursor(egui::text::CCursor::new(0));
        let p_after_newline = galley.pos_from_ccursor(egui::text::CCursor::new(6));
        assert!(
            p_after_newline.min.y > p0.min.y,
            "second line must be below the first ({:?} vs {:?})",
            p_after_newline,
            p0
        );
    }

    /// Empty prose buffer skips anchor production so the input client
    /// falls back to view-rect anchoring instead of pinning to (0, 0).
    #[test]
    fn prose_mode_render_skips_anchor_for_empty_buffer() {
        use crate::config::EffectiveEditorConfig;
        use crate::editor::buffer::Buffer;
        use crate::editor::search::EditorSearch;
        use crate::editor::syntax::SyntaxEngine;
        use crate::editor::BufferView;
        use crate::keybindings::KeybindingPreset;
        use rustc_hash::FxHashMap;

        let mut buf = Buffer::empty_prose();
        let mut view = BufferView::default();
        let syntax = SyntaxEngine::new();
        let editor_config = EffectiveEditorConfig {
            tab_size: 4,
            insert_spaces: true,
            rulers: Vec::new(),
            word_wrap: false,
            visible_whitespace: false,
            font_size: 14.0,
            line_height: 1.4,
            show_line_numbers: false,
            highlight_current_line: false,
        };
        let syntax_colors = FxHashMap::default();
        let mut status_msg = None;
        let mut clipboard_out = None;
        let mut clipboard_in = None;
        let mut editor_search = EditorSearch::default();

        let ctx = egui::Context::default();
        let mut saw_some = false;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let result = render_text_editor(
                    ui,
                    TextEditorState {
                        buf: &mut buf,
                        view: &mut view,
                        status_msg: &mut status_msg,
                        clipboard_out: &mut clipboard_out,
                        clipboard_in: &mut clipboard_in,
                        editor_search: &mut editor_search,
                    },
                    TextEditorInput {
                        syntax: &syntax,
                        editor_config: &editor_config,
                        syntax_colors: &syntax_colors,
                        diagnostics: None,
                        hover_text: None,
                        completions: None,
                        signature_help: None,
                        inlay_hints: &[],
                        code_lenses: &[],
                        lsp_status: "",
                        keybinding_preset: KeybindingPreset::VsCode,
                        prose_mode: true,
                    },
                );
                saw_some = result.input_anchor.is_some();
            });
        });

        assert!(!saw_some, "empty prose buffer should not emit an anchor");
    }

    /// Long single-line prompts (no embedded newlines) wrap to multiple
    /// galley rows and the anchor reports the last char on a later row
    /// than the first. Guards against a regression where `wrap.max_width`
    /// is wrong or word-wrap is silently skipped in prose mode.
    #[test]
    fn prose_mode_wraps_long_single_line_into_multiple_rows() {
        use crate::config::EffectiveEditorConfig;
        use crate::editor::buffer::Buffer;
        use crate::editor::search::EditorSearch;
        use crate::editor::syntax::SyntaxEngine;
        use crate::editor::BufferView;
        use crate::keybindings::KeybindingPreset;
        use rustc_hash::FxHashMap;

        let mut buf = Buffer::empty_prose();
        // 2000-char single line, no newlines.
        let long_line: String = std::iter::repeat('a').take(2000).collect();
        buf.insert(crate::editor::buffer::Position::default(), &long_line);
        let mut view = BufferView::default();
        let syntax = SyntaxEngine::new();
        let editor_config = EffectiveEditorConfig {
            tab_size: 4,
            insert_spaces: true,
            rulers: Vec::new(),
            word_wrap: false, // intentionally off — prose_mode must force it
            visible_whitespace: false,
            font_size: 14.0,
            line_height: 1.4,
            show_line_numbers: false,
            highlight_current_line: false,
        };
        let syntax_colors = FxHashMap::default();
        let mut status_msg = None;
        let mut clipboard_out = None;
        let mut clipboard_in = None;
        let mut editor_search = EditorSearch::default();

        let ctx = egui::Context::default();
        let mut anchor_opt = None;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let result = render_text_editor(
                    ui,
                    TextEditorState {
                        buf: &mut buf,
                        view: &mut view,
                        status_msg: &mut status_msg,
                        clipboard_out: &mut clipboard_out,
                        clipboard_in: &mut clipboard_in,
                        editor_search: &mut editor_search,
                    },
                    TextEditorInput {
                        syntax: &syntax,
                        editor_config: &editor_config,
                        syntax_colors: &syntax_colors,
                        diagnostics: None,
                        hover_text: None,
                        completions: None,
                        signature_help: None,
                        inlay_hints: &[],
                        code_lenses: &[],
                        lsp_status: "",
                        keybinding_preset: KeybindingPreset::VsCode,
                        prose_mode: true,
                    },
                );
                anchor_opt = result.input_anchor;
            });
        });

        let (galley, _origin) = anchor_opt.expect("anchor for non-empty prose");
        assert!(
            galley.rows.len() > 1,
            "2000-char single line should wrap into multiple rows, got {} row(s)",
            galley.rows.len()
        );
        let last = galley.pos_from_ccursor(egui::text::CCursor::new(2000));
        let first = galley.pos_from_ccursor(egui::text::CCursor::new(0));
        assert!(
            last.min.y > first.min.y,
            "last char should sit on a later row than the first ({:?} vs {:?})",
            last,
            first
        );
    }

    /// Code mode never emits an `input_anchor` — that field is reserved
    /// for the prose path's macOS input-client integration.
    #[test]
    fn code_mode_render_does_not_emit_input_anchor() {
        use crate::config::EffectiveEditorConfig;
        use crate::editor::buffer::Buffer;
        use crate::editor::search::EditorSearch;
        use crate::editor::syntax::SyntaxEngine;
        use crate::editor::BufferView;
        use crate::keybindings::KeybindingPreset;
        use rustc_hash::FxHashMap;

        let mut buf = Buffer::empty();
        buf.insert(crate::editor::buffer::Position::default(), "let x = 1;");
        let mut view = BufferView::default();
        let syntax = SyntaxEngine::new();
        let editor_config = EffectiveEditorConfig {
            tab_size: 4,
            insert_spaces: true,
            rulers: Vec::new(),
            word_wrap: false,
            visible_whitespace: false,
            font_size: 14.0,
            line_height: 1.4,
            show_line_numbers: true,
            highlight_current_line: true,
        };
        let syntax_colors = FxHashMap::default();
        let mut status_msg = None;
        let mut clipboard_out = None;
        let mut clipboard_in = None;
        let mut editor_search = EditorSearch::default();

        let ctx = egui::Context::default();
        let mut anchor_opt = None;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let result = render_text_editor(
                    ui,
                    TextEditorState {
                        buf: &mut buf,
                        view: &mut view,
                        status_msg: &mut status_msg,
                        clipboard_out: &mut clipboard_out,
                        clipboard_in: &mut clipboard_in,
                        editor_search: &mut editor_search,
                    },
                    TextEditorInput {
                        syntax: &syntax,
                        editor_config: &editor_config,
                        syntax_colors: &syntax_colors,
                        diagnostics: None,
                        hover_text: None,
                        completions: None,
                        signature_help: None,
                        inlay_hints: &[],
                        code_lenses: &[],
                        lsp_status: "",
                        keybinding_preset: KeybindingPreset::VsCode,
                        prose_mode: false,
                    },
                );
                anchor_opt = result.input_anchor;
            });
        });

        assert!(anchor_opt.is_none());
    }
}
