use std::fs;
use std::path::Path;

use crate::atomic_write::atomic_write;
use crate::lsp::FormatEdit;

use super::super::byte_index_for_char_col;

pub(super) fn apply_format_edits_to_file(
    path: &Path,
    edits: &[FormatEdit],
) -> Result<usize, String> {
    if edits.is_empty() {
        return Ok(0);
    }
    let text =
        fs::read_to_string(path).map_err(|err| format!("read {} failed: {err}", path.display()))?;
    let (new_text, applied) = apply_format_edits_to_text(&text, edits)?;
    if applied > 0 {
        atomic_write(path, new_text.as_bytes())
            .map_err(|err| format!("write {} failed: {err}", path.display()))?;
    }
    Ok(applied)
}

pub(super) fn apply_format_edits_to_text(
    text: &str,
    edits: &[FormatEdit],
) -> Result<(String, usize), String> {
    let mut output = text.to_string();
    let mut sorted = edits.to_vec();
    sorted.sort_by(|a, b| {
        b.start_line
            .cmp(&a.start_line)
            .then(b.start_col.cmp(&a.start_col))
    });

    let mut applied = 0;
    for edit in sorted {
        let start =
            text_position_to_byte_index(&output, edit.start_line as usize, edit.start_col as usize)
                .ok_or_else(|| "edit start is out of bounds".to_string())?;
        let end =
            text_position_to_byte_index(&output, edit.end_line as usize, edit.end_col as usize)
                .ok_or_else(|| "edit end is out of bounds".to_string())?;
        if start > end {
            return Err("edit start is after edit end".to_string());
        }
        output.replace_range(start..end, &edit.new_text);
        applied += 1;
    }
    Ok((output, applied))
}

pub(super) fn text_position_to_byte_index(text: &str, line: usize, col: usize) -> Option<usize> {
    let mut current_line = 0;
    let mut line_start = 0;
    for segment in text.split_inclusive('\n') {
        let line_without_newline = segment.trim_end_matches('\n').trim_end_matches('\r');
        if current_line == line {
            return Some(line_start + byte_index_for_char_col(line_without_newline, col));
        }
        line_start += segment.len();
        current_line += 1;
    }

    if current_line == line {
        let tail = &text[line_start..];
        return Some(line_start + byte_index_for_char_col(tail, col));
    }
    None
}
