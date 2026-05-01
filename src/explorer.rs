use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use crate::editor::buffer::Buffer;

const MAX_IMAGE_SIZE: u64 = 20_971_520; // 20 MB
/// Maximum number of files to index for the fuzzy finder.
const MAX_INDEX_FILES: usize = 10_000;

/// Directories and patterns to always ignore.
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

pub struct DirEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
}

pub enum FileContent {
    Text(Buffer),
    Image {
        rgba: Vec<u8>,
        width: u32,
        height: u32,
        texture: Option<egui::TextureHandle>,
    },
}

pub struct OpenFile {
    pub path: PathBuf,
    pub name: String,
    pub content: FileContent,
}

/// A node in the file tree.
pub struct TreeNode {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub children: Option<Vec<TreeNode>>,
    pub expanded: bool,
}

impl TreeNode {
    fn file(name: String, path: PathBuf, size: u64) -> Self {
        Self {
            name,
            path,
            is_dir: false,
            size,
            children: None,
            expanded: false,
        }
    }

    fn dir(name: String, path: PathBuf) -> Self {
        Self {
            name,
            path,
            is_dir: true,
            size: 0,
            children: None,
            expanded: false,
        }
    }

    /// Load children if not already loaded.
    pub fn ensure_children(&mut self) {
        if !self.is_dir || self.children.is_some() {
            return;
        }
        self.children = Some(read_dir_sorted(&self.path));
    }

    /// Toggle expand/collapse. Loads children on first expand.
    pub fn toggle(&mut self) {
        if !self.is_dir {
            return;
        }
        self.expanded = !self.expanded;
        if self.expanded {
            self.ensure_children();
        }
    }

    // collect_files removed — use walk_files_capped() instead for safety
}

/// Read a directory and return sorted TreeNodes (dirs first, then files).
fn read_dir_sorted(path: &Path) -> Vec<TreeNode> {
    let Ok(read_dir) = fs::read_dir(path) else {
        return Vec::new();
    };

    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for entry in read_dir.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();

        // Skip hidden files and ignored directories
        if name.starts_with('.') && name != ".env" && name != ".gitignore" {
            continue;
        }

        let meta = entry.metadata().ok();
        let is_dir = meta.as_ref().is_some_and(|m| m.is_dir());

        if is_dir {
            if IGNORED_DIRS.contains(&name.as_str()) {
                continue;
            }
            dirs.push(TreeNode::dir(name, path));
        } else {
            let size = meta.as_ref().map_or(0, |m| m.len());
            files.push(TreeNode::file(name, path, size));
        }
    }

    dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let mut result = Vec::with_capacity(dirs.len() + files.len());
    result.append(&mut dirs);
    result.append(&mut files);
    result
}

fn collect_expanded_paths(nodes: &[TreeNode], expanded: &mut HashSet<PathBuf>) {
    for node in nodes {
        if !node.is_dir {
            continue;
        }
        if node.expanded {
            expanded.insert(comparable_path(&node.path));
        }
        if let Some(children) = &node.children {
            collect_expanded_paths(children, expanded);
        }
    }
}

fn apply_expanded_paths(nodes: &mut [TreeNode], expanded: &HashSet<PathBuf>) {
    for node in nodes {
        if !node.is_dir || !expanded.contains(&comparable_path(&node.path)) {
            continue;
        }
        node.expanded = true;
        node.ensure_children();
        if let Some(children) = &mut node.children {
            apply_expanded_paths(children, expanded);
        }
    }
}

fn comparable_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

pub struct ExplorerState {
    /// Project root directory.
    pub root: PathBuf,
    /// Tree of files/dirs.
    pub tree: Vec<TreeNode>,
    /// Currently open file (for image preview only; text goes through EditorState).
    pub open_file: Option<OpenFile>,
    pub error: Option<String>,
    /// Fuzzy finder state.
    pub finder_open: bool,
    pub finder_query: String,
    pub finder_results: Vec<PathBuf>,
    pub finder_selected: usize,
    /// Cached project file index for fuzzy finding.
    file_index: Option<Vec<PathBuf>>,
    file_index_rx: Option<Receiver<(PathBuf, Vec<PathBuf>)>>,
    indexing_root: Option<PathBuf>,
}

impl Default for ExplorerState {
    fn default() -> Self {
        Self::new()
    }
}

impl ExplorerState {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let tree = read_dir_sorted(&home);
        ExplorerState {
            root: home,
            tree,
            open_file: None,
            error: None,
            finder_open: false,
            finder_query: String::new(),
            finder_results: Vec::new(),
            finder_selected: 0,
            file_index: None,
            file_index_rx: None,
            indexing_root: None,
        }
    }

    /// Clear the project — reset to empty state.
    pub fn clear(&mut self) {
        self.root = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        self.tree.clear();
        self.open_file = None;
        self.error = None;
        self.file_index = None;
        self.file_index_rx = None;
        self.indexing_root = None;
        self.finder_open = false;
        self.finder_query.clear();
        self.finder_results.clear();
    }

    /// Set the project root and rebuild the tree.
    pub fn set_root(&mut self, path: PathBuf) {
        self.root = path;
        self.tree = read_dir_sorted(&self.root);
        self.clear_file_index();
    }

    /// Rebuild the current tree while keeping expanded folders open.
    pub fn refresh_preserving_expansion(&mut self, additionally_expand: &[PathBuf]) {
        let mut expanded = HashSet::new();
        collect_expanded_paths(&self.tree, &mut expanded);
        expanded.extend(additionally_expand.iter().map(|path| comparable_path(path)));

        self.tree = read_dir_sorted(&self.root);
        apply_expanded_paths(&mut self.tree, &expanded);
        self.clear_file_index();
    }

    fn clear_file_index(&mut self) {
        self.file_index = None;
        self.file_index_rx = None;
        self.indexing_root = None;
    }

    /// Open a file (image only -- text files go through EditorState).
    pub fn open(&mut self, path: PathBuf) {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".to_string());

        if is_image(&path) {
            self.open_image(path, name);
        }
    }

    fn open_image(&mut self, path: PathBuf, name: String) {
        match fs::metadata(&path) {
            Ok(meta) if meta.len() > MAX_IMAGE_SIZE => {
                self.error = Some(format!(
                    "Image too large ({:.0} MB limit)",
                    MAX_IMAGE_SIZE as f64 / 1_048_576.0
                ));
                return;
            }
            Err(e) => {
                self.error = Some(format!("Cannot read file: {e}"));
                return;
            }
            _ => {}
        }

        match image::open(&path) {
            Ok(img) => {
                let rgba_image = img.to_rgba8();
                let width = rgba_image.width();
                let height = rgba_image.height();
                let rgba = rgba_image.into_raw();
                self.error = None;
                self.open_file = Some(OpenFile {
                    path,
                    name,
                    content: FileContent::Image {
                        rgba,
                        width,
                        height,
                        texture: None,
                    },
                });
            }
            Err(e) => self.error = Some(format!("Cannot decode image: {e}")),
        }
    }

    pub fn close_file(&mut self) {
        self.open_file = None;
        self.error = None;
    }

    /// Start building the file index for fuzzy finding on a background thread.
    pub fn ensure_file_index(&mut self) {
        if self.file_index.is_some() {
            return;
        }
        if self.file_index_rx.is_some() {
            self.poll_file_index();
            return;
        }

        let root = self.root.clone();
        let (tx, rx) = mpsc::channel();
        self.indexing_root = Some(root.clone());
        self.file_index_rx = Some(rx);
        let _ = thread::Builder::new()
            .name("llnzy-file-index".to_string())
            .spawn(move || {
                let files = walk_files_capped(&root, MAX_INDEX_FILES);
                let _ = tx.send((root, files));
            });
    }

    /// Poll the background file-indexing job without blocking the UI.
    pub fn poll_file_index(&mut self) {
        let Some(rx) = self.file_index_rx.take() else {
            return;
        };
        match rx.try_recv() {
            Ok((root, files)) => {
                if root == self.root {
                    self.file_index = Some(files);
                    self.refresh_finder_results();
                }
                self.indexing_root = None;
            }
            Err(TryRecvError::Empty) => {
                self.file_index_rx = Some(rx);
            }
            Err(TryRecvError::Disconnected) => {
                self.indexing_root = None;
            }
        }
    }

    pub fn is_indexing(&self) -> bool {
        self.file_index.is_none() && self.file_index_rx.is_some()
    }

    /// Update fuzzy finder results based on the query.
    pub fn update_finder(&mut self) {
        self.ensure_file_index();
        self.poll_file_index();
        self.refresh_finder_results();
    }

    fn refresh_finder_results(&mut self) {
        let Some(index) = &self.file_index else {
            return;
        };
        let query = self.finder_query.to_lowercase();

        if query.is_empty() {
            self.finder_results = index.iter().take(50).cloned().collect();
        } else {
            self.finder_results = index
                .iter()
                .filter(|p| {
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    let path_str = p.to_string_lossy().to_lowercase();
                    fuzzy_match(&query, &path_str) || fuzzy_match(&query, &name.to_lowercase())
                })
                .take(30)
                .cloned()
                .collect();
        }
        self.finder_selected = 0;
    }

    /// Open the fuzzy finder.
    pub fn open_finder(&mut self) {
        self.finder_open = true;
        self.finder_query.clear();
        self.update_finder();
    }

    /// Close the fuzzy finder.
    pub fn close_finder(&mut self) {
        self.finder_open = false;
        self.finder_query.clear();
        self.finder_results.clear();
    }

    /// Get the relative path from the project root.
    pub fn relative_path(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string()
    }
}

/// Walk a directory tree iteratively, collecting file paths up to a cap.
/// Skips ignored directories and hidden files. Uses a stack (no recursion).
fn walk_files_capped(root: &Path, max_files: usize) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut dirs_to_visit = vec![root.to_path_buf()];

    while let Some(dir) = dirs_to_visit.pop() {
        if files.len() >= max_files {
            break;
        }
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };

        for entry in entries.flatten() {
            if files.len() >= max_files {
                break;
            }
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();

            // Skip hidden files (except .env, .gitignore)
            if name.starts_with('.') && name != ".env" && name != ".gitignore" {
                continue;
            }

            let meta = entry.metadata().ok();
            let is_dir = meta.as_ref().is_some_and(|m| m.is_dir());

            if is_dir {
                if !IGNORED_DIRS.contains(&name.as_str()) {
                    dirs_to_visit.push(path);
                }
            } else {
                files.push(path);
            }
        }
    }

    files.sort_by(|a, b| {
        let a_name = a.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let b_name = b.file_name().and_then(|n| n.to_str()).unwrap_or("");
        a_name.to_lowercase().cmp(&b_name.to_lowercase())
    });
    files
}

/// Simple fuzzy match: all query chars appear in order in the target.
fn fuzzy_match(query: &str, target: &str) -> bool {
    let mut target_chars = target.chars();
    for qc in query.chars() {
        loop {
            match target_chars.next() {
                Some(tc) if tc == qc => break,
                Some(_) => continue,
                None => return false,
            }
        }
    }
    true
}

fn is_image(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "tiff" | "tif" | "ico"
    )
}

pub fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1_048_576 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    }
}

// ── Recent projects persistence ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_index_builds_on_background_thread() {
        let root =
            std::env::temp_dir().join(format!("llnzy_file_index_{}_{}", std::process::id(), 1));
        let nested = root.join("src");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(root.join("README.md"), "# test").unwrap();
        std::fs::write(nested.join("main.rs"), "fn main() {}").unwrap();

        let mut explorer = ExplorerState::new();
        explorer.set_root(root.clone());
        explorer.open_finder();

        for _ in 0..100 {
            explorer.poll_file_index();
            if explorer.file_index.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        assert!(explorer.file_index.is_some());
        assert!(explorer
            .finder_results
            .iter()
            .any(|path| path.file_name().is_some_and(|name| name == "main.rs")));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn refresh_preserving_expansion_updates_open_source_and_destination_folders() {
        let root = std::env::temp_dir().join(format!(
            "llnzy_explorer_refresh_{}_{}",
            std::process::id(),
            1
        ));
        let source = root.join("source");
        let destination = root.join("destination");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&destination).unwrap();
        let moved_file = source.join("note.md");
        std::fs::write(&moved_file, "hello").unwrap();
        std::fs::write(source.join("zeta.md"), "keep").unwrap();

        let mut explorer = ExplorerState::new();
        explorer.set_root(root.clone());
        expand_top_level_dir(&mut explorer.tree, "source");
        expand_top_level_dir(&mut explorer.tree, "destination");

        std::fs::rename(&moved_file, destination.join("note.md")).unwrap();
        explorer.refresh_preserving_expansion(std::slice::from_ref(&destination));

        let source_node = find_top_level_dir(&explorer.tree, "source").unwrap();
        let destination_node = find_top_level_dir(&explorer.tree, "destination").unwrap();
        assert!(source_node.expanded);
        assert!(destination_node.expanded);
        assert!(dir_contains_file(source_node, "zeta.md"));
        assert!(!dir_contains_file(source_node, "note.md"));
        assert!(dir_contains_file(destination_node, "note.md"));

        let _ = std::fs::remove_dir_all(root);
    }

    fn expand_top_level_dir(tree: &mut [TreeNode], name: &str) {
        let node = tree.iter_mut().find(|node| node.name == name).unwrap();
        node.toggle();
    }

    fn find_top_level_dir<'a>(tree: &'a [TreeNode], name: &str) -> Option<&'a TreeNode> {
        tree.iter().find(|node| node.is_dir && node.name == name)
    }

    fn dir_contains_file(node: &TreeNode, name: &str) -> bool {
        node.children.as_ref().is_some_and(|children| {
            children
                .iter()
                .any(|child| !child.is_dir && child.name == name)
        })
    }
}

const MAX_RECENT: usize = 5;

fn recent_projects_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("llnzy").join("recent_projects.json"))
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
        .filter(|p| p.exists())
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
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    if let Ok(json) = serde_json::to_string_pretty(&strings) {
        let _ = fs::write(path, json);
    }
}

/// Add a project to the recent list (moves to front, caps at MAX_RECENT).
pub fn add_recent_project(projects: &mut Vec<PathBuf>, path: PathBuf) {
    projects.retain(|p| p != &path);
    projects.insert(0, path);
    projects.truncate(MAX_RECENT);
    save_recent_projects(projects);
}

/// Get the display name for a project path (last directory component).
pub fn project_name(path: &Path) -> &str {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
}
