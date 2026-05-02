use crate::editor::buffer::Position;
use crate::editor::syntax::FoldRange;
use crate::editor::BufferView;

use super::editor_folding::{best_fold_range_starting_at, toggle_fold_range};
use super::editor_paint::pixel_to_editor_pos;
use super::editor_wrap::{pixel_to_editor_pos_wrapped, WrapRow};

const MINIMAP_WIDTH: f32 = 50.0;

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_editor_pointer_input(
    ui: &egui::Ui,
    response: &egui::Response,
    painter: &egui::Painter,
    buf: &crate::editor::buffer::Buffer,
    view: &mut BufferView,
    foldable_ranges: &[FoldRange],
    visible_doc_lines: &[usize],
    wrap_rows: &[WrapRow],
    rect: egui::Rect,
    gutter_width: f32,
    text_margin: f32,
    h_offset: f32,
    char_width: f32,
    line_height: f32,
    max_scroll: usize,
    word_wrap: bool,
) -> egui::Rect {
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

    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(25, 25, 32));
    let gutter_rect = egui::Rect::from_min_size(
        rect.left_top(),
        egui::Vec2::new(gutter_width, rect.height()),
    );
    painter.rect_filled(gutter_rect, 0.0, egui::Color32::from_rgb(30, 30, 38));

    let handled_gutter_click = handle_gutter_click(
        response,
        view,
        foldable_ranges,
        visible_doc_lines,
        rect,
        gutter_width,
        line_height,
    );
    handle_editor_click(
        response,
        buf,
        view,
        visible_doc_lines,
        wrap_rows,
        rect,
        gutter_width,
        text_margin,
        h_offset,
        char_width,
        line_height,
        word_wrap,
        handled_gutter_click,
    );
    handle_editor_drag(
        response,
        buf,
        view,
        visible_doc_lines,
        wrap_rows,
        rect,
        gutter_width,
        text_margin,
        h_offset,
        char_width,
        line_height,
        word_wrap,
    );

    text_clip
}

fn handle_gutter_click(
    response: &egui::Response,
    view: &mut BufferView,
    foldable_ranges: &[FoldRange],
    visible_doc_lines: &[usize],
    rect: egui::Rect,
    gutter_width: f32,
    line_height: f32,
) -> bool {
    if !response.clicked() {
        return false;
    }
    let Some(pos) = response.interact_pointer_pos() else {
        return false;
    };
    let Some(range) = gutter_fold_range_at_pos(
        pos,
        view.scroll_line,
        foldable_ranges,
        visible_doc_lines,
        rect,
        gutter_width,
        line_height,
    ) else {
        return false;
    };

    toggle_fold_range(&mut view.folded_ranges, range);
    true
}

fn gutter_fold_range_at_pos(
    pos: egui::Pos2,
    scroll_line: usize,
    foldable_ranges: &[FoldRange],
    visible_doc_lines: &[usize],
    rect: egui::Rect,
    gutter_width: f32,
    line_height: f32,
) -> Option<FoldRange> {
    if !is_gutter_pos(pos, rect, gutter_width) {
        return None;
    }

    let visible_idx = visible_index_for_y(pos.y, rect, line_height, scroll_line);
    let doc_line = visible_doc_lines.get(visible_idx).copied()?;
    best_fold_range_starting_at(foldable_ranges, doc_line)
}

fn is_gutter_pos(pos: egui::Pos2, rect: egui::Rect, gutter_width: f32) -> bool {
    pos.x < rect.left() + gutter_width
}

fn visible_index_for_y(
    pos_y: f32,
    rect: egui::Rect,
    line_height: f32,
    scroll_line: usize,
) -> usize {
    if line_height <= 0.0 {
        return scroll_line;
    }

    scroll_line + ((pos_y - rect.top()) / line_height).max(0.0) as usize
}

fn editor_pointer_allowed(pos: egui::Pos2, rect: egui::Rect, handled_gutter_click: bool) -> bool {
    !handled_gutter_click && pos.x < rect.right() - MINIMAP_WIDTH
}

#[allow(clippy::too_many_arguments)]
fn handle_editor_click(
    response: &egui::Response,
    buf: &crate::editor::buffer::Buffer,
    view: &mut BufferView,
    visible_doc_lines: &[usize],
    wrap_rows: &[WrapRow],
    rect: egui::Rect,
    gutter_width: f32,
    text_margin: f32,
    h_offset: f32,
    char_width: f32,
    line_height: f32,
    word_wrap: bool,
    handled_gutter_click: bool,
) {
    if !response.clicked() || handled_gutter_click {
        return;
    }
    let Some(pos) = response.interact_pointer_pos() else {
        return;
    };
    if !editor_pointer_allowed(pos, rect, handled_gutter_click) {
        return;
    }

    let (click_line, click_col) = if word_wrap {
        pixel_to_editor_pos_wrapped(
            pos,
            rect,
            gutter_width,
            text_margin,
            char_width,
            line_height,
            view.scroll_line,
            wrap_rows,
            buf,
        )
    } else {
        pixel_to_editor_pos(
            pos,
            rect,
            gutter_width,
            text_margin,
            h_offset,
            char_width,
            line_height,
            view.scroll_line,
            visible_doc_lines,
            buf,
        )
    };
    view.cursor.clear_selection();
    view.cursor.clear_extra_cursors();
    view.cursor.pos = Position::new(click_line, click_col);
    view.cursor.desired_col = None;
}

#[allow(clippy::too_many_arguments)]
fn handle_editor_drag(
    response: &egui::Response,
    buf: &crate::editor::buffer::Buffer,
    view: &mut BufferView,
    visible_doc_lines: &[usize],
    wrap_rows: &[WrapRow],
    rect: egui::Rect,
    gutter_width: f32,
    text_margin: f32,
    h_offset: f32,
    char_width: f32,
    line_height: f32,
    word_wrap: bool,
) {
    if !response.dragged() {
        return;
    }
    let Some(pos) = response.interact_pointer_pos() else {
        return;
    };
    if !editor_pointer_allowed(pos, rect, false) {
        return;
    }

    let (drag_line, drag_col) = if word_wrap {
        pixel_to_editor_pos_wrapped(
            pos,
            rect,
            gutter_width,
            text_margin,
            char_width,
            line_height,
            view.scroll_line,
            wrap_rows,
            buf,
        )
    } else {
        pixel_to_editor_pos(
            pos,
            rect,
            gutter_width,
            text_margin,
            h_offset,
            char_width,
            line_height,
            view.scroll_line,
            visible_doc_lines,
            buf,
        )
    };
    if !view.cursor.has_selection() {
        view.cursor.anchor = Some(view.cursor.pos);
    }
    view.cursor.pos = Position::new(drag_line, drag_col);
    view.cursor.desired_col = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor_rect() -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(300.0, 120.0))
    }

    fn fold(start_line: usize, end_line: usize) -> FoldRange {
        FoldRange {
            start_line,
            end_line,
        }
    }

    #[test]
    fn gutter_hit_testing_excludes_gutter_right_edge() {
        let rect = editor_rect();

        assert!(is_gutter_pos(egui::pos2(49.9, 30.0), rect, 40.0));
        assert!(!is_gutter_pos(egui::pos2(50.0, 30.0), rect, 40.0));
    }

    #[test]
    fn gutter_fold_hit_clamps_negative_y_to_scroll_line() {
        let rect = editor_rect();
        let ranges = [fold(4, 8)];
        let visible_doc_lines = [0, 1, 4, 9];

        let range = gutter_fold_range_at_pos(
            egui::pos2(20.0, -100.0),
            2,
            &ranges,
            &visible_doc_lines,
            rect,
            40.0,
            20.0,
        );

        assert_eq!(range, Some(fold(4, 8)));
    }

    #[test]
    fn gutter_fold_hit_returns_none_when_scrolled_past_visible_lines() {
        let rect = editor_rect();
        let ranges = [fold(10, 12)];
        let visible_doc_lines = [0, 1, 2];

        let range = gutter_fold_range_at_pos(
            egui::pos2(20.0, 35.0),
            10,
            &ranges,
            &visible_doc_lines,
            rect,
            40.0,
            20.0,
        );

        assert_eq!(range, None);
    }

    #[test]
    fn editor_pointer_excludes_minimap_for_clicks_and_drags() {
        let rect = editor_rect();

        assert!(editor_pointer_allowed(
            egui::pos2(rect.right() - MINIMAP_WIDTH - 0.1, 30.0),
            rect,
            false
        ));
        assert!(!editor_pointer_allowed(
            egui::pos2(rect.right() - MINIMAP_WIDTH, 30.0),
            rect,
            false
        ));
    }

    #[test]
    fn editor_pointer_is_blocked_after_gutter_handles_fold_click() {
        let rect = editor_rect();

        assert!(!editor_pointer_allowed(egui::pos2(60.0, 30.0), rect, true));
    }
}
