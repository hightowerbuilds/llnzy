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
    SplitRight,
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
    Split,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DragDropCommand {
    InsertTerminalPaths { tab_idx: usize, paths: Vec<PathBuf> },
    OpenFiles { paths: Vec<PathBuf> },
    OpenProject(PathBuf),
    ReorderTab { from: usize, to: usize },
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
            DropTarget::Editor { .. } | DropTarget::TabBar { .. } => {
                Some(DragDropCommand::OpenFiles { paths })
            }
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
    fn external_folder_drop_to_home_emits_open_project_command() {
        let root = std::env::temp_dir().join(format!("llnzy-dnd-project-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();

        let mut state = DragDropState::default();
        let command = state.command_for_external_files(vec![root.clone()], DropTarget::Home);

        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(command, Some(DragDropCommand::OpenProject(root)));
    }
}
