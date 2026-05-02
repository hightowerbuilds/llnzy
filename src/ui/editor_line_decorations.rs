use crate::editor::BufferView;
use crate::lsp::{CodeLensInfo, InlayHintInfo};

use super::editor_folding::folded_range_starting_at;

#[allow(clippy::too_many_arguments)]
pub(super) fn render_fold_placeholder(
    painter: &egui::Painter,
    text_clip: egui::Rect,
    view: &BufferView,
    line_idx: usize,
    line_text: &str,
    text_x_base: f32,
    y: f32,
    char_width: f32,
    font: &egui::FontId,
) {
    if let Some(range) = folded_range_starting_at(&view.folded_ranges, line_idx) {
        let hidden_count = range.end_line.saturating_sub(range.start_line);
        let placeholder = format!(
            " ... {hidden_count} folded line{}",
            if hidden_count == 1 { "" } else { "s" }
        );
        let placeholder_x = text_x_base + line_text.chars().count() as f32 * char_width;
        painter.with_clip_rect(text_clip).text(
            egui::pos2(placeholder_x, y + 1.0),
            egui::Align2::LEFT_TOP,
            placeholder,
            font.clone(),
            egui::Color32::from_rgb(110, 120, 145),
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_line_inlay_hints(
    painter: &egui::Painter,
    text_clip: egui::Rect,
    inlay_hints: &[InlayHintInfo],
    line_idx: usize,
    text_x_base: f32,
    y: f32,
    char_width: f32,
    editor_font_size: f32,
) {
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
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_line_code_lenses(
    painter: &egui::Painter,
    text_clip: egui::Rect,
    code_lenses: &[CodeLensInfo],
    line_idx: usize,
    text_x_base: f32,
    y: f32,
    line_height: f32,
    rect: egui::Rect,
    editor_font_size: f32,
) {
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
