use super::explorer_view::EditorViewState;
use crate::explorer::ExplorerState;
use crate::path_utils::{path_contains, same_path};
use std::fs::OpenOptions;
use std::path::Path;

pub(super) fn render_sidebar_file_modals(
    ui: &mut egui::Ui,
    explorer: &mut ExplorerState,
    editor_state: &mut EditorViewState,
) {
    render_rename_modal(ui, explorer, editor_state);
    render_delete_modal(ui, explorer, editor_state);
    render_new_entry_modal(ui, explorer, editor_state);
}

fn render_rename_modal(
    ui: &mut egui::Ui,
    explorer: &mut ExplorerState,
    editor_state: &mut EditorViewState,
) {
    if editor_state.sidebar_rename.is_none() {
        return;
    }

    let (rename_path, mut rename_text) = editor_state.sidebar_rename.take().unwrap();
    let file_name = rename_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();
    let mut done = false;
    let mut cancel = false;
    egui::Window::new("Rename")
        .id(egui::Id::new("sidebar_rename_modal"))
        .fixed_pos(egui::pos2(
            ui.ctx().screen_rect().center().x - 140.0,
            ui.ctx().screen_rect().center().y - 40.0,
        ))
        .resizable(false)
        .show(ui.ctx(), |ui| {
            ui.label(
                egui::RichText::new(format!("Rename: {file_name}"))
                    .size(13.0)
                    .color(egui::Color32::WHITE),
            );
            ui.add_space(4.0);
            let resp = ui.add(
                egui::TextEdit::singleline(&mut rename_text)
                    .desired_width(250.0)
                    .text_color(egui::Color32::WHITE)
                    .font(egui::TextStyle::Monospace),
            );
            resp.request_focus();
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancel = true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) && !rename_text.trim().is_empty() {
                done = true;
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Rename")
                                .size(12.0)
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(40, 100, 200)),
                    )
                    .clicked()
                    && !rename_text.trim().is_empty()
                {
                    done = true;
                }
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Cancel")
                                .size(12.0)
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(50, 52, 62)),
                    )
                    .clicked()
                {
                    cancel = true;
                }
            });
        });

    if done {
        match rename_sidebar_entry(&rename_path, &rename_text, editor_state) {
            Ok(RenameOutcome::Unchanged(name)) => {
                editor_state.status_msg = Some(format!("Name unchanged: {name}"));
            }
            Ok(RenameOutcome::Renamed {
                new_name,
                expand_paths,
                remap_open_file,
            }) => {
                explorer.refresh_preserving_expansion(&expand_paths);
                if let Some(remap) = remap_open_file {
                    editor_state.pending_file_remap = Some(remap);
                }
                editor_state.status_msg = Some(format!("Renamed to {new_name}"));
            }
            Err(e) => {
                editor_state.status_msg = Some(format!("Rename failed: {e}"));
            }
        }
    } else if !cancel {
        editor_state.sidebar_rename = Some((rename_path, rename_text));
    }
}

fn render_delete_modal(
    ui: &mut egui::Ui,
    explorer: &mut ExplorerState,
    editor_state: &mut EditorViewState,
) {
    let Some(delete_path) = editor_state.sidebar_delete_confirm.clone() else {
        return;
    };

    let display_name = delete_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("item");
    let is_dir = delete_path.is_dir();
    let mut confirm = false;
    let mut cancel = false;
    egui::Window::new("Delete")
        .id(egui::Id::new("sidebar_delete_modal"))
        .fixed_pos(egui::pos2(
            ui.ctx().screen_rect().center().x - 160.0,
            ui.ctx().screen_rect().center().y - 40.0,
        ))
        .resizable(false)
        .show(ui.ctx(), |ui| {
            ui.label(
                egui::RichText::new(format!("Delete \"{display_name}\"? This cannot be undone."))
                    .size(13.0)
                    .color(egui::Color32::from_rgb(210, 215, 225)),
            );
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Delete")
                                .size(12.0)
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(180, 50, 50)),
                    )
                    .clicked()
                {
                    confirm = true;
                }
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Cancel")
                                .size(12.0)
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(50, 52, 62)),
                    )
                    .clicked()
                {
                    cancel = true;
                }
            });
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancel = true;
            }
        });

    if confirm {
        if let Some(message) =
            blocking_open_file_lifecycle_message(editor_state, &delete_path, "deleting")
        {
            editor_state.status_msg = Some(message);
        } else {
            let result = if is_dir {
                std::fs::remove_dir_all(&delete_path)
            } else {
                std::fs::remove_file(&delete_path)
            };
            match result {
                Ok(_) => {
                    let expand_paths = delete_path
                        .parent()
                        .map(|path| vec![path.to_path_buf()])
                        .unwrap_or_default();
                    explorer.refresh_preserving_expansion(&expand_paths);
                    editor_state.status_msg = Some(format!("Deleted {display_name}"));
                }
                Err(e) => editor_state.status_msg = Some(format!("Delete failed: {e}")),
            }
        }
        editor_state.sidebar_delete_confirm = None;
    } else if cancel {
        editor_state.sidebar_delete_confirm = None;
    }
}

fn blocking_open_file_lifecycle_message(
    editor_state: &EditorViewState,
    target: &Path,
    action: &str,
) -> Option<String> {
    let affected = affected_open_buffers(editor_state, target);
    let first_dirty = affected
        .iter()
        .find(|(_, is_modified)| *is_modified)
        .map(|(file_name, _)| file_name.as_str());
    if let Some(file_name) = first_dirty {
        return Some(format!("Save or close {file_name} before {action} it."));
    }

    let target_is_dir = target.is_dir();
    if target_is_dir && !affected.is_empty() {
        let target_name = target
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("folder");
        return Some(format!(
            "Close open files inside {target_name} before {action} it."
        ));
    }

    None
}

fn affected_open_buffers(editor_state: &EditorViewState, target: &Path) -> Vec<(String, bool)> {
    let target_is_dir = target.is_dir();
    editor_state
        .editor
        .buffers
        .iter()
        .filter_map(|buffer| {
            let buffer_path = buffer.path()?;
            let affected = if target_is_dir {
                path_contains(target, buffer_path)
            } else {
                same_path(buffer_path, target)
            };
            affected.then(|| (buffer.file_name().to_string(), buffer.is_modified()))
        })
        .collect()
}

fn render_new_entry_modal(
    ui: &mut egui::Ui,
    explorer: &mut ExplorerState,
    editor_state: &mut EditorViewState,
) {
    if editor_state.sidebar_new_entry.is_none() {
        return;
    }

    let (parent_dir, mut input_text, is_folder) = editor_state.sidebar_new_entry.take().unwrap();
    let kind = if is_folder { "Folder" } else { "File" };
    let mut done = false;
    let mut cancel = false;
    egui::Window::new(format!("New {kind}"))
        .id(egui::Id::new("sidebar_new_entry_modal"))
        .fixed_pos(egui::pos2(
            ui.ctx().screen_rect().center().x - 140.0,
            ui.ctx().screen_rect().center().y - 40.0,
        ))
        .resizable(false)
        .show(ui.ctx(), |ui| {
            ui.label(
                egui::RichText::new(format!("New {kind} name:"))
                    .size(13.0)
                    .color(egui::Color32::WHITE),
            );
            ui.add_space(4.0);
            let resp = ui.add(
                egui::TextEdit::singleline(&mut input_text)
                    .desired_width(250.0)
                    .text_color(egui::Color32::WHITE)
                    .font(egui::TextStyle::Monospace),
            );
            resp.request_focus();
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancel = true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) && !input_text.trim().is_empty() {
                done = true;
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Create")
                                .size(12.0)
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(40, 100, 200)),
                    )
                    .clicked()
                    && !input_text.trim().is_empty()
                {
                    done = true;
                }
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Cancel")
                                .size(12.0)
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(50, 52, 62)),
                    )
                    .clicked()
                {
                    cancel = true;
                }
            });
        });

    if done {
        match create_sidebar_entry(&parent_dir, &input_text, is_folder) {
            Ok(new_path) => {
                let mut expand_paths = vec![parent_dir.clone()];
                if is_folder {
                    expand_paths.push(new_path.clone());
                }
                explorer.refresh_preserving_expansion(&expand_paths);
                let name = new_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("item");
                editor_state.status_msg = Some(format!("Created {name}"));
            }
            Err(e) => editor_state.status_msg = Some(format!("Create failed: {e}")),
        }
    } else if !cancel {
        editor_state.sidebar_new_entry = Some((parent_dir, input_text, is_folder));
    }
}

#[derive(Debug)]
enum RenameOutcome {
    Renamed {
        new_name: String,
        expand_paths: Vec<std::path::PathBuf>,
        remap_open_file: Option<(std::path::PathBuf, std::path::PathBuf)>,
    },
    Unchanged(String),
}

fn rename_sidebar_entry(
    rename_path: &Path,
    raw_name: &str,
    editor_state: &EditorViewState,
) -> Result<RenameOutcome, String> {
    let new_name = validate_entry_name(raw_name)?.to_string();
    let current_name = rename_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Cannot determine current name".to_string())?;
    if current_name == new_name {
        return Ok(RenameOutcome::Unchanged(new_name));
    }
    if !rename_path.exists() {
        return Err("item no longer exists".to_string());
    }
    if let Some(message) =
        blocking_open_file_lifecycle_message(editor_state, rename_path, "renaming")
    {
        return Err(message);
    }

    let parent = rename_path
        .parent()
        .ok_or_else(|| "Cannot rename project root".to_string())?;
    let new_path = parent.join(&new_name);
    if new_path.exists() && !same_path(rename_path, &new_path) {
        return Err(format!("{new_name} already exists"));
    }

    let was_dir = rename_path.is_dir();
    std::fs::rename(rename_path, &new_path).map_err(|e| e.to_string())?;
    let mut expand_paths = vec![parent.to_path_buf()];
    if was_dir {
        expand_paths.push(new_path.clone());
    }
    let remap_open_file = (!was_dir).then(|| (rename_path.to_path_buf(), new_path.clone()));

    Ok(RenameOutcome::Renamed {
        new_name,
        expand_paths,
        remap_open_file,
    })
}

fn create_sidebar_entry(
    parent_dir: &Path,
    raw_name: &str,
    is_folder: bool,
) -> Result<std::path::PathBuf, String> {
    let name = validate_entry_name(raw_name)?;
    if !parent_dir.is_dir() {
        return Err("parent folder no longer exists".to_string());
    }
    let new_path = parent_dir.join(name);
    if new_path.exists() {
        return Err(format!("{name} already exists"));
    }

    if is_folder {
        std::fs::create_dir(&new_path).map_err(|e| e.to_string())?;
    } else {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&new_path)
            .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
    }

    Ok(new_path)
}

fn validate_entry_name(raw_name: &str) -> Result<&str, String> {
    let name = raw_name.trim();
    if name.is_empty() {
        return Err("name cannot be empty".to_string());
    }
    if name == "." || name == ".." {
        return Err("name cannot be . or ..".to_string());
    }
    if name.contains('/') || name.contains('\\') {
        return Err("name cannot contain path separators".to_string());
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::buffer::Position;

    #[test]
    fn dirty_open_file_blocks_sidebar_rename_or_delete() {
        let root = temp_root("dirty-file");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("note.md");
        std::fs::write(&path, "saved").unwrap();

        let mut editor_state = EditorViewState::default();
        editor_state.editor.open(path.clone()).unwrap();
        editor_state.editor.buffers[0].insert(Position::new(0, 0), "unsaved ");

        let message = blocking_open_file_lifecycle_message(&editor_state, &path, "renaming")
            .expect("dirty open file should block sidebar lifecycle action");

        assert_eq!(message, "Save or close note.md before renaming it.");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn clean_open_file_does_not_block_exact_file_rename() {
        let root = temp_root("clean-file");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("note.md");
        std::fs::write(&path, "saved").unwrap();

        let mut editor_state = EditorViewState::default();
        editor_state.editor.open(path.clone()).unwrap();

        assert_eq!(
            blocking_open_file_lifecycle_message(&editor_state, &path, "renaming"),
            None
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn folder_lifecycle_blocks_clean_open_child_buffers() {
        let root = temp_root("folder-clean-child");
        let child_dir = root.join("docs");
        std::fs::create_dir_all(&child_dir).unwrap();
        let path = child_dir.join("note.md");
        std::fs::write(&path, "saved").unwrap();

        let mut editor_state = EditorViewState::default();
        editor_state.editor.open(path).unwrap();

        let message = blocking_open_file_lifecycle_message(&editor_state, &child_dir, "deleting")
            .expect("folder delete should block open child buffers");

        assert_eq!(message, "Close open files inside docs before deleting it.");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn folder_lifecycle_prioritizes_dirty_child_buffer_message() {
        let root = temp_root("folder-dirty-child");
        let child_dir = root.join("docs");
        std::fs::create_dir_all(&child_dir).unwrap();
        let path = child_dir.join("note.md");
        std::fs::write(&path, "saved").unwrap();

        let mut editor_state = EditorViewState::default();
        editor_state.editor.open(path).unwrap();
        editor_state.editor.buffers[0].insert(Position::new(0, 0), "unsaved ");

        let message = blocking_open_file_lifecycle_message(&editor_state, &child_dir, "deleting")
            .expect("dirty child buffer should block folder lifecycle action");

        assert_eq!(message, "Save or close note.md before deleting it.");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn sidebar_entry_name_validation_rejects_path_like_names() {
        assert_eq!(validate_entry_name(" note.md "), Ok("note.md"));
        assert_eq!(
            validate_entry_name(""),
            Err("name cannot be empty".to_string())
        );
        assert_eq!(
            validate_entry_name("nested/file.md"),
            Err("name cannot contain path separators".to_string())
        );
        assert_eq!(
            validate_entry_name("nested\\file.md"),
            Err("name cannot contain path separators".to_string())
        );
        assert_eq!(
            validate_entry_name(".."),
            Err("name cannot be . or ..".to_string())
        );
    }

    #[test]
    fn create_sidebar_file_uses_create_new_semantics() {
        let root = temp_root("create-file");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let path = create_sidebar_entry(&root, "note.md", false).unwrap();
        assert!(path.exists());

        std::fs::write(&path, "keep me").unwrap();
        let err = create_sidebar_entry(&root, "note.md", false).unwrap_err();
        assert_eq!(err, "note.md already exists");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "keep me");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn create_sidebar_folder_rejects_existing_folder() {
        let root = temp_root("create-folder");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let path = create_sidebar_entry(&root, "docs", true).unwrap();
        assert!(path.is_dir());

        let err = create_sidebar_entry(&root, "docs", true).unwrap_err();
        assert_eq!(err, "docs already exists");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn rename_sidebar_file_remaps_clean_open_file() {
        let root = temp_root("rename-file");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("before.md");
        std::fs::write(&path, "saved").unwrap();

        let mut editor_state = EditorViewState::default();
        editor_state.editor.open(path.clone()).unwrap();

        let outcome = rename_sidebar_entry(&path, "after.md", &editor_state).unwrap();
        match outcome {
            RenameOutcome::Renamed {
                new_name,
                remap_open_file,
                ..
            } => {
                let new_path = root.join("after.md");
                assert_eq!(new_name, "after.md");
                assert_eq!(remap_open_file, Some((path.clone(), new_path.clone())));
                assert!(!path.exists());
                assert!(new_path.exists());
            }
            RenameOutcome::Unchanged(_) => panic!("rename should change the file"),
        }

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn rename_sidebar_file_rejects_sibling_collision() {
        let root = temp_root("rename-collision");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("before.md");
        let existing = root.join("after.md");
        std::fs::write(&path, "saved").unwrap();
        std::fs::write(&existing, "existing").unwrap();

        let editor_state = EditorViewState::default();
        let err = rename_sidebar_entry(&path, "after.md", &editor_state).unwrap_err();
        assert_eq!(err, "after.md already exists");
        assert!(path.exists());
        assert_eq!(std::fs::read_to_string(existing).unwrap(), "existing");

        let _ = std::fs::remove_dir_all(root);
    }

    fn temp_root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "llnzy-sidebar-file-modals-{}-{label}",
            std::process::id()
        ))
    }
}
