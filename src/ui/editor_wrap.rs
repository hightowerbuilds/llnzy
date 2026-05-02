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
        if row.doc_line == line && col >= row.col_end {
            if rows.get(i + 1).is_none_or(|next| next.doc_line != line) {
                return i;
            }
        }
    }
    rows.len().saturating_sub(1)
}

pub(super) fn pixel_to_editor_pos_wrapped(
    pos: egui::Pos2,
    rect: egui::Rect,
    gutter_width: f32,
    text_margin: f32,
    char_width: f32,
    line_height: f32,
    scroll_row: usize,
    wrap_rows: &[WrapRow],
    buf: &crate::editor::buffer::Buffer,
) -> (usize, usize) {
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
    let doc_col = (row.col_start + col_in_row).min(buf.line_len(row.doc_line));
    (row.doc_line, doc_col)
}
