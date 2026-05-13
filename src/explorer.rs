use std::fs;
use std::path::{Path, PathBuf};

const MAX_RECENT: usize = 5;

fn recent_projects_path() -> Option<PathBuf> {
    crate::platform::paths::current_paths().map(|paths| paths.recent_projects_file())
}

/// Load recent project paths from disk.
pub fn load_recent_projects() -> Vec<PathBuf> {
    let Some(path) = recent_projects_path() else {
        return Vec::new();
    };
    let Ok(data) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    let Ok(paths) = serde_json::from_str::<Vec<String>>(&data) else {
        return Vec::new();
    };
    paths
        .into_iter()
        .map(PathBuf::from)
        .filter(|path| path.exists())
        .collect()
}

/// Save recent project paths to disk.
pub fn save_recent_projects(projects: &[PathBuf]) {
    let Some(path) = recent_projects_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let strings: Vec<String> = projects
        .iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect();
    if let Ok(json) = serde_json::to_string_pretty(&strings) {
        let _ = fs::write(path, json);
    }
}

/// Add a project to the recent list, moving existing entries to the front.
pub fn add_recent_project(projects: &mut Vec<PathBuf>, path: PathBuf) {
    insert_recent_project(projects, path);
    save_recent_projects(projects);
}

fn insert_recent_project(projects: &mut Vec<PathBuf>, path: PathBuf) {
    projects.retain(|project| project != &path);
    projects.insert(0, path);
    projects.truncate(MAX_RECENT);
}

/// Get the display name for a project path.
pub fn project_name(path: &Path) -> &str {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_recent_project_moves_existing_project_to_front() {
        let mut projects = vec![PathBuf::from("/a"), PathBuf::from("/b")];

        insert_recent_project(&mut projects, PathBuf::from("/b"));

        assert_eq!(projects, vec![PathBuf::from("/b"), PathBuf::from("/a")]);
    }

    #[test]
    fn recent_projects_are_capped() {
        let mut projects = Vec::new();

        for index in 0..8 {
            insert_recent_project(&mut projects, PathBuf::from(format!("/{index}")));
        }

        assert_eq!(projects.len(), MAX_RECENT);
        assert_eq!(projects.first(), Some(&PathBuf::from("/7")));
    }
}
