use std::collections::HashMap;

use crate::config::EffectiveEditorConfig;
use crate::editor::buffer::IndentStyle;
use crate::editor::buffer::Position;
use crate::editor::keymap::{handle_editor_keys, KeyAction};
use crate::editor::perf;
use crate::editor::search::EditorSearch;
use crate::editor::syntax::{HighlightGroup, SyntaxEngine};
use crate::editor::BufferView;
use crate::keybindings::KeybindingPreset;
use crate::lsp::{CodeLensInfo, FileDiagnostic, InlayHintInfo};

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
    /// Cursor position before the frame's edits were applied.
    pub cursor_before: Position,
}

/// Render the code editor for a text buffer.
pub(crate) fn render_text_editor(
    ui: &mut egui::Ui,
    buf: &mut crate::editor::buffer::Buffer,
    view: &mut BufferView,
    syntax: &SyntaxEngine,
    editor_config: &EffectiveEditorConfig,
    syntax_colors: &HashMap<HighlightGroup, [u8; 3]>,
    diagnostics: Option<&[FileDiagnostic]>,
    hover_text: Option<&str>,
    completions: Option<(&[&crate::lsp::CompletionItem], usize)>,
    signature_help: Option<&crate::lsp::SignatureInfo>,
    inlay_hints: &[InlayHintInfo],
    code_lenses: &[CodeLensInfo],
    status_msg: &mut Option<String>,
    clipboard_out: &mut Option<String>,
    clipboard_in: &mut Option<String>,
    editor_search: &mut EditorSearch,
    lsp_status: &str,
    keybinding_preset: KeybindingPreset,
) -> EditorFrameResult {
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
    let line_height = (editor_font_size * 1.38).max(14.0);
    let gutter_width = (gutter_digits as f32 + 1.5) * char_width;
    let text_margin = 2.0;

    // Handle keyboard input (may modify buffer)
    let content_before = buf.len_chars();
    let cursor_before = view.cursor.pos;
    result.cursor_before = cursor_before;
    let ctx = ui.ctx().clone();
    let completion_active = completions.is_some();
    result.key_action = handle_editor_keys(
        &ctx,
        buf,
        view,
        status_msg,
        clipboard_out,
        clipboard_in,
        line_height,
        completion_active,
        keybinding_preset,
    );
    if buf.len_chars() != content_before {
        view.tree_dirty = true;
        view.folded_ranges.clear();
    }

    // Handle multi-cursor actions (Cmd+D and Cmd+Shift+L)
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

    // Word wrap: compute wrap rows for all visible doc lines
    let text_area_w_for_wrap = (ui.available_width() - gutter_width - text_margin).max(char_width);
    let wrap_cols = (text_area_w_for_wrap / char_width).max(10.0) as usize;
    let word_wrap = editor_config.word_wrap;
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

    // ── Find / Replace bar ──
    let mut search_bar_h = 0.0;
    if editor_search.active {
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

    // Update git gutter
    if let Some(gutter) = &mut view.git_gutter {
        let current_lines: Vec<&str> = (0..line_count).map(|i| buf.line(i)).collect();
        gutter.update_if_needed(&current_lines);
    }

    // Syntax highlights for visible lines (disabled for very large files)
    let source_text = buf.text();
    let highlight_spans = if perf::syntax_enabled(line_count) {
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

    let bracket_match = buf.matching_bracket(view.cursor.pos);
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

    // Diagnostic underlines
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

    result
}
