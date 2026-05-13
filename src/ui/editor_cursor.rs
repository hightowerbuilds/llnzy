use crate::editor::BufferView;

use super::editor_paint::EditorPaintContext;
use super::editor_wrap::WrapRow;

pub(super) fn render_primary_cursor(
    ui: &egui::Ui,
    ctx: EditorPaintContext<'_>,
    view: &mut BufferView,
    visible_window: &[usize],
    visible_wrap_window: &[WrapRow],
    word_wrap: bool,
) {
    let geometry = ctx.geometry;
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

    let target_x = if word_wrap {
        geometry.wrapped_text_x(col_offset as usize)
    } else {
        geometry.text_x(col_offset as usize)
    };
    let target_y = geometry.rect.top() + vis_row * geometry.line_height;

    view.cursor_display_x = target_x;
    view.cursor_display_y = target_y;
    view.cursor_display_init = true;

    let cursor_x = view.cursor_display_x;
    let cursor_y = view.cursor_display_y;
    if cursor_x >= geometry.text_clip.left() && cursor_x <= geometry.text_clip.right() {
        let time = ui.ctx().input(|i| i.time);
        let blink_cycle = (time * 2.0 * std::f64::consts::PI / 1.2) as f32;
        let opacity = (blink_cycle.sin() * 0.5 + 0.5).clamp(0.15, 1.0);
        let alpha = (opacity * 255.0) as u8;
        let cursor_color = egui::Color32::from_rgba_unmultiplied(80, 160, 255, alpha);
        ctx.painter.with_clip_rect(geometry.text_clip).line_segment(
            [
                egui::pos2(cursor_x, cursor_y + 1.0),
                egui::pos2(cursor_x, cursor_y + geometry.line_height - 1.0),
            ],
            egui::Stroke::new(2.0, cursor_color),
        );
    }
    ui.ctx().request_repaint();
}
