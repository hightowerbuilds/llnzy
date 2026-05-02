use std::path::{Path, PathBuf};

use crate::editor::BufferId;
use crate::path_utils::comparable_path;
use crate::session::Session;

/// What kind of content a workspace tab holds.
pub enum TabContent {
    /// The Home screen with project/workspace launch actions.
    Home,
    /// A terminal shell session.
    Terminal(Box<Session>),
    /// A source code file open in the editor.
    CodeFile { path: PathBuf, buffer_id: BufferId },
    /// The prompt queue manager (singleton).
    Stacker,
    /// The drawing canvas (singleton).
    Sketch,
    /// Local Git repository dashboard (singleton).
    Git,
    /// Theme/effects configuration (singleton).
    Appearances,
    /// App settings (singleton).
    Settings,
}

/// Discriminant-only version of TabContent for matching without borrowing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabKind {
    Home,
    Terminal,
    CodeFile,
    Stacker,
    Sketch,
    Git,
    Appearances,
    Settings,
}

impl TabContent {
    pub fn kind(&self) -> TabKind {
        match self {
            TabContent::Home => TabKind::Home,
            TabContent::Terminal(_) => TabKind::Terminal,
            TabContent::CodeFile { .. } => TabKind::CodeFile,
            TabContent::Stacker => TabKind::Stacker,
            TabContent::Sketch => TabKind::Sketch,
            TabContent::Git => TabKind::Git,
            TabContent::Appearances => TabKind::Appearances,
            TabContent::Settings => TabKind::Settings,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, TabContent::Terminal(_))
    }

    pub fn is_singleton(&self) -> bool {
        matches!(
            self,
            TabContent::Home
                | TabContent::Stacker
                | TabContent::Sketch
                | TabContent::Git
                | TabContent::Appearances
                | TabContent::Settings
        )
    }

    pub fn as_terminal(&self) -> Option<&Session> {
        match self {
            TabContent::Terminal(s) => Some(s),
            _ => None,
        }
    }
}

/// A tab in the workspace tab bar.
pub struct WorkspaceTab {
    pub content: TabContent,
    pub name: Option<String>,
    pub id: u64,
}

impl WorkspaceTab {
    /// Display name for the tab bar.
    pub fn display_name(&self, index: usize) -> String {
        if let Some(name) = &self.name {
            return name.clone();
        }
        match &self.content {
            TabContent::Home => "Home".to_string(),
            TabContent::Terminal(session) => {
                let sname = session.display_name();
                if sname != "shell" {
                    sname.to_string()
                } else {
                    format!("Shell {}", index + 1)
                }
            }
            TabContent::CodeFile { path, .. } => path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("untitled")
                .to_string(),
            TabContent::Stacker => "Stacker".to_string(),
            TabContent::Sketch => "Sketch".to_string(),
            TabContent::Git => "Git".to_string(),
            TabContent::Appearances => "Appearances".to_string(),
            TabContent::Settings => "Settings".to_string(),
        }
    }
}

/// Find the index of an existing singleton tab of the given kind.
pub fn find_singleton(tabs: &[WorkspaceTab], kind: TabKind) -> Option<usize> {
    tabs.iter().position(|t| t.content.kind() == kind)
}

pub fn remap_code_file_tab_paths(
    tabs: &mut [WorkspaceTab],
    old_path: &Path,
    new_path: &Path,
    remapped_buffer_ids: &[BufferId],
) {
    let old_key = comparable_path(old_path);
    for tab in tabs {
        if let TabContent::CodeFile { path, buffer_id } = &mut tab.content {
            if comparable_path(path) == old_key || remapped_buffer_ids.contains(buffer_id) {
                *path = new_path.to_path_buf();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::EditorState;

    fn temp_file(name: &str, text: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("llnzy-workspace-{name}-{}", std::process::id()));
        std::fs::write(&path, text).unwrap();
        path
    }

    #[test]
    fn remap_code_file_tab_paths_updates_matching_path_or_buffer_id() {
        let old_path = temp_file("old.rs", "old");
        let other_path = temp_file("other.rs", "other");
        let new_path =
            std::env::temp_dir().join(format!("llnzy-workspace-new.rs-{}", std::process::id()));
        let mut editor = EditorState::new();
        let old_id = editor.open(old_path.clone()).unwrap();
        let other_id = editor.open(other_path.clone()).unwrap();
        let stale_path = PathBuf::from("/tmp/stale-but-same-buffer.rs");

        let mut tabs = vec![
            WorkspaceTab {
                content: TabContent::CodeFile {
                    path: old_path.clone(),
                    buffer_id: old_id,
                },
                name: None,
                id: 1,
            },
            WorkspaceTab {
                content: TabContent::CodeFile {
                    path: stale_path,
                    buffer_id: old_id,
                },
                name: None,
                id: 2,
            },
            WorkspaceTab {
                content: TabContent::CodeFile {
                    path: other_path.clone(),
                    buffer_id: other_id,
                },
                name: None,
                id: 3,
            },
        ];

        remap_code_file_tab_paths(&mut tabs, &old_path, &new_path, &[old_id]);

        assert!(matches!(
            &tabs[0].content,
            TabContent::CodeFile { path, .. } if path == &new_path
        ));
        assert!(matches!(
            &tabs[1].content,
            TabContent::CodeFile { path, .. } if path == &new_path
        ));
        assert!(matches!(
            &tabs[2].content,
            TabContent::CodeFile { path, .. } if path == &other_path
        ));

        let _ = std::fs::remove_file(old_path);
        let _ = std::fs::remove_file(other_path);
    }
}
