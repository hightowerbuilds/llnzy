use std::path::PathBuf;

use crate::editor::buffer::{BufferEdit, Position};
use crate::lsp::{CodeAction, CompletionItem, DiagSeverity, FormatEdit, ReferenceLocation};

use super::super::{EditorDiagnosticLineRange, EditorDiagnosticSnapshot};
use super::formatting::{apply_format_edits_to_text, text_position_to_byte_index};
use super::panels::{
    code_action_panel_items, completion_panel_items, lsp_panel, references_panel_items,
    sanitize_lsp_insert_text, truncate_panel_text, GpuiLspPanelItem,
};
use super::*;

#[test]
fn diagnostic_for_line_prefers_highest_severity_on_that_line() {
    let diagnostics = vec![
        EditorDiagnosticSnapshot {
            line: 3,
            col: 8,
            end_line: 3,
            end_col: 15,
            severity: DiagSeverity::Warning,
            message: "warning".into(),
        },
        EditorDiagnosticSnapshot {
            line: 3,
            col: 2,
            end_line: 3,
            end_col: 6,
            severity: DiagSeverity::Error,
            message: "error".into(),
        },
        EditorDiagnosticSnapshot {
            line: 4,
            col: 0,
            end_line: 4,
            end_col: 4,
            severity: DiagSeverity::Hint,
            message: "hint".into(),
        },
    ];

    let diagnostic = diagnostic_for_line(&diagnostics, 3).unwrap();
    assert_eq!(diagnostic.severity, DiagSeverity::Error);
    assert_eq!(diagnostic.message, "error");
    assert!(diagnostic_for_line(&diagnostics, 9).is_none());
}

#[test]
fn diagnostic_line_range_clips_to_visible_columns() {
    let diagnostic = EditorDiagnosticSnapshot {
        line: 2,
        col: 4,
        end_line: 2,
        end_col: 12,
        severity: DiagSeverity::Warning,
        message: "range".into(),
    };

    assert_eq!(
        diagnostic_line_range(&diagnostic, 2, 20, 0, 80),
        Some(EditorDiagnosticLineRange {
            start_col: 4,
            end_col: 12,
            severity: DiagSeverity::Warning,
        })
    );
    assert_eq!(
        diagnostic_line_range(&diagnostic, 2, 20, 8, 4),
        Some(EditorDiagnosticLineRange {
            start_col: 0,
            end_col: 4,
            severity: DiagSeverity::Warning,
        })
    );
    assert!(diagnostic_line_range(&diagnostic, 2, 20, 13, 4).is_none());
}

#[test]
fn diagnostic_for_line_includes_multiline_diagnostics() {
    let diagnostics = vec![EditorDiagnosticSnapshot {
        line: 1,
        col: 5,
        end_line: 3,
        end_col: 2,
        severity: DiagSeverity::Info,
        message: "multi".into(),
    }];

    assert!(diagnostic_for_line(&diagnostics, 1).is_some());
    assert!(diagnostic_for_line(&diagnostics, 2).is_some());
    assert!(diagnostic_for_line(&diagnostics, 3).is_some());
    assert!(diagnostic_for_line(&diagnostics, 4).is_none());
}

#[test]
fn diagnostic_at_position_prefers_highest_severity() {
    let diagnostics = vec![
        EditorDiagnosticSnapshot {
            line: 1,
            col: 0,
            end_line: 1,
            end_col: 10,
            severity: DiagSeverity::Info,
            message: "info".into(),
        },
        EditorDiagnosticSnapshot {
            line: 1,
            col: 2,
            end_line: 1,
            end_col: 4,
            severity: DiagSeverity::Error,
            message: "error".into(),
        },
    ];

    let diagnostic = diagnostic_at_position(&diagnostics, Position::new(1, 3)).unwrap();
    assert_eq!(diagnostic.severity, DiagSeverity::Error);
    assert!(diagnostic_at_position(&diagnostics, Position::new(1, 10)).is_none());
}

#[test]
fn lsp_change_kind_uses_incremental_until_edits_coalesce() {
    let edit = BufferEdit {
        start: Position::new(1, 2),
        old_end: Position::new(1, 2),
        new_end: Position::new(1, 5),
        new_text: "abc".into(),
    };

    assert_eq!(
        next_lsp_change_kind(None, Some(edit.clone())),
        GpuiPendingLspChangeKind::Incremental {
            start: Position::new(1, 2),
            old_end: Position::new(1, 2),
            new_text: "abc".into(),
        }
    );
    assert_eq!(
        next_lsp_change_kind(
            Some(&GpuiPendingLspChangeKind::Incremental {
                start: edit.start,
                old_end: edit.old_end,
                new_text: edit.new_text.clone(),
            }),
            Some(edit)
        ),
        GpuiPendingLspChangeKind::Full
    );
}

#[test]
fn lsp_panel_helpers_keep_items_compact() {
    let truncated = truncate_panel_text("abcdefghijklmnopqrstuvwxyz".to_string(), 12);
    assert_eq!(truncated, "abcdefghi...");

    let completions = completion_panel_items(
        vec![
            CompletionItem {
                label: "render".into(),
                detail: Some("fn()".into()),
                insert_text: Some("render()".into()),
                kind: None,
            },
            CompletionItem {
                label: "ignored".into(),
                detail: None,
                insert_text: None,
                kind: None,
            },
        ],
        1,
    );
    assert_eq!(
        completions
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        vec!["render  fn()"]
    );
    assert!(matches!(
        &completions[0].action,
        GpuiLspPanelAction::Complete { text } if text == "render()"
    ));

    let references = references_panel_items(
        vec![ReferenceLocation {
            path: PathBuf::from("/tmp/app.rs"),
            line: 1,
            col: 2,
            context: "  render();  ".into(),
        }],
        1,
    );
    assert_eq!(
        references
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        vec!["app.rs:2:3  render();"]
    );
    assert!(matches!(
        &references[0].action,
        GpuiLspPanelAction::GoTo { path, line: 1, col: 2 } if path == &PathBuf::from("/tmp/app.rs")
    ));

    let code_actions = code_action_panel_items(
        vec![CodeAction {
            title: "Apply fix".into(),
            edits: vec![(
                PathBuf::from("/tmp/app.rs"),
                vec![FormatEdit {
                    start_line: 0,
                    start_col: 0,
                    end_line: 0,
                    end_col: 0,
                    new_text: "fixed".into(),
                }],
            )],
        }],
        1,
    );
    assert_eq!(code_actions[0].label, "Apply fix");
    assert!(matches!(
        &code_actions[0].action,
        GpuiLspPanelAction::ApplyWorkspaceEdit { edits } if edits.len() == 1
    ));
}

#[test]
fn lsp_panel_selects_first_actionable_item() {
    let panel = lsp_panel(
        "Mixed",
        vec![
            GpuiLspPanelItem::plain("Header".into()),
            GpuiLspPanelItem {
                label: "Action".into(),
                action: GpuiLspPanelAction::Complete {
                    text: "done".into(),
                },
            },
        ],
    );

    assert_eq!(panel.selected, 1);
}

#[test]
fn snippet_insert_text_is_sanitized_for_plain_insert() {
    assert_eq!(
        sanitize_lsp_insert_text("println!(\"${1:value}\");$0"),
        "println!(\"value\");"
    );
    assert_eq!(sanitize_lsp_insert_text("$1name"), "name");
}

#[test]
fn format_edits_apply_to_unopened_file_text() {
    let edits = vec![FormatEdit {
        start_line: 0,
        start_col: 3,
        end_line: 1,
        end_col: 2,
        new_text: "X".into(),
    }];

    let (text, applied) = apply_format_edits_to_text("abc\ndef\n", &edits).unwrap();

    assert_eq!(applied, 1);
    assert_eq!(text, "abcXf\n");
}

#[test]
fn text_position_to_byte_index_handles_trailing_empty_line() {
    assert_eq!(text_position_to_byte_index("abc\n", 1, 0), Some(4));
    assert_eq!(text_position_to_byte_index("", 0, 0), Some(0));
    assert_eq!(text_position_to_byte_index("abc", 2, 0), None);
}
