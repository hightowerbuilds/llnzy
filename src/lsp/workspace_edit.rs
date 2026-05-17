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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    #[allow(clippy::mutable_key_type)]
    fn parse_workspace_edit_decodes_file_uri_paths() {
        let uri = "file:///tmp/llnzy%20dir/main%23.rs".parse().unwrap();
        let edit = lsp_types::TextEdit {
            range: lsp_types::Range {
                start: lsp_types::Position {
                    line: 1,
                    character: 2,
                },
                end: lsp_types::Position {
                    line: 3,
                    character: 4,
                },
            },
            new_text: "replacement".to_string(),
        };
        let changes = HashMap::from([(uri, vec![edit])]);

        let parsed = parse_workspace_edit(lsp_types::WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        });

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].0, PathBuf::from("/tmp/llnzy dir/main#.rs"));
        assert_eq!(parsed[0].1[0].start_line, 1);
        assert_eq!(parsed[0].1[0].start_col, 2);
        assert_eq!(parsed[0].1[0].end_line, 3);
        assert_eq!(parsed[0].1[0].end_col, 4);
        assert_eq!(parsed[0].1[0].new_text, "replacement");
    }
}
