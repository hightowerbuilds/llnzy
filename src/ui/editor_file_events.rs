use crate::editor::buffer::Buffer;
use crate::editor::file_watcher::FileChange;
use crate::editor::BufferId;
use crate::path_utils::same_path;

use super::explorer_view::EditorViewState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ExternalFileDecision {
    NoAction,
    ReloadCleanModified,
    PromptDirtyModified,
    PromptDeleted,
    PromptMoved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ExternalFileLifecycleState {
    CleanCurrent,
    CleanExternallyChanged,
    DirtyExternallyChanged,
    DeletedClean,
    DeletedDirty,
    MovedOnDiskClean,
    MovedOnDiskDirty,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ExternalFilePrompt {
    buffer_id: BufferId,
    path: std::path::PathBuf,
    moved_to: Option<std::path::PathBuf>,
    kind: ExternalFilePromptKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExternalFilePromptKind {
    Modified,
    Deleted,
    Moved,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ExternalPromptAction {
    NoAction,
    Reload(std::path::PathBuf),
    Remap {
        old_path: std::path::PathBuf,
        new_path: std::path::PathBuf,
    },
    Status(String),
    Stale,
}

pub(super) fn external_file_lifecycle_state(
    change: Option<&FileChange>,
    buffer_is_modified: bool,
) -> ExternalFileLifecycleState {
    match (change, buffer_is_modified) {
        (None, _) => ExternalFileLifecycleState::CleanCurrent,
        (Some(FileChange::Modified(_)), false) => {
            ExternalFileLifecycleState::CleanExternallyChanged
        }
        (Some(FileChange::Modified(_)), true) => ExternalFileLifecycleState::DirtyExternallyChanged,
        (Some(FileChange::Deleted(_)), false) => ExternalFileLifecycleState::DeletedClean,
        (Some(FileChange::Deleted(_)), true) => ExternalFileLifecycleState::DeletedDirty,
        (Some(FileChange::Moved { .. }), false) => ExternalFileLifecycleState::MovedOnDiskClean,
        (Some(FileChange::Moved { .. }), true) => ExternalFileLifecycleState::MovedOnDiskDirty,
    }
}

pub(super) fn plan_external_file_event(
    change: &FileChange,
    buffer_is_modified: bool,
) -> ExternalFileDecision {
    match external_file_lifecycle_state(Some(change), buffer_is_modified) {
        ExternalFileLifecycleState::CleanCurrent => ExternalFileDecision::NoAction,
        ExternalFileLifecycleState::CleanExternallyChanged => {
            ExternalFileDecision::ReloadCleanModified
        }
        ExternalFileLifecycleState::DirtyExternallyChanged => {
            ExternalFileDecision::PromptDirtyModified
        }
        ExternalFileLifecycleState::DeletedClean | ExternalFileLifecycleState::DeletedDirty => {
            ExternalFileDecision::PromptDeleted
        }
        ExternalFileLifecycleState::MovedOnDiskClean
        | ExternalFileLifecycleState::MovedOnDiskDirty => ExternalFileDecision::PromptMoved,
    }
}

pub(super) fn poll_file_watcher(editor_state: &mut EditorViewState) {
    let Some(watcher) = &mut editor_state.file_watcher else {
        return;
    };

    for change in watcher.poll() {
        match change {
            FileChange::Modified(path) => {
                let Some(idx) = find_buffer_index_for_path(editor_state, &path) else {
                    continue;
                };
                let Some(buffer_id) = editor_state.editor.buffer_id(idx) else {
                    continue;
                };
                if !editor_state.editor.buffers[idx].is_modified()
                    && buffer_matches_file_content(&editor_state.editor.buffers[idx], &path)
                {
                    continue;
                }

                match plan_external_file_event(
                    &FileChange::Modified(path.clone()),
                    editor_state.editor.buffers[idx].is_modified(),
                ) {
                    ExternalFileDecision::ReloadCleanModified => {
                        reload_buffer(editor_state, idx, &path, "File reloaded (external change)");
                    }
                    ExternalFileDecision::PromptDirtyModified => {
                        editor_state.reload_prompt = Some(ExternalFilePrompt {
                            buffer_id,
                            path,
                            moved_to: None,
                            kind: ExternalFilePromptKind::Modified,
                        });
                    }
                    ExternalFileDecision::NoAction
                    | ExternalFileDecision::PromptDeleted
                    | ExternalFileDecision::PromptMoved => {}
                }
            }
            FileChange::Deleted(path) => {
                let Some(idx) = find_buffer_index_for_path(editor_state, &path) else {
                    continue;
                };
                let Some(buffer_id) = editor_state.editor.buffer_id(idx) else {
                    continue;
                };

                if plan_external_file_event(
                    &FileChange::Deleted(path.clone()),
                    editor_state.editor.buffers[idx].is_modified(),
                ) == ExternalFileDecision::PromptDeleted
                {
                    editor_state.reload_prompt = Some(ExternalFilePrompt {
                        buffer_id,
                        path,
                        moved_to: None,
                        kind: ExternalFilePromptKind::Deleted,
                    });
                }
            }
            FileChange::Moved { from, to } => {
                let Some(idx) = find_buffer_index_for_path(editor_state, &from) else {
                    continue;
                };
                let Some(buffer_id) = editor_state.editor.buffer_id(idx) else {
                    continue;
                };

                if plan_external_file_event(
                    &FileChange::Moved {
                        from: from.clone(),
                        to: to.clone(),
                    },
                    editor_state.editor.buffers[idx].is_modified(),
                ) == ExternalFileDecision::PromptMoved
                {
                    editor_state.reload_prompt = Some(ExternalFilePrompt {
                        buffer_id,
                        path: from,
                        moved_to: to,
                        kind: ExternalFilePromptKind::Moved,
                    });
                }
            }
        }
    }
}

pub(super) fn render_reload_prompt(ui: &mut egui::Ui, editor_state: &mut EditorViewState) {
    let Some(prompt) = editor_state.reload_prompt.clone() else {
        return;
    };

    let is_deleted = prompt.kind == ExternalFilePromptKind::Deleted;
    let is_moved = prompt.kind == ExternalFilePromptKind::Moved;
    let path = &prompt.path;
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let msg = if is_deleted {
        format!("\"{}\" has been deleted from disk.", file_name)
    } else if is_moved {
        if let Some(to) = &prompt.moved_to {
            let to_name = to
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("a new path");
            format!("\"{}\" was moved on disk to \"{}\".", file_name, to_name)
        } else {
            format!("\"{}\" was moved on disk.", file_name)
        }
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
                if is_moved
                    && prompt.moved_to.is_some()
                    && ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Use Moved Path")
                                    .size(12.0)
                                    .color(egui::Color32::WHITE),
                            )
                            .fill(egui::Color32::from_rgb(40, 100, 200)),
                        )
                        .clicked()
                {
                    action = Some(true);
                }
                if !is_deleted
                    && !is_moved
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

    if let Some(accept_primary) = action {
        let target_current = prompt_target_index(editor_state, &prompt).is_some();
        match plan_external_prompt_response(&prompt, accept_primary, target_current, file_name) {
            ExternalPromptAction::NoAction => {}
            ExternalPromptAction::Reload(path) => {
                if let Some(buf_idx) = prompt_target_index(editor_state, &prompt) {
                    reload_buffer(editor_state, buf_idx, &path, "File reloaded");
                } else {
                    editor_state.status_msg = Some("External change no longer applies".to_string());
                }
            }
            ExternalPromptAction::Remap { old_path, new_path } => {
                editor_state.pending_file_remap = Some((old_path, new_path));
                editor_state.status_msg = Some(format!("Tracking moved file: {}", file_name));
            }
            ExternalPromptAction::Status(message) => {
                editor_state.status_msg = Some(message);
            }
            ExternalPromptAction::Stale => {
                editor_state.status_msg = Some("External change no longer applies".to_string());
            }
        }
        editor_state.reload_prompt = None;
    }
}

fn plan_external_prompt_response(
    prompt: &ExternalFilePrompt,
    accept_primary: bool,
    target_current: bool,
    file_name: &str,
) -> ExternalPromptAction {
    if accept_primary && !target_current {
        return ExternalPromptAction::Stale;
    }

    match (accept_primary, prompt.kind) {
        (true, ExternalFilePromptKind::Modified) => {
            ExternalPromptAction::Reload(prompt.path.clone())
        }
        (true, ExternalFilePromptKind::Moved) => prompt
            .moved_to
            .clone()
            .map(|new_path| ExternalPromptAction::Remap {
                old_path: prompt.path.clone(),
                new_path,
            })
            .unwrap_or(ExternalPromptAction::NoAction),
        (false, ExternalFilePromptKind::Deleted) => {
            ExternalPromptAction::Status(format!("File deleted: {}", file_name))
        }
        (false, ExternalFilePromptKind::Moved) => {
            ExternalPromptAction::Status(format!("File moved: {}", file_name))
        }
        _ => ExternalPromptAction::NoAction,
    }
}

fn find_buffer_index_for_path(
    editor_state: &EditorViewState,
    path: &std::path::Path,
) -> Option<usize> {
    editor_state.editor.buffers.iter().position(|buffer| {
        buffer
            .path()
            .is_some_and(|buffer_path| same_path(buffer_path, path))
    })
}

fn prompt_target_index(
    editor_state: &EditorViewState,
    prompt: &ExternalFilePrompt,
) -> Option<usize> {
    let idx = editor_state.editor.index_for_id(prompt.buffer_id)?;
    let buffer_path = editor_state.editor.buffers.get(idx)?.path()?;
    same_path(buffer_path, &prompt.path).then_some(idx)
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

fn buffer_matches_file_content(buffer: &Buffer, path: &std::path::Path) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    text.replace("\r\n", "\n") == buffer.text()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_external_modification_reloads_without_prompt() {
        let change = FileChange::Modified(std::path::PathBuf::from("/tmp/file.rs"));

        assert_eq!(
            external_file_lifecycle_state(Some(&change), false),
            ExternalFileLifecycleState::CleanExternallyChanged
        );
        assert_eq!(
            plan_external_file_event(&change, false),
            ExternalFileDecision::ReloadCleanModified
        );
    }

    #[test]
    fn matching_file_content_does_not_require_reload() {
        let path = temp_file_path("matching-content");
        std::fs::write(&path, "hello\nworld").unwrap();
        let buffer = Buffer::from_file(&path).unwrap();

        assert!(buffer_matches_file_content(&buffer, &path));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn different_file_content_requires_reload() {
        let path = temp_file_path("different-content");
        std::fs::write(&path, "hello").unwrap();
        let buffer = Buffer::from_file(&path).unwrap();
        std::fs::write(&path, "changed").unwrap();

        assert!(!buffer_matches_file_content(&buffer, &path));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn dirty_external_modification_prompts_before_reload() {
        let change = FileChange::Modified(std::path::PathBuf::from("/tmp/file.rs"));

        assert_eq!(
            external_file_lifecycle_state(Some(&change), true),
            ExternalFileLifecycleState::DirtyExternallyChanged
        );
        assert_eq!(
            plan_external_file_event(&change, true),
            ExternalFileDecision::PromptDirtyModified
        );
    }

    #[test]
    fn deleted_file_prompts_even_when_clean() {
        let change = FileChange::Deleted(std::path::PathBuf::from("/tmp/file.rs"));

        assert_eq!(
            external_file_lifecycle_state(Some(&change), false),
            ExternalFileLifecycleState::DeletedClean
        );
        assert_eq!(
            external_file_lifecycle_state(Some(&change), true),
            ExternalFileLifecycleState::DeletedDirty
        );
        assert_eq!(
            plan_external_file_event(&change, false),
            ExternalFileDecision::PromptDeleted
        );
        assert_eq!(
            plan_external_file_event(&change, true),
            ExternalFileDecision::PromptDeleted
        );
    }

    #[test]
    fn clean_current_state_has_no_action() {
        assert_eq!(
            external_file_lifecycle_state(None, false),
            ExternalFileLifecycleState::CleanCurrent
        );
        assert_eq!(
            external_file_lifecycle_state(None, true),
            ExternalFileLifecycleState::CleanCurrent
        );
    }

    #[test]
    fn moved_file_prompts_as_moved_lifecycle() {
        let change = FileChange::Moved {
            from: std::path::PathBuf::from("/tmp/file.rs"),
            to: Some(std::path::PathBuf::from("/tmp/moved.rs")),
        };

        assert_eq!(
            external_file_lifecycle_state(Some(&change), false),
            ExternalFileLifecycleState::MovedOnDiskClean
        );
        assert_eq!(
            external_file_lifecycle_state(Some(&change), true),
            ExternalFileLifecycleState::MovedOnDiskDirty
        );
        assert_eq!(
            plan_external_file_event(&change, false),
            ExternalFileDecision::PromptMoved
        );
        assert_eq!(
            plan_external_file_event(&change, true),
            ExternalFileDecision::PromptMoved
        );
    }

    #[test]
    fn prompt_target_follows_buffer_identity_after_index_changes() {
        let first_path = temp_file_path("first");
        let second_path = temp_file_path("second");
        std::fs::write(&first_path, "first").unwrap();
        std::fs::write(&second_path, "second").unwrap();

        let mut editor_state = EditorViewState::default();
        let _first_id = editor_state.editor.open(first_path.clone()).unwrap();
        let second_id = editor_state.editor.open(second_path.clone()).unwrap();
        let prompt = ExternalFilePrompt {
            buffer_id: second_id,
            path: second_path.clone(),
            moved_to: None,
            kind: ExternalFilePromptKind::Modified,
        };

        assert_eq!(prompt_target_index(&editor_state, &prompt), Some(1));

        assert!(editor_state.editor.close(0));

        assert_eq!(prompt_target_index(&editor_state, &prompt), Some(0));

        let _ = std::fs::remove_file(first_path);
        let _ = std::fs::remove_file(second_path);
    }

    #[test]
    fn prompt_target_rejects_stale_path_after_buffer_remap() {
        let old_path = temp_file_path("old-path");
        let new_path = temp_file_path("new-path");
        std::fs::write(&old_path, "old").unwrap();
        std::fs::write(&new_path, "new").unwrap();

        let mut editor_state = EditorViewState::default();
        let buffer_id = editor_state.editor.open(old_path.clone()).unwrap();
        let prompt = ExternalFilePrompt {
            buffer_id,
            path: old_path.clone(),
            moved_to: None,
            kind: ExternalFilePromptKind::Modified,
        };

        assert_eq!(prompt_target_index(&editor_state, &prompt), Some(0));

        assert!(editor_state.editor.update_path(buffer_id, new_path.clone()));

        assert_eq!(prompt_target_index(&editor_state, &prompt), None);

        let _ = std::fs::remove_file(old_path);
        let _ = std::fs::remove_file(new_path);
    }

    #[test]
    fn prompt_response_reloads_current_modified_prompt() {
        let path = temp_file_path("reload-action");
        let prompt = prompt(
            test_buffer_id("reload-action-id"),
            path.clone(),
            None,
            ExternalFilePromptKind::Modified,
        );

        assert_eq!(
            plan_external_prompt_response(&prompt, true, true, "file.rs"),
            ExternalPromptAction::Reload(path)
        );
    }

    #[test]
    fn prompt_response_remaps_detected_move() {
        let old_path = temp_file_path("move-old");
        let new_path = temp_file_path("move-new");
        let prompt = prompt(
            test_buffer_id("move-action-id"),
            old_path.clone(),
            Some(new_path.clone()),
            ExternalFilePromptKind::Moved,
        );

        assert_eq!(
            plan_external_prompt_response(&prompt, true, true, "file.rs"),
            ExternalPromptAction::Remap { old_path, new_path }
        );
    }

    #[test]
    fn prompt_response_rejects_stale_primary_action() {
        let path = temp_file_path("stale-action");
        let prompt = prompt(
            test_buffer_id("stale-action-id"),
            path,
            None,
            ExternalFilePromptKind::Modified,
        );

        assert_eq!(
            plan_external_prompt_response(&prompt, true, false, "file.rs"),
            ExternalPromptAction::Stale
        );
    }

    #[test]
    fn prompt_response_reports_deleted_or_moved_keep_action() {
        let path = temp_file_path("keep-action");
        let deleted = prompt(
            test_buffer_id("deleted-action-id"),
            path.clone(),
            None,
            ExternalFilePromptKind::Deleted,
        );
        let moved = prompt(
            test_buffer_id("moved-action-id"),
            path,
            None,
            ExternalFilePromptKind::Moved,
        );

        assert_eq!(
            plan_external_prompt_response(&deleted, false, true, "file.rs"),
            ExternalPromptAction::Status("File deleted: file.rs".to_string())
        );
        assert_eq!(
            plan_external_prompt_response(&moved, false, true, "file.rs"),
            ExternalPromptAction::Status("File moved: file.rs".to_string())
        );
    }

    #[test]
    fn deleted_path_matching_does_not_treat_all_missing_paths_as_same_file() {
        let buffer_path = temp_file_path("missing-buffer");
        let event_path = temp_file_path("missing-event");

        assert!(same_path(&buffer_path, &buffer_path));
        assert!(!same_path(&buffer_path, &event_path));
    }

    fn temp_file_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "llnzy-file-events-{}-{label}.txt",
            std::process::id()
        ))
    }

    fn prompt(
        buffer_id: BufferId,
        path: std::path::PathBuf,
        moved_to: Option<std::path::PathBuf>,
        kind: ExternalFilePromptKind,
    ) -> ExternalFilePrompt {
        ExternalFilePrompt {
            buffer_id,
            path,
            moved_to,
            kind,
        }
    }

    fn test_buffer_id(label: &str) -> BufferId {
        let path = temp_file_path(label);
        std::fs::write(&path, "buffer").unwrap();
        let mut editor_state = EditorViewState::default();
        let buffer_id = editor_state.editor.open(path.clone()).unwrap();
        let _ = std::fs::remove_file(path);
        buffer_id
    }
}
