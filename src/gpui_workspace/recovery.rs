use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{atomic_write::atomic_write, tab_groups::PartitionAxis};

pub(super) const WORKSPACE_RECOVERY_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(super) enum WorkspaceRecoverySurface {
    Home,
    Stacker,
    Editor,
    Terminal,
    Explorer,
    Sketch,
    Appearances,
    Settings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum WorkspaceRecoveryAxis {
    Vertical,
    Horizontal,
}

impl From<PartitionAxis> for WorkspaceRecoveryAxis {
    fn from(axis: PartitionAxis) -> Self {
        match axis {
            PartitionAxis::Vertical => Self::Vertical,
            PartitionAxis::Horizontal => Self::Horizontal,
        }
    }
}

impl From<WorkspaceRecoveryAxis> for PartitionAxis {
    fn from(axis: WorkspaceRecoveryAxis) -> Self {
        match axis {
            WorkspaceRecoveryAxis::Vertical => Self::Vertical,
            WorkspaceRecoveryAxis::Horizontal => Self::Horizontal,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct WorkspaceRecoveryTab {
    pub(super) id: u64,
    pub(super) surface: WorkspaceRecoverySurface,
    #[serde(default)]
    pub(super) file_path: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct WorkspaceRecoveryTabNameOverride {
    pub(super) id: u64,
    pub(super) name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct WorkspaceRecoveryJoinedGroup {
    pub(super) members: Vec<u64>,
    #[serde(default)]
    pub(super) shares: Vec<f32>,
    pub(super) axis: WorkspaceRecoveryAxis,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct WorkspaceRecoverySnapshot {
    pub(super) version: u32,
    pub(super) clean_shutdown: bool,
    #[serde(default)]
    pub(super) workspace_root: Option<PathBuf>,
    pub(super) active_tab_id: u64,
    pub(super) next_tab_id: u64,
    #[serde(default)]
    pub(super) tabs: Vec<WorkspaceRecoveryTab>,
    #[serde(default)]
    pub(super) tab_name_overrides: Vec<WorkspaceRecoveryTabNameOverride>,
    #[serde(default)]
    pub(super) joined_groups: Vec<WorkspaceRecoveryJoinedGroup>,
    pub(super) sidebar_visible: bool,
    pub(super) sidebar_width: f32,
    pub(super) last_sidebar_width: f32,
    #[serde(default)]
    pub(super) sidebar_selected_path: Option<PathBuf>,
    #[serde(default)]
    pub(super) sidebar_expanded_dirs: Vec<PathBuf>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct WorkspaceRecoveryPlan {
    pub(super) workspace_root: Option<PathBuf>,
    pub(super) active_tab_id: u64,
    pub(super) next_tab_id: u64,
    pub(super) tabs: Vec<WorkspaceRecoveryTab>,
    pub(super) tab_name_overrides: BTreeMap<u64, String>,
    pub(super) joined_groups: Vec<WorkspaceRecoveryJoinedGroup>,
    pub(super) sidebar_visible: bool,
    pub(super) sidebar_width: f32,
    pub(super) last_sidebar_width: f32,
    pub(super) sidebar_selected_path: Option<PathBuf>,
    pub(super) sidebar_expanded_dirs: BTreeSet<PathBuf>,
    pub(super) skipped_missing_project: Option<PathBuf>,
    pub(super) skipped_missing_files: Vec<PathBuf>,
}

impl WorkspaceRecoveryPlan {
    pub(super) fn status_message(&self) -> Option<String> {
        let mut parts = Vec::new();
        if let Some(path) = &self.skipped_missing_project {
            parts.push(format!("Skipped missing project {}", path.display()));
        }
        if !self.skipped_missing_files.is_empty() {
            let count = self.skipped_missing_files.len();
            parts.push(if count == 1 {
                format!(
                    "Skipped missing file {}",
                    self.skipped_missing_files[0].display()
                )
            } else {
                format!("Skipped {count} missing files")
            });
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("; "))
        }
    }
}

pub(super) fn recovery_file() -> Option<PathBuf> {
    crate::platform::paths::current_paths().map(|paths| paths.last_session_file())
}

pub(super) fn load_snapshot(path: &Path) -> Result<Option<WorkspaceRecoverySnapshot>, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(format!(
                "Failed to read workspace recovery snapshot {}: {err}",
                path.display()
            ));
        }
    };
    let snapshot = toml::from_str::<WorkspaceRecoverySnapshot>(&text).map_err(|err| {
        format!(
            "Failed to parse workspace recovery snapshot {}: {err}",
            path.display()
        )
    })?;
    Ok(Some(snapshot))
}

pub(super) fn save_snapshot(
    path: &Path,
    snapshot: &WorkspaceRecoverySnapshot,
) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Invalid workspace recovery snapshot path".to_string())?;
    fs::create_dir_all(parent).map_err(|err| {
        format!(
            "Failed to create workspace recovery snapshot dir {}: {err}",
            parent.display()
        )
    })?;
    let text = toml::to_string_pretty(snapshot)
        .map_err(|err| format!("Failed to serialize workspace recovery snapshot: {err}"))?;
    atomic_write(path, text.as_bytes()).map_err(|err| err.to_string())
}

pub(super) fn remove_snapshot(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!(
            "Failed to remove workspace recovery snapshot {}: {err}",
            path.display()
        )),
    }
}

pub(super) fn plan_restore(snapshot: WorkspaceRecoverySnapshot) -> Option<WorkspaceRecoveryPlan> {
    if snapshot.clean_shutdown || snapshot.version != WORKSPACE_RECOVERY_VERSION {
        return None;
    }

    let mut skipped_missing_project = None;
    let workspace_root = match snapshot.workspace_root {
        Some(path) if path.is_dir() => Some(path),
        Some(path) => {
            skipped_missing_project = Some(path);
            None
        }
        None => None,
    };

    let mut seen_ids = HashSet::new();
    let mut seen_singletons = HashSet::new();
    let mut skipped_missing_files = Vec::new();
    let mut tabs = Vec::new();

    for tab in snapshot.tabs {
        if tab.id == 0 || !seen_ids.insert(tab.id) {
            continue;
        }

        if tab.surface == WorkspaceRecoverySurface::Editor {
            if let Some(path) = tab.file_path.as_ref() {
                if !path.is_file() {
                    skipped_missing_files.push(path.clone());
                    continue;
                }
            }
        }

        if !is_multi_instance_surface(tab.surface)
            && tab.file_path.is_none()
            && !seen_singletons.insert(tab.surface)
        {
            continue;
        }

        tabs.push(tab);
    }

    if tabs.is_empty() {
        tabs.push(WorkspaceRecoveryTab {
            id: 1,
            surface: WorkspaceRecoverySurface::Home,
            file_path: None,
        });
    }

    let valid_ids = tabs.iter().map(|tab| tab.id).collect::<HashSet<_>>();
    let active_tab_id = if valid_ids.contains(&snapshot.active_tab_id) {
        snapshot.active_tab_id
    } else {
        tabs.iter()
            .find(|tab| tab.surface == WorkspaceRecoverySurface::Home)
            .or_else(|| tabs.first())
            .map(|tab| tab.id)
            .unwrap_or(1)
    };

    let max_tab_id = tabs.iter().map(|tab| tab.id).max().unwrap_or(1);
    let next_tab_id = snapshot
        .next_tab_id
        .max(max_tab_id.saturating_add(1))
        .max(2);
    let tab_name_overrides = snapshot
        .tab_name_overrides
        .into_iter()
        .filter(|entry| valid_ids.contains(&entry.id) && !entry.name.trim().is_empty())
        .map(|entry| (entry.id, entry.name))
        .collect();
    let joined_groups = snapshot
        .joined_groups
        .into_iter()
        .filter_map(|group| {
            let members = group
                .members
                .into_iter()
                .filter(|member| valid_ids.contains(member))
                .take(4)
                .collect::<Vec<_>>();
            (members.len() >= 2).then_some(WorkspaceRecoveryJoinedGroup { members, ..group })
        })
        .collect();

    let sidebar_selected_path = snapshot
        .sidebar_selected_path
        .filter(|path| path.exists() && workspace_root.is_some());
    let sidebar_expanded_dirs = snapshot
        .sidebar_expanded_dirs
        .into_iter()
        .filter(|path| path.is_dir() && workspace_root.is_some())
        .collect();

    Some(WorkspaceRecoveryPlan {
        workspace_root,
        active_tab_id,
        next_tab_id,
        tabs,
        tab_name_overrides,
        joined_groups,
        sidebar_visible: snapshot.sidebar_visible,
        sidebar_width: sanitize_sidebar_width(snapshot.sidebar_width),
        last_sidebar_width: sanitize_sidebar_width(snapshot.last_sidebar_width),
        sidebar_selected_path,
        sidebar_expanded_dirs,
        skipped_missing_project,
        skipped_missing_files,
    })
}

fn is_multi_instance_surface(surface: WorkspaceRecoverySurface) -> bool {
    matches!(surface, WorkspaceRecoverySurface::Terminal)
}

fn sanitize_sidebar_width(width: f32) -> f32 {
    if width.is_finite() {
        width
    } else {
        220.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "llnzy-workspace-recovery-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn snapshot(root: Option<PathBuf>, clean_shutdown: bool) -> WorkspaceRecoverySnapshot {
        WorkspaceRecoverySnapshot {
            version: WORKSPACE_RECOVERY_VERSION,
            clean_shutdown,
            workspace_root: root,
            active_tab_id: 3,
            next_tab_id: 5,
            tabs: vec![
                WorkspaceRecoveryTab {
                    id: 1,
                    surface: WorkspaceRecoverySurface::Home,
                    file_path: None,
                },
                WorkspaceRecoveryTab {
                    id: 2,
                    surface: WorkspaceRecoverySurface::Terminal,
                    file_path: None,
                },
            ],
            tab_name_overrides: Vec::new(),
            joined_groups: Vec::new(),
            sidebar_visible: true,
            sidebar_width: 240.0,
            last_sidebar_width: 240.0,
            sidebar_selected_path: None,
            sidebar_expanded_dirs: Vec::new(),
        }
    }

    #[test]
    fn clean_shutdown_snapshot_does_not_restore() {
        assert!(plan_restore(snapshot(None, true)).is_none());
    }

    #[test]
    fn restore_plan_skips_missing_project_and_files() {
        let dir = temp_dir("missing");
        let existing = dir.join("existing.rs");
        let missing = dir.join("missing.rs");
        fs::write(&existing, "fn main() {}\n").unwrap();

        let mut snapshot = snapshot(Some(dir.join("gone-project")), false);
        snapshot.active_tab_id = 3;
        snapshot.tabs.push(WorkspaceRecoveryTab {
            id: 3,
            surface: WorkspaceRecoverySurface::Editor,
            file_path: Some(existing.clone()),
        });
        snapshot.tabs.push(WorkspaceRecoveryTab {
            id: 4,
            surface: WorkspaceRecoverySurface::Editor,
            file_path: Some(missing.clone()),
        });
        snapshot.sidebar_selected_path = Some(existing.clone());
        snapshot.sidebar_expanded_dirs = vec![dir.clone()];

        let plan = plan_restore(snapshot).unwrap();
        assert_eq!(plan.workspace_root, None);
        assert_eq!(plan.active_tab_id, 3);
        assert_eq!(plan.skipped_missing_files, vec![missing]);
        assert!(plan.skipped_missing_project.is_some());
        assert_eq!(plan.sidebar_selected_path, None);
        assert!(plan.sidebar_expanded_dirs.is_empty());
        assert_eq!(plan.tabs.len(), 3);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn restore_plan_falls_back_to_home_when_active_tab_was_skipped() {
        let dir = temp_dir("active");
        let missing = dir.join("missing.rs");

        let mut snapshot = snapshot(None, false);
        snapshot.active_tab_id = 3;
        snapshot.tabs.push(WorkspaceRecoveryTab {
            id: 3,
            surface: WorkspaceRecoverySurface::Editor,
            file_path: Some(missing),
        });

        let plan = plan_restore(snapshot).unwrap();
        assert_eq!(plan.active_tab_id, 1);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn restore_plan_filters_joined_groups_to_valid_tabs() {
        let mut snapshot = snapshot(None, false);
        snapshot.joined_groups.push(WorkspaceRecoveryJoinedGroup {
            members: vec![1, 2, 99],
            shares: vec![0.3, 0.4, 0.3],
            axis: WorkspaceRecoveryAxis::Horizontal,
        });
        snapshot.joined_groups.push(WorkspaceRecoveryJoinedGroup {
            members: vec![1, 99],
            shares: vec![0.5, 0.5],
            axis: WorkspaceRecoveryAxis::Vertical,
        });

        let plan = plan_restore(snapshot).unwrap();
        assert_eq!(plan.joined_groups.len(), 1);
        assert_eq!(plan.joined_groups[0].members, vec![1, 2]);
    }

    #[test]
    fn restore_plan_keeps_only_one_explorer_tab() {
        let mut snapshot = snapshot(None, false);
        snapshot.tabs.push(WorkspaceRecoveryTab {
            id: 3,
            surface: WorkspaceRecoverySurface::Explorer,
            file_path: None,
        });
        snapshot.tabs.push(WorkspaceRecoveryTab {
            id: 4,
            surface: WorkspaceRecoverySurface::Explorer,
            file_path: None,
        });

        let plan = plan_restore(snapshot).unwrap();
        let explorer_count = plan
            .tabs
            .iter()
            .filter(|tab| tab.surface == WorkspaceRecoverySurface::Explorer)
            .count();
        assert_eq!(explorer_count, 1);
    }

    #[test]
    fn save_load_round_trips_tab_name_overrides() {
        let dir = temp_dir("overrides");
        let path = dir.join("last_session.toml");
        let mut snapshot = snapshot(Some(dir.clone()), false);
        snapshot.tab_name_overrides = vec![
            WorkspaceRecoveryTabNameOverride {
                id: 1,
                name: "Notes".to_string(),
            },
            WorkspaceRecoveryTabNameOverride {
                id: 2,
                name: "Build".to_string(),
            },
        ];

        save_snapshot(&path, &snapshot).unwrap();
        let loaded = load_snapshot(&path).unwrap().unwrap();
        assert_eq!(loaded.tab_name_overrides, snapshot.tab_name_overrides);

        let plan = plan_restore(loaded).unwrap();
        assert_eq!(
            plan.tab_name_overrides.get(&1).map(String::as_str),
            Some("Notes")
        );
        assert_eq!(
            plan.tab_name_overrides.get(&2).map(String::as_str),
            Some("Build")
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn save_load_and_mark_clean_round_trip() {
        let dir = temp_dir("roundtrip");
        let path = dir.join("last_session.toml");
        let snapshot = snapshot(Some(dir.clone()), false);

        save_snapshot(&path, &snapshot).unwrap();
        assert_eq!(load_snapshot(&path).unwrap(), Some(snapshot.clone()));

        let mut clean = snapshot.clone();
        clean.clean_shutdown = true;
        save_snapshot(&path, &clean).unwrap();
        let loaded = load_snapshot(&path).unwrap().unwrap();
        assert!(loaded.clean_shutdown);

        remove_snapshot(&path).unwrap();
        assert!(load_snapshot(&path).unwrap().is_none());

        let _ = fs::remove_dir_all(dir);
    }
}
