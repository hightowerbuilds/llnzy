use crate::editor::buffer::Buffer;
use crate::editor::file_watcher::FileChange;

use super::explorer_view::EditorViewState;

pub(super) fn poll_file_watcher(editor_state: &mut EditorViewState) {
    let Some(watcher) = &mut editor_state.file_watcher else {
        return;
    };

    for change in watcher.poll() {
        match change {
            FileChange::Modified(path) => {
                let Some(idx) = editor_state.editor.buffers.iter().position(|buffer| {
                    buffer.path().and_then(|p| p.canonicalize().ok()) == path.canonicalize().ok()
                }) else {
                    continue;
                };

                if editor_state.editor.buffers[idx].is_modified() {
                    editor_state.reload_prompt = Some((idx, path, false));
                } else {
                    reload_buffer(editor_state, idx, &path, "File reloaded (external change)");
                }
            }
            FileChange::Deleted(path) => {
                if let Some(idx) = editor_state.editor.buffers.iter().position(|buffer| {
                    buffer.path().and_then(|p| p.canonicalize().ok()) == path.canonicalize().ok()
                }) {
                    editor_state.reload_prompt = Some((idx, path, true));
                }
            }
        }
    }
}

pub(super) fn render_reload_prompt(ui: &mut egui::Ui, editor_state: &mut EditorViewState) {
    let Some((buf_idx, ref path, is_deleted)) = editor_state.reload_prompt.clone() else {
        return;
    };

    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let msg = if is_deleted {
        format!("\"{}\" has been deleted from disk.", file_name)
    } else {
        format!("\"{}\" was modified externally. Reload?", file_name)
    };

    let mut action: Option<bool> = None; // true = reload, false = keep
    egui::Window::new("External Change")
        .id(egui::Id::new("reload_prompt"))
        .fixed_pos(egui::pos2(
            ui.ctx().screen_rect().center().x - 160.0,
            ui.ctx().screen_rect().center().y - 40.0,
        ))
        .resizable(false)
        .show(ui.ctx(), |ui| {
            ui.label(
                egui::RichText::new(&msg)
                    .size(13.0)
                    .color(egui::Color32::from_rgb(210, 215, 225)),
            );
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if !is_deleted
                    && ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Reload")
                                    .size(12.0)
                                    .color(egui::Color32::WHITE),
                            )
                            .fill(egui::Color32::from_rgb(40, 100, 200)),
                        )
                        .clicked()
                {
                    action = Some(true);
                }
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Keep My Version")
                                .size(12.0)
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(50, 52, 62)),
                    )
                    .clicked()
                {
                    action = Some(false);
                }
            });
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                action = Some(false);
            }
        });

    if let Some(reload) = action {
        if reload && buf_idx < editor_state.editor.buffers.len() {
            reload_buffer(editor_state, buf_idx, path, "File reloaded");
        } else if is_deleted {
            editor_state.status_msg = Some(format!("File deleted: {}", file_name));
        }
        editor_state.reload_prompt = None;
    }
}

fn reload_buffer(
    editor_state: &mut EditorViewState,
    buf_idx: usize,
    path: &std::path::Path,
    status: &str,
) {
    if let Ok(new_buf) = Buffer::from_file(path) {
        editor_state.editor.buffers[buf_idx] = new_buf;
        editor_state.editor.views[buf_idx].tree_dirty = true;
        editor_state.status_msg = Some(status.to_string());
    }
}
