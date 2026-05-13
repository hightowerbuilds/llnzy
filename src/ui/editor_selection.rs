use crate::editor::buffer::Position;
use crate::editor::search::EditorSearch;
use crate::editor::BufferView;

use super::editor_line_decorations::visible_line_offset;
use super::editor_paint::EditorPaintContext;
use super::editor_wrap::WrapRow;

pub(super) fn render_primary_selection(
    ctx: EditorPaintContext<'_>,
    buf: &crate::editor::buffer::Buffer,
    view: &BufferView,
    visible_window: &[usize],
    visible_wrap_window: &[WrapRow],
    word_wrap: bool,
) {
    let Some((sel_start, sel_end)) = view.cursor.selection() else {
        return;
    };

    let geometry = ctx.geometry;
    let sel_color = egui::Color32::from_rgba_unmultiplied(60, 100, 180, 80);
    if word_wrap {
        for (vis_idx, row) in visible_wrap_window.iter().enumerate() {
            if row.doc_line < sel_start.line || row.doc_line > sel_end.line {
                continue;
            }
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
            let x1 = geometry.wrapped_text_x(row_sel_start);
            let x2 = geometry.wrapped_text_x(row_sel_end);
            let y = geometry.line_y(vis_idx);
            let sel_rect = egui::Rect::from_min_max(
                egui::pos2(x1.max(geometry.text_clip.left()), y),
                egui::pos2(
                    x2.max(x1 + geometry.char_width)
                        .min(geometry.text_clip.right()),
                    y + geometry.line_height,
                ),
            );
            if sel_rect.width() > 0.0 {
                ctx.painter.rect_filled(sel_rect, 0.0, sel_color);
            }
        }
    } else {
        render_selection_range(ctx, buf, visible_window, sel_start, sel_end);
    }
}

pub(super) fn render_search_matches(
    ctx: EditorPaintContext<'_>,
    buf: &crate::editor::buffer::Buffer,
    editor_search: &mut EditorSearch,
    visible_window: &[usize],
) {
    if !editor_search.active {
        return;
    }

    let geometry = ctx.geometry;
    editor_search.update_if_dirty(buf);
    let match_bg = egui::Color32::from_rgba_unmultiplied(230, 180, 50, 40);
    let focus_bg = egui::Color32::from_rgba_unmultiplied(230, 180, 50, 100);

    let mut matches_by_visible_line = vec![Vec::new(); visible_window.len()];
    for (i, m) in editor_search.matches.iter().enumerate() {
        let start_line = m.start.line;
        let end_line = m.end.line;
        let first_visible = visible_window.partition_point(|&line| line < start_line);
        for (visible_offset, &line_idx) in visible_window.iter().enumerate().skip(first_visible) {
            if line_idx > end_line {
                break;
            }
            matches_by_visible_line[visible_offset].push((i, m));
        }
    }

    for (visible_offset, (&line_idx, line_matches)) in visible_window
        .iter()
        .zip(matches_by_visible_line.iter())
        .enumerate()
    {
        for &(i, m) in line_matches {
            let start_line = m.start.line;
            let end_line = m.end.line;
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
            let x1 = geometry.text_x(col_start);
            let x2 = geometry.text_x(col_end);
            let color = if i == editor_search.focus {
                focus_bg
            } else {
                match_bg
            };
            let y = geometry.line_y(visible_offset);
            let match_rect = egui::Rect::from_min_max(
                egui::pos2(x1.max(geometry.text_clip.left()), y),
                egui::pos2(x2.min(geometry.text_clip.right()), y + geometry.line_height),
            );
            if match_rect.width() > 0.0 {
                ctx.painter.rect_filled(match_rect, 0.0, color);
            }
        }
    }
}

pub(super) fn render_extra_cursors(
    ui: &egui::Ui,
    ctx: EditorPaintContext<'_>,
    buf: &crate::editor::buffer::Buffer,
    view: &BufferView,
    visible_window: &[usize],
) {
    let geometry = ctx.geometry;
    for extra in &view.cursor.extra_cursors {
        if let Some(extra_visible_offset) = visible_line_offset(visible_window, extra.pos.line) {
            let cursor_x = geometry.text_x(extra.pos.col);
            let cursor_y = geometry.line_y(extra_visible_offset);
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
        }

        if let Some(anchor) = extra.anchor {
            if anchor != extra.pos {
                let (sel_start, sel_end) = if anchor <= extra.pos {
                    (anchor, extra.pos)
                } else {
                    (extra.pos, anchor)
                };
                render_selection_range(ctx, buf, visible_window, sel_start, sel_end);
            }
        }
    }
}

fn render_selection_range(
    ctx: EditorPaintContext<'_>,
    buf: &crate::editor::buffer::Buffer,
    visible_window: &[usize],
    sel_start: Position,
    sel_end: Position,
) {
    let geometry = ctx.geometry;
    let sel_color = egui::Color32::from_rgba_unmultiplied(60, 100, 180, 80);
    let first_visible = visible_window.partition_point(|&line| line < sel_start.line);
    for (visible_offset, &line_idx) in visible_window.iter().enumerate().skip(first_visible) {
        if line_idx > sel_end.line {
            break;
        }
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
        let x1 = geometry.text_x(col_start);
        let x2 = geometry.text_x(col_end);
        let y = geometry.line_y(visible_offset);
        let sel_rect = egui::Rect::from_min_max(
            egui::pos2(x1.max(geometry.text_clip.left()), y),
            egui::pos2(
                x2.max(x1 + geometry.char_width)
                    .min(geometry.text_clip.right()),
                y + geometry.line_height,
            ),
        );
        if sel_rect.width() > 0.0 {
            ctx.painter.rect_filled(sel_rect, 0.0, sel_color);
        }
    }
}
