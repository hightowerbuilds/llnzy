use std::path::PathBuf;

use super::client::uri_to_path;
use super::types::FormatEdit;

pub(super) fn parse_workspace_edit(
    edit: lsp_types::WorkspaceEdit,
) -> Vec<(PathBuf, Vec<FormatEdit>)> {
    let mut result = Vec::new();
    if let Some(changes) = edit.changes {
        for (uri, edits) in changes {
            let Some(path) = uri_to_path(&uri) else {
                continue;
            };
            let file_edits: Vec<FormatEdit> = edits
                .into_iter()
                .map(|e| FormatEdit {
                    start_line: e.range.start.line,
                    start_col: e.range.start.character,
                    end_line: e.range.end.line,
                    end_col: e.range.end.character,
                    new_text: e.new_text,
                })
                .collect();
            result.push((path, file_edits));
        }
    }
    result
}
