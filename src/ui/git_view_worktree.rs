use super::explorer_view::EditorViewState;
use super::git_view::{
    badge, muted, open_repo_file, panel_header, short_display, state_color, wrapped_label, BLUE,
    DIM, GREEN, TEXT, YELLOW,
};
use crate::app::commands::AppCommand;
use crate::git::{file_state_label, GitFileState, GitSnapshot, GitStatusEntry};
use crate::path_utils::file_name_or_display;

pub(super) fn render_worktree_panel(
    ui: &mut egui::Ui,
    snapshot: &GitSnapshot,
    mut editor_state: Option<&mut EditorViewState>,
    commands: &mut Vec<AppCommand>,
) {
    panel_header(ui, "Working Tree");
    ui.add_space(6.0);
    egui::ScrollArea::vertical()
        .id_salt("git_worktree_scroll")
        .auto_shrink([false, false])
        .max_height(ui.available_height().max(1.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if snapshot.status.is_empty() {
                ui.label(egui::RichText::new("Clean").size(13.0).color(GREEN));
                ui.add_space(14.0);
            } else {
                render_status_group(
                    ui,
                    "Conflicts",
                    snapshot.status.iter().filter(|e| e.conflicted),
                    snapshot,
                    reborrow_editor_state(&mut editor_state),
                    commands,
                );
                render_status_group(
                    ui,
                    "Staged",
                    snapshot
                        .status
                        .iter()
                        .filter(|e| e.index != GitFileState::Unmodified && !e.conflicted),
                    snapshot,
                    reborrow_editor_state(&mut editor_state),
                    commands,
                );
                render_status_group(
                    ui,
                    "Unstaged",
                    snapshot
                        .status
                        .iter()
                        .filter(|e| e.worktree != GitFileState::Unmodified && !e.conflicted),
                    snapshot,
                    reborrow_editor_state(&mut editor_state),
                    commands,
                );
            }

            ui.add_space(10.0);
            panel_header(ui, "Worktrees");
            if snapshot.worktrees.is_empty() {
                muted(ui, "None");
            } else {
                for worktree in snapshot.worktrees.iter().take(6) {
                    ui.horizontal_wrapped(|ui| {
                        badge(
                            ui,
                            worktree.branch.as_deref().unwrap_or(if worktree.detached {
                                "detached"
                            } else {
                                "worktree"
                            }),
                            if worktree.detached { YELLOW } else { BLUE },
                        );
                        wrapped_label(ui, &worktree.path.display().to_string(), 12.0, TEXT);
                    });
                }
            }

            ui.add_space(10.0);
            panel_header(ui, "Stashes");
            if snapshot.stashes.is_empty() {
                muted(ui, "None");
            } else {
                for stash in snapshot.stashes.iter().take(5) {
                    ui.horizontal_wrapped(|ui| {
                        badge(ui, &stash.selector, BLUE);
                        wrapped_label(ui, &stash.message, 12.0, TEXT);
                    });
                }
            }

            ui.add_space(10.0);
            panel_header(ui, "Local Activity");
            if snapshot.reflog.is_empty() {
                muted(ui, "No reflog entries");
            } else {
                for entry in snapshot.reflog.iter().take(20) {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            egui::RichText::new(short_display(&entry.oid))
                                .size(11.0)
                                .monospace()
                                .color(DIM),
                        );
                        wrapped_label(ui, &entry.message, 12.0, TEXT);
                    });
                }
            }
        });
}

fn reborrow_editor_state<'a>(
    editor_state: &'a mut Option<&mut EditorViewState>,
) -> Option<&'a mut EditorViewState> {
    editor_state.as_mut().map(|state| &mut **state)
}

fn render_status_group<'a>(
    ui: &mut egui::Ui,
    title: &str,
    entries: impl Iterator<Item = &'a GitStatusEntry>,
    snapshot: &GitSnapshot,
    mut editor_state: Option<&mut EditorViewState>,
    commands: &mut Vec<AppCommand>,
) {
    let mut entries = entries.peekable();
    if entries.peek().is_none() {
        return;
    }
    ui.add_space(4.0);
    ui.label(egui::RichText::new(title).size(11.0).strong().color(DIM));
    for entry in entries {
        let state = if entry.conflicted {
            GitFileState::Unknown
        } else if entry.index != GitFileState::Unmodified {
            entry.index
        } else {
            entry.worktree
        };
        ui.horizontal_wrapped(|ui| {
            badge(ui, file_state_label(state), state_color(state));
            let response = ui
                .selectable_label(false, file_name_or_display(&entry.path).as_ref())
                .on_hover_text(entry.path.display().to_string());
            if response.clicked() {
                open_repo_file(snapshot, &entry.path, editor_state.as_deref_mut(), commands);
            }
        });
        if let Some(old_path) = &entry.old_path {
            wrapped_label(ui, &format!("from {}", old_path.display()), 11.0, DIM);
        }
    }
}
