use crate::editor::buffer::Position;
use crate::lsp::{DiagSeverity, FileDiagnostic};

use super::super::{EditorDiagnosticLineRange, EditorDiagnosticSnapshot};

pub(super) fn diagnostic_snapshot(diagnostic: &FileDiagnostic) -> EditorDiagnosticSnapshot {
    EditorDiagnosticSnapshot {
        line: diagnostic.line as usize,
        col: diagnostic.col as usize,
        end_line: diagnostic.end_line as usize,
        end_col: diagnostic.end_col as usize,
        severity: diagnostic.severity,
        message: diagnostic.message.clone(),
    }
}

pub(in crate::gpui_editor) fn diagnostic_for_line(
    diagnostics: &[EditorDiagnosticSnapshot],
    line: usize,
) -> Option<EditorDiagnosticSnapshot> {
    diagnostics
        .iter()
        .filter(|diagnostic| line >= diagnostic.line && line <= diagnostic.end_line)
        .min_by_key(|diagnostic| diagnostic_severity_rank(diagnostic.severity))
        .cloned()
}

pub(in crate::gpui_editor) fn diagnostic_at_position(
    diagnostics: &[EditorDiagnosticSnapshot],
    position: Position,
) -> Option<EditorDiagnosticSnapshot> {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic_contains_position(diagnostic, position))
        .min_by_key(|diagnostic| diagnostic_severity_rank(diagnostic.severity))
        .cloned()
}

fn diagnostic_contains_position(diagnostic: &EditorDiagnosticSnapshot, position: Position) -> bool {
    let start = Position::new(diagnostic.line, diagnostic.col);
    let end = Position::new(diagnostic.end_line, diagnostic.end_col);
    position >= start && (position < end || start == end)
}

pub(in crate::gpui_editor) fn diagnostic_line_range(
    diagnostic: &EditorDiagnosticSnapshot,
    line: usize,
    line_len: usize,
    scroll_col: usize,
    visible_cols: usize,
) -> Option<EditorDiagnosticLineRange> {
    if line < diagnostic.line || line > diagnostic.end_line {
        return None;
    }

    let mut start = if line == diagnostic.line {
        diagnostic.col
    } else {
        0
    }
    .min(line_len);
    let mut end = if line == diagnostic.end_line {
        diagnostic.end_col
    } else {
        line_len
    }
    .min(line_len.max(start + 1));

    if end <= start {
        end = start + 1;
    }

    let visible_start = scroll_col;
    let visible_end = scroll_col + visible_cols.max(1);
    if end <= visible_start || start >= visible_end {
        return None;
    }

    start = start.max(visible_start) - visible_start;
    end = end.min(visible_end) - visible_start;
    (end > start).then_some(EditorDiagnosticLineRange {
        start_col: start,
        end_col: end,
        severity: diagnostic.severity,
    })
}

fn diagnostic_severity_rank(severity: DiagSeverity) -> u8 {
    match severity {
        DiagSeverity::Error => 0,
        DiagSeverity::Warning => 1,
        DiagSeverity::Info => 2,
        DiagSeverity::Hint => 3,
    }
}

pub(in crate::gpui_editor) fn diagnostic_status(
    diagnostics: &[EditorDiagnosticSnapshot],
) -> String {
    if diagnostics.is_empty() {
        return String::new();
    }

    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == DiagSeverity::Error)
        .count();
    let warnings = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == DiagSeverity::Warning)
        .count();
    match (errors, warnings) {
        (0, 0) => format!("{} diagnostic(s)", diagnostics.len()),
        (0, warnings) => format!("{warnings} warning(s)"),
        (errors, 0) => format!("{errors} error(s)"),
        (errors, warnings) => format!("{errors} error(s), {warnings} warning(s)"),
    }
}
