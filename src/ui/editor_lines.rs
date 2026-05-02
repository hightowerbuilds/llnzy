use std::collections::HashMap;

use crate::config::EffectiveEditorConfig;
use crate::editor::syntax::{FoldRange, HighlightGroup, HighlightSpan};
use crate::editor::BufferView;
use crate::lsp::{CodeLensInfo, InlayHintInfo};

use super::editor_gutter::{render_fold_marker, render_git_gutter_change};
use super::editor_line_decorations::{
    render_fold_placeholder, render_line_code_lenses, render_line_inlay_hints,
};
use super::editor_paint::{
    indent_level, render_highlighted_line, render_indentation_guides,
    render_visible_whitespace_line,
};
use super::editor_wrap::WrapRow;

#[allow(clippy::too_many_arguments)]
pub(super) fn render_editor_lines(
    painter: &egui::Painter,
    text_clip: egui::Rect,
    buf: &crate::editor::buffer::Buffer,
    view: &BufferView,
    editor_config: &EffectiveEditorConfig,
    syntax_colors: &HashMap<HighlightGroup, [u8; 3]>,
    inlay_hints: &[InlayHintInfo],
    code_lenses: &[CodeLensInfo],
    highlight_spans: &[Vec<HighlightSpan>],
    foldable_ranges: &[FoldRange],
    visible_window: &[usize],
    visible_wrap_window: &[WrapRow],
    rect: egui::Rect,
    gutter_digits: usize,
    gutter_width: f32,
    text_margin: f32,
    char_width: f32,
    line_height: f32,
    h_offset: f32,
    syntax_start_line: usize,
    editor_font_size: f32,
    word_wrap: bool,
) {
    let text_color = egui::Color32::WHITE;
    let gutter_color = egui::Color32::from_rgb(100, 100, 120);
    let current_line_gutter = egui::Color32::from_rgb(180, 180, 200);
    let current_line_bg = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 8);
    let font = egui::FontId::monospace(editor_font_size);
    let active_indent_level = indent_level(buf.line(view.cursor.pos.line), buf.indent_style);

    if !word_wrap {
        render_indentation_guides(
            painter,
            text_clip,
            buf,
            visible_window,
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
        render_wrapped_lines(
            painter,
            text_clip,
            buf,
            view,
            syntax_colors,
            highlight_spans,
            visible_wrap_window,
            rect,
            gutter_digits,
            gutter_width,
            text_margin,
            char_width,
            line_height,
            syntax_start_line,
            &font,
            text_color,
            gutter_color,
            current_line_gutter,
            current_line_bg,
        );
    } else {
        render_unwrapped_lines(
            painter,
            text_clip,
            buf,
            view,
            editor_config,
            syntax_colors,
            inlay_hints,
            code_lenses,
            highlight_spans,
            foldable_ranges,
            visible_window,
            rect,
            gutter_digits,
            gutter_width,
            text_margin,
            char_width,
            line_height,
            h_offset,
            syntax_start_line,
            editor_font_size,
            &font,
            text_color,
            gutter_color,
            current_line_gutter,
            current_line_bg,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn render_wrapped_lines(
    painter: &egui::Painter,
    text_clip: egui::Rect,
    buf: &crate::editor::buffer::Buffer,
    view: &BufferView,
    syntax_colors: &HashMap<HighlightGroup, [u8; 3]>,
    highlight_spans: &[Vec<HighlightSpan>],
    visible_wrap_window: &[WrapRow],
    rect: egui::Rect,
    gutter_digits: usize,
    gutter_width: f32,
    text_margin: f32,
    char_width: f32,
    line_height: f32,
    syntax_start_line: usize,
    font: &egui::FontId,
    text_color: egui::Color32,
    gutter_color: egui::Color32,
    current_line_gutter: egui::Color32,
    current_line_bg: egui::Color32,
) {
    for (vis_idx, row) in visible_wrap_window.iter().enumerate() {
        let vis_y = vis_idx as f32 * line_height;
        let y = rect.top() + vis_y;

        if row.doc_line == view.cursor.pos.line {
            let line_rect = egui::Rect::from_min_size(
                egui::pos2(rect.left() + gutter_width, y),
                egui::Vec2::new(rect.width() - gutter_width, line_height),
            );
            painter.rect_filled(line_rect, 0.0, current_line_bg);
        }

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
            render_git_gutter_change(
                painter,
                view,
                row.doc_line,
                rect,
                gutter_width,
                y,
                line_height,
            );
        } else {
            painter.text(
                egui::pos2(rect.left() + gutter_width - 12.0, y + 1.0),
                egui::Align2::LEFT_TOP,
                "~",
                font.clone(),
                egui::Color32::from_rgb(60, 65, 80),
            );
        }

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
        render_wrapped_row_text(
            painter,
            text_clip,
            syntax_colors,
            row,
            &row_text,
            text_x_base,
            y,
            char_width,
            font,
            text_color,
            spans,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn render_wrapped_row_text(
    painter: &egui::Painter,
    text_clip: egui::Rect,
    syntax_colors: &HashMap<HighlightGroup, [u8; 3]>,
    row: &WrapRow,
    row_text: &str,
    text_x_base: f32,
    y: f32,
    char_width: f32,
    font: &egui::FontId,
    text_color: egui::Color32,
    spans: &[HighlightSpan],
) {
    if spans.is_empty() {
        painter.with_clip_rect(text_clip).text(
            egui::pos2(text_x_base, y + 1.0),
            egui::Align2::LEFT_TOP,
            row_text,
            font.clone(),
            text_color,
        );
        return;
    }

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
                    let rgb =
                        crate::editor::syntax::group_color_with_overrides(s.group, syntax_colors);
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

#[allow(clippy::too_many_arguments)]
fn render_unwrapped_lines(
    painter: &egui::Painter,
    text_clip: egui::Rect,
    buf: &crate::editor::buffer::Buffer,
    view: &BufferView,
    editor_config: &EffectiveEditorConfig,
    syntax_colors: &HashMap<HighlightGroup, [u8; 3]>,
    inlay_hints: &[InlayHintInfo],
    code_lenses: &[CodeLensInfo],
    highlight_spans: &[Vec<HighlightSpan>],
    foldable_ranges: &[FoldRange],
    visible_window: &[usize],
    rect: egui::Rect,
    gutter_digits: usize,
    gutter_width: f32,
    text_margin: f32,
    char_width: f32,
    line_height: f32,
    h_offset: f32,
    syntax_start_line: usize,
    editor_font_size: f32,
    font: &egui::FontId,
    text_color: egui::Color32,
    gutter_color: egui::Color32,
    current_line_gutter: egui::Color32,
    current_line_bg: egui::Color32,
) {
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

        render_git_gutter_change(painter, view, line_idx, rect, gutter_width, y, line_height);
        render_fold_marker(
            painter,
            view,
            foldable_ranges,
            line_idx,
            rect,
            gutter_width,
            y,
            font,
            gutter_color,
        );

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
                painter,
                text_clip,
                spans,
                syntax_colors,
                line_text,
                text_x_base,
                y,
                char_width,
                font,
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
                painter,
                text_clip,
                spans,
                syntax_colors,
                line_text,
                text_x_base,
                y,
                char_width,
                font,
                text_color,
            );
        }

        render_fold_placeholder(
            painter,
            text_clip,
            view,
            line_idx,
            line_text,
            text_x_base,
            y,
            char_width,
            font,
        );
        render_line_inlay_hints(
            painter,
            text_clip,
            inlay_hints,
            line_idx,
            text_x_base,
            y,
            char_width,
            editor_font_size,
        );
        render_line_code_lenses(
            painter,
            text_clip,
            code_lenses,
            line_idx,
            text_x_base,
            y,
            line_height,
            rect,
            editor_font_size,
        );
    }
}
