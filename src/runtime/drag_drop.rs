use std::path::{Path, PathBuf};

use llnzy::app::commands::AppCommand;
use llnzy::app::drag_drop::{
    remap_index_after_reorder, tab_insert_index, terminal_paths_text, DragDropCommand, TabDropZone,
};
use llnzy::editor::git_gutter::GitGutter;
use llnzy::path_utils::{comparable_path, path_contains, same_path};
use llnzy::sidebar_move::{
    plan_sidebar_move, MoveOrigin, SidebarMovePlan, SidebarMovePlanItem, SidebarMoveRequest,
};
use llnzy::workspace::remap_code_file_tab_paths;

use crate::runtime::commands::remap_joined_tabs_after_reorder;
use crate::App;

#[derive(Clone, Debug, PartialEq, Eq)]
struct OpenFileMoveCandidate {
    path: PathBuf,
    file_name: String,
    modified: bool,
}

fn modified_open_file_move_message<I>(plan: &SidebarMovePlan, open_files: I) -> Option<String>
where
    I: IntoIterator<Item = OpenFileMoveCandidate>,
{
    let open_files: Vec<_> = open_files.into_iter().collect();
    for item in &plan.items {
        let Some(file) = open_files
            .iter()
            .find(|file| move_plan_item_affects_path(item, &file.path))
        else {
            continue;
        };
        if file.modified {
            return Some(format!(
                "Save or close {} before moving it.",
                file.file_name
            ));
        }
    }
    None
}

impl App {
    pub(crate) fn handle_drag_drop_command(&mut self, command: DragDropCommand) -> bool {
        match command {
            DragDropCommand::InsertTerminalPaths { tab_idx, paths } => {
                let text = terminal_paths_text(&paths);
                if text.is_empty() {
                    return false;
                }
                self.write_to_terminal_tab(tab_idx, text.as_bytes())
            }
            DragDropCommand::OpenFiles { paths } => self.open_dropped_files(paths),
            DragDropCommand::OpenFilesNearTab {
                paths,
                tab_idx,
                zone,
            } => self.open_dropped_files_near_tab(paths, tab_idx, zone),
            DragDropCommand::OpenProject(project_path) => {
                let mut sidebar_changed = false;
                let handled = self.handle_app_command(
                    AppCommand::OpenProject(project_path),
                    &mut sidebar_changed,
                );
                if sidebar_changed {
                    self.recompute_layout();
                    self.resize_terminal_tabs();
                }
                handled
            }
            DragDropCommand::ReorderTab { from, to } => {
                if from >= self.tabs.len() || to >= self.tabs.len() || from == to {
                    return false;
                }
                let tab = self.tabs.remove(from);
                self.tabs.insert(to, tab);
                self.active_tab = remap_index_after_reorder(self.active_tab, from, to);
                remap_joined_tabs_after_reorder(self.ui.as_mut(), from, to);
                true
            }
            DragDropCommand::MoveFilesToFolder { files, folder } => {
                self.move_files_to_folder(&files, &folder)
            }
        }
    }

    fn open_dropped_files(&mut self, paths: Vec<PathBuf>) -> bool {
        let Some(opened) = self.open_file_buffers_for_drop(paths) else {
            return false;
        };
        let opened_any = !opened.is_empty();
        for (path, buffer_id) in opened {
            self.open_code_file_tab(path, buffer_id);
        }
        opened_any
    }

    fn open_dropped_files_near_tab(
        &mut self,
        paths: Vec<PathBuf>,
        tab_idx: usize,
        zone: TabDropZone,
    ) -> bool {
        let Some(opened) = self.open_file_buffers_for_drop(paths) else {
            return false;
        };
        let opened_any = !opened.is_empty();
        let mut insert_at = tab_insert_index(tab_idx, zone, self.tabs.len());
        for (path, buffer_id) in opened {
            if self.open_code_file_tab_at(path, buffer_id, insert_at) {
                insert_at = self.active_tab + 1;
            }
        }
        opened_any
    }

    fn open_file_buffers_for_drop(
        &mut self,
        paths: Vec<PathBuf>,
    ) -> Option<Vec<(PathBuf, llnzy::editor::BufferId)>> {
        let ui = self.ui.as_mut()?;
        let mut opened = Vec::new();
        let mut errors = Vec::new();
        for path in paths {
            match ui.editor_view.open_file(path.clone()) {
                Ok(buffer_id) => opened.push((path, buffer_id)),
                Err(e) => errors.push(format!("Drop: {e}")),
            }
        }
        for error in errors {
            self.error_log.error(error);
        }
        Some(opened)
    }

    fn move_files_to_folder(&mut self, files: &[PathBuf], folder: &Path) -> bool {
        let request =
            SidebarMoveRequest::new(files.to_vec(), folder.to_path_buf(), MoveOrigin::DragDrop);
        let plan = match plan_sidebar_move(&request) {
            Ok(plan) => plan,
            Err(message) => {
                self.report_file_move_status(message.clone());
                self.error_log.error(message);
                return false;
            }
        };

        if let Some(message) = self.modified_open_file_move_error(&plan) {
            self.report_file_move_status(message.clone());
            self.error_log.error(message);
            return false;
        }

        for item in &plan.items {
            if let Err(error) = std::fs::rename(&item.source, &item.destination) {
                let message = format!("Move failed: {error}");
                self.report_file_move_status(message.clone());
                self.error_log.error(message);
                return false;
            }
        }

        self.remap_moved_open_files(&plan.items);
        if let Some(ui) = &mut self.ui {
            ui.explorer
                .refresh_preserving_expansion(&plan.refresh_paths());
            let moved_count = plan.len();
            ui.editor_view.status_msg = Some(if moved_count == 1 {
                "Moved item".to_string()
            } else {
                format!("Moved {moved_count} items")
            });
        }
        true
    }

    fn modified_open_file_move_error(&self, plan: &SidebarMovePlan) -> Option<String> {
        let ui = self.ui.as_ref()?;
        modified_open_file_move_message(
            plan,
            ui.editor_view.editor.buffers.iter().filter_map(|buffer| {
                let path = buffer.path()?.to_path_buf();
                Some(OpenFileMoveCandidate {
                    path,
                    file_name: buffer.file_name().to_string(),
                    modified: buffer.is_modified(),
                })
            }),
        )
    }

    fn remap_moved_open_files(&mut self, moved: &[SidebarMovePlanItem]) {
        let Some(ui) = self.ui.as_ref() else { return };
        let remaps = ui
            .editor_view
            .editor
            .buffers
            .iter()
            .filter_map(|buffer| {
                let buffer_path = buffer.path()?.to_path_buf();
                let item = moved
                    .iter()
                    .find(|item| move_plan_item_affects_path(item, &buffer_path))?;
                let new_path = if item.is_dir {
                    let relative = buffer_path.strip_prefix(&item.source).ok()?;
                    item.destination.join(relative)
                } else {
                    item.destination.clone()
                };
                Some((buffer_path, new_path))
            })
            .collect::<Vec<_>>();

        for (old_path, new_path) in remaps {
            self.remap_open_file_path(&old_path, &new_path);
        }
    }

    pub(crate) fn remap_open_file_path(&mut self, old_path: &PathBuf, new_path: &PathBuf) {
        let Some(ui) = &mut self.ui else { return };
        let old_key = comparable_path(old_path);
        let mut remapped_buffer_ids = Vec::new();
        for (idx, buffer) in ui.editor_view.editor.buffers.iter_mut().enumerate() {
            let Some(buffer_old_path) = buffer.path().map(PathBuf::from) else {
                continue;
            };
            if comparable_path(&buffer_old_path) != old_key {
                continue;
            }

            let lang_id = ui.editor_view.editor.views[idx].lang_id;
            let text = buffer.text();
            buffer.set_path(new_path.clone());
            if let Some(view) = ui.editor_view.editor.views.get_mut(idx) {
                view.tree_dirty = true;
                view.git_gutter = GitGutter::load(new_path);
            }
            if let Some(watcher) = &mut ui.editor_view.file_watcher {
                watcher.unwatch(&buffer_old_path);
                watcher.watch(new_path);
            }
            if let (Some(lsp), Some(lang_id)) = (&mut ui.editor_view.lsp, lang_id) {
                lsp.did_move(&buffer_old_path, new_path, lang_id, &text);
            }
            if let Some(buffer_id) = ui.editor_view.editor.buffer_ids.get(idx).copied() {
                remapped_buffer_ids.push(buffer_id);
            }
        }

        remap_code_file_tab_paths(&mut self.tabs, old_path, new_path, &remapped_buffer_ids);
    }

    fn report_file_move_status(&mut self, message: String) {
        if let Some(ui) = &mut self.ui {
            ui.editor_view.status_msg = Some(message);
        }
    }
}

fn move_plan_item_affects_path(item: &SidebarMovePlanItem, path: &Path) -> bool {
    if item.is_dir {
        path_contains(&item.source, path)
    } else {
        same_path(&item.source, path) || comparable_path(&item.source) == comparable_path(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn move_plan(source: &str, destination: &str) -> SidebarMovePlan {
        SidebarMovePlan {
            destination_folder: PathBuf::from(destination)
                .parent()
                .unwrap_or(Path::new("/"))
                .to_path_buf(),
            items: vec![SidebarMovePlanItem {
                source: PathBuf::from(source),
                destination: PathBuf::from(destination),
                is_dir: false,
            }],
        }
    }

    fn folder_move_plan(source: &str, destination: &str) -> SidebarMovePlan {
        SidebarMovePlan {
            destination_folder: PathBuf::from(destination)
                .parent()
                .unwrap_or(Path::new("/"))
                .to_path_buf(),
            items: vec![SidebarMovePlanItem {
                source: PathBuf::from(source),
                destination: PathBuf::from(destination),
                is_dir: true,
            }],
        }
    }

    fn open_file(path: &str, file_name: &str, modified: bool) -> OpenFileMoveCandidate {
        OpenFileMoveCandidate {
            path: PathBuf::from(path),
            file_name: file_name.to_string(),
            modified,
        }
    }

    #[test]
    fn open_clean_file_move_is_not_blocked() {
        let plan = move_plan("/project/src/note.md", "/project/archive/note.md");
        let message = modified_open_file_move_message(
            &plan,
            [open_file("/project/src/note.md", "note.md", false)],
        );

        assert_eq!(message, None);
    }

    #[test]
    fn open_dirty_file_move_is_blocked() {
        let plan = move_plan("/project/src/note.md", "/project/archive/note.md");
        let message = modified_open_file_move_message(
            &plan,
            [open_file("/project/src/note.md", "note.md", true)],
        );

        assert_eq!(
            message,
            Some("Save or close note.md before moving it.".to_string())
        );
    }

    #[test]
    fn unrelated_dirty_open_file_does_not_block_move() {
        let plan = move_plan("/project/src/note.md", "/project/archive/note.md");
        let message = modified_open_file_move_message(
            &plan,
            [open_file("/project/src/other.md", "other.md", true)],
        );

        assert_eq!(message, None);
    }

    #[test]
    fn dirty_open_file_inside_folder_move_is_blocked() {
        let plan = folder_move_plan("/project/src", "/project/archive/src");
        let message = modified_open_file_move_message(
            &plan,
            [open_file("/project/src/note.md", "note.md", true)],
        );

        assert_eq!(
            message,
            Some("Save or close note.md before moving it.".to_string())
        );
    }
}
