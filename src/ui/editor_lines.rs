use rustc_hash::FxHashMap;

use crate::config::EffectiveEditorConfig;
use crate::editor::syntax::{FoldRange, HighlightGroup, HighlightSpan};
use crate::editor::BufferView;
use crate::lsp::{CodeLensInfo, InlayHintInfo};
use crate::text_utils::char_range_slice;

use super::editor_gutter::{render_fold_marker, render_git_gutter_change};
use super::editor_line_decorations::{
    render_fold_placeholder, render_line_code_lenses, render_line_inlay_hints,
    LineDecorationBuckets,
};
use super::editor_paint::{
    indent_level, render_highlighted_line, render_indentation_guides,
    render_visible_whitespace_line, EditorPaintContext, HighlightedLineRenderInput,
};
use super::editor_wrap::WrapRow;

pub(super) struct EditorLinesInput<'a> {
    pub paint: EditorPaintContext<'a>,
    pub buf: &'a crate::editor::buffer::Buffer,
    pub view: &'a BufferView,
    pub editor_config: &'a EffectiveEditorConfig,
    pub syntax_colors: &'a FxHashMap<HighlightGroup, [u8; 3]>,
    pub inlay_hints: &'a [InlayHintInfo],
    pub code_lenses: &'a [CodeLensInfo],
    pub highlight_spans: &'a [Vec<HighlightSpan>],
    pub foldable_ranges: &'a [FoldRange],
    pub visible_window: &'a [usize],
    pub visible_wrap_window: &'a [WrapRow],
    pub gutter_digits: usize,
    pub syntax_start_line: usize,
    pub editor_font_size: f32,
    pub word_wrap: bool,
}

pub(super) fn render_editor_lines(input: EditorLinesInput<'_>) {
    let EditorLinesInput {
        paint,
        buf,
        view,
        editor_config,
        syntax_colors,
        inlay_hints,
        code_lenses,
        highlight_spans,
        foldable_ranges,
        visible_window,
        visible_wrap_window,
        gutter_digits,
        syntax_start_line,
        editor_font_size,
        word_wrap,
    } = input;
    let text_color = egui::Color32::WHITE;
    let gutter_color = egui::Color32::from_rgb(100, 100, 120);
    let current_line_gutter = egui::Color32::from_rgb(180, 180, 200);
    let current_line_bg = if editor_config.highlight_current_line {
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 8)
    } else {
        egui::Color32::TRANSPARENT
    };
    let font = egui::FontId::monospace(editor_font_size);
    let active_indent_level = indent_level(buf.line(view.cursor.pos.line), buf.indent_style);

    if !word_wrap {
        render_indentation_guides(paint, buf, visible_window, active_indent_level);
    }

    if word_wrap {
        render_wrapped_lines(
            paint,
            buf,
            view,
            editor_config,
            syntax_colors,
            highlight_spans,
            visible_wrap_window,
            gutter_digits,
            syntax_start_line,
            &font,
            text_color,
            gutter_color,
            current_line_gutter,
            current_line_bg,
        );
    } else {
        render_unwrapped_lines(
            paint,
            buf,
            view,
            editor_config,
            syntax_colors,
            inlay_hints,
            code_lenses,
            highlight_spans,
            foldable_ranges,
            visible_window,
            gutter_digits,
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

#[expect(
    clippy::too_many_arguments,
    reason = "wrapped line rendering still carries syntax and style state until Phase 2 split"
)]
fn render_wrapped_lines(
    paint: EditorPaintContext<'_>,
    buf: &crate::editor::buffer::Buffer,
    view: &BufferView,
    editor_config: &EffectiveEditorConfig,
    syntax_colors: &FxHashMap<HighlightGroup, [u8; 3]>,
    highlight_spans: &[Vec<HighlightSpan>],
    visible_wrap_window: &[WrapRow],
    gutter_digits: usize,
    syntax_start_line: usize,
    font: &egui::FontId,
    text_color: egui::Color32,
    gutter_color: egui::Color32,
    current_line_gutter: egui::Color32,
    current_line_bg: egui::Color32,
) {
    let painter = paint.painter;
    let geometry = paint.geometry;
    for (vis_idx, row) in visible_wrap_window.iter().enumerate() {
        let y = geometry.line_y(vis_idx);

        if row.doc_line == view.cursor.pos.line {
            let line_rect = egui::Rect::from_min_size(
                egui::pos2(geometry.rect.left() + geometry.gutter_width, y),
                egui::Vec2::new(
                    geometry.rect.width() - geometry.gutter_width,
                    geometry.line_height,
                ),
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
            if editor_config.show_line_numbers {
                painter.text(
                    egui::pos2(geometry.rect.left() + 4.0, y + 1.0),
                    egui::Align2::LEFT_TOP,
                    &num_str,
                    font.clone(),
                    num_color,
                );
            }
            render_git_gutter_change(
                painter,
                view,
                row.doc_line,
                geometry.rect,
                geometry.gutter_width,
                y,
                geometry.line_height,
            );
        } else if editor_config.show_line_numbers {
            painter.text(
                egui::pos2(geometry.rect.left() + geometry.gutter_width - 12.0, y + 1.0),
                egui::Align2::LEFT_TOP,
                "~",
                font.clone(),
                egui::Color32::from_rgb(60, 65, 80),
            );
        }

        let line_text = buf.line(row.doc_line);
        let Some(row_text) = char_range_slice(line_text, row.col_start, row.col_end) else {
            continue;
        };

        let text_x_base = geometry.wrapped_text_x(0);
        let spans = highlight_spans
            .get(row.doc_line.saturating_sub(syntax_start_line))
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        render_wrapped_row_text(
            painter,
            geometry.text_clip,
            syntax_colors,
            row,
            row_text,
            text_x_base,
            y,
            geometry.char_width,
            font,
            text_color,
            spans,
        );
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "row text chunk rendering carries syntax/style state until line renderer extraction"
)]
fn render_wrapped_row_text(
    painter: &egui::Painter,
    text_clip: egui::Rect,
    syntax_colors: &FxHashMap<HighlightGroup, [u8; 3]>,
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

    let mut col = 0;
    let mut chars = row_text.char_indices().peekable();
    while let Some((chunk_start, _)) = chars.next() {
        let doc_col = row.col_start + col;
        let color = spans
            .iter()
            .find(|s| doc_col >= s.col_start && doc_col < s.col_end)
            .map(|s| {
                let rgb = crate::editor::syntax::group_color_with_overrides(s.group, syntax_colors);
                egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2])
            })
            .unwrap_or(text_color);

        let mut chunk_end = row_text.len();
        let mut next_col = col + 1;
        while let Some(&(next_byte, _)) = chars.peek() {
            let next_doc_col = row.col_start + next_col;
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
                chunk_end = next_byte;
                break;
            }
            chars.next();
            next_col += 1;
        }

        let x = text_x_base + col as f32 * char_width;
        painter.with_clip_rect(text_clip).text(
            egui::pos2(x, y + 1.0),
            egui::Align2::LEFT_TOP,
            &row_text[chunk_start..chunk_end],
            font.clone(),
            color,
        );
        col = next_col;
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "unwrapped line rendering still carries syntax and decoration state until Phase 2 split"
)]
fn render_unwrapped_lines(
    paint: EditorPaintContext<'_>,
    buf: &crate::editor::buffer::Buffer,
    view: &BufferView,
    editor_config: &EffectiveEditorConfig,
    syntax_colors: &FxHashMap<HighlightGroup, [u8; 3]>,
    inlay_hints: &[InlayHintInfo],
    code_lenses: &[CodeLensInfo],
    highlight_spans: &[Vec<HighlightSpan>],
    foldable_ranges: &[FoldRange],
    visible_window: &[usize],
    gutter_digits: usize,
    syntax_start_line: usize,
    editor_font_size: f32,
    font: &egui::FontId,
    text_color: egui::Color32,
    gutter_color: egui::Color32,
    current_line_gutter: egui::Color32,
    current_line_bg: egui::Color32,
) {
    let inlay_hint_buckets =
        LineDecorationBuckets::new(inlay_hints, visible_window, |hint| hint.line as usize);
    let code_lens_buckets =
        LineDecorationBuckets::new(code_lenses, visible_window, |lens| lens.line as usize);
    let painter = paint.painter;
    let geometry = paint.geometry;

    for (visible_offset, &line_idx) in visible_window.iter().enumerate() {
        let y = geometry.line_y(visible_offset);

        if line_idx == view.cursor.pos.line {
            let line_rect = egui::Rect::from_min_size(
                egui::pos2(geometry.rect.left() + geometry.gutter_width, y),
                egui::Vec2::new(
                    geometry.rect.width() - geometry.gutter_width,
                    geometry.line_height,
                ),
            );
            painter.rect_filled(line_rect, 0.0, current_line_bg);
        }

        let num_str = format!("{:>width$}", line_idx + 1, width = gutter_digits);
        let num_color = if line_idx == view.cursor.pos.line {
            current_line_gutter
        } else {
            gutter_color
        };
        if editor_config.show_line_numbers {
            painter.text(
                egui::pos2(geometry.rect.left() + 4.0, y + 1.0),
                egui::Align2::LEFT_TOP,
                &num_str,
                font.clone(),
                num_color,
            );
        }

        render_git_gutter_change(
            painter,
            view,
            line_idx,
            geometry.rect,
            geometry.gutter_width,
            y,
            geometry.line_height,
        );
        render_fold_marker(
            painter,
            view,
            foldable_ranges,
            line_idx,
            geometry.rect,
            geometry.gutter_width,
            y,
            font,
            gutter_color,
        );

        let line_text = buf.line(line_idx);
        if line_text.is_empty() {
            continue;
        }
        let text_x_base = geometry.text_x(0);
        let spans = highlight_spans
            .get(line_idx.saturating_sub(syntax_start_line))
            .map(Vec::as_slice)
            .unwrap_or(&[]);

        if editor_config.visible_whitespace {
            render_visible_whitespace_line(
                painter,
                geometry.text_clip,
                spans,
                syntax_colors,
                line_text,
                text_x_base,
                y,
                geometry.char_width,
                font,
                text_color,
            );
        } else if spans.is_empty() {
            painter.with_clip_rect(geometry.text_clip).text(
                egui::pos2(text_x_base, y + 1.0),
                egui::Align2::LEFT_TOP,
                line_text,
                font.clone(),
                text_color,
            );
        } else {
            render_highlighted_line(HighlightedLineRenderInput {
                painter,
                clip: geometry.text_clip,
                spans,
                syntax_colors,
                line_text,
                text_x_base,
                y,
                char_width: geometry.char_width,
                font,
                default_color: text_color,
            });
        }

        render_fold_placeholder(
            painter,
            geometry.text_clip,
            view,
            line_idx,
            line_text,
            text_x_base,
            y,
            geometry.char_width,
            font,
        );
        render_line_inlay_hints(
            painter,
            geometry.text_clip,
            inlay_hint_buckets.get(visible_offset),
            text_x_base,
            y,
            geometry.char_width,
            editor_font_size,
        );
        render_line_code_lenses(
            painter,
            geometry.text_clip,
            code_lens_buckets.get(visible_offset),
            text_x_base,
            y,
            geometry.line_height,
            geometry.rect,
            editor_font_size,
        );
    }
}
