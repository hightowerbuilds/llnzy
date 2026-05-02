use std::path::{Path, PathBuf};

use llnzy::app::commands::AppCommand;
use llnzy::app::drag_drop::{
    plan_file_moves, remap_index_after_reorder, tab_insert_index, terminal_paths_text,
    DragDropCommand, FileMovePlan, TabDropZone,
};
use llnzy::editor::git_gutter::GitGutter;
use llnzy::path_utils::comparable_path;
use llnzy::workspace::remap_code_file_tab_paths;

use crate::runtime::commands::remap_joined_tabs_after_reorder;
use crate::App;

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
        let plan = match plan_file_moves(files, folder) {
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

        let mut moved = Vec::with_capacity(plan.len());
        for item in &plan {
            let source_key = comparable_path(&item.source);
            if let Err(error) = std::fs::rename(&item.source, &item.destination) {
                let message = format!("Move failed: {error}");
                self.report_file_move_status(message.clone());
                self.error_log.error(message);
                return false;
            }
            moved.push((source_key, item.clone()));
        }

        self.remap_moved_open_files(&moved);
        if let Some(ui) = &mut self.ui {
            ui.explorer
                .refresh_preserving_expansion(&[folder.to_path_buf()]);
            let moved_count = moved.len();
            ui.editor_view.status_msg = Some(if moved_count == 1 {
                "Moved file".to_string()
            } else {
                format!("Moved {moved_count} files")
            });
        }
        true
    }

    fn modified_open_file_move_error(&self, plan: &[FileMovePlan]) -> Option<String> {
        let ui = self.ui.as_ref()?;
        for item in plan {
            let source_key = comparable_path(&item.source);
            let Some(buffer) = ui.editor_view.editor.buffers.iter().find(|buffer| {
                buffer
                    .path()
                    .is_some_and(|path| comparable_path(path) == source_key)
            }) else {
                continue;
            };
            if buffer.is_modified() {
                return Some(format!(
                    "Save or close {} before moving it.",
                    buffer.file_name()
                ));
            }
        }
        None
    }

    fn remap_moved_open_files(&mut self, moved: &[(PathBuf, FileMovePlan)]) {
        for (source_key, item) in moved {
            self.remap_open_file_path(source_key, &item.destination);
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
                lsp.did_close(&buffer_old_path, lang_id);
                lsp.open_document(new_path, lang_id, &text);
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
