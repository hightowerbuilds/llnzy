use std::path::{Path, PathBuf};

/// A saved workspace definition (theme + project + tab layout).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SavedWorkspace {
    pub name: String,
    /// Name of the theme to apply (from user themes or built-in).
    pub theme: Option<String>,
    /// Project folder to open.
    pub project_path: Option<PathBuf>,
    /// Tab layout: list of tab descriptors to create on launch.
    pub tabs: Vec<TabEntry>,
}

/// A tab descriptor for workspace serialization.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TabEntry {
    /// The Home launch screen.
    Home,
    /// A terminal shell.
    Terminal,
    /// A code file to open.
    CodeFile { path: PathBuf },
    /// The Stacker singleton.
    Stacker,
    /// The Sketch singleton.
    Sketch,
    /// The local Git dashboard singleton.
    Git,
}

impl TabEntry {
    pub fn display_name(&self) -> String {
        match self {
            TabEntry::Home => "Home".to_string(),
            TabEntry::Terminal => "Terminal".to_string(),
            TabEntry::CodeFile { path } => path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("File")
                .to_string(),
            TabEntry::Stacker => "Stacker".to_string(),
            TabEntry::Sketch => "Sketch".to_string(),
            TabEntry::Git => "Git".to_string(),
        }
    }

    pub fn kind_label(&self) -> &'static str {
        match self {
            TabEntry::Home => "Home",
            TabEntry::Terminal => "Terminal",
            TabEntry::CodeFile { .. } => "Code File",
            TabEntry::Stacker => "Stacker",
            TabEntry::Sketch => "Sketch",
            TabEntry::Git => "Git",
        }
    }
}

/// Get the workspaces directory.
fn workspaces_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("llnzy").join("workspaces"))
}

/// Save a workspace definition.
pub fn save_workspace(workspace: &SavedWorkspace) -> Result<PathBuf, String> {
    let dir = workspaces_dir().ok_or("No config directory")?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create dir: {e}"))?;

    let safe_name: String = workspace
        .name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let path = dir.join(format!("{safe_name}.toml"));

    let toml_str =
        toml::to_string_pretty(workspace).map_err(|e| format!("Serialize failed: {e}"))?;
    std::fs::write(&path, toml_str).map_err(|e| format!("Write failed: {e}"))?;
    Ok(path)
}

/// Load all saved workspaces.
pub fn load_workspaces() -> Vec<SavedWorkspace> {
    let Some(dir) = workspaces_dir() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };

    let mut workspaces = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("toml") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(ws) = toml::from_str::<SavedWorkspace>(&text) else {
            continue;
        };
        workspaces.push(ws);
    }
    workspaces.sort_by(|a, b| a.name.cmp(&b.name));
    workspaces
}

/// Delete a saved workspace by name.
pub fn delete_workspace(name: &str) -> Result<(), String> {
    let dir = workspaces_dir().ok_or("No config directory")?;
    let safe_name: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let path = dir.join(format!("{safe_name}.toml"));
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("Delete failed: {e}"))?;
    }
    Ok(())
}

// ── Session auto-save ──

/// Path for the last session file.
fn last_session_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("llnzy").join("last_session.toml"))
}

/// A snapshot of the current session for auto-restore.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SessionSnapshot {
    pub theme: Option<String>,
    pub project_path: Option<PathBuf>,
    #[serde(default)]
    pub active_tab: Option<usize>,
    pub tabs: Vec<TabEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionRestorePlan {
    pub theme: Option<String>,
    pub project_path: Option<PathBuf>,
    pub missing_project_path: Option<PathBuf>,
    pub active_tab: Option<usize>,
    pub tabs: Vec<TabEntry>,
    pub skipped_files: Vec<PathBuf>,
}

impl SessionRestorePlan {
    pub fn needs_home_fallback(&self) -> bool {
        self.tabs.is_empty()
    }
}

pub fn plan_session_restore(snapshot: SessionSnapshot) -> SessionRestorePlan {
    plan_session_restore_with(snapshot, Path::is_dir, Path::is_file)
}

pub fn plan_session_restore_with(
    snapshot: SessionSnapshot,
    project_exists: impl Fn(&Path) -> bool,
    file_exists: impl Fn(&Path) -> bool,
) -> SessionRestorePlan {
    let (project_path, missing_project_path) = match snapshot.project_path {
        Some(path) if project_exists(&path) => (Some(path), None),
        Some(path) => (None, Some(path)),
        None => (None, None),
    };

    let mut active_tab = None;
    let mut tabs = Vec::new();
    let mut skipped_files = Vec::new();
    for (snapshot_index, entry) in snapshot.tabs.into_iter().enumerate() {
        if let TabEntry::CodeFile { path } = &entry {
            if !file_exists(path) {
                skipped_files.push(path.clone());
                continue;
            }
        }

        if snapshot.active_tab == Some(snapshot_index) {
            active_tab = Some(tabs.len());
        }
        tabs.push(entry);
    }

    SessionRestorePlan {
        theme: snapshot.theme,
        project_path,
        missing_project_path,
        active_tab,
        tabs,
        skipped_files,
    }
}

/// Save the current session state.
pub fn save_session(snapshot: &SessionSnapshot) -> Result<(), String> {
    let path = last_session_path().ok_or("No config directory")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir: {e}"))?;
    }
    let toml_str =
        toml::to_string_pretty(snapshot).map_err(|e| format!("Serialize failed: {e}"))?;
    std::fs::write(&path, toml_str).map_err(|e| format!("Write failed: {e}"))?;
    Ok(())
}

/// Load the last session snapshot, if any.
pub fn load_last_session() -> Option<SessionSnapshot> {
    let path = last_session_path()?;
    let text = std::fs::read_to_string(&path).ok()?;
    toml::from_str(&text).ok()
}

/// Clear the last session file.
pub fn clear_last_session() {
    if let Some(path) = last_session_path() {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_roundtrip() {
        let ws = SavedWorkspace {
            name: "Test Workspace".to_string(),
            theme: Some("Minimalist".to_string()),
            project_path: Some(PathBuf::from("/tmp/test-project")),
            tabs: vec![
                TabEntry::Terminal,
                TabEntry::CodeFile {
                    path: PathBuf::from("/tmp/test-project/main.rs"),
                },
                TabEntry::Sketch,
            ],
        };

        let toml_str = toml::to_string_pretty(&ws).unwrap();
        let loaded: SavedWorkspace = toml::from_str(&toml_str).unwrap();
        assert_eq!(loaded.name, "Test Workspace");
        assert_eq!(loaded.tabs.len(), 3);
    }

    #[test]
    fn session_snapshot_roundtrip() {
        let snap = SessionSnapshot {
            theme: Some("Buzz".to_string()),
            project_path: Some(PathBuf::from("/home/user/project")),
            active_tab: Some(1),
            tabs: vec![TabEntry::Terminal, TabEntry::Stacker],
        };

        let toml_str = toml::to_string_pretty(&snap).unwrap();
        let loaded: SessionSnapshot = toml::from_str(&toml_str).unwrap();
        assert_eq!(loaded.theme.as_deref(), Some("Buzz"));
        assert_eq!(loaded.active_tab, Some(1));
        assert_eq!(loaded.tabs.len(), 2);
    }

    #[test]
    fn session_snapshot_deserializes_without_active_tab() {
        let toml_str = r#"
theme = "Buzz"
project_path = "/home/user/project"

[[tabs]]
Terminal = {}

[[tabs]]
Stacker = {}
"#;

        let loaded: SessionSnapshot = toml::from_str(toml_str).unwrap();

        assert_eq!(loaded.active_tab, None);
        assert_eq!(loaded.tabs.len(), 2);
    }

    #[test]
    fn restore_plan_keeps_existing_project_and_tabs() {
        let project = PathBuf::from("/project");
        let file = project.join("src/main.rs");
        let snapshot = SessionSnapshot {
            theme: Some("Buzz".to_string()),
            project_path: Some(project.clone()),
            active_tab: Some(1),
            tabs: vec![
                TabEntry::Terminal,
                TabEntry::CodeFile { path: file.clone() },
                TabEntry::Stacker,
            ],
        };

        let plan = plan_session_restore_with(snapshot, |path| path == project, |path| path == file);

        assert_eq!(plan.theme.as_deref(), Some("Buzz"));
        assert_eq!(plan.project_path, Some(project));
        assert_eq!(plan.missing_project_path, None);
        assert_eq!(plan.active_tab, Some(1));
        assert_eq!(
            plan.tabs,
            vec![
                TabEntry::Terminal,
                TabEntry::CodeFile { path: file },
                TabEntry::Stacker,
            ]
        );
        assert!(plan.skipped_files.is_empty());
        assert!(!plan.needs_home_fallback());
    }

    #[test]
    fn restore_plan_skips_missing_project_and_files() {
        let missing_project = PathBuf::from("/missing-project");
        let missing_file = missing_project.join("missing.rs");
        let existing_file = PathBuf::from("/project/src/lib.rs");
        let snapshot = SessionSnapshot {
            theme: None,
            project_path: Some(missing_project.clone()),
            active_tab: Some(1),
            tabs: vec![
                TabEntry::Terminal,
                TabEntry::CodeFile {
                    path: missing_file.clone(),
                },
                TabEntry::CodeFile {
                    path: existing_file.clone(),
                },
            ],
        };

        let plan = plan_session_restore_with(snapshot, |_| false, |path| path == existing_file);

        assert_eq!(plan.project_path, None);
        assert_eq!(plan.missing_project_path, Some(missing_project));
        assert_eq!(plan.skipped_files, vec![missing_file]);
        assert_eq!(
            plan.tabs,
            vec![
                TabEntry::Terminal,
                TabEntry::CodeFile {
                    path: existing_file
                }
            ]
        );
        assert_eq!(plan.active_tab, None);
    }

    #[test]
    fn restore_plan_remaps_active_tab_after_skips() {
        let missing_file = PathBuf::from("/project/missing.rs");
        let existing_file = PathBuf::from("/project/existing.rs");
        let snapshot = SessionSnapshot {
            theme: None,
            project_path: None,
            active_tab: Some(2),
            tabs: vec![
                TabEntry::Terminal,
                TabEntry::CodeFile { path: missing_file },
                TabEntry::CodeFile {
                    path: existing_file.clone(),
                },
            ],
        };

        let plan = plan_session_restore_with(snapshot, |_| false, |path| path == existing_file);

        assert_eq!(
            plan.tabs,
            vec![
                TabEntry::Terminal,
                TabEntry::CodeFile {
                    path: existing_file
                }
            ]
        );
        assert_eq!(plan.active_tab, Some(1));
    }

    #[test]
    fn restore_plan_requests_home_fallback_when_no_tabs_are_usable() {
        let missing_file = PathBuf::from("/project/missing.rs");
        let snapshot = SessionSnapshot {
            theme: None,
            project_path: None,
            active_tab: Some(0),
            tabs: vec![TabEntry::CodeFile { path: missing_file }],
        };

        let plan = plan_session_restore_with(snapshot, |_| false, |_| false);

        assert!(plan.tabs.is_empty());
        assert!(plan.needs_home_fallback());
        assert_eq!(plan.active_tab, None);
    }

    #[test]
    fn tab_entry_display_names() {
        assert_eq!(TabEntry::Terminal.display_name(), "Terminal");
        assert_eq!(TabEntry::Stacker.display_name(), "Stacker");
        assert_eq!(TabEntry::Git.display_name(), "Git");
        assert_eq!(
            TabEntry::CodeFile {
                path: PathBuf::from("/foo/bar.rs")
            }
            .display_name(),
            "bar.rs"
        );
    }
}
