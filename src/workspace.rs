use std::path::PathBuf;

use crate::editor::BufferId;
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
