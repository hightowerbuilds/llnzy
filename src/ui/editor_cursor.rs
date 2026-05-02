use crate::editor::BufferView;

use super::editor_wrap::WrapRow;

#[allow(clippy::too_many_arguments)]
pub(super) fn render_primary_cursor(
    ui: &egui::Ui,
    painter: &egui::Painter,
    text_clip: egui::Rect,
    view: &mut BufferView,
    visible_window: &[usize],
    visible_wrap_window: &[WrapRow],
    rect: egui::Rect,
    gutter_width: f32,
    text_margin: f32,
    char_width: f32,
    line_height: f32,
    h_offset: f32,
    word_wrap: bool,
) {
    let cursor_vis_info: Option<(f32, f32)> = if word_wrap {
        visible_wrap_window
            .iter()
            .enumerate()
            .find_map(|(vis_idx, row)| {
                if row.doc_line == view.cursor.pos.line
                    && view.cursor.pos.col >= row.col_start
                    && (view.cursor.pos.col < row.col_end
                        || (row.col_end == row.col_start && view.cursor.pos.col == 0)
                        || visible_wrap_window
                            .get(vis_idx + 1)
                            .is_none_or(|next| next.doc_line != row.doc_line))
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

    let Some((vis_row, col_offset)) = cursor_vis_info else {
        return;
    };

    let vis_y = vis_row * line_height;
    let target_x = rect.left() + gutter_width + text_margin + col_offset * char_width
        - if word_wrap { 0.0 } else { h_offset };
    let target_y = rect.top() + vis_y;

    if !view.cursor_display_init {
        view.cursor_display_x = target_x;
        view.cursor_display_y = target_y;
        view.cursor_display_init = true;
    } else {
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
    ui.ctx().request_repaint();
}
