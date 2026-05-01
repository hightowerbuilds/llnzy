use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::editor::buffer::Position;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DragPayload {
    ExternalFiles(Vec<PathBuf>),
    ExplorerItems(Vec<PathBuf>),
    EditorSelection { buffer_idx: usize, text: String },
    WorkspaceTab { tab_idx: usize },
    StackerPrompt { prompt_idx: usize, text: String },
    SketchElements { element_ids: Vec<usize> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalDropMode {
    InsertEscapedPath,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabDropZone {
    Before,
    After,
    Center,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DropTarget {
    Terminal {
        tab_idx: usize,
        mode: TerminalDropMode,
    },
    Editor {
        buffer_idx: usize,
        position: Position,
    },
    ExplorerFolder {
        path: PathBuf,
    },
    TabBar {
        index: usize,
        zone: TabDropZone,
    },
    Stacker,
    SketchCanvas,
    Home,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragOperation {
    Move,
    Copy,
    Open,
    Insert,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DragDropCommand {
    InsertTerminalPaths {
        tab_idx: usize,
        paths: Vec<PathBuf>,
    },
    OpenFiles {
        paths: Vec<PathBuf>,
    },
    OpenFilesNearTab {
        paths: Vec<PathBuf>,
        tab_idx: usize,
        zone: TabDropZone,
    },
    OpenProject(PathBuf),
    ReorderTab {
        from: usize,
        to: usize,
    },
    MoveFilesToFolder {
        files: Vec<PathBuf>,
        folder: PathBuf,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileMovePlan {
    pub source: PathBuf,
    pub destination: PathBuf,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DragDropState {
    pub payload: Option<DragPayload>,
    pub active_target: Option<DropTarget>,
    pub operation: Option<DragOperation>,
    pub hovered_native_files: Vec<PathBuf>,
}

impl DragDropState {
    pub fn hover_native_file(&mut self, path: PathBuf) {
        if !self.hovered_native_files.iter().any(|p| p == &path) {
            self.hovered_native_files.push(path);
        }
        self.payload = Some(DragPayload::ExternalFiles(
            self.hovered_native_files.clone(),
        ));
    }

    pub fn cancel(&mut self) {
        self.payload = None;
        self.active_target = None;
        self.operation = None;
        self.hovered_native_files.clear();
    }

    pub fn command_for_external_files(
        &mut self,
        paths: Vec<PathBuf>,
        target: DropTarget,
    ) -> Option<DragDropCommand> {
        self.payload = Some(DragPayload::ExternalFiles(paths.clone()));
        self.active_target = Some(target.clone());
        let command = match target {
            DropTarget::Terminal {
                tab_idx,
                mode: TerminalDropMode::InsertEscapedPath,
            } => Some(DragDropCommand::InsertTerminalPaths { tab_idx, paths }),
            DropTarget::Home | DropTarget::ExplorerFolder { .. } => paths
                .iter()
                .find(|path| path.is_dir())
                .cloned()
                .map(DragDropCommand::OpenProject)
                .or_else(|| Some(DragDropCommand::OpenFiles { paths })),
            DropTarget::Editor { .. } => Some(DragDropCommand::OpenFiles { paths }),
            DropTarget::TabBar { index, zone } => Some(DragDropCommand::OpenFilesNearTab {
                paths,
                tab_idx: index,
                zone,
            }),
            _ => None,
        };
        self.cancel();
        command
    }
}

pub fn shell_escape_path(path: &Path) -> String {
    let path = path.to_string_lossy();
    format!("'{}'", path.replace('\'', "'\\''"))
}

pub fn terminal_paths_text(paths: &[PathBuf]) -> String {
    if paths.is_empty() {
        return String::new();
    }
    let mut text = paths
        .iter()
        .map(|path| shell_escape_path(path))
        .collect::<Vec<_>>()
        .join(" ");
    text.push(' ');
    text
}

pub fn plan_file_moves(files: &[PathBuf], folder: &Path) -> Result<Vec<FileMovePlan>, String> {
    if files.is_empty() {
        return Err("No files selected to move".to_string());
    }
    if !folder.is_dir() {
        return Err(format!("Drop target is not a folder: {}", folder.display()));
    }

    let folder_key = comparable_path(folder);
    let mut destinations = HashSet::new();
    let mut plan = Vec::with_capacity(files.len());
    for source in files {
        if !source.is_file() {
            return Err(format!("Only files can be moved: {}", source.display()));
        }
        if source.parent().map(comparable_path) == Some(folder_key.clone()) {
            return Err(format!(
                "{} is already in {}",
                source
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("File"),
                folder.display()
            ));
        }

        let Some(file_name) = source.file_name() else {
            return Err(format!("File has no name: {}", source.display()));
        };
        let destination = folder.join(file_name);
        if destination.exists() {
            return Err(format!(
                "A file named {} already exists in {}",
                file_name.to_string_lossy(),
                folder.display()
            ));
        }
        if !destinations.insert(destination.clone()) {
            return Err(format!(
                "Multiple moved files would land at {}",
                destination.display()
            ));
        }

        plan.push(FileMovePlan {
            source: source.clone(),
            destination,
        });
    }

    Ok(plan)
}

pub fn comparable_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

pub fn tab_index_at_x(
    pointer_x: f32,
    tab_bar_left: f32,
    tab_bar_width: f32,
    tab_count: usize,
) -> Option<usize> {
    if tab_count == 0 || tab_bar_width <= 0.0 || pointer_x < tab_bar_left {
        return None;
    }
    let rel_x = pointer_x - tab_bar_left;
    if rel_x >= tab_bar_width {
        return None;
    }
    let tab_width = (tab_bar_width / tab_count as f32).min(200.0).max(1.0);
    Some((rel_x / tab_width).floor().min((tab_count - 1) as f32) as usize)
}

pub fn tab_drop_zone_at_x(
    pointer_x: f32,
    tab_bar_left: f32,
    tab_bar_width: f32,
    tab_count: usize,
) -> Option<TabDropZone> {
    let idx = tab_index_at_x(pointer_x, tab_bar_left, tab_bar_width, tab_count)?;
    let tab_width = (tab_bar_width / tab_count as f32).min(200.0).max(1.0);
    let tab_left = tab_bar_left + idx as f32 * tab_width;
    let rel = ((pointer_x - tab_left) / tab_width).clamp(0.0, 1.0);
    if rel < 0.33 {
        Some(TabDropZone::Before)
    } else if rel > 0.66 {
        Some(TabDropZone::After)
    } else {
        Some(TabDropZone::Center)
    }
}

pub fn tab_insert_index(target: usize, zone: TabDropZone, tab_count: usize) -> usize {
    if tab_count == 0 {
        return 0;
    }
    let target = target.min(tab_count - 1);
    match zone {
        TabDropZone::Before => target,
        TabDropZone::After | TabDropZone::Center => target + 1,
    }
    .min(tab_count)
}

pub fn tab_reorder_destination(
    from: usize,
    target: usize,
    zone: TabDropZone,
    tab_count: usize,
) -> Option<usize> {
    if tab_count == 0 || from >= tab_count || target >= tab_count {
        return None;
    }

    let insertion_idx = match zone {
        TabDropZone::Before => target,
        TabDropZone::After | TabDropZone::Center => target + 1,
    };
    let to = if from < insertion_idx {
        insertion_idx.saturating_sub(1)
    } else {
        insertion_idx
    };

    (to < tab_count && to != from).then_some(to)
}

pub fn remap_index_after_reorder(index: usize, from: usize, to: usize) -> usize {
    if index == from {
        to
    } else if from < to && index > from && index <= to {
        index - 1
    } else if to < from && index >= to && index < from {
        index + 1
    } else {
        index
    }
}

pub fn remap_index_after_insert(index: usize, insert_at: usize) -> usize {
    if insert_at <= index {
        index + 1
    } else {
        index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_escape_path_wraps_plain_path() {
        assert_eq!(
            shell_escape_path(Path::new("/Users/me/example file.txt")),
            "'/Users/me/example file.txt'"
        );
    }

    #[test]
    fn shell_escape_path_escapes_single_quote() {
        assert_eq!(
            shell_escape_path(Path::new("/tmp/that's-it.txt")),
            "'/tmp/that'\\''s-it.txt'"
        );
    }

    #[test]
    fn terminal_paths_text_formats_multiple_paths() {
        let paths = vec![
            PathBuf::from("/tmp/one.txt"),
            PathBuf::from("/tmp/two words.txt"),
        ];
        assert_eq!(
            terminal_paths_text(&paths),
            "'/tmp/one.txt' '/tmp/two words.txt' "
        );
    }

    #[test]
    fn external_file_drop_to_terminal_emits_insert_command() {
        let mut state = DragDropState::default();
        let path = PathBuf::from("/tmp/file.txt");
        let command = state.command_for_external_files(
            vec![path.clone()],
            DropTarget::Terminal {
                tab_idx: 2,
                mode: TerminalDropMode::InsertEscapedPath,
            },
        );

        assert_eq!(
            command,
            Some(DragDropCommand::InsertTerminalPaths {
                tab_idx: 2,
                paths: vec![path]
            })
        );
        assert_eq!(state, DragDropState::default());
    }

    #[test]
    fn external_file_drop_to_editor_emits_open_files_command() {
        let mut state = DragDropState::default();
        let path = PathBuf::from("/tmp/file.rs");
        let command = state.command_for_external_files(
            vec![path.clone()],
            DropTarget::Editor {
                buffer_idx: 0,
                position: Position::new(0, 0),
            },
        );

        assert_eq!(
            command,
            Some(DragDropCommand::OpenFiles { paths: vec![path] })
        );
    }

    #[test]
    fn external_file_drop_to_tab_bar_opens_near_target_tab() {
        let mut state = DragDropState::default();
        let path = PathBuf::from("/tmp/file.rs");
        let command = state.command_for_external_files(
            vec![path.clone()],
            DropTarget::TabBar {
                index: 1,
                zone: TabDropZone::After,
            },
        );

        assert_eq!(
            command,
            Some(DragDropCommand::OpenFilesNearTab {
                paths: vec![path],
                tab_idx: 1,
                zone: TabDropZone::After
            })
        );
    }

    #[test]
    fn external_folder_drop_to_home_emits_open_project_command() {
        let root = std::env::temp_dir().join(format!("llnzy-dnd-project-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();

        let mut state = DragDropState::default();
        let command = state.command_for_external_files(vec![root.clone()], DropTarget::Home);

        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(command, Some(DragDropCommand::OpenProject(root)));
    }

    #[test]
    fn tab_reorder_destination_moves_left() {
        assert_eq!(
            tab_reorder_destination(3, 1, TabDropZone::Before, 5),
            Some(1)
        );
    }

    #[test]
    fn tab_index_at_x_uses_tab_strip_geometry() {
        assert_eq!(tab_index_at_x(100.0, 100.0, 500.0, 5), Some(0));
        assert_eq!(tab_index_at_x(299.0, 100.0, 500.0, 5), Some(1));
        assert_eq!(tab_index_at_x(599.0, 100.0, 500.0, 5), Some(4));
        assert_eq!(tab_index_at_x(600.0, 100.0, 500.0, 5), None);
    }

    #[test]
    fn tab_drop_zone_at_x_partitions_tab_region() {
        assert_eq!(
            tab_drop_zone_at_x(120.0, 100.0, 500.0, 5),
            Some(TabDropZone::Before)
        );
        assert_eq!(
            tab_drop_zone_at_x(150.0, 100.0, 500.0, 5),
            Some(TabDropZone::Center)
        );
        assert_eq!(
            tab_drop_zone_at_x(180.0, 100.0, 500.0, 5),
            Some(TabDropZone::After)
        );
    }

    #[test]
    fn tab_insert_index_places_files_near_target() {
        assert_eq!(tab_insert_index(2, TabDropZone::Before, 5), 2);
        assert_eq!(tab_insert_index(2, TabDropZone::Center, 5), 3);
        assert_eq!(tab_insert_index(2, TabDropZone::After, 5), 3);
        assert_eq!(tab_insert_index(9, TabDropZone::After, 5), 5);
    }

    #[test]
    fn tab_reorder_destination_moves_right_after_target() {
        assert_eq!(
            tab_reorder_destination(1, 3, TabDropZone::After, 5),
            Some(3)
        );
    }

    #[test]
    fn tab_reorder_destination_ignores_self_drop() {
        assert_eq!(tab_reorder_destination(2, 2, TabDropZone::Before, 5), None);
        assert_eq!(tab_reorder_destination(2, 2, TabDropZone::After, 5), None);
    }

    #[test]
    fn remap_index_after_reorder_tracks_moved_and_shifted_tabs() {
        assert_eq!(remap_index_after_reorder(1, 1, 3), 3);
        assert_eq!(remap_index_after_reorder(2, 1, 3), 1);
        assert_eq!(remap_index_after_reorder(3, 1, 3), 2);
        assert_eq!(remap_index_after_reorder(0, 1, 3), 0);

        assert_eq!(remap_index_after_reorder(3, 3, 1), 1);
        assert_eq!(remap_index_after_reorder(1, 3, 1), 2);
        assert_eq!(remap_index_after_reorder(2, 3, 1), 3);
        assert_eq!(remap_index_after_reorder(4, 3, 1), 4);
    }

    #[test]
    fn remap_index_after_insert_shifts_indexes_at_or_after_insert() {
        assert_eq!(remap_index_after_insert(0, 1), 0);
        assert_eq!(remap_index_after_insert(1, 1), 2);
        assert_eq!(remap_index_after_insert(3, 1), 4);
    }

    #[test]
    fn plan_file_moves_moves_file_to_destination_folder() {
        let root =
            std::env::temp_dir().join(format!("llnzy-file-move-plan-{}-a", std::process::id()));
        let source_dir = root.join("source");
        let destination_dir = root.join("destination");
        std::fs::create_dir_all(&source_dir).unwrap();
        std::fs::create_dir_all(&destination_dir).unwrap();
        let source = source_dir.join("note.md");
        std::fs::write(&source, "hello").unwrap();

        let plan = plan_file_moves(std::slice::from_ref(&source), &destination_dir).unwrap();

        assert_eq!(
            plan,
            vec![FileMovePlan {
                source: source.clone(),
                destination: destination_dir.join("note.md")
            }]
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn plan_file_moves_rejects_folder_sources() {
        let root =
            std::env::temp_dir().join(format!("llnzy-file-move-plan-{}-b", std::process::id()));
        let source_dir = root.join("source-folder");
        let destination_dir = root.join("destination");
        std::fs::create_dir_all(&source_dir).unwrap();
        std::fs::create_dir_all(&destination_dir).unwrap();

        let error =
            plan_file_moves(std::slice::from_ref(&source_dir), &destination_dir).unwrap_err();

        assert!(error.contains("Only files can be moved"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn plan_file_moves_rejects_existing_destination() {
        let root =
            std::env::temp_dir().join(format!("llnzy-file-move-plan-{}-c", std::process::id()));
        let source_dir = root.join("source");
        let destination_dir = root.join("destination");
        std::fs::create_dir_all(&source_dir).unwrap();
        std::fs::create_dir_all(&destination_dir).unwrap();
        let source = source_dir.join("note.md");
        std::fs::write(&source, "hello").unwrap();
        std::fs::write(destination_dir.join("note.md"), "existing").unwrap();

        let error = plan_file_moves(std::slice::from_ref(&source), &destination_dir).unwrap_err();

        assert!(error.contains("already exists"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn plan_file_moves_rejects_same_parent_folder() {
        let root =
            std::env::temp_dir().join(format!("llnzy-file-move-plan-{}-d", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("note.md");
        std::fs::write(&source, "hello").unwrap();

        let error = plan_file_moves(std::slice::from_ref(&source), &root).unwrap_err();

        assert!(error.contains("already in"));
        let _ = std::fs::remove_dir_all(root);
    }
}
