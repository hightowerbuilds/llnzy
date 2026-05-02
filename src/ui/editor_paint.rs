use std::collections::HashMap;

use crate::editor::buffer::{IndentStyle, Position};
use crate::editor::syntax::HighlightGroup;
use crate::lsp::{DiagSeverity, FileDiagnostic};

#[expect(
    clippy::too_many_arguments,
    reason = "layout geometry must be passed explicitly"
)]
pub(super) fn render_bracket_match(
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
            painter
                .with_clip_rect(text_clip)
                .rect_filled(bracket_rect, 1.0, fill);
            painter
                .with_clip_rect(text_clip)
                .rect_stroke(bracket_rect, 1.0, stroke);
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "layout geometry must be passed explicitly"
)]
pub(super) fn render_indentation_guides(
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

pub(super) fn indent_level(line: &str, style: IndentStyle) -> usize {
    let width = style.width().max(1);
    indentation_columns(line, style) / width
}

pub(super) fn indentation_columns(line: &str, style: IndentStyle) -> usize {
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

pub(super) fn render_highlighted_line(
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
pub(super) fn render_visible_whitespace_line(
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
                        let rgb = crate::editor::syntax::group_color_with_overrides(
                            s.group,
                            syntax_colors,
                        );
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

#[expect(
    clippy::too_many_arguments,
    reason = "layout geometry must be passed explicitly"
)]
pub(super) fn render_diagnostics(
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

#[expect(
    clippy::too_many_arguments,
    reason = "layout geometry must be passed explicitly"
)]
pub(super) fn pixel_to_editor_pos(
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
    let visible_line = (scroll_line + (rel_y / line_height).max(0.0) as usize)
        .min(visible_doc_lines.len().saturating_sub(1));
    let line = visible_doc_lines.get(visible_line).copied().unwrap_or(0);
    let col = (rel_x / char_width).max(0.0) as usize;
    let col = col.min(buf.line_len(line));
    (line, col)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::buffer::Buffer;

    fn buf_with(text: &str) -> Buffer {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), text);
        buf
    }

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
    fn pixel_hit_testing_accounts_for_horizontal_scroll() {
        let buf = buf_with("abcdefghijklmnopqrstuvwxyz");
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(320.0, 80.0));

        let (line, col) = pixel_to_editor_pos(
            egui::pos2(44.0, 5.0),
            rect,
            20.0,
            4.0,
            80.0,
            10.0,
            20.0,
            0,
            &[0],
            &buf,
        );

        assert_eq!((line, col), (0, 10));
    }

    #[test]
    fn pixel_hit_testing_uses_visible_doc_lines_after_folding() {
        let buf = buf_with("line0\nhidden1\nhidden2\nline3");
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(320.0, 120.0));

        let (line, col) = pixel_to_editor_pos(
            egui::pos2(34.0, 25.0),
            rect,
            20.0,
            4.0,
            0.0,
            10.0,
            20.0,
            0,
            &[0, 3],
            &buf,
        );

        assert_eq!((line, col), (3, 1));
    }
}
