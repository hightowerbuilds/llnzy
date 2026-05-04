use std::path::{Path, PathBuf};

use super::model::{MoveOrigin, SidebarMoveRequest};
use super::plan::plan_sidebar_move;

const MAX_DESTINATIONS: usize = 1_500;
const IGNORED_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "__pycache__",
    ".venv",
    "venv",
    ".mypy_cache",
    ".pytest_cache",
    ".tox",
    "dist",
    "build",
    ".next",
    ".nuxt",
    ".svelte-kit",
    ".turbo",
    ".cache",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidebarMoveDestination {
    pub path: PathBuf,
    pub depth: usize,
    pub label: String,
    pub is_valid: bool,
    pub reason: Option<String>,
}

pub fn collect_sidebar_move_destinations(
    root: &Path,
    sources: &[PathBuf],
) -> Vec<SidebarMoveDestination> {
    let mut destinations = Vec::new();
    collect_destination(root, 0, sources, &mut destinations);
    destinations
}

fn collect_destination(
    path: &Path,
    depth: usize,
    sources: &[PathBuf],
    destinations: &mut Vec<SidebarMoveDestination>,
) {
    if destinations.len() >= MAX_DESTINATIONS || !path.is_dir() {
        return;
    }

    let label = if depth == 0 {
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Project Root")
            .to_string()
    } else {
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Folder")
            .to_string()
    };
    let request = SidebarMoveRequest::new(
        sources.to_vec(),
        path.to_path_buf(),
        MoveOrigin::ContextMenu,
    );
    let validation = plan_sidebar_move(&request);
    destinations.push(SidebarMoveDestination {
        path: path.to_path_buf(),
        depth,
        label,
        is_valid: validation.is_ok(),
        reason: validation.err(),
    });

    let Ok(read_dir) = std::fs::read_dir(path) else {
        return;
    };
    let mut child_dirs = read_dir
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.metadata().ok().is_some_and(|meta| meta.is_dir());
            (is_dir && should_show_dir(&name)).then_some((name, path))
        })
        .collect::<Vec<_>>();
    child_dirs.sort_by(|left, right| left.0.to_lowercase().cmp(&right.0.to_lowercase()));

    for (_, child) in child_dirs {
        collect_destination(&child, depth + 1, sources, destinations);
        if destinations.len() >= MAX_DESTINATIONS {
            break;
        }
    }
}

fn should_show_dir(name: &str) -> bool {
    !name.starts_with('.') && !IGNORED_DIRS.contains(&name)
}
