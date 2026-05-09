#[derive(Clone, Copy, Debug)]
pub(super) struct WrapRow {
    pub(super) doc_line: usize,
    pub(super) col_start: usize,
    pub(super) col_end: usize,
    pub(super) is_first: bool,
}

pub(super) fn compute_wrap_rows(
    visible_doc_lines: &[usize],
    buf: &crate::editor::buffer::Buffer,
    visible_cols: usize,
) -> Vec<WrapRow> {
    let wrap_col = visible_cols.max(10);
    let mut rows = Vec::new();
    for &doc_line in visible_doc_lines {
        let line_len = buf.line_len(doc_line);
        if line_len == 0 {
            rows.push(WrapRow {
                doc_line,
                col_start: 0,
                col_end: 0,
                is_first: true,
            });
            continue;
        }
        let mut col = 0;
        let mut first = true;
        while col < line_len {
            let end = (col + wrap_col).min(line_len);
            rows.push(WrapRow {
                doc_line,
                col_start: col,
                col_end: end,
                is_first: first,
            });
            first = false;
            col = end;
        }
    }
    rows
}

pub(super) fn wrap_row_for_cursor(rows: &[WrapRow], line: usize, col: usize) -> usize {
    for (i, row) in rows.iter().enumerate() {
        if row.doc_line == line
            && col >= row.col_start
            && (col < row.col_end || (row.col_end == row.col_start && col == 0))
        {
            return i;
        }
        if row.doc_line == line
            && col >= row.col_end
            && rows.get(i + 1).is_none_or(|next| next.doc_line != line)
        {
            return i;
        }
    }
    rows.len().saturating_sub(1)
}

pub(super) struct WrappedHitTestInput<'a> {
    pub pos: egui::Pos2,
    pub rect: egui::Rect,
    pub gutter_width: f32,
    pub text_margin: f32,
    pub char_width: f32,
    pub line_height: f32,
    pub scroll_row: usize,
    pub wrap_rows: &'a [WrapRow],
    pub buf: &'a crate::editor::buffer::Buffer,
}

pub(super) fn pixel_to_editor_pos_wrapped(input: WrappedHitTestInput<'_>) -> (usize, usize) {
    let WrappedHitTestInput {
        pos,
        rect,
        gutter_width,
        text_margin,
        char_width,
        line_height,
        scroll_row,
        wrap_rows,
        buf,
    } = input;
    let rel_x = pos.x - rect.left() - gutter_width - text_margin;
    let rel_y = pos.y - rect.top();
    let visual_row = (scroll_row + (rel_y / line_height).max(0.0) as usize)
        .min(wrap_rows.len().saturating_sub(1));
    let row = wrap_rows.get(visual_row).copied().unwrap_or(WrapRow {
        doc_line: 0,
        col_start: 0,
        col_end: 0,
        is_first: true,
    });
    let col_in_row = (rel_x / char_width).max(0.0) as usize;
    let doc_col = (row.col_start + col_in_row)
        .min(row.col_end)
        .min(buf.line_len(row.doc_line));
    (row.doc_line, doc_col)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::buffer::{Buffer, Position};

    fn buf_with(text: &str) -> Buffer {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), text);
        buf
    }

    #[test]
    fn wrapped_hit_testing_clamps_to_current_visual_row_end() {
        let buf = buf_with("abcdefghijklmnopqrstuvwxy");
        let rows = compute_wrap_rows(&[0], &buf, 10);
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(300.0, 80.0));

        let (line, col) = pixel_to_editor_pos_wrapped(WrappedHitTestInput {
            pos: egui::pos2(260.0, 5.0),
            rect,
            gutter_width: 20.0,
            text_margin: 4.0,
            char_width: 10.0,
            line_height: 20.0,
            scroll_row: 0,
            wrap_rows: &rows,
            buf: &buf,
        });

        assert_eq!((line, col), (0, 10));
    }

    #[test]
    fn wrapped_hit_testing_uses_scrolled_visual_row() {
        let buf = buf_with("abcdefghijklmnopqrstuvwxy");
        let rows = compute_wrap_rows(&[0], &buf, 10);
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(300.0, 80.0));

        let (line, col) = pixel_to_editor_pos_wrapped(WrappedHitTestInput {
            pos: egui::pos2(44.0, 5.0),
            rect,
            gutter_width: 20.0,
            text_margin: 4.0,
            char_width: 10.0,
            line_height: 20.0,
            scroll_row: 1,
            wrap_rows: &rows,
            buf: &buf,
        });

        assert_eq!((line, col), (0, 12));
    }

    #[test]
    fn wrap_row_for_cursor_uses_last_row_at_line_end() {
        let buf = buf_with("abcdefghijklmnopqrstuvwxy");
        let rows = compute_wrap_rows(&[0], &buf, 10);

        assert_eq!(wrap_row_for_cursor(&rows, 0, buf.line_len(0)), 2);
    }
}
