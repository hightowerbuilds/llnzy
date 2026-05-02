use crate::editor::buffer::Position;
use crate::editor::syntax::FoldRange;
use crate::editor::BufferView;

use super::editor_folding::{best_fold_range_starting_at, toggle_fold_range};
use super::editor_paint::pixel_to_editor_pos;
use super::editor_wrap::{pixel_to_editor_pos_wrapped, WrapRow};

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
    if pos.x >= rect.left() + gutter_width {
        return false;
    }

    let visible_idx = view.scroll_line + ((pos.y - rect.top()) / line_height).max(0.0) as usize;
    let Some(&doc_line) = visible_doc_lines.get(visible_idx) else {
        return false;
    };
    let Some(range) = best_fold_range_starting_at(foldable_ranges, doc_line) else {
        return false;
    };

    toggle_fold_range(&mut view.folded_ranges, range);
    true
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
    let minimap_w = 50.0_f32;
    if !response.clicked() || handled_gutter_click {
        return;
    }
    let Some(pos) = response.interact_pointer_pos() else {
        return;
    };
    if pos.x >= rect.right() - minimap_w {
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
