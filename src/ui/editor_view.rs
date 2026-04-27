use std::collections::HashMap;

use crate::config::EffectiveEditorConfig;
use crate::editor::buffer::IndentStyle;
use crate::editor::buffer::Position;
use crate::editor::keymap::{handle_editor_keys, KeyAction};
use crate::editor::perf;
use crate::editor::git_gutter::GutterChange;
use crate::editor::search::EditorSearch;
use crate::editor::syntax::{FoldRange, HighlightGroup, SyntaxEngine};
use crate::editor::BufferView;
use crate::keybindings::KeybindingPreset;
use crate::lsp::{CodeLensInfo, DiagSeverity, FileDiagnostic, InlayHintInfo};

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
    let text_area_w_for_wrap = ui.available_width() - gutter_width - text_margin;
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
    let available_h = ui.available_height() - 20.0;
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
    let text_area_w = ui.available_width() - gutter_width - text_margin;
    let visible_cols = (text_area_w / char_width).max(1.0) as usize;
    if word_wrap {
        view.scroll_col = 0;
    } else {
        let margin_cols = 4;
        if view.cursor.pos.col < view.scroll_col {
            view.scroll_col = view.cursor.pos.col;
        } else if view.cursor.pos.col >= view.scroll_col + visible_cols.saturating_sub(margin_cols) {
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

    // Status bar text
    let indent_label = match buf.indent_style {
        crate::editor::buffer::IndentStyle::Spaces(n) => format!("Spaces: {n}"),
        crate::editor::buffer::IndentStyle::Tabs => "Tabs".to_string(),
    };
    let diag_count = diagnostics.map_or(0, |d| d.len());
    let diag_label = if diag_count > 0 {
        format!(
            "  |  {diag_count} diagnostic{}",
            if diag_count == 1 { "" } else { "s" }
        )
    } else {
        String::new()
    };
let lsp_label = if lsp_status.is_empty() {
        String::new()
    } else {
        format!("  |  {lsp_status}")
    };
    let vim_label = match view.vim_mode {
        Some(crate::keybindings::VimMode::Normal) => "  |  VIM NORMAL",
        Some(crate::keybindings::VimMode::Insert) => "  |  VIM INSERT",
        Some(crate::keybindings::VimMode::Visual) => "  |  VIM VISUAL",
        None => "",
    };
    let preset_label = match keybinding_preset {
        KeybindingPreset::VsCode => "",
        KeybindingPreset::Vim => "",
        KeybindingPreset::Emacs => "  |  Emacs",
    };
    let status_text = format!(
        "Ln {}, Col {}  |  {} lines  |  {}  |  {}  |  {}{}{}{}{}",
        view.cursor.pos.line + 1,
        view.cursor.pos.col + 1,
        line_count,
        indent_label,
        if editor_config.word_wrap { "Wrap" } else { "No wrap" },
        if buf.is_modified() {
            "Modified"
        } else {
            "Saved"
        },
        diag_label,
lsp_label,
        vim_label,
        preset_label,
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

    // Mouse wheel scrolling (sets smooth scroll target)
    if response.hovered() {
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll_delta != 0.0 {
            let scroll_lines = (-scroll_delta / line_height).round() as i32;
            let base = view.scroll_target.unwrap_or(view.scroll_line as f32);
            let new_target = (base + scroll_lines as f32).clamp(0.0, max_scroll as f32);
            view.scroll_target = Some(new_target);
        }
    }
    let text_clip = egui::Rect::from_min_max(
        egui::pos2(rect.left() + gutter_width, rect.top()),
        rect.right_bottom(),
    );

    // Background
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(25, 25, 32));
    let gutter_rect = egui::Rect::from_min_size(
        rect.left_top(),
        egui::Vec2::new(gutter_width, rect.height()),
    );
    painter.rect_filled(gutter_rect, 0.0, egui::Color32::from_rgb(30, 30, 38));
    let mut handled_gutter_click = false;
    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            if pos.x < rect.left() + gutter_width {
                let visible_idx = view.scroll_line + ((pos.y - rect.top()) / line_height).max(0.0) as usize;
                if let Some(&doc_line) = visible_doc_lines.get(visible_idx) {
                    if let Some(range) = best_fold_range_starting_at(&foldable_ranges, doc_line) {
                        toggle_fold_range(&mut view.folded_ranges, range);
                        handled_gutter_click = true;
                    }
                }
            }
        }
    }

    // Mouse click to position cursor, drag to select
    let minimap_w = 50.0_f32;
    if response.clicked() && !handled_gutter_click {
        if let Some(pos) = response.interact_pointer_pos() {
            // Don't handle clicks in the minimap area as editor clicks
            if pos.x < rect.right() - minimap_w {
                let (click_line, click_col) = if word_wrap {
                    pixel_to_editor_pos_wrapped(
                        pos, rect, gutter_width, text_margin, char_width, line_height,
                        view.scroll_line, &wrap_rows, buf,
                    )
                } else {
                    pixel_to_editor_pos(
                        pos, rect, gutter_width, text_margin, h_offset, char_width, line_height,
                        view.scroll_line, &visible_doc_lines, buf,
                    )
                };
                view.cursor.clear_selection();
                view.cursor.clear_extra_cursors();
                view.cursor.pos = Position::new(click_line, click_col);
                view.cursor.desired_col = None;
            }
        }
    }
    if response.dragged() {
        if let Some(pos) = response.interact_pointer_pos() {
            let (drag_line, drag_col) = if word_wrap {
                pixel_to_editor_pos_wrapped(
                    pos, rect, gutter_width, text_margin, char_width, line_height,
                    view.scroll_line, &wrap_rows, buf,
                )
            } else {
                pixel_to_editor_pos(
                    pos, rect, gutter_width, text_margin, h_offset, char_width, line_height,
                    view.scroll_line, &visible_doc_lines, buf,
                )
            };
            // Start selection on first drag if not already selecting
            if !view.cursor.has_selection() {
                view.cursor.anchor = Some(view.cursor.pos);
            }
            view.cursor.pos = Position::new(drag_line, drag_col);
            view.cursor.desired_col = None;
        }
    }

    // Selection highlight
    if let Some((sel_start, sel_end)) = view.cursor.selection() {
        let sel_color = egui::Color32::from_rgba_unmultiplied(60, 100, 180, 80);
        if word_wrap {
            for (vis_idx, row) in visible_wrap_window.iter().enumerate() {
                if row.doc_line < sel_start.line || row.doc_line > sel_end.line {
                    continue;
                }
                let vis_y = vis_idx as f32 * line_height;
                let line_len = buf.line_len(row.doc_line);
                let doc_col_start = if row.doc_line == sel_start.line { sel_start.col } else { 0 };
                let doc_col_end = if row.doc_line == sel_end.line { sel_end.col } else { line_len };
                // Clamp to this row's column range
                let row_sel_start = doc_col_start.max(row.col_start).saturating_sub(row.col_start);
                let row_sel_end = doc_col_end.min(row.col_end).saturating_sub(row.col_start);
                if row_sel_start >= row_sel_end {
                    continue;
                }
                let x1 = rect.left() + gutter_width + text_margin + row_sel_start as f32 * char_width;
                let x2 = rect.left() + gutter_width + text_margin + row_sel_end as f32 * char_width;
                let sel_rect = egui::Rect::from_min_max(
                    egui::pos2(x1.max(text_clip.left()), rect.top() + vis_y),
                    egui::pos2(x2.max(x1 + char_width).min(text_clip.right()), rect.top() + vis_y + line_height),
                );
                if sel_rect.width() > 0.0 {
                    painter.rect_filled(sel_rect, 0.0, sel_color);
                }
            }
        } else {
            for (visible_offset, &line_idx) in visible_window.iter().enumerate() {
                if line_idx < sel_start.line || line_idx > sel_end.line {
                    continue;
                }
                let vis_y = visible_offset as f32 * line_height;
                let line_len = buf.line_len(line_idx);
                let col_start = if line_idx == sel_start.line {
                    sel_start.col
                } else {
                    0
                };
                let col_end = if line_idx == sel_end.line {
                    sel_end.col
                } else {
                    line_len
                };
                let x1 =
                    rect.left() + gutter_width + text_margin + col_start as f32 * char_width - h_offset;
                let x2 =
                    rect.left() + gutter_width + text_margin + col_end as f32 * char_width - h_offset;
                let sel_rect = egui::Rect::from_min_max(
                    egui::pos2(x1.max(text_clip.left()), rect.top() + vis_y),
                    egui::pos2(
                        x2.max(x1 + char_width).min(text_clip.right()),
                        rect.top() + vis_y + line_height,
                    ),
                );
                if sel_rect.width() > 0.0 {
                    painter.rect_filled(sel_rect, 0.0, sel_color);
                }
            }
        }
    }

    // Search match highlights
    if editor_search.active {
        editor_search.update_if_dirty(buf);
        let match_bg = egui::Color32::from_rgba_unmultiplied(230, 180, 50, 40);
        let focus_bg = egui::Color32::from_rgba_unmultiplied(230, 180, 50, 100);
        for (i, m) in editor_search.matches.iter().enumerate() {
            // Only render matches that overlap visible window
            let start_line = m.start.line;
            let end_line = m.end.line;
            for (visible_offset, &line_idx) in visible_window.iter().enumerate() {
                if line_idx < start_line || line_idx > end_line {
                    continue;
                }
                let vis_y = visible_offset as f32 * line_height;
                let col_start = if line_idx == start_line { m.start.col } else { 0 };
                let col_end = if line_idx == end_line { m.end.col } else { buf.line_len(line_idx) };
                let x1 = rect.left() + gutter_width + text_margin + col_start as f32 * char_width - h_offset;
                let x2 = rect.left() + gutter_width + text_margin + col_end as f32 * char_width - h_offset;
                let color = if i == editor_search.focus { focus_bg } else { match_bg };
                let match_rect = egui::Rect::from_min_max(
                    egui::pos2(x1.max(text_clip.left()), rect.top() + vis_y),
                    egui::pos2(x2.min(text_clip.right()), rect.top() + vis_y + line_height),
                );
                if match_rect.width() > 0.0 {
                    painter.rect_filled(match_rect, 0.0, color);
                }
            }
        }
    }

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

    // Render visible lines
    let text_color = egui::Color32::WHITE;
    let gutter_color = egui::Color32::from_rgb(100, 100, 120);
    let current_line_gutter = egui::Color32::from_rgb(180, 180, 200);
    let current_line_bg = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 8);
    let font = egui::FontId::monospace(editor_font_size);
    let active_indent_level = indent_level(buf.line(view.cursor.pos.line), buf.indent_style);
    let bracket_match = buf.matching_bracket(view.cursor.pos);

    render_rulers(
        &painter,
        text_clip,
        &editor_config.rulers,
        rect,
        gutter_width,
        text_margin,
        char_width,
        h_offset,
    );

    if !word_wrap {
        render_indentation_guides(
            &painter,
            text_clip,
            buf,
            &visible_window,
            active_indent_level,
            rect,
            gutter_width,
            text_margin,
            char_width,
            line_height,
            h_offset,
        );
    }

    if word_wrap {
        // ── Word-wrap rendering path ──
        for (vis_idx, row) in visible_wrap_window.iter().enumerate() {
            let vis_y = vis_idx as f32 * line_height;
            let y = rect.top() + vis_y;

            // Current line highlight
            if row.doc_line == view.cursor.pos.line {
                let line_rect = egui::Rect::from_min_size(
                    egui::pos2(rect.left() + gutter_width, y),
                    egui::Vec2::new(rect.width() - gutter_width, line_height),
                );
                painter.rect_filled(line_rect, 0.0, current_line_bg);
            }

            // Line number (only on first visual row of each doc line)
            if row.is_first {
                let num_str = format!("{:>width$}", row.doc_line + 1, width = gutter_digits);
                let num_color = if row.doc_line == view.cursor.pos.line {
                    current_line_gutter
                } else {
                    gutter_color
                };
                painter.text(
                    egui::pos2(rect.left() + 4.0, y + 1.0),
                    egui::Align2::LEFT_TOP,
                    &num_str,
                    font.clone(),
                    num_color,
                );

                // Git gutter (only on first row)
                if let Some(gutter) = &view.git_gutter {
                    if let Some(change) = gutter.change_at(row.doc_line) {
                        let (color, bar_h) = match change {
                            GutterChange::Added => (egui::Color32::from_rgb(80, 200, 80), line_height),
                            GutterChange::Modified => (egui::Color32::from_rgb(80, 140, 230), line_height),
                            GutterChange::Deleted => (egui::Color32::from_rgb(220, 70, 70), 4.0),
                        };
                        let bar_y = if change == GutterChange::Deleted { y - 2.0 } else { y };
                        painter.rect_filled(
                            egui::Rect::from_min_size(
                                egui::pos2(rect.left() + gutter_width - 4.0, bar_y),
                                egui::Vec2::new(3.0, bar_h),
                            ),
                            0.0,
                            color,
                        );
                    }
                }
            } else {
                // Continuation indicator for wrapped lines
                let wrap_indicator = egui::Color32::from_rgb(60, 65, 80);
                painter.text(
                    egui::pos2(rect.left() + gutter_width - 12.0, y + 1.0),
                    egui::Align2::LEFT_TOP,
                    "~",
                    font.clone(),
                    wrap_indicator,
                );
            }

            // Render the portion of the line for this wrap row
            let line_text = buf.line(row.doc_line);
            if line_text.is_empty() || row.col_start >= line_text.chars().count() {
                continue;
            }
            let chars: Vec<char> = line_text.chars().collect();
            let end = row.col_end.min(chars.len());
            let row_text: String = chars[row.col_start..end].iter().collect();
            if row_text.is_empty() {
                continue;
            }

            let text_x_base = rect.left() + gutter_width + text_margin;
            let spans = highlight_spans
                .get(row.doc_line.saturating_sub(syntax_start_line))
                .map(Vec::as_slice)
                .unwrap_or(&[]);

            // Render with syntax highlighting, offset spans to wrap row
            if spans.is_empty() {
                painter.with_clip_rect(text_clip).text(
                    egui::pos2(text_x_base, y + 1.0),
                    egui::Align2::LEFT_TOP,
                    &row_text,
                    font.clone(),
                    text_color,
                );
            } else {
                // Render spans shifted by col_start
                let row_chars: Vec<char> = row_text.chars().collect();
                let mut col = 0;
                while col < row_chars.len() {
                    let doc_col = row.col_start + col;
                    let color = spans
                        .iter()
                        .find(|s| doc_col >= s.col_start && doc_col < s.col_end)
                        .map(|s| {
                            let rgb = crate::editor::syntax::group_color_with_overrides(s.group, syntax_colors);
                            egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2])
                        })
                        .unwrap_or(text_color);

                    let mut batch_end = col + 1;
                    while batch_end < row_chars.len() {
                        let next_doc_col = row.col_start + batch_end;
                        let next_color = spans
                            .iter()
                            .find(|s| next_doc_col >= s.col_start && next_doc_col < s.col_end)
                            .map(|s| {
                                let rgb = crate::editor::syntax::group_color_with_overrides(s.group, syntax_colors);
                                egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2])
                            })
                            .unwrap_or(text_color);
                        if next_color != color {
                            break;
                        }
                        batch_end += 1;
                    }

                    let chunk: String = row_chars[col..batch_end].iter().collect();
                    let x = text_x_base + col as f32 * char_width;
                    painter.with_clip_rect(text_clip).text(
                        egui::pos2(x, y + 1.0),
                        egui::Align2::LEFT_TOP,
                        &chunk,
                        font.clone(),
                        color,
                    );
                    col = batch_end;
                }
            }
        }
    } else {
        // ── Non-wrap rendering path (original) ──
        for (visible_offset, &line_idx) in visible_window.iter().enumerate() {
            let vis_y = visible_offset as f32 * line_height;
            let y = rect.top() + vis_y;

            if line_idx == view.cursor.pos.line {
                let line_rect = egui::Rect::from_min_size(
                    egui::pos2(rect.left() + gutter_width, y),
                    egui::Vec2::new(rect.width() - gutter_width, line_height),
                );
                painter.rect_filled(line_rect, 0.0, current_line_bg);
            }

            // Line number
            let num_str = format!("{:>width$}", line_idx + 1, width = gutter_digits);
            let num_color = if line_idx == view.cursor.pos.line {
                current_line_gutter
            } else {
                gutter_color
            };
            painter.text(
                egui::pos2(rect.left() + 4.0, y + 1.0),
                egui::Align2::LEFT_TOP,
                &num_str,
                font.clone(),
                num_color,
            );

            // Git gutter indicator
            if let Some(gutter) = &view.git_gutter {
                if let Some(change) = gutter.change_at(line_idx) {
                    let (color, bar_h) = match change {
                        GutterChange::Added => (egui::Color32::from_rgb(80, 200, 80), line_height),
                        GutterChange::Modified => (egui::Color32::from_rgb(80, 140, 230), line_height),
                        GutterChange::Deleted => (egui::Color32::from_rgb(220, 70, 70), 4.0),
                    };
                    let bar_y = if change == GutterChange::Deleted {
                        y - 2.0 // position between lines
                    } else {
                        y
                    };
                    painter.rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(rect.left() + gutter_width - 4.0, bar_y),
                            egui::Vec2::new(3.0, bar_h),
                        ),
                        0.0,
                        color,
                    );
                }
            }

            if let Some(range) = foldable_ranges.iter().find(|r| r.start_line == line_idx) {
                let marker = if is_range_folded(&view.folded_ranges, range.start_line) { ">" } else { "v" };
                painter.text(
                    egui::pos2(rect.left() + gutter_width - 12.0, y + 1.0),
                    egui::Align2::LEFT_TOP,
                    marker,
                    font.clone(),
                    gutter_color,
                );
            }

            // Line text with syntax highlighting
            let line_text = buf.line(line_idx);
            if line_text.is_empty() {
                continue;
            }
            let text_x_base = rect.left() + gutter_width + text_margin - h_offset;
            let spans = highlight_spans
                .get(line_idx.saturating_sub(syntax_start_line))
                .map(Vec::as_slice)
                .unwrap_or(&[]);

            if editor_config.visible_whitespace {
                render_visible_whitespace_line(
                    &painter,
                    text_clip,
                    spans,
                    syntax_colors,
                    line_text,
                    text_x_base,
                    y,
                    char_width,
                    &font,
                    text_color,
                );
            } else if spans.is_empty() {
                painter.with_clip_rect(text_clip).text(
                    egui::pos2(text_x_base, y + 1.0),
                    egui::Align2::LEFT_TOP,
                    line_text,
                    font.clone(),
                    text_color,
                );
            } else {
                render_highlighted_line(
                    &painter,
                    text_clip,
                    spans,
                    syntax_colors,
                    line_text,
                    text_x_base,
                    y,
                    char_width,
                    &font,
                    text_color,
                );
            }

            if let Some(range) = folded_range_starting_at(&view.folded_ranges, line_idx) {
                let hidden_count = range.end_line.saturating_sub(range.start_line);
                let placeholder = format!(" ... {hidden_count} folded line{}", if hidden_count == 1 { "" } else { "s" });
                let placeholder_x = text_x_base + line_text.chars().count() as f32 * char_width;
                painter.with_clip_rect(text_clip).text(
                    egui::pos2(placeholder_x, y + 1.0),
                    egui::Align2::LEFT_TOP,
                    placeholder,
                    font.clone(),
                    egui::Color32::from_rgb(110, 120, 145),
                );
            }

            // Inlay hints for this line (rendered after the code text)
            let hint_color = egui::Color32::from_rgba_unmultiplied(140, 150, 175, 160);
            let hint_font = egui::FontId::monospace(editor_font_size * 0.85);
            for hint in inlay_hints.iter().filter(|h| h.line as usize == line_idx) {
                let hint_x = text_x_base + hint.col as f32 * char_width;
                let label = format!(
                    "{}{}{}",
                    if hint.padding_left { " " } else { "" },
                    hint.label,
                    if hint.padding_right { " " } else { "" },
                );
                painter.with_clip_rect(text_clip).text(
                    egui::pos2(hint_x, y + 1.0),
                    egui::Align2::LEFT_TOP,
                    &label,
                    hint_font.clone(),
                    hint_color,
                );
            }

            // Code lenses for this line (rendered above the line, dimmed)
            for lens in code_lenses.iter().filter(|l| l.line as usize == line_idx) {
                let lens_y = y - line_height * 0.5;
                if lens_y >= rect.top() {
                    painter.with_clip_rect(text_clip).text(
                        egui::pos2(text_x_base, lens_y),
                        egui::Align2::LEFT_TOP,
                        &lens.title,
                        egui::FontId::monospace(editor_font_size * 0.8),
                        egui::Color32::from_rgba_unmultiplied(120, 140, 180, 140),
                    );
                }
            }
        }
    }

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

    // Cursor (smooth animation + smooth blink)
    let cursor_vis_info: Option<(f32, f32)> = if word_wrap {
        // Find the wrap row containing the cursor
        visible_wrap_window.iter().enumerate().find_map(|(vis_idx, row)| {
            if row.doc_line == view.cursor.pos.line
                && view.cursor.pos.col >= row.col_start
                && (view.cursor.pos.col < row.col_end
                    || (row.col_end == row.col_start && view.cursor.pos.col == 0)
                    || visible_wrap_window.get(vis_idx + 1).is_none_or(|next| next.doc_line != row.doc_line))
            {
                let col_in_row = view.cursor.pos.col.saturating_sub(row.col_start);
                Some((vis_idx as f32, col_in_row as f32))
            } else {
                None
            }
        })
    } else {
        visible_window
            .iter()
            .position(|&line| line == view.cursor.pos.line)
            .map(|offset| (offset as f32, view.cursor.pos.col as f32))
    };
    if let Some((vis_row, col_offset)) = cursor_vis_info {
        let vis_y = vis_row * line_height;
        let target_x =
            rect.left() + gutter_width + text_margin + col_offset * char_width
                - if word_wrap { 0.0 } else { h_offset };
        let target_y = rect.top() + vis_y;

        // Initialize or lerp the display position
        if !view.cursor_display_init {
            view.cursor_display_x = target_x;
            view.cursor_display_y = target_y;
            view.cursor_display_init = true;
        } else {
            // Lerp toward target (~50ms at 60fps = factor ~0.25 per frame)
            let lerp_factor = 0.25;
            let dx = target_x - view.cursor_display_x;
            let dy = target_y - view.cursor_display_y;
            if dx.abs() < 0.5 && dy.abs() < 0.5 {
                view.cursor_display_x = target_x;
                view.cursor_display_y = target_y;
            } else {
                view.cursor_display_x += dx * lerp_factor;
                view.cursor_display_y += dy * lerp_factor;
                ui.ctx().request_repaint();
            }
        }

        let cursor_x = view.cursor_display_x;
        let cursor_y = view.cursor_display_y;

        if cursor_x >= text_clip.left() && cursor_x <= text_clip.right() {
            // Smooth blink using sin wave for opacity fade
            let time = ui.ctx().input(|i| i.time);
            let blink_cycle = (time * 2.0 * std::f64::consts::PI / 1.2) as f32; // ~1.2s full cycle
            let opacity = (blink_cycle.sin() * 0.5 + 0.5).clamp(0.15, 1.0);
            let alpha = (opacity * 255.0) as u8;
            let cursor_color = egui::Color32::from_rgba_unmultiplied(80, 160, 255, alpha);
            painter.with_clip_rect(text_clip).line_segment(
                [
                    egui::pos2(cursor_x, cursor_y + 1.0),
                    egui::pos2(cursor_x, cursor_y + line_height - 1.0),
                ],
                egui::Stroke::new(2.0, cursor_color),
            );
        }
        ui.ctx().request_repaint();
    }

    // Render extra cursors (multi-cursor, same style)
    for extra in &view.cursor.extra_cursors {
        if let Some(extra_visible_offset) = visible_window
            .iter()
            .position(|&line| line == extra.pos.line)
        {
            let vis_y = extra_visible_offset as f32 * line_height;
            let cursor_x =
                rect.left() + gutter_width + text_margin + extra.pos.col as f32 * char_width
                    - h_offset;
            let cursor_y = rect.top() + vis_y;
            if cursor_x >= text_clip.left() && cursor_x <= text_clip.right() {
                let time = ui.ctx().input(|i| i.time);
                let blink_cycle = (time * 2.0 * std::f64::consts::PI / 1.2) as f32;
                let opacity = (blink_cycle.sin() * 0.5 + 0.5).clamp(0.15, 1.0);
                let alpha = (opacity * 255.0) as u8;
                let cursor_color = egui::Color32::from_rgba_unmultiplied(80, 160, 255, alpha);
                painter.with_clip_rect(text_clip).line_segment(
                    [
                        egui::pos2(cursor_x, cursor_y + 1.0),
                        egui::pos2(cursor_x, cursor_y + line_height - 1.0),
                    ],
                    egui::Stroke::new(2.0, cursor_color),
                );
            }
        }

        // Render extra cursor selections
        if let Some(anchor) = extra.anchor {
            if anchor != extra.pos {
                let (sel_start, sel_end) = if anchor <= extra.pos {
                    (anchor, extra.pos)
                } else {
                    (extra.pos, anchor)
                };
                let sel_color = egui::Color32::from_rgba_unmultiplied(60, 100, 180, 80);
                for (visible_offset, &line_idx) in visible_window.iter().enumerate() {
                    if line_idx < sel_start.line || line_idx > sel_end.line {
                        continue;
                    }
                    let vis_y = visible_offset as f32 * line_height;
                    let line_len = buf.line_len(line_idx);
                    let col_start = if line_idx == sel_start.line { sel_start.col } else { 0 };
                    let col_end = if line_idx == sel_end.line { sel_end.col } else { line_len };
                    let x1 = rect.left() + gutter_width + text_margin + col_start as f32 * char_width - h_offset;
                    let x2 = rect.left() + gutter_width + text_margin + col_end as f32 * char_width - h_offset;
                    let sel_rect = egui::Rect::from_min_max(
                        egui::pos2(x1.max(text_clip.left()), rect.top() + vis_y),
                        egui::pos2(x2.max(x1 + char_width).min(text_clip.right()), rect.top() + vis_y + line_height),
                    );
                    if sel_rect.width() > 0.0 {
                        painter.rect_filled(sel_rect, 0.0, sel_color);
                    }
                }
            }
        }
    }

    // Minimap (right edge, replaces scrollbar) -- disabled for very large files
    let minimap_x = rect.right() - minimap_w;
    if line_count > 1 && perf::minimap_enabled(line_count) {
        // Minimap click-to-scroll
        if response.clicked() {
            if let Some(click_pos) = response.interact_pointer_pos() {
                if click_pos.x >= minimap_x && click_pos.x <= rect.right() {
                    let rel_y = (click_pos.y - rect.top()) / rect.height();
                    let target_line = (rel_y * visible_line_count as f32)
                        .clamp(0.0, max_scroll as f32);
                    // Center the viewport on the clicked position
                    let centered = (target_line - visible_lines as f32 / 2.0).clamp(0.0, max_scroll as f32);
                    view.scroll_target = Some(centered);
                }
            }
        }

        // Background
        painter.rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(minimap_x, rect.top()),
                egui::Vec2::new(minimap_w, rect.height()),
            ),
            0.0,
            egui::Color32::from_rgba_unmultiplied(20, 22, 28, 200),
        );

        // Viewport indicator
        let track_h = rect.height();
        let top_doc_line = visible_doc_lines.get(view.scroll_line).copied().unwrap_or(0);
        let view_top = (top_doc_line as f32 / line_count as f32) * track_h;
        let view_h = (visible_lines as f32 / line_count as f32) * track_h;
        painter.rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(minimap_x, rect.top() + view_top),
                egui::Vec2::new(minimap_w, view_h.max(4.0)),
            ),
            0.0,
            egui::Color32::from_rgba_unmultiplied(80, 120, 200, 30),
        );

        // Line density: draw a tiny colored dot per line (sampled for perf)
        let line_h = (track_h / line_count as f32).max(0.5).min(2.0);
        let sample_step = if line_count > 2000 {
            line_count / 1000
        } else {
            1
        };
        for line_idx in (0..line_count).step_by(sample_step.max(1)) {
            let y = rect.top() + (line_idx as f32 / line_count as f32) * track_h;
            let line_text = buf.line(line_idx);
            if line_text.trim().is_empty() {
                continue;
            }

            // Color from first syntax span if available, else dim white
            let color = if line_idx < syntax_end_line
                && line_idx >= syntax_start_line
            {
                let span_idx = line_idx - syntax_start_line;
                highlight_spans
                    .get(span_idx)
                    .and_then(|spans| spans.first())
                    .map(|s| {
                        let rgb =
                            crate::editor::syntax::group_color_with_overrides(s.group, syntax_colors);
                        egui::Color32::from_rgba_unmultiplied(rgb[0], rgb[1], rgb[2], 120)
                    })
                    .unwrap_or(egui::Color32::from_rgba_unmultiplied(150, 155, 165, 60))
            } else {
                egui::Color32::from_rgba_unmultiplied(150, 155, 165, 40)
            };

            let text_w = (line_text.len() as f32 * 0.4).min(minimap_w - 4.0);
            painter.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(minimap_x + 2.0, y),
                    egui::Vec2::new(text_w, line_h),
                ),
                0.0,
                color,
            );
        }

        // Diagnostic markers in minimap
        if let Some(diags) = diagnostics {
            for diag in diags {
                let dy = rect.top() + (diag.line as f32 / line_count as f32) * track_h;
                let color = match diag.severity {
                    DiagSeverity::Error => egui::Color32::from_rgb(255, 80, 80),
                    DiagSeverity::Warning => egui::Color32::from_rgb(230, 180, 50),
                    _ => continue,
                };
                painter.rect_filled(
                    egui::Rect::from_min_size(egui::pos2(minimap_x, dy), egui::Vec2::new(3.0, 2.0)),
                    0.0,
                    color,
                );
            }
        }
    }

    // Hover tooltip
    if let Some(hover) = hover_text {
        if let Some(cursor_visible_offset) = visible_window
            .iter()
            .position(|&line| line == view.cursor.pos.line)
        {
            let vis_y = cursor_visible_offset as f32 * line_height;
            let tooltip_x =
                rect.left() + gutter_width + text_margin + view.cursor.pos.col as f32 * char_width
                    - h_offset;
            let tooltip_y = rect.top() + vis_y - 4.0; // Above the line

            // Render tooltip background + text
            let max_w = (rect.width() - gutter_width - 40.0).max(200.0);
            let lines: Vec<&str> = hover.lines().take(12).collect(); // Cap at 12 lines
            let tooltip_h = lines.len() as f32 * 16.0 + 8.0;
            let tooltip_y = if tooltip_y - tooltip_h < rect.top() {
                // Show below cursor if no room above
                rect.top() + vis_y + line_height + 4.0
            } else {
                tooltip_y - tooltip_h
            };

            let bg_rect = egui::Rect::from_min_size(
                egui::pos2(tooltip_x.max(rect.left() + gutter_width), tooltip_y),
                egui::Vec2::new(max_w, tooltip_h),
            );
            painter.rect_filled(bg_rect, 4.0, egui::Color32::from_rgb(40, 42, 54));
            painter.rect_stroke(
                bg_rect,
                4.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 85, 100)),
            );

            for (i, line) in lines.iter().enumerate() {
                painter.text(
                    egui::pos2(bg_rect.left() + 6.0, bg_rect.top() + 4.0 + i as f32 * 16.0),
                    egui::Align2::LEFT_TOP,
                    line,
                    egui::FontId::monospace(12.0),
                    egui::Color32::from_rgb(200, 205, 215),
                );
            }
        }
    }

    // Signature help tooltip (shown above cursor)
    if let Some(sig) = signature_help {
        if let Some(cursor_visible_offset) = visible_window
            .iter()
            .position(|&line| line == view.cursor.pos.line)
        {
            let vis_y = cursor_visible_offset as f32 * line_height;
            let sig_x =
                rect.left() + gutter_width + text_margin + view.cursor.pos.col as f32 * char_width
                    - h_offset;
            let sig_y = rect.top() + vis_y - 4.0;

            // Build the label with the active parameter highlighted
            let label = &sig.label;
            let sig_h = 20.0;
            let sig_y = if sig_y - sig_h < rect.top() {
                rect.top() + vis_y + line_height + 4.0
            } else {
                sig_y - sig_h
            };

            let max_w = (rect.width() - gutter_width - 40.0).max(200.0);
            let bg_rect = egui::Rect::from_min_size(
                egui::pos2(sig_x.max(rect.left() + gutter_width), sig_y),
                egui::Vec2::new(max_w, sig_h),
            );
            painter.rect_filled(bg_rect, 4.0, egui::Color32::from_rgb(35, 38, 52));
            painter.rect_stroke(bg_rect, 4.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 75, 95)));

            // Render label, highlighting the active parameter
            if sig.active_parameter < sig.parameters.len() {
                let active_param = &sig.parameters[sig.active_parameter];
                if let Some(start) = label.find(active_param) {
                    let before = &label[..start];
                    let after = &label[start + active_param.len()..];

                    let x = bg_rect.left() + 6.0;
                    let dim_color = egui::Color32::from_rgb(170, 175, 190);
                    let highlight_color = egui::Color32::from_rgb(255, 220, 100);
                    let sig_font = egui::FontId::monospace(12.0);

                    painter.text(egui::pos2(x, bg_rect.top() + 3.0), egui::Align2::LEFT_TOP, before, sig_font.clone(), dim_color);
                    let before_w = before.len() as f32 * 7.2;
                    painter.text(egui::pos2(x + before_w, bg_rect.top() + 3.0), egui::Align2::LEFT_TOP, active_param, sig_font.clone(), highlight_color);
                    let param_w = active_param.len() as f32 * 7.2;
                    painter.text(egui::pos2(x + before_w + param_w, bg_rect.top() + 3.0), egui::Align2::LEFT_TOP, after, sig_font, dim_color);
                } else {
                    painter.text(
                        egui::pos2(bg_rect.left() + 6.0, bg_rect.top() + 3.0),
                        egui::Align2::LEFT_TOP,
                        label,
                        egui::FontId::monospace(12.0),
                        egui::Color32::from_rgb(200, 205, 215),
                    );
                }
            } else {
                painter.text(
                    egui::pos2(bg_rect.left() + 6.0, bg_rect.top() + 3.0),
                    egui::Align2::LEFT_TOP,
                    label,
                    egui::FontId::monospace(12.0),
                    egui::Color32::from_rgb(200, 205, 215),
                );
            }
        }
    }

    // Completion popup
    if let Some((items, selected)) = completions {
        if !items.is_empty()
            && visible_window
                .iter()
                .any(|&line| line == view.cursor.pos.line)
        {
            let vis_y = visible_window
                .iter()
                .position(|&line| line == view.cursor.pos.line)
                .unwrap_or(0) as f32
                * line_height;
            let popup_x =
                rect.left() + gutter_width + text_margin + view.cursor.pos.col as f32 * char_width
                    - h_offset;
            let popup_y = rect.top() + vis_y + line_height + 2.0;

            let item_h = 20.0;
            let popup_w = 320.0;
            let popup_h = (items.len() as f32 * item_h).min(200.0) + 4.0;

            // Clamp to screen
            let popup_x = popup_x
                .min(rect.right() - popup_w - 4.0)
                .max(rect.left() + gutter_width);

            let bg = egui::Rect::from_min_size(
                egui::pos2(popup_x, popup_y),
                egui::Vec2::new(popup_w, popup_h),
            );
            painter.rect_filled(bg, 4.0, egui::Color32::from_rgb(30, 32, 42));
            painter.rect_stroke(
                bg,
                4.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 65, 80)),
            );

            for (i, item) in items.iter().enumerate() {
                let y = popup_y + 2.0 + i as f32 * item_h;
                if y + item_h > popup_y + popup_h {
                    break;
                }

                // Selected item highlight
                if i == selected {
                    painter.rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(popup_x + 2.0, y),
                            egui::Vec2::new(popup_w - 4.0, item_h),
                        ),
                        2.0,
                        egui::Color32::from_rgb(50, 80, 130),
                    );
                }

                // Kind icon
                let kind_char = match item.kind {
                    Some(lsp_types::CompletionItemKind::FUNCTION)
                    | Some(lsp_types::CompletionItemKind::METHOD) => "f",
                    Some(lsp_types::CompletionItemKind::VARIABLE) => "v",
                    Some(lsp_types::CompletionItemKind::CLASS)
                    | Some(lsp_types::CompletionItemKind::STRUCT) => "S",
                    Some(lsp_types::CompletionItemKind::MODULE) => "M",
                    Some(lsp_types::CompletionItemKind::KEYWORD) => "k",
                    Some(lsp_types::CompletionItemKind::FIELD)
                    | Some(lsp_types::CompletionItemKind::PROPERTY) => "p",
                    Some(lsp_types::CompletionItemKind::CONSTANT) => "C",
                    Some(lsp_types::CompletionItemKind::ENUM_MEMBER) => "e",
                    Some(lsp_types::CompletionItemKind::INTERFACE) => "I",
                    Some(lsp_types::CompletionItemKind::TYPE_PARAMETER) => "T",
                    _ => " ",
                };
                painter.text(
                    egui::pos2(popup_x + 6.0, y + 2.0),
                    egui::Align2::LEFT_TOP,
                    kind_char,
                    egui::FontId::monospace(11.0),
                    egui::Color32::from_rgb(120, 130, 160),
                );

                // Label
                let label_color = if i == selected {
                    egui::Color32::WHITE
                } else {
                    egui::Color32::from_rgb(200, 205, 215)
                };
                painter.text(
                    egui::pos2(popup_x + 22.0, y + 2.0),
                    egui::Align2::LEFT_TOP,
                    &item.label,
                    egui::FontId::monospace(12.0),
                    label_color,
                );

                // Detail (right-aligned, dimmed)
                if let Some(detail) = &item.detail {
                    let short = if detail.len() > 30 {
                        &detail[..30]
                    } else {
                        detail
                    };
                    painter.text(
                        egui::pos2(popup_x + popup_w - 8.0, y + 2.0),
                        egui::Align2::RIGHT_TOP,
                        short,
                        egui::FontId::monospace(10.0),
                        egui::Color32::from_rgb(100, 105, 120),
                    );
                }
            }
        }
    }

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

#[expect(
    clippy::too_many_arguments,
    reason = "layout geometry must be passed explicitly"
)]
fn render_bracket_match(
    painter: &egui::Painter,
    text_clip: egui::Rect,
    bracket_match: Option<(Position, Position)>,
    visible_window: &[usize],
    rect: egui::Rect,
    gutter_width: f32,
    text_margin: f32,
    char_width: f32,
    line_height: f32,
    h_offset: f32,
) {
    let Some((a, b)) = bracket_match else { return };
    let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(120, 170, 255));
    let fill = egui::Color32::from_rgba_unmultiplied(120, 170, 255, 35);

    for pos in [a, b] {
        let Some(visible_offset) = visible_window.iter().position(|&line| line == pos.line) else {
            continue;
        };
        let y = rect.top() + visible_offset as f32 * line_height;
        let x = rect.left() + gutter_width + text_margin + pos.col as f32 * char_width - h_offset;
        let bracket_rect = egui::Rect::from_min_size(
            egui::pos2(x, y + 1.0),
            egui::Vec2::new(char_width, line_height - 2.0),
        );
        if bracket_rect.intersects(text_clip) {
            painter.with_clip_rect(text_clip).rect_filled(bracket_rect, 1.0, fill);
            painter.with_clip_rect(text_clip).rect_stroke(bracket_rect, 1.0, stroke);
        }
    }
}

/// Render indentation guides for the visible editor rows.
#[expect(
    clippy::too_many_arguments,
    reason = "layout geometry must be passed explicitly"
)]
fn render_indentation_guides(
    painter: &egui::Painter,
    text_clip: egui::Rect,
    buf: &crate::editor::buffer::Buffer,
    visible_window: &[usize],
    active_indent_level: usize,
    rect: egui::Rect,
    gutter_width: f32,
    text_margin: f32,
    char_width: f32,
    line_height: f32,
    h_offset: f32,
) {
    let indent_width = buf.indent_style.width().max(1);
    let guide_color = egui::Color32::from_rgba_unmultiplied(190, 200, 220, 26);
    let active_guide_color = egui::Color32::from_rgba_unmultiplied(120, 170, 255, 82);

    for (visible_offset, &line_idx) in visible_window.iter().enumerate() {
        let line = buf.line(line_idx);
        let level = indent_level(line, buf.indent_style);
        if level == 0 {
            continue;
        }

        let y1 = rect.top() + visible_offset as f32 * line_height;
        let y2 = y1 + line_height;

        for guide_level in 1..=level {
            let col = guide_level * indent_width;
            let x = rect.left() + gutter_width + text_margin + col as f32 * char_width - h_offset;
            if x < text_clip.left() || x > text_clip.right() {
                continue;
            }

            let color = if guide_level == active_indent_level {
                active_guide_color
            } else {
                guide_color
            };
            painter.with_clip_rect(text_clip).line_segment(
                [egui::pos2(x, y1), egui::pos2(x, y2)],
                egui::Stroke::new(1.0, color),
            );
        }
    }
}

fn indent_level(line: &str, style: IndentStyle) -> usize {
    let width = style.width().max(1);
    indentation_columns(line, style) / width
}

fn indentation_columns(line: &str, style: IndentStyle) -> usize {
    let width = style.width().max(1);
    let mut columns = 0;
    for ch in line.chars() {
        match ch {
            ' ' => columns += 1,
            '\t' => columns += width,
            _ => break,
        }
    }
    columns
}

fn apply_folding_actions(
    action: &KeyAction,
    view: &mut BufferView,
    foldable_ranges: &[FoldRange],
    line_count: usize,
) {
    if action.fold_all {
        view.folded_ranges = top_level_fold_ranges(foldable_ranges);
    } else if action.unfold_all {
        view.folded_ranges.clear();
    } else if action.fold_current {
        if let Some(range) = best_fold_range_containing(foldable_ranges, view.cursor.pos.line) {
            add_fold_range(&mut view.folded_ranges, range);
        }
    } else if action.unfold_current {
        unfold_at_line(&mut view.folded_ranges, view.cursor.pos.line);
    }

    view.folded_ranges
        .retain(|range| range.start_line < range.end_line && range.end_line < line_count);
    view.folded_ranges.sort_by_key(|range| (range.start_line, range.end_line));
    view.folded_ranges.dedup();
}

fn top_level_fold_ranges(foldable_ranges: &[FoldRange]) -> Vec<FoldRange> {
    let mut ranges = Vec::new();
    let mut sorted = foldable_ranges.to_vec();
    sorted.sort_by_key(|range| (range.start_line, std::cmp::Reverse(range.end_line)));
    for range in sorted {
        if ranges
            .iter()
            .any(|parent: &FoldRange| parent.start_line <= range.start_line && parent.end_line >= range.end_line)
        {
            continue;
        }
        ranges.push(range);
    }
    ranges
}

fn visible_doc_lines(line_count: usize, folded_ranges: &[FoldRange]) -> Vec<usize> {
    let mut lines = Vec::with_capacity(line_count);
    let mut line = 0usize;
    while line < line_count {
        lines.push(line);
        if let Some(range) = folded_range_starting_at(folded_ranges, line) {
            line = range.end_line.saturating_add(1);
        } else {
            line += 1;
        }
    }
    if lines.is_empty() {
        lines.push(0);
    }
    lines
}

fn visible_index_for_doc_line(visible_doc_lines: &[usize], doc_line: usize) -> usize {
    match visible_doc_lines.binary_search(&doc_line) {
        Ok(idx) => idx,
        Err(idx) => idx.saturating_sub(1).min(visible_doc_lines.len().saturating_sub(1)),
    }
}

fn snap_cursor_to_visible_line(view: &mut BufferView, buf: &crate::editor::buffer::Buffer) {
    for range in &view.folded_ranges {
        if view.cursor.pos.line > range.start_line && view.cursor.pos.line <= range.end_line {
            view.cursor.pos.line = range.start_line;
            view.cursor.pos.col = view.cursor.pos.col.min(buf.line_len(range.start_line));
            view.cursor.clear_selection();
            return;
        }
    }
}

fn best_fold_range_containing(ranges: &[FoldRange], line: usize) -> Option<FoldRange> {
    ranges
        .iter()
        .copied()
        .filter(|range| range.start_line <= line && line < range.end_line)
        .min_by_key(|range| range.end_line - range.start_line)
}

fn best_fold_range_starting_at(ranges: &[FoldRange], line: usize) -> Option<FoldRange> {
    ranges
        .iter()
        .copied()
        .filter(|range| range.start_line == line)
        .min_by_key(|range| range.end_line - range.start_line)
}

fn add_fold_range(folded_ranges: &mut Vec<FoldRange>, range: FoldRange) {
    if range.start_line >= range.end_line {
        return;
    }
    if !folded_ranges.iter().any(|existing| *existing == range) {
        folded_ranges.push(range);
    }
}

fn unfold_at_line(folded_ranges: &mut Vec<FoldRange>, line: usize) {
    if let Some(idx) = folded_ranges
        .iter()
        .position(|range| range.start_line <= line && line <= range.end_line)
    {
        folded_ranges.remove(idx);
    }
}

fn toggle_fold_range(folded_ranges: &mut Vec<FoldRange>, range: FoldRange) {
    if let Some(idx) = folded_ranges.iter().position(|existing| *existing == range) {
        folded_ranges.remove(idx);
    } else {
        add_fold_range(folded_ranges, range);
    }
}

fn folded_range_starting_at(folded_ranges: &[FoldRange], line: usize) -> Option<FoldRange> {
    folded_ranges
        .iter()
        .copied()
        .find(|range| range.start_line == line)
}

fn is_range_folded(folded_ranges: &[FoldRange], line: usize) -> bool {
    folded_ranges.iter().any(|range| range.start_line == line)
}

/// A visual row in word-wrap mode. Maps a visual row index to a doc line and column range.
#[derive(Clone, Copy, Debug)]
struct WrapRow {
    /// The document line this visual row belongs to.
    doc_line: usize,
    /// The character column where this visual row starts.
    col_start: usize,
    /// The character column where this visual row ends (exclusive).
    col_end: usize,
    /// Whether this is the first visual row of the doc line (shows line number).
    is_first: bool,
}

/// Compute visual wrap rows for a set of visible document lines.
fn compute_wrap_rows(
    visible_doc_lines: &[usize],
    buf: &crate::editor::buffer::Buffer,
    visible_cols: usize,
) -> Vec<WrapRow> {
    let wrap_col = visible_cols.max(10);
    let mut rows = Vec::new();
    for &doc_line in visible_doc_lines {
        let line_len = buf.line_len(doc_line);
        if line_len == 0 {
            rows.push(WrapRow {
                doc_line,
                col_start: 0,
                col_end: 0,
                is_first: true,
            });
            continue;
        }
        let mut col = 0;
        let mut first = true;
        while col < line_len {
            let end = (col + wrap_col).min(line_len);
            rows.push(WrapRow {
                doc_line,
                col_start: col,
                col_end: end,
                is_first: first,
            });
            first = false;
            col = end;
        }
    }
    rows
}

/// Find the visual row index for a given cursor position in wrap mode.
fn wrap_row_for_cursor(rows: &[WrapRow], line: usize, col: usize) -> usize {
    for (i, row) in rows.iter().enumerate() {
        if row.doc_line == line && col >= row.col_start && (col < row.col_end || (row.col_end == row.col_start && col == 0)) {
            return i;
        }
        // Cursor at end of line: last row of that line
        if row.doc_line == line && col >= row.col_end {
            // Check if next row is same line
            if rows.get(i + 1).is_none_or(|next| next.doc_line != line) {
                return i;
            }
        }
    }
    rows.len().saturating_sub(1)
}

/// Convert a pixel position to (doc_line, doc_col) in word-wrap mode.
fn pixel_to_editor_pos_wrapped(
    pos: egui::Pos2,
    rect: egui::Rect,
    gutter_width: f32,
    text_margin: f32,
    char_width: f32,
    line_height: f32,
    scroll_row: usize,
    wrap_rows: &[WrapRow],
    buf: &crate::editor::buffer::Buffer,
) -> (usize, usize) {
    let rel_x = pos.x - rect.left() - gutter_width - text_margin;
    let rel_y = pos.y - rect.top();
    let visual_row = (scroll_row + (rel_y / line_height).max(0.0) as usize)
        .min(wrap_rows.len().saturating_sub(1));
    let row = wrap_rows.get(visual_row).copied().unwrap_or(WrapRow {
        doc_line: 0,
        col_start: 0,
        col_end: 0,
        is_first: true,
    });
    let col_in_row = (rel_x / char_width).max(0.0) as usize;
    let doc_col = (row.col_start + col_in_row).min(buf.line_len(row.doc_line));
    (row.doc_line, doc_col)
}

#[expect(
    clippy::too_many_arguments,
    reason = "layout geometry must be passed explicitly"
)]
fn render_rulers(
    painter: &egui::Painter,
    text_clip: egui::Rect,
    rulers: &[usize],
    rect: egui::Rect,
    gutter_width: f32,
    text_margin: f32,
    char_width: f32,
    h_offset: f32,
) {
    let color = egui::Color32::from_rgba_unmultiplied(220, 220, 235, 28);
    for &column in rulers {
        let x = rect.left() + gutter_width + text_margin + column as f32 * char_width - h_offset;
        if x < text_clip.left() || x > text_clip.right() {
            continue;
        }
        painter.with_clip_rect(text_clip).line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(1.0, color),
        );
    }
}

/// Render a single line with syntax-colored spans.
fn render_highlighted_line(
    painter: &egui::Painter,
    clip: egui::Rect,
    spans: &[crate::editor::syntax::HighlightSpan],
    syntax_colors: &HashMap<HighlightGroup, [u8; 3]>,
    line_text: &str,
    text_x_base: f32,
    y: f32,
    char_width: f32,
    font: &egui::FontId,
    default_color: egui::Color32,
) {
    let chars: Vec<char> = line_text.chars().collect();
    let mut col = 0;
    while col < chars.len() {
        let color = spans
            .iter()
            .find(|s| col >= s.col_start && col < s.col_end)
            .map(|s| {
                let rgb = crate::editor::syntax::group_color_with_overrides(s.group, syntax_colors);
                egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2])
            })
            .unwrap_or(default_color);

        let span_end = spans
            .iter()
            .find(|s| col >= s.col_start && col < s.col_end)
            .map(|s| s.col_end.min(chars.len()))
            .unwrap_or(chars.len());

        let mut batch_end = col + 1;
        while batch_end < span_end {
            let next_color = spans
                .iter()
                .find(|s| batch_end >= s.col_start && batch_end < s.col_end)
                .map(|s| {
                    let rgb =
                        crate::editor::syntax::group_color_with_overrides(s.group, syntax_colors);
                    egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2])
                })
                .unwrap_or(default_color);
            if next_color != color {
                break;
            }
            batch_end += 1;
        }

        let chunk: String = chars[col..batch_end].iter().collect();
        let x = text_x_base + col as f32 * char_width;
        painter.with_clip_rect(clip).text(
            egui::pos2(x, y + 1.0),
            egui::Align2::LEFT_TOP,
            &chunk,
            font.clone(),
            color,
        );
        col = batch_end;
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "layout geometry must be passed explicitly"
)]
fn render_visible_whitespace_line(
    painter: &egui::Painter,
    clip: egui::Rect,
    spans: &[crate::editor::syntax::HighlightSpan],
    syntax_colors: &HashMap<HighlightGroup, [u8; 3]>,
    line_text: &str,
    text_x_base: f32,
    y: f32,
    char_width: f32,
    font: &egui::FontId,
    default_color: egui::Color32,
) {
    let whitespace_color = egui::Color32::from_rgb(85, 92, 112);
    for (col, ch) in line_text.chars().enumerate() {
        let (display, color) = match ch {
            ' ' => ("·", whitespace_color),
            '\t' => ("→", whitespace_color),
            _ => {
                let color = spans
                    .iter()
                    .find(|s| col >= s.col_start && col < s.col_end)
                    .map(|s| {
                        let rgb =
                            crate::editor::syntax::group_color_with_overrides(s.group, syntax_colors);
                        egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2])
                    })
                    .unwrap_or(default_color);
                let x = text_x_base + col as f32 * char_width;
                painter.with_clip_rect(clip).text(
                    egui::pos2(x, y + 1.0),
                    egui::Align2::LEFT_TOP,
                    ch.to_string(),
                    font.clone(),
                    color,
                );
                continue;
            }
        };

        let x = text_x_base + col as f32 * char_width;
        painter.with_clip_rect(clip).text(
            egui::pos2(x, y + 1.0),
            egui::Align2::LEFT_TOP,
            display,
            font.clone(),
            color,
        );
    }
}

/// Render diagnostic underlines and gutter markers.
#[expect(
    clippy::too_many_arguments,
    reason = "layout geometry must be passed explicitly"
)]
fn render_diagnostics(
    painter: &egui::Painter,
    text_clip: egui::Rect,
    diagnostics: Option<&[FileDiagnostic]>,
    visible_window: &[usize],
    rect: egui::Rect,
    gutter_width: f32,
    text_margin: f32,
    char_width: f32,
    line_height: f32,
    h_offset: f32,
) {
    let Some(diags) = diagnostics else { return };
    for diag in diags {
        let Some(visible_offset) = visible_window
            .iter()
            .position(|&line| line == diag.line as usize)
        else {
            continue;
        };
        let vis_y = visible_offset as f32 * line_height;
        let y_base = rect.top() + vis_y + line_height - 2.0;
        let x_start =
            rect.left() + gutter_width + text_margin + diag.col as f32 * char_width - h_offset;
        let x_end =
            rect.left() + gutter_width + text_margin + diag.end_col as f32 * char_width - h_offset;
        let width = (x_end - x_start).max(char_width);

        let color = match diag.severity {
            DiagSeverity::Error => egui::Color32::from_rgb(255, 80, 80),
            DiagSeverity::Warning => egui::Color32::from_rgb(230, 180, 50),
            DiagSeverity::Info => egui::Color32::from_rgb(80, 160, 255),
            DiagSeverity::Hint => egui::Color32::from_rgb(130, 130, 150),
        };

        // Squiggly underline
        let segments = ((width / 4.0) as usize).max(2);
        let seg_w = width / segments as f32;
        for i in 0..segments {
            let sx = x_start + i as f32 * seg_w;
            let offset = if i % 2 == 0 { 0.0 } else { 2.0 };
            painter.with_clip_rect(text_clip).line_segment(
                [
                    egui::pos2(sx, y_base + offset),
                    egui::pos2(sx + seg_w, y_base + 2.0 - offset),
                ],
                egui::Stroke::new(1.0, color),
            );
        }

        // Gutter marker
        let marker = match diag.severity {
            DiagSeverity::Error => "E",
            DiagSeverity::Warning => "W",
            DiagSeverity::Info => "i",
            DiagSeverity::Hint => ".",
        };
        painter.text(
            egui::pos2(rect.left() + 1.0, rect.top() + vis_y + 1.0),
            egui::Align2::LEFT_TOP,
            marker,
            egui::FontId::monospace(10.0),
            color,
        );
    }
}

/// Convert a pixel position to an editor (line, col) position.
fn pixel_to_editor_pos(
    pos: egui::Pos2,
    rect: egui::Rect,
    gutter_width: f32,
    text_margin: f32,
    h_offset: f32,
    char_width: f32,
    line_height: f32,
    scroll_line: usize,
    visible_doc_lines: &[usize],
    buf: &crate::editor::buffer::Buffer,
) -> (usize, usize) {
    let rel_x = pos.x - rect.left() - gutter_width - text_margin + h_offset;
    let rel_y = pos.y - rect.top();
    let visible_line =
        (scroll_line + (rel_y / line_height).max(0.0) as usize).min(visible_doc_lines.len().saturating_sub(1));
    let line = visible_doc_lines.get(visible_line).copied().unwrap_or(0);
    let col = (rel_x / char_width).max(0.0) as usize;
    let col = col.min(buf.line_len(line));
    (line, col)
}

/// Render the find/replace bar. Returns the height consumed.
fn render_search_bar(
    ui: &mut egui::Ui,
    search: &mut EditorSearch,
    buf: &mut crate::editor::buffer::Buffer,
    view: &mut BufferView,
) -> f32 {
    let bar_bg = egui::Color32::from_rgb(35, 37, 48);
    let btn_color = egui::Color32::from_rgb(100, 180, 255);
    let toggle_on = egui::Color32::from_rgb(60, 100, 180);
    let toggle_off = egui::Color32::from_rgb(50, 52, 62);
    let status_color = egui::Color32::from_rgb(150, 155, 170);

    let mut total_h = 0.0;

    // Find row
    let find_response = egui::Frame::none()
        .fill(bar_bg)
        .inner_margin(egui::Margin::symmetric(8.0, 4.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Find:").size(12.0).color(egui::Color32::from_rgb(180, 185, 200)));

                let mut query = search.query.clone();
                let response = ui.add(
                    egui::TextEdit::singleline(&mut query)
                        .desired_width(ui.available_width() - 280.0)
                        .hint_text("Search...")
                        .text_color(egui::Color32::WHITE)
                        .font(egui::TextStyle::Monospace),
                );
                if !search.replace_mode || response.has_focus() {
                    response.request_focus();
                }
                if query != search.query {
                    search.query = query;
                    search.update_matches(buf);
                    search.focus_nearest(view.cursor.pos);
                }

                // Toggle buttons
                let case_label = if search.case_sensitive { "Aa" } else { "Aa" };
                let case_bg = if search.case_sensitive { toggle_on } else { toggle_off };
                if ui.add(egui::Button::new(egui::RichText::new(case_label).size(11.0).color(egui::Color32::WHITE)).fill(case_bg).min_size(egui::Vec2::new(28.0, 20.0))).on_hover_text("Case sensitive").clicked() {
                    search.case_sensitive = !search.case_sensitive;
                    search.update_matches(buf);
                    search.focus_nearest(view.cursor.pos);
                }
                let word_bg = if search.whole_word { toggle_on } else { toggle_off };
                if ui.add(egui::Button::new(egui::RichText::new("W").size(11.0).color(egui::Color32::WHITE)).fill(word_bg).min_size(egui::Vec2::new(28.0, 20.0))).on_hover_text("Whole word").clicked() {
                    search.whole_word = !search.whole_word;
                    search.update_matches(buf);
                    search.focus_nearest(view.cursor.pos);
                }
                let regex_bg = if search.regex_mode { toggle_on } else { toggle_off };
                if ui.add(egui::Button::new(egui::RichText::new(".*").size(11.0).color(egui::Color32::WHITE)).fill(regex_bg).min_size(egui::Vec2::new(28.0, 20.0))).on_hover_text("Regex").clicked() {
                    search.regex_mode = !search.regex_mode;
                    search.update_matches(buf);
                    search.focus_nearest(view.cursor.pos);
                }

                // Status
                ui.label(egui::RichText::new(search.status()).size(11.0).color(status_color));

                // Nav buttons
                if ui.add(egui::Button::new(egui::RichText::new("<").size(12.0).color(btn_color)).fill(egui::Color32::TRANSPARENT).min_size(egui::Vec2::new(24.0, 20.0))).on_hover_text("Previous (Shift+Enter)").clicked() {
                    if let Some(pos) = search.prev() {
                        view.cursor.pos = pos;
                        view.cursor.clear_selection();
                        view.cursor.desired_col = None;
                    }
                }
                if ui.add(egui::Button::new(egui::RichText::new(">").size(12.0).color(btn_color)).fill(egui::Color32::TRANSPARENT).min_size(egui::Vec2::new(24.0, 20.0))).on_hover_text("Next (Enter)").clicked() {
                    if let Some(pos) = search.next() {
                        view.cursor.pos = pos;
                        view.cursor.clear_selection();
                        view.cursor.desired_col = None;
                    }
                }

                // Close button
                if ui.add(egui::Button::new(egui::RichText::new("x").size(12.0).color(egui::Color32::from_rgb(180, 180, 190))).fill(egui::Color32::TRANSPARENT).min_size(egui::Vec2::new(20.0, 20.0))).on_hover_text("Close (Escape)").clicked() {
                    search.close();
                }
            });
        });
    total_h += find_response.response.rect.height();

    // Replace row (when in replace mode)
    if search.replace_mode && search.active {
        let replace_response = egui::Frame::none()
            .fill(bar_bg)
            .inner_margin(egui::Margin::symmetric(8.0, 4.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Replace:").size(12.0).color(egui::Color32::from_rgb(180, 185, 200)));

                    let mut replacement = search.replacement.clone();
                    ui.add(
                        egui::TextEdit::singleline(&mut replacement)
                            .desired_width(ui.available_width() - 180.0)
                            .hint_text("Replace with...")
                            .text_color(egui::Color32::WHITE)
                            .font(egui::TextStyle::Monospace),
                    );
                    if replacement != search.replacement {
                        search.replacement = replacement;
                    }

                    // Replace current
                    if ui.add(egui::Button::new(egui::RichText::new("Replace").size(11.0).color(btn_color)).fill(toggle_off).min_size(egui::Vec2::new(56.0, 20.0))).clicked() {
                        if let Some(pos) = search.replace_current(buf) {
                            view.cursor.pos = pos;
                            view.cursor.clear_selection();
                            view.cursor.desired_col = None;
                            view.tree_dirty = true;
                        }
                    }

                    // Replace all
                    if ui.add(egui::Button::new(egui::RichText::new("Replace All").size(11.0).color(btn_color)).fill(toggle_off).min_size(egui::Vec2::new(80.0, 20.0))).clicked() {
                        let count = search.replace_all(buf);
                        if count > 0 {
                            view.tree_dirty = true;
                        }
                    }
                });
            });
        total_h += replace_response.response.rect.height();
    }

    // Handle keyboard shortcuts in the search bar
    let ctx = ui.ctx().clone();
    ctx.input(|input| {
        if input.key_pressed(egui::Key::Escape) {
            search.close();
        } else if input.key_pressed(egui::Key::Enter) && !input.modifiers.shift {
            if let Some(pos) = search.next() {
                view.cursor.pos = pos;
                view.cursor.clear_selection();
                view.cursor.desired_col = None;
            }
        } else if input.key_pressed(egui::Key::Enter) && input.modifiers.shift {
            if let Some(pos) = search.prev() {
                view.cursor.pos = pos;
                view.cursor.clear_selection();
                view.cursor.desired_col = None;
            }
        }
    });

    total_h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indentation_columns_counts_spaces() {
        assert_eq!(
            indentation_columns("        value", IndentStyle::Spaces(4)),
            8
        );
    }

    #[test]
    fn indentation_columns_counts_tabs_as_indent_width() {
        assert_eq!(indentation_columns("\t\tvalue", IndentStyle::Spaces(4)), 8);
        assert_eq!(indentation_columns("\t\tvalue", IndentStyle::Tabs), 2);
    }

    #[test]
    fn indent_level_uses_detected_style_width() {
        assert_eq!(indent_level("    value", IndentStyle::Spaces(2)), 2);
        assert_eq!(indent_level("    value", IndentStyle::Spaces(4)), 1);
        assert_eq!(indent_level("\t\tvalue", IndentStyle::Tabs), 2);
    }

    #[test]
    fn indent_level_ignores_non_indented_lines() {
        assert_eq!(indent_level("value", IndentStyle::Spaces(4)), 0);
    }

    #[test]
    fn visible_doc_lines_skips_folded_interiors() {
        let folds = vec![FoldRange {
            start_line: 1,
            end_line: 3,
        }];
        assert_eq!(visible_doc_lines(6, &folds), vec![0, 1, 4, 5]);
    }

    #[test]
    fn visible_index_for_hidden_line_snaps_to_fold_start() {
        let visible = vec![0, 1, 4, 5];
        assert_eq!(visible_index_for_doc_line(&visible, 3), 1);
    }

    #[test]
    fn fold_current_uses_innermost_range() {
        let mut view = BufferView::default();
        view.cursor.pos = Position::new(3, 0);
        let ranges = vec![
            FoldRange {
                start_line: 0,
                end_line: 10,
            },
            FoldRange {
                start_line: 2,
                end_line: 4,
            },
        ];
        let action = KeyAction {
            fold_current: true,
            ..KeyAction::default()
        };
        apply_folding_actions(&action, &mut view, &ranges, 12);
        assert_eq!(
            view.folded_ranges,
            vec![FoldRange {
                start_line: 2,
                end_line: 4
            }]
        );
    }
}
