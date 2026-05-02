use super::explorer_view::EditorViewState;
use crate::explorer::ExplorerState;
use crate::path_utils::{path_contains, same_path};
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
        let new_name = rename_text.trim().to_string();
        let new_path = rename_path.parent().map(|p| p.join(&new_name));
        if let Some(new_path) = new_path {
            if let Some(message) =
                blocking_open_file_lifecycle_message(editor_state, &rename_path, "renaming")
            {
                editor_state.status_msg = Some(message);
            } else {
                match std::fs::rename(&rename_path, &new_path) {
                    Ok(_) => {
                        explorer.set_root(explorer.root.clone());
                        editor_state.pending_file_remap = Some((rename_path.clone(), new_path));
                        editor_state.status_msg = Some(format!("Renamed to {new_name}"));
                    }
                    Err(e) => editor_state.status_msg = Some(format!("Rename failed: {e}")),
                }
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
                    explorer.set_root(explorer.root.clone());
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
        let name = input_text.trim().to_string();
        let new_path = parent_dir.join(&name);
        let result = if is_folder {
            std::fs::create_dir_all(&new_path)
        } else {
            if let Some(p) = new_path.parent() {
                let _ = std::fs::create_dir_all(p);
            }
            std::fs::write(&new_path, "")
        };
        match result {
            Ok(_) => {
                explorer.set_root(explorer.root.clone());
                editor_state.status_msg = Some(format!("Created {name}"));
            }
            Err(e) => editor_state.status_msg = Some(format!("Create failed: {e}")),
        }
    } else if !cancel {
        editor_state.sidebar_new_entry = Some((parent_dir, input_text, is_folder));
    }
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

    fn temp_root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "llnzy-sidebar-file-modals-{}-{label}",
            std::process::id()
        ))
    }
}
