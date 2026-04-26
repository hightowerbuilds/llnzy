use crate::editor::buffer::Position;
use crate::editor::keymap::{handle_editor_keys, KeyAction};
use crate::editor::perf;
use crate::editor::syntax::SyntaxEngine;
use crate::editor::BufferView;
use crate::lsp::{DiagSeverity, FileDiagnostic};

/// Result from rendering that the host needs to act on.
#[derive(Default)]
pub(crate) struct EditorFrameResult {
    pub key_action: KeyAction,
}

/// Render the code editor for a text buffer.
pub(crate) fn render_text_editor(
    ui: &mut egui::Ui,
    buf: &mut crate::editor::buffer::Buffer,
    view: &mut BufferView,
    syntax: &SyntaxEngine,
    diagnostics: Option<&[FileDiagnostic]>,
    hover_text: Option<&str>,
    completions: Option<(&[&crate::lsp::CompletionItem], usize)>,
    status_msg: &mut Option<String>,
    clipboard_out: &mut Option<String>,
    clipboard_in: &mut Option<String>,
) -> EditorFrameResult {
    let mut result = EditorFrameResult::default();

    let line_count = buf.line_count();
    let gutter_digits = ((line_count as f64).log10().floor() as usize + 1).max(2);
    let char_width = 8.0;
    let line_height = 18.0;
    let gutter_width = (gutter_digits as f32 + 1.5) * char_width;
    let text_margin = 2.0;

    // Handle keyboard input (may modify buffer)
    let content_before = buf.len_chars();
    let ctx = ui.ctx().clone();
    let completion_active = completions.is_some();
    result.key_action = handle_editor_keys(&ctx, buf, view, status_msg, clipboard_out, clipboard_in, line_height, completion_active);
    if buf.len_chars() != content_before {
        view.tree_dirty = true;
    }

    view.cursor.clamp(buf);

    // Vertical scroll
    let available_h = ui.available_height() - 20.0;
    let visible_lines = (available_h / line_height).max(1.0) as usize;
    if view.cursor.pos.line < view.scroll_line {
        view.scroll_line = view.cursor.pos.line;
    } else if view.cursor.pos.line >= view.scroll_line + visible_lines {
        view.scroll_line = view.cursor.pos.line.saturating_sub(visible_lines - 1);
    }

    // Horizontal scroll
    let text_area_w = ui.available_width() - gutter_width - text_margin;
    let visible_cols = (text_area_w / char_width).max(1.0) as usize;
    let margin_cols = 4;
    if view.cursor.pos.col < view.scroll_col {
        view.scroll_col = view.cursor.pos.col;
    } else if view.cursor.pos.col >= view.scroll_col + visible_cols.saturating_sub(margin_cols) {
        view.scroll_col = view.cursor.pos.col.saturating_sub(visible_cols.saturating_sub(margin_cols));
    }

    let end_line = (view.scroll_line + visible_lines + 2).min(line_count);
    let h_offset = view.scroll_col as f32 * char_width;

    // Status bar text
    let indent_label = match buf.indent_style {
        crate::editor::buffer::IndentStyle::Spaces(n) => format!("Spaces: {n}"),
        crate::editor::buffer::IndentStyle::Tabs => "Tabs".to_string(),
    };
    let diag_count = diagnostics.map_or(0, |d| d.len());
    let diag_label = if diag_count > 0 {
        format!("  |  {diag_count} diagnostic{}", if diag_count == 1 { "" } else { "s" })
    } else {
        String::new()
    };
    let status_text = format!(
        "Ln {}, Col {}  |  {} lines  |  {}  |  {}{}",
        view.cursor.pos.line + 1,
        view.cursor.pos.col + 1,
        line_count,
        indent_label,
        if buf.is_modified() { "Modified" } else { "Saved" },
        diag_label,
    );

    // Main editor area — click + drag for cursor, scroll for navigation
    let (response, painter) = ui.allocate_painter(
        egui::Vec2::new(ui.available_width(), available_h),
        egui::Sense::click_and_drag(),
    );
    let rect = response.rect;

    // Mouse wheel scrolling
    if response.hovered() {
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll_delta != 0.0 {
            let scroll_lines = (-scroll_delta / line_height).round() as i32;
            if scroll_lines > 0 {
                view.scroll_line = (view.scroll_line + scroll_lines as usize).min(line_count.saturating_sub(1));
            } else if scroll_lines < 0 {
                view.scroll_line = view.scroll_line.saturating_sub((-scroll_lines) as usize);
            }
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

    // Mouse click to position cursor, drag to select
    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let (click_line, click_col) = pixel_to_editor_pos(
                pos, rect, gutter_width, text_margin, h_offset, char_width, line_height,
                view.scroll_line, line_count, buf,
            );
            view.cursor.clear_selection();
            view.cursor.pos = Position::new(click_line, click_col);
            view.cursor.desired_col = None;
        }
    }
    if response.dragged() {
        if let Some(pos) = response.interact_pointer_pos() {
            let (drag_line, drag_col) = pixel_to_editor_pos(
                pos, rect, gutter_width, text_margin, h_offset, char_width, line_height,
                view.scroll_line, line_count, buf,
            );
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
        for line_idx in sel_start.line..=sel_end.line {
            if line_idx < view.scroll_line || line_idx >= end_line { continue; }
            let vis_y = (line_idx - view.scroll_line) as f32 * line_height;
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

    // Syntax highlights for visible lines (disabled for very large files)
    let source_text = buf.text();
    let highlight_spans = if perf::syntax_enabled(line_count) {
        match (view.lang_id, &view.tree) {
            (Some(lang_id), Some(tree)) => {
                syntax.highlights_for_range(lang_id, tree, source_text.as_bytes(), view.scroll_line, end_line)
            }
            _ => vec![Vec::new(); end_line.saturating_sub(view.scroll_line)],
        }
    } else {
        vec![Vec::new(); end_line.saturating_sub(view.scroll_line)]
    };

    // Render visible lines
    let text_color = egui::Color32::WHITE;
    let gutter_color = egui::Color32::from_rgb(100, 100, 120);
    let current_line_gutter = egui::Color32::from_rgb(180, 180, 200);
    let current_line_bg = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 8);
    let font = egui::FontId::monospace(13.0);

    for line_idx in view.scroll_line..end_line {
        let vis_y = (line_idx - view.scroll_line) as f32 * line_height;
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
        let num_color = if line_idx == view.cursor.pos.line { current_line_gutter } else { gutter_color };
        painter.text(egui::pos2(rect.left() + 4.0, y + 1.0), egui::Align2::LEFT_TOP, &num_str, font.clone(), num_color);

        // Line text with syntax highlighting
        let line_text = buf.line(line_idx);
        if line_text.is_empty() { continue; }
        let text_x_base = rect.left() + gutter_width + text_margin - h_offset;
        let spans = &highlight_spans[line_idx - view.scroll_line];

        if spans.is_empty() {
            painter.with_clip_rect(text_clip).text(
                egui::pos2(text_x_base, y + 1.0), egui::Align2::LEFT_TOP,
                line_text, font.clone(), text_color,
            );
        } else {
            render_highlighted_line(&painter, text_clip, spans, line_text, text_x_base, y, char_width, &font, text_color);
        }
    }

    // Diagnostic underlines
    render_diagnostics(&painter, text_clip, diagnostics, view.scroll_line, end_line, rect, gutter_width, text_margin, char_width, line_height, h_offset);

    // Cursor
    if view.cursor.pos.line >= view.scroll_line && view.cursor.pos.line < end_line {
        let vis_y = (view.cursor.pos.line - view.scroll_line) as f32 * line_height;
        let cursor_x = rect.left() + gutter_width + text_margin + view.cursor.pos.col as f32 * char_width - h_offset;
        let cursor_y = rect.top() + vis_y;
        if cursor_x >= text_clip.left() && cursor_x <= text_clip.right() {
            let time = ui.ctx().input(|i| i.time);
            if (time * 2.0) as u64 % 2 == 0 {
                painter.with_clip_rect(text_clip).line_segment(
                    [egui::pos2(cursor_x, cursor_y + 1.0), egui::pos2(cursor_x, cursor_y + line_height - 1.0)],
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(80, 160, 255)),
                );
            }
        }
        ui.ctx().request_repaint();
    }

    // Minimap (right edge, replaces scrollbar) -- disabled for very large files
    let minimap_w = 50.0;
    let minimap_x = rect.right() - minimap_w;
    if line_count > 1 && perf::minimap_enabled(line_count) {
        // Background
        painter.rect_filled(
            egui::Rect::from_min_size(egui::pos2(minimap_x, rect.top()), egui::Vec2::new(minimap_w, rect.height())),
            0.0, egui::Color32::from_rgba_unmultiplied(20, 22, 28, 200),
        );

        // Viewport indicator
        let track_h = rect.height();
        let view_top = (view.scroll_line as f32 / line_count as f32) * track_h;
        let view_h = (visible_lines as f32 / line_count as f32) * track_h;
        painter.rect_filled(
            egui::Rect::from_min_size(egui::pos2(minimap_x, rect.top() + view_top), egui::Vec2::new(minimap_w, view_h.max(4.0))),
            0.0, egui::Color32::from_rgba_unmultiplied(80, 120, 200, 30),
        );

        // Line density: draw a tiny colored dot per line (sampled for perf)
        let line_h = (track_h / line_count as f32).max(0.5).min(2.0);
        let sample_step = if line_count > 2000 { line_count / 1000 } else { 1 };
        for line_idx in (0..line_count).step_by(sample_step.max(1)) {
            let y = rect.top() + (line_idx as f32 / line_count as f32) * track_h;
            let line_text = buf.line(line_idx);
            if line_text.trim().is_empty() { continue; }

            // Color from first syntax span if available, else dim white
            let color = if line_idx < highlight_spans.len() + view.scroll_line && line_idx >= view.scroll_line {
                let span_idx = line_idx - view.scroll_line;
                highlight_spans.get(span_idx)
                    .and_then(|spans| spans.first())
                    .map(|s| {
                        let rgb = crate::editor::syntax::group_color(s.group);
                        egui::Color32::from_rgba_unmultiplied(rgb[0], rgb[1], rgb[2], 120)
                    })
                    .unwrap_or(egui::Color32::from_rgba_unmultiplied(150, 155, 165, 60))
            } else {
                egui::Color32::from_rgba_unmultiplied(150, 155, 165, 40)
            };

            let text_w = (line_text.len() as f32 * 0.4).min(minimap_w - 4.0);
            painter.rect_filled(
                egui::Rect::from_min_size(egui::pos2(minimap_x + 2.0, y), egui::Vec2::new(text_w, line_h)),
                0.0, color,
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
                    0.0, color,
                );
            }
        }
    }

    // Hover tooltip
    if let Some(hover) = hover_text {
        if view.cursor.pos.line >= view.scroll_line && view.cursor.pos.line < end_line {
            let vis_y = (view.cursor.pos.line - view.scroll_line) as f32 * line_height;
            let tooltip_x = rect.left() + gutter_width + text_margin + view.cursor.pos.col as f32 * char_width - h_offset;
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
            painter.rect_stroke(bg_rect, 4.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 85, 100)));

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

    // Completion popup
    if let Some((items, selected)) = completions {
        if !items.is_empty() && view.cursor.pos.line >= view.scroll_line && view.cursor.pos.line < end_line {
            let vis_y = (view.cursor.pos.line - view.scroll_line) as f32 * line_height;
            let popup_x = rect.left() + gutter_width + text_margin + view.cursor.pos.col as f32 * char_width - h_offset;
            let popup_y = rect.top() + vis_y + line_height + 2.0;

            let item_h = 20.0;
            let popup_w = 320.0;
            let popup_h = (items.len() as f32 * item_h).min(200.0) + 4.0;

            // Clamp to screen
            let popup_x = popup_x.min(rect.right() - popup_w - 4.0).max(rect.left() + gutter_width);

            let bg = egui::Rect::from_min_size(egui::pos2(popup_x, popup_y), egui::Vec2::new(popup_w, popup_h));
            painter.rect_filled(bg, 4.0, egui::Color32::from_rgb(30, 32, 42));
            painter.rect_stroke(bg, 4.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 65, 80)));

            for (i, item) in items.iter().enumerate() {
                let y = popup_y + 2.0 + i as f32 * item_h;
                if y + item_h > popup_y + popup_h { break; }

                // Selected item highlight
                if i == selected {
                    painter.rect_filled(
                        egui::Rect::from_min_size(egui::pos2(popup_x + 2.0, y), egui::Vec2::new(popup_w - 4.0, item_h)),
                        2.0, egui::Color32::from_rgb(50, 80, 130),
                    );
                }

                // Kind icon
                let kind_char = match item.kind {
                    Some(lsp_types::CompletionItemKind::FUNCTION) | Some(lsp_types::CompletionItemKind::METHOD) => "f",
                    Some(lsp_types::CompletionItemKind::VARIABLE) => "v",
                    Some(lsp_types::CompletionItemKind::CLASS) | Some(lsp_types::CompletionItemKind::STRUCT) => "S",
                    Some(lsp_types::CompletionItemKind::MODULE) => "M",
                    Some(lsp_types::CompletionItemKind::KEYWORD) => "k",
                    Some(lsp_types::CompletionItemKind::FIELD) | Some(lsp_types::CompletionItemKind::PROPERTY) => "p",
                    Some(lsp_types::CompletionItemKind::CONSTANT) => "C",
                    Some(lsp_types::CompletionItemKind::ENUM_MEMBER) => "e",
                    Some(lsp_types::CompletionItemKind::INTERFACE) => "I",
                    Some(lsp_types::CompletionItemKind::TYPE_PARAMETER) => "T",
                    _ => " ",
                };
                painter.text(
                    egui::pos2(popup_x + 6.0, y + 2.0), egui::Align2::LEFT_TOP,
                    kind_char, egui::FontId::monospace(11.0),
                    egui::Color32::from_rgb(120, 130, 160),
                );

                // Label
                let label_color = if i == selected { egui::Color32::WHITE } else { egui::Color32::from_rgb(200, 205, 215) };
                painter.text(
                    egui::pos2(popup_x + 22.0, y + 2.0), egui::Align2::LEFT_TOP,
                    &item.label, egui::FontId::monospace(12.0), label_color,
                );

                // Detail (right-aligned, dimmed)
                if let Some(detail) = &item.detail {
                    let short = if detail.len() > 30 { &detail[..30] } else { detail };
                    painter.text(
                        egui::pos2(popup_x + popup_w - 8.0, y + 2.0), egui::Align2::RIGHT_TOP,
                        short, egui::FontId::monospace(10.0),
                        egui::Color32::from_rgb(100, 105, 120),
                    );
                }
            }
        }
    }

    // Status bar
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(status_text).size(12.0).color(egui::Color32::from_rgb(130, 130, 145)).monospace());
    });

    result
}

/// Render a single line with syntax-colored spans.
fn render_highlighted_line(
    painter: &egui::Painter,
    clip: egui::Rect,
    spans: &[crate::editor::syntax::HighlightSpan],
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
                let rgb = crate::editor::syntax::group_color(s.group);
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
                    let rgb = crate::editor::syntax::group_color(s.group);
                    egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2])
                })
                .unwrap_or(default_color);
            if next_color != color { break; }
            batch_end += 1;
        }

        let chunk: String = chars[col..batch_end].iter().collect();
        let x = text_x_base + col as f32 * char_width;
        painter.with_clip_rect(clip).text(
            egui::pos2(x, y + 1.0), egui::Align2::LEFT_TOP, &chunk, font.clone(), color,
        );
        col = batch_end;
    }
}

/// Render diagnostic underlines and gutter markers.
#[expect(clippy::too_many_arguments, reason = "layout geometry must be passed explicitly")]
fn render_diagnostics(
    painter: &egui::Painter,
    text_clip: egui::Rect,
    diagnostics: Option<&[FileDiagnostic]>,
    scroll_line: usize,
    end_line: usize,
    rect: egui::Rect,
    gutter_width: f32,
    text_margin: f32,
    char_width: f32,
    line_height: f32,
    h_offset: f32,
) {
    let Some(diags) = diagnostics else { return };
    for diag in diags {
        if diag.line < scroll_line as u32 || diag.line >= end_line as u32 { continue; }
        let vis_y = (diag.line as usize - scroll_line) as f32 * line_height;
        let y_base = rect.top() + vis_y + line_height - 2.0;
        let x_start = rect.left() + gutter_width + text_margin + diag.col as f32 * char_width - h_offset;
        let x_end = rect.left() + gutter_width + text_margin + diag.end_col as f32 * char_width - h_offset;
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
                [egui::pos2(sx, y_base + offset), egui::pos2(sx + seg_w, y_base + 2.0 - offset)],
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
            egui::Align2::LEFT_TOP, marker, egui::FontId::monospace(10.0), color,
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
    line_count: usize,
    buf: &crate::editor::buffer::Buffer,
) -> (usize, usize) {
    let rel_x = pos.x - rect.left() - gutter_width - text_margin + h_offset;
    let rel_y = pos.y - rect.top();
    let line = (scroll_line + (rel_y / line_height) as usize).min(line_count.saturating_sub(1));
    let col = (rel_x / char_width).max(0.0) as usize;
    let col = col.min(buf.line_len(line));
    (line, col)
}
