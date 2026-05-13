use std::path::{Path, PathBuf};

use lsp_types::*;

use crate::path_utils::file_name_or_display;

use super::super::document::path_to_uri;

#[allow(deprecated)] // root_uri remains populated for compatibility with older servers.
pub(super) fn initialize_params(
    root_uri: Option<Uri>,
    workspace_folders: Vec<WorkspaceFolder>,
) -> InitializeParams {
    InitializeParams {
        root_uri,
        workspace_folders: if workspace_folders.is_empty() {
            None
        } else {
            Some(workspace_folders)
        },
        capabilities: ClientCapabilities {
            workspace: Some(WorkspaceClientCapabilities {
                workspace_folders: Some(true),
                ..Default::default()
            }),
            text_document: Some(TextDocumentClientCapabilities {
                synchronization: Some(TextDocumentSyncClientCapabilities {
                    dynamic_registration: Some(false),
                    will_save: Some(false),
                    will_save_wait_until: Some(false),
                    did_save: Some(true),
                }),
                completion: Some(CompletionClientCapabilities {
                    completion_item: Some(CompletionItemCapability {
                        snippet_support: Some(false),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                hover: Some(HoverClientCapabilities {
                    content_format: Some(vec![MarkupKind::PlainText]),
                    ..Default::default()
                }),
                publish_diagnostics: Some(PublishDiagnosticsClientCapabilities {
                    related_information: Some(true),
                    ..Default::default()
                }),
                definition: Some(GotoCapability {
                    dynamic_registration: Some(false),
                    link_support: Some(false),
                }),
                references: Some(DynamicRegistrationClientCapabilities {
                    dynamic_registration: Some(false),
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub(super) fn workspace_folders_from_paths(paths: &[PathBuf]) -> Vec<WorkspaceFolder> {
    paths
        .iter()
        .filter_map(|path| {
            Some(WorkspaceFolder {
                uri: path_to_uri(path).ok()?,
                name: workspace_folder_name(path),
            })
        })
        .collect()
}

fn workspace_folder_name(path: &Path) -> String {
    file_name_or_display(path).into_owned()
}

pub(super) fn workspace_folder_additions(
    current: &[WorkspaceFolder],
    desired: &[WorkspaceFolder],
) -> Vec<WorkspaceFolder> {
    desired
        .iter()
        .filter(|folder| !current.iter().any(|current| current.uri == folder.uri))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(deprecated)] // This test verifies the compatibility root_uri fallback.
    fn initialize_params_include_workspace_folders_and_capability() {
        let roots = vec![
            PathBuf::from("/workspace/app"),
            PathBuf::from("/workspace/tools"),
        ];
        let folders = workspace_folders_from_paths(&roots);
        let params = initialize_params(Some(folders[0].uri.clone()), folders.clone());

        assert_eq!(params.root_uri, Some(folders[0].uri.clone()));
        assert_eq!(params.workspace_folders, Some(folders));
        assert_eq!(
            params
                .capabilities
                .workspace
                .and_then(|workspace| workspace.workspace_folders),
            Some(true)
        );
    }

    #[test]
    fn workspace_folder_additions_only_include_new_uris() {
        let roots = vec![
            PathBuf::from("/workspace/app"),
            PathBuf::from("/workspace/tools"),
        ];
        let folders = workspace_folders_from_paths(&roots);

        assert_eq!(
            workspace_folder_additions(&folders[..1], &folders),
            vec![folders[1].clone()]
        );
        assert!(workspace_folder_additions(&folders, &folders).is_empty());
    }
}
