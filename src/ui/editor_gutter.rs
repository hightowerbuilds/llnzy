use crate::editor::git_gutter::GutterChange;
use crate::editor::syntax::FoldRange;
use crate::editor::BufferView;

use super::editor_folding::is_range_folded;

pub(super) fn render_git_gutter_change(
    painter: &egui::Painter,
    view: &BufferView,
    line_idx: usize,
    rect: egui::Rect,
    gutter_width: f32,
    y: f32,
    line_height: f32,
) {
    let Some(gutter) = &view.git_gutter else {
        return;
    };
    let Some(change) = gutter.change_at(line_idx) else {
        return;
    };

    let (color, bar_h) = match change {
        GutterChange::Added => (egui::Color32::from_rgb(80, 200, 80), line_height),
        GutterChange::Modified => (egui::Color32::from_rgb(80, 140, 230), line_height),
        GutterChange::Deleted => (egui::Color32::from_rgb(220, 70, 70), 4.0),
    };
    let bar_y = if change == GutterChange::Deleted {
        y - 2.0
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

#[allow(clippy::too_many_arguments)]
pub(super) fn render_fold_marker(
    painter: &egui::Painter,
    view: &BufferView,
    foldable_ranges: &[FoldRange],
    line_idx: usize,
    rect: egui::Rect,
    gutter_width: f32,
    y: f32,
    font: &egui::FontId,
    gutter_color: egui::Color32,
) {
    if let Some(range) = foldable_ranges.iter().find(|r| r.start_line == line_idx) {
        let marker = if is_range_folded(&view.folded_ranges, range.start_line) {
            ">"
        } else {
            "v"
        };
        painter.text(
            egui::pos2(rect.left() + gutter_width - 12.0, y + 1.0),
            egui::Align2::LEFT_TOP,
            marker,
            font.clone(),
            gutter_color,
        );
    }
}
