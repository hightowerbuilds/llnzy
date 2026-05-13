use rustc_hash::FxHashMap;

use crate::editor::buffer::{IndentStyle, Position};
use crate::editor::syntax::HighlightGroup;
use crate::lsp::{DiagSeverity, FileDiagnostic};

use super::editor_line_decorations::visible_line_offset;

#[derive(Clone, Copy)]
pub(super) struct EditorGeometry {
    pub text_clip: egui::Rect,
    pub rect: egui::Rect,
    pub gutter_width: f32,
    pub text_margin: f32,
    pub char_width: f32,
    pub line_height: f32,
    pub h_offset: f32,
}

impl EditorGeometry {
    pub(super) fn line_y(self, visible_offset: usize) -> f32 {
        self.rect.top() + visible_offset as f32 * self.line_height
    }

    pub(super) fn text_x(self, col: usize) -> f32 {
        self.wrapped_text_x(col) - self.h_offset
    }

    pub(super) fn wrapped_text_x(self, col: usize) -> f32 {
        self.rect.left() + self.gutter_width + self.text_margin + col as f32 * self.char_width
    }
}

#[derive(Clone, Copy)]
pub(super) struct EditorPaintContext<'a> {
    pub painter: &'a egui::Painter,
    pub geometry: EditorGeometry,
}

pub(super) fn render_bracket_match(
    ctx: EditorPaintContext<'_>,
    bracket_match: Option<(Position, Position)>,
    visible_window: &[usize],
) {
    let Some((a, b)) = bracket_match else { return };
    let geometry = ctx.geometry;
    let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(120, 170, 255));
    let fill = egui::Color32::from_rgba_unmultiplied(120, 170, 255, 35);

    for pos in [a, b] {
        let Some(visible_offset) = visible_line_offset(visible_window, pos.line) else {
            continue;
        };
        let y = geometry.line_y(visible_offset);
        let x = geometry.text_x(pos.col);
        let bracket_rect = egui::Rect::from_min_size(
            egui::pos2(x, y + 1.0),
            egui::Vec2::new(geometry.char_width, geometry.line_height - 2.0),
        );
        if bracket_rect.intersects(geometry.text_clip) {
            ctx.painter
                .with_clip_rect(geometry.text_clip)
                .rect_filled(bracket_rect, 1.0, fill);
            ctx.painter
                .with_clip_rect(geometry.text_clip)
                .rect_stroke(bracket_rect, 1.0, stroke);
        }
    }
}

pub(super) fn render_indentation_guides(
    ctx: EditorPaintContext<'_>,
    buf: &crate::editor::buffer::Buffer,
    visible_window: &[usize],
    active_indent_level: usize,
) {
    let geometry = ctx.geometry;
    let indent_width = buf.indent_style.width().max(1);
    let guide_color = egui::Color32::from_rgba_unmultiplied(190, 200, 220, 26);
    let active_guide_color = egui::Color32::from_rgba_unmultiplied(120, 170, 255, 82);

    for (visible_offset, &line_idx) in visible_window.iter().enumerate() {
        let line = buf.line(line_idx);
        let level = indent_level(line, buf.indent_style);
        if level == 0 {
            continue;
        }

        let y1 = geometry.line_y(visible_offset);
        let y2 = y1 + geometry.line_height;

        for guide_level in 1..=level {
            let col = guide_level * indent_width;
            let x = geometry.text_x(col);
            if x < geometry.text_clip.left() || x > geometry.text_clip.right() {
                continue;
            }

            let color = if guide_level == active_indent_level {
                active_guide_color
            } else {
                guide_color
            };
            ctx.painter.with_clip_rect(geometry.text_clip).line_segment(
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

pub(super) struct HighlightedLineRenderInput<'a> {
    pub painter: &'a egui::Painter,
    pub clip: egui::Rect,
    pub spans: &'a [crate::editor::syntax::HighlightSpan],
    pub syntax_colors: &'a FxHashMap<HighlightGroup, [u8; 3]>,
    pub line_text: &'a str,
    pub text_x_base: f32,
    pub y: f32,
    pub char_width: f32,
    pub font: &'a egui::FontId,
    pub default_color: egui::Color32,
}

pub(super) fn render_highlighted_line(input: HighlightedLineRenderInput<'_>) {
    let HighlightedLineRenderInput {
        painter,
        clip,
        spans,
        syntax_colors,
        line_text,
        text_x_base,
        y,
        char_width,
        font,
        default_color,
    } = input;
    let mut col = 0;
    let mut chars = line_text.char_indices().peekable();
    while let Some((chunk_start, _)) = chars.next() {
        let color = spans
            .iter()
            .find(|s| col >= s.col_start && col < s.col_end)
            .map(|s| {
                let rgb = crate::editor::syntax::group_color_with_overrides(s.group, syntax_colors);
                egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2])
            })
            .unwrap_or(default_color);

        let mut chunk_end = line_text.len();
        let mut next_col = col + 1;
        while let Some(&(next_byte, _)) = chars.peek() {
            let next_color = spans
                .iter()
                .find(|s| next_col >= s.col_start && next_col < s.col_end)
                .map(|s| {
                    let rgb =
                        crate::editor::syntax::group_color_with_overrides(s.group, syntax_colors);
                    egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2])
                })
                .unwrap_or(default_color);
            if next_color != color {
                chunk_end = next_byte;
                break;
            }
            chars.next();
            next_col += 1;
        }

        let x = text_x_base + col as f32 * char_width;
        painter.with_clip_rect(clip).text(
            egui::pos2(x, y + 1.0),
            egui::Align2::LEFT_TOP,
            &line_text[chunk_start..chunk_end],
            font.clone(),
            color,
        );
        col = next_col;
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
    syntax_colors: &FxHashMap<HighlightGroup, [u8; 3]>,
    line_text: &str,
    text_x_base: f32,
    y: f32,
    char_width: f32,
    font: &egui::FontId,
    default_color: egui::Color32,
) {
    let whitespace_color = egui::Color32::from_rgb(85, 92, 112);
    let mut chars = line_text.char_indices().peekable();
    let mut col = 0;
    while let Some((byte_idx, ch)) = chars.next() {
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
                    chars
                        .peek()
                        .map(|(next_byte, _)| &line_text[byte_idx..*next_byte])
                        .unwrap_or(&line_text[byte_idx..]),
                    font.clone(),
                    color,
                );
                col += 1;
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
        col += 1;
    }
}

pub(super) fn render_diagnostics(
    ctx: EditorPaintContext<'_>,
    diagnostics: Option<&[FileDiagnostic]>,
    visible_window: &[usize],
) {
    let Some(diags) = diagnostics else { return };
    let geometry = ctx.geometry;
    for diag in diags {
        let Some(visible_offset) = visible_line_offset(visible_window, diag.line as usize) else {
            continue;
        };
        let y = geometry.line_y(visible_offset);
        let y_base = y + geometry.line_height - 2.0;
        let x_start = geometry.text_x(diag.col as usize);
        let x_end = geometry.text_x(diag.end_col as usize);
        let width = (x_end - x_start).max(geometry.char_width);

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
            ctx.painter.with_clip_rect(geometry.text_clip).line_segment(
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
        ctx.painter.text(
            egui::pos2(geometry.rect.left() + 1.0, y + 1.0),
            egui::Align2::LEFT_TOP,
            marker,
            egui::FontId::monospace(10.0),
            color,
        );
    }
}

pub(super) fn pixel_to_editor_pos(
    pos: egui::Pos2,
    geometry: EditorGeometry,
    scroll_line: usize,
    visible_doc_lines: &[usize],
    buf: &crate::editor::buffer::Buffer,
) -> (usize, usize) {
    let rel_x = pos.x - geometry.rect.left() - geometry.gutter_width - geometry.text_margin
        + geometry.h_offset;
    let rel_y = pos.y - geometry.rect.top();
    let visible_line = (scroll_line + (rel_y / geometry.line_height).max(0.0) as usize)
        .min(visible_doc_lines.len().saturating_sub(1));
    let line = visible_doc_lines.get(visible_line).copied().unwrap_or(0);
    let col = (rel_x / geometry.char_width).max(0.0) as usize;
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

    fn geometry(rect: egui::Rect, h_offset: f32) -> EditorGeometry {
        EditorGeometry {
            text_clip: egui::Rect::from_min_max(
                egui::pos2(rect.left() + 20.0, rect.top()),
                rect.right_bottom(),
            ),
            rect,
            gutter_width: 20.0,
            text_margin: 4.0,
            char_width: 10.0,
            line_height: 20.0,
            h_offset,
        }
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

        let (line, col) =
            pixel_to_editor_pos(egui::pos2(44.0, 5.0), geometry(rect, 80.0), 0, &[0], &buf);

        assert_eq!((line, col), (0, 10));
    }

    #[test]
    fn pixel_hit_testing_uses_visible_doc_lines_after_folding() {
        let buf = buf_with("line0\nhidden1\nhidden2\nline3");
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(320.0, 120.0));

        let (line, col) = pixel_to_editor_pos(
            egui::pos2(34.0, 25.0),
            geometry(rect, 0.0),
            0,
            &[0, 3],
            &buf,
        );

        assert_eq!((line, col), (3, 1));
    }
}
