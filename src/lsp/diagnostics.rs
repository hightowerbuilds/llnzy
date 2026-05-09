use std::path::{Path, PathBuf};

use rustc_hash::FxHashMap;
use serde_json::Value;

use super::client::uri_to_path;
use super::manager::LspManager;
use super::types::FileDiagnostic;

impl LspManager {
    /// Get diagnostics for a specific file.
    pub fn get_diagnostics(&self, path: &Path) -> &[FileDiagnostic] {
        self.diagnostics
            .get(path)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub(super) fn handle_diagnostics_notification(&mut self, params: Value) {
        let Some((path, diags)) = diagnostics_from_publish_params(params) else {
            return;
        };

        self.diagnostics.insert(path, diags);
    }
}

fn diagnostics_from_publish_params(params: Value) -> Option<(PathBuf, Vec<FileDiagnostic>)> {
    let params = serde_json::from_value::<lsp_types::PublishDiagnosticsParams>(params).ok()?;
    let path = uri_to_path(&params.uri)?;
    let diags = params
        .diagnostics
        .into_iter()
        .map(|d| FileDiagnostic {
            line: d.range.start.line,
            col: d.range.start.character,
            end_line: d.range.end.line,
            end_col: d.range.end.character,
            severity: d.severity.into(),
            message: d.message,
            source: d.source,
        })
        .collect();

    Some((path, diags))
}

pub(super) fn clear_document_diagnostics(
    diagnostics: &mut FxHashMap<PathBuf, Vec<FileDiagnostic>>,
    path: &Path,
) {
    diagnostics.remove(path);
}

pub(super) fn remap_document_diagnostics(
    diagnostics: &mut FxHashMap<PathBuf, Vec<FileDiagnostic>>,
    old_path: &Path,
    new_path: PathBuf,
) {
    if old_path == new_path.as_path() {
        return;
    }
    if let Some(diags) = diagnostics.remove(old_path) {
        diagnostics.insert(new_path, diags);
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::DiagSeverity;
    use super::*;

    fn diagnostic(message: &str) -> FileDiagnostic {
        FileDiagnostic {
            line: 1,
            col: 2,
            end_line: 1,
            end_col: 5,
            severity: DiagSeverity::Warning,
            message: message.to_string(),
            source: Some("test".to_string()),
        }
    }

    #[test]
    fn clearing_document_diagnostics_removes_only_that_file() {
        let first = PathBuf::from("/workspace/src/main.rs");
        let second = PathBuf::from("/workspace/src/lib.rs");
        let mut diagnostics = FxHashMap::from_iter([
            (first.clone(), vec![diagnostic("first")]),
            (second.clone(), vec![diagnostic("second")]),
        ]);

        clear_document_diagnostics(&mut diagnostics, &first);

        assert!(!diagnostics.contains_key(&first));
        assert_eq!(diagnostics[&second][0].message, "second");
    }

    #[test]
    fn remapping_document_diagnostics_moves_existing_entries() {
        let old_path = PathBuf::from("/workspace/src/old.rs");
        let new_path = PathBuf::from("/workspace/src/new.rs");
        let mut diagnostics = FxHashMap::from_iter([(old_path.clone(), vec![diagnostic("moved")])]);

        remap_document_diagnostics(&mut diagnostics, &old_path, new_path.clone());

        assert!(!diagnostics.contains_key(&old_path));
        assert_eq!(diagnostics[&new_path][0].message, "moved");
    }
}
