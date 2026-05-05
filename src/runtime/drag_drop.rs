use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use llnzy::app::commands::AppCommand;
use llnzy::app::drag_drop::{
    remap_index_after_reorder, tab_insert_index, terminal_paths_text, DragDropCommand, TabDropZone,
};
use llnzy::editor::git_gutter::GitGutter;
use llnzy::explorer::is_image_path;
use llnzy::path_utils::{comparable_path, path_contains, same_path};
use llnzy::sidebar_move::{
    plan_sidebar_move, MoveOrigin, SidebarMovePlan, SidebarMovePlanItem, SidebarMoveRequest,
};
use llnzy::workspace::remap_code_file_tab_paths;

use crate::App;

#[derive(Clone, Debug, PartialEq, Eq)]
struct OpenFileMoveCandidate {
    path: PathBuf,
    file_name: String,
    modified: bool,
}

#[derive(Default)]
struct DroppedOpenResult {
    buffers: Vec<(PathBuf, llnzy::editor::BufferId)>,
    images: Vec<PathBuf>,
    skipped: usize,
}

impl DroppedOpenResult {
    fn opened_any(&self) -> bool {
        !self.buffers.is_empty() || !self.images.is_empty()
    }
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
                true
            }
            DragDropCommand::MoveFilesToFolder { files, folder } => {
                self.move_files_to_folder(&files, &folder)
            }
            DragDropCommand::CopyExternalFilesToFolder { files, folder } => {
                self.copy_external_files_to_folder(&files, &folder)
            }
        }
    }

    fn open_dropped_files(&mut self, paths: Vec<PathBuf>) -> bool {
        let Some(opened) = self.open_file_buffers_for_drop(paths) else {
            return false;
        };
        let opened_any = opened.opened_any();
        for path in opened.images {
            self.open_image_file_tab(path);
        }
        for (path, buffer_id) in opened.buffers {
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
        let opened_any = opened.opened_any();
        let mut insert_at = tab_insert_index(tab_idx, zone, self.tabs.len());
        for path in opened.images {
            self.open_image_file_tab(path);
        }
        for (path, buffer_id) in opened.buffers {
            if self.open_code_file_tab_at(path, buffer_id, insert_at) {
                insert_at = self.active_tab + 1;
            }
        }
        opened_any
    }

    fn open_file_buffers_for_drop(&mut self, paths: Vec<PathBuf>) -> Option<DroppedOpenResult> {
        let mut result = DroppedOpenResult::default();
        let mut errors = Vec::new();

        {
            let ui = self.ui.as_mut()?;
            for path in paths {
                if path.is_dir() {
                    result.skipped += 1;
                    errors.push(format!(
                        "Drop skipped: {} is a folder. Drop folders on Home or the sidebar.",
                        dropped_file_name(&path)
                    ));
                    continue;
                }

                if is_image_path(&path) {
                    result.images.push(path);
                    continue;
                }

                match ui.editor_view.open_file(path.clone()) {
                    Ok(buffer_id) => result.buffers.push((path, buffer_id)),
                    Err(e) => {
                        result.skipped += 1;
                        errors.push(format!(
                            "Drop skipped: {} could not be opened as text ({e})",
                            dropped_file_name(&path)
                        ));
                    }
                }
            }

            if !result.images.is_empty() {
                ui.editor_view.status_msg = Some(if result.images.len() == 1 {
                    "Opened image tab".to_string()
                } else {
                    format!("Opened {} image tabs", result.images.len())
                });
            } else if result.buffers.is_empty() && result.skipped > 0 {
                ui.editor_view.status_msg = errors.last().cloned();
            }
        }

        for error in errors {
            self.error_log.error(error);
        }
        Some(result)
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

    fn copy_external_files_to_folder(&mut self, files: &[PathBuf], folder: &Path) -> bool {
        let plan = match plan_external_file_copy(files, folder) {
            Ok(plan) => plan,
            Err(message) => {
                self.report_file_move_status(message.clone());
                self.error_log.error(message);
                return false;
            }
        };

        for item in &plan.items {
            if let Err(error) = copy_external_item(&item.source, &item.destination) {
                let message = format!("Import failed: {error}");
                self.report_file_move_status(message.clone());
                self.error_log.error(message);
                return false;
            }
        }

        if let Some(ui) = &mut self.ui {
            ui.explorer
                .refresh_preserving_expansion(&[plan.destination_folder.clone()]);
            let copied_count = plan.items.len();
            ui.editor_view.status_msg = Some(if copied_count == 1 {
                "Imported item".to_string()
            } else {
                format!("Imported {copied_count} items")
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExternalCopyPlan {
    destination_folder: PathBuf,
    items: Vec<ExternalCopyPlanItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExternalCopyPlanItem {
    source: PathBuf,
    destination: PathBuf,
}

fn plan_external_file_copy(files: &[PathBuf], folder: &Path) -> Result<ExternalCopyPlan, String> {
    if files.is_empty() {
        return Err("No files to import.".to_string());
    }
    if !folder.is_dir() {
        return Err("Import target is not a folder.".to_string());
    }

    let mut seen_destinations = HashSet::new();
    let mut items = Vec::with_capacity(files.len());
    for source in files {
        if !source.exists() {
            return Err(format!(
                "Import failed: {} does not exist.",
                source.display()
            ));
        }
        let Some(name) = source.file_name() else {
            return Err("Import failed: source has no file name.".to_string());
        };
        let destination = folder.join(name);
        if same_path(source, &destination)
            || comparable_path(source) == comparable_path(&destination)
        {
            return Err("Import skipped: item is already in that folder.".to_string());
        }
        if source.is_dir() && path_contains(source, &destination) {
            return Err("Import failed: cannot copy a folder into itself.".to_string());
        }
        if destination.exists() {
            return Err(format!(
                "Import skipped: {} already exists.",
                destination.display()
            ));
        }
        if !seen_destinations.insert(comparable_path(&destination)) {
            return Err("Import failed: duplicate destination names in this drop.".to_string());
        }
        items.push(ExternalCopyPlanItem {
            source: source.clone(),
            destination,
        });
    }

    Ok(ExternalCopyPlan {
        destination_folder: folder.to_path_buf(),
        items,
    })
}

fn copy_external_item(source: &Path, destination: &Path) -> std::io::Result<()> {
    if source.is_dir() {
        copy_dir_recursive(source, destination)
    } else {
        fs::copy(source, destination).map(|_| ())
    }
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::create_dir(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        copy_external_item(&source_path, &destination_path)?;
    }
    Ok(())
}

fn dropped_file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| path.display().to_string())
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

    #[test]
    fn external_copy_plan_targets_destination_folder_by_name() {
        let root = temp_path("copy-plan");
        let source = root.join("desktop").join("screenshot.png");
        let destination_folder = root.join("repo").join("assets");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::create_dir_all(&destination_folder).unwrap();
        fs::write(&source, "image").unwrap();

        let plan =
            plan_external_file_copy(std::slice::from_ref(&source), &destination_folder).unwrap();

        assert_eq!(
            plan.items,
            vec![ExternalCopyPlanItem {
                source,
                destination: destination_folder.join("screenshot.png"),
            }]
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn external_copy_plan_rejects_existing_destination() {
        let root = temp_path("copy-existing");
        let source = root.join("desktop").join("screenshot.png");
        let destination_folder = root.join("repo");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::create_dir_all(&destination_folder).unwrap();
        fs::write(&source, "image").unwrap();
        fs::write(destination_folder.join("screenshot.png"), "old").unwrap();

        let message = plan_external_file_copy(&[source], &destination_folder).unwrap_err();

        assert!(message.contains("already exists"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn external_copy_plan_rejects_duplicate_destination_names_in_one_drop() {
        let root = temp_path("copy-duplicate-drop");
        let source_a = root.join("desktop-a").join("screenshot.png");
        let source_b = root.join("desktop-b").join("screenshot.png");
        let destination_folder = root.join("repo");
        fs::create_dir_all(source_a.parent().unwrap()).unwrap();
        fs::create_dir_all(source_b.parent().unwrap()).unwrap();
        fs::create_dir_all(&destination_folder).unwrap();
        fs::write(&source_a, "a").unwrap();
        fs::write(&source_b, "b").unwrap();

        let message =
            plan_external_file_copy(&[source_a, source_b], &destination_folder).unwrap_err();

        assert!(message.contains("duplicate destination names"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn copy_external_item_copies_directories_recursively() {
        let root = temp_path("copy-recursive");
        let source = root.join("desktop").join("assets");
        let nested = source.join("nested");
        let destination = root.join("repo").join("assets");
        fs::create_dir_all(&nested).unwrap();
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::write(source.join("top.txt"), "top").unwrap();
        fs::write(nested.join("deep.txt"), "deep").unwrap();

        copy_external_item(&source, &destination).unwrap();

        assert_eq!(
            fs::read_to_string(destination.join("top.txt")).unwrap(),
            "top"
        );
        assert_eq!(
            fs::read_to_string(destination.join("nested").join("deep.txt")).unwrap(),
            "deep"
        );

        let _ = fs::remove_dir_all(root);
    }

    fn temp_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!("llnzy-runtime-dnd-{}-{label}", std::process::id()))
    }
}
