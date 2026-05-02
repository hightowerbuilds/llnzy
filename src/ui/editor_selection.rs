use crate::editor::buffer::Position;
use crate::editor::search::EditorSearch;
use crate::editor::BufferView;

use super::editor_wrap::WrapRow;

#[allow(clippy::too_many_arguments)]
pub(super) fn render_primary_selection(
    painter: &egui::Painter,
    text_clip: egui::Rect,
    buf: &crate::editor::buffer::Buffer,
    view: &BufferView,
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
    let Some((sel_start, sel_end)) = view.cursor.selection() else {
        return;
    };

    let sel_color = egui::Color32::from_rgba_unmultiplied(60, 100, 180, 80);
    if word_wrap {
        for (vis_idx, row) in visible_wrap_window.iter().enumerate() {
            if row.doc_line < sel_start.line || row.doc_line > sel_end.line {
                continue;
            }
            let vis_y = vis_idx as f32 * line_height;
            let line_len = buf.line_len(row.doc_line);
            let doc_col_start = if row.doc_line == sel_start.line {
                sel_start.col
            } else {
                0
            };
            let doc_col_end = if row.doc_line == sel_end.line {
                sel_end.col
            } else {
                line_len
            };
            let row_sel_start = doc_col_start
                .max(row.col_start)
                .saturating_sub(row.col_start);
            let row_sel_end = doc_col_end.min(row.col_end).saturating_sub(row.col_start);
            if row_sel_start >= row_sel_end {
                continue;
            }
            let x1 = rect.left() + gutter_width + text_margin + row_sel_start as f32 * char_width;
            let x2 = rect.left() + gutter_width + text_margin + row_sel_end as f32 * char_width;
            let sel_rect = egui::Rect::from_min_max(
                egui::pos2(x1.max(text_clip.left()), rect.top() + vis_y),
                egui::pos2(
                    x2.max(x1 + char_width).min(text_clip.right()),
                    rect.top() + vis_y + line_height,
                ),
            );
            if sel_rect.width() > 0.0 {
                painter.rect_filled(sel_rect, 0.0, sel_color);
            }
        }
    } else {
        render_selection_range(
            painter,
            text_clip,
            buf,
            visible_window,
            rect,
            gutter_width,
            text_margin,
            char_width,
            line_height,
            h_offset,
            sel_start,
            sel_end,
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_search_matches(
    painter: &egui::Painter,
    text_clip: egui::Rect,
    buf: &crate::editor::buffer::Buffer,
    editor_search: &mut EditorSearch,
    visible_window: &[usize],
    rect: egui::Rect,
    gutter_width: f32,
    text_margin: f32,
    char_width: f32,
    line_height: f32,
    h_offset: f32,
) {
    if !editor_search.active {
        return;
    }

    editor_search.update_if_dirty(buf);
    let match_bg = egui::Color32::from_rgba_unmultiplied(230, 180, 50, 40);
    let focus_bg = egui::Color32::from_rgba_unmultiplied(230, 180, 50, 100);
    for (i, m) in editor_search.matches.iter().enumerate() {
        let start_line = m.start.line;
        let end_line = m.end.line;
        for (visible_offset, &line_idx) in visible_window.iter().enumerate() {
            if line_idx < start_line || line_idx > end_line {
                continue;
            }
            let vis_y = visible_offset as f32 * line_height;
            let col_start = if line_idx == start_line {
                m.start.col
            } else {
                0
            };
            let col_end = if line_idx == end_line {
                m.end.col
            } else {
                buf.line_len(line_idx)
            };
            let x1 =
                rect.left() + gutter_width + text_margin + col_start as f32 * char_width - h_offset;
            let x2 =
                rect.left() + gutter_width + text_margin + col_end as f32 * char_width - h_offset;
            let color = if i == editor_search.focus {
                focus_bg
            } else {
                match_bg
            };
            let match_rect = egui::Rect::from_min_max(
                egui::pos2(x1.max(text_clip.left()), rect.top() + vis_y),
                egui::pos2(x2.min(text_clip.right()), rect.top() + vis_y + line_height),
            );
            if match_rect.width() > 0.0 {
                painter.rect_filled(match_rect, 0.0, color);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_extra_cursors(
    ui: &egui::Ui,
    painter: &egui::Painter,
    text_clip: egui::Rect,
    buf: &crate::editor::buffer::Buffer,
    view: &BufferView,
    visible_window: &[usize],
    rect: egui::Rect,
    gutter_width: f32,
    text_margin: f32,
    char_width: f32,
    line_height: f32,
    h_offset: f32,
) {
    for extra in &view.cursor.extra_cursors {
        if let Some(extra_visible_offset) = visible_window
            .iter()
            .position(|&line| line == extra.pos.line)
        {
            let vis_y = extra_visible_offset as f32 * line_height;
            let cursor_x =
                rect.left() + gutter_width + text_margin + extra.pos.col as f32 * char_width
                    - h_offset;
            let cursor_y = rect.top() + vis_y;
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
        }

        if let Some(anchor) = extra.anchor {
            if anchor != extra.pos {
                let (sel_start, sel_end) = if anchor <= extra.pos {
                    (anchor, extra.pos)
                } else {
                    (extra.pos, anchor)
                };
                render_selection_range(
                    painter,
                    text_clip,
                    buf,
                    visible_window,
                    rect,
                    gutter_width,
                    text_margin,
                    char_width,
                    line_height,
                    h_offset,
                    sel_start,
                    sel_end,
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_selection_range(
    painter: &egui::Painter,
    text_clip: egui::Rect,
    buf: &crate::editor::buffer::Buffer,
    visible_window: &[usize],
    rect: egui::Rect,
    gutter_width: f32,
    text_margin: f32,
    char_width: f32,
    line_height: f32,
    h_offset: f32,
    sel_start: Position,
    sel_end: Position,
) {
    let sel_color = egui::Color32::from_rgba_unmultiplied(60, 100, 180, 80);
    for (visible_offset, &line_idx) in visible_window.iter().enumerate() {
        if line_idx < sel_start.line || line_idx > sel_end.line {
            continue;
        }
        let vis_y = visible_offset as f32 * line_height;
        let line_len = buf.line_len(line_idx);
        let col_start = if line_idx == sel_start.line {
            sel_start.col
        } else {
            0
        };
        let col_end = if line_idx == sel_end.line {
            sel_end.col
        } else {
            line_len
        };
        let x1 =
            rect.left() + gutter_width + text_margin + col_start as f32 * char_width - h_offset;
        let x2 = rect.left() + gutter_width + text_margin + col_end as f32 * char_width - h_offset;
        let sel_rect = egui::Rect::from_min_max(
            egui::pos2(x1.max(text_clip.left()), rect.top() + vis_y),
            egui::pos2(
                x2.max(x1 + char_width).min(text_clip.right()),
                rect.top() + vis_y + line_height,
            ),
        );
        if sel_rect.width() > 0.0 {
            painter.rect_filled(sel_rect, 0.0, sel_color);
        }
    }
}
