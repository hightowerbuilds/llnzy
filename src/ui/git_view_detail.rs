use super::explorer_view::EditorViewState;
use super::git_state::GitUiState;
use super::git_view::{
    badge, muted, open_repo_file, panel_header, short_display, state_color, BLUE, DIM, GREEN,
    PANEL_DARK, RED, TEXT,
};
use crate::app::commands::AppCommand;
use crate::git::{file_state_label, GitSnapshot};
use crate::path_utils::file_name_or_display;
use crate::text_utils::truncate_chars;

pub(super) fn render_detail_panel(
    ui: &mut egui::Ui,
    state: &mut GitUiState,
    snapshot: &GitSnapshot,
    mut editor_state: Option<&mut EditorViewState>,
    commands: &mut Vec<AppCommand>,
) {
    panel_header(ui, "Commit Detail");
    ui.add_space(6.0);
    let Some(selected_oid) = state.selected_commit.as_deref() else {
        if snapshot.commits.is_empty() {
            muted(ui, "No commits yet");
        } else {
            muted(ui, "Select a commit");
        }
        return;
    };
    let selected_summary = snapshot
        .commits
        .iter()
        .find(|commit| commit.oid == selected_oid)
        .map(|commit| {
            format!(
                "{}  {}  {}",
                commit.short_oid, commit.summary, commit.relative_time
            )
        })
        .unwrap_or_else(|| short_display(selected_oid));

    egui::Frame::none()
        .fill(PANEL_DARK)
        .inner_margin(egui::Margin::symmetric(10.0, 7.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(truncate_chars(&selected_summary, 110).as_ref())
                        .size(13.0)
                        .color(TEXT),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let label = if state.detail_expanded {
                        "Hide details"
                    } else {
                        "Show details"
                    };
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(label).size(12.0).strong().color(GREEN),
                            )
                            .fill(egui::Color32::BLACK)
                            .rounding(egui::Rounding::same(3.0)),
                        )
                        .clicked()
                    {
                        state.detail_expanded = !state.detail_expanded;
                        if state.detail_expanded {
                            state.ensure_detail_loaded();
                            ui.ctx().request_repaint();
                        }
                    }
                });
            });
        });

    if !state.detail_expanded {
        return;
    }

    ui.add_space(6.0);
    if state.detail_loading {
        muted(ui, "Loading commit");
        return;
    }
    if let Some(error) = &state.detail_error {
        ui.label(egui::RichText::new(error).size(13.0).color(RED));
        return;
    }
    let Some(detail) = state.selected_detail.as_ref() else {
        muted(ui, "Open detail is loading");
        return;
    };

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(detail.subject.as_str())
                .size(16.0)
                .strong()
                .color(TEXT),
        );
        badge(ui, short_display(detail.oid.as_str()), BLUE);
    });
    ui.horizontal(|ui| {
        muted(ui, detail.author.as_str());
        muted(ui, detail.author_date.as_str());
    });
    if !detail.body.is_empty() {
        ui.label(
            egui::RichText::new(detail.body.as_str())
                .size(12.0)
                .color(DIM),
        );
    }

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(310.0, 180.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                panel_header(ui, "Files");
                egui::ScrollArea::vertical()
                    .id_salt("git_detail_files")
                    .show(ui, |ui| {
                        for file in &detail.files {
                            ui.horizontal(|ui| {
                                badge(ui, file_state_label(file.status), state_color(file.status));
                                let response = ui
                                    .selectable_label(
                                        false,
                                        file_name_or_display(&file.path).as_ref(),
                                    )
                                    .on_hover_text(file.path.display().to_string());
                                if response.clicked() {
                                    open_repo_file(
                                        snapshot,
                                        &file.path,
                                        editor_state.as_deref_mut(),
                                        commands,
                                    );
                                }
                            });
                        }
                    });
            },
        );
        ui.separator();
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), 180.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                panel_header(ui, "Patch");
                egui::ScrollArea::vertical()
                    .id_salt("git_patch_scroll")
                    .show(ui, |ui| {
                        for line in detail.patch.lines().take(220) {
                            let color = if line.starts_with('+') && !line.starts_with("+++") {
                                GREEN
                            } else if line.starts_with('-') && !line.starts_with("---") {
                                RED
                            } else if line.starts_with("@@") {
                                BLUE
                            } else {
                                DIM
                            };
                            ui.label(
                                egui::RichText::new(line)
                                    .size(11.0)
                                    .monospace()
                                    .color(color),
                            );
                        }
                    });
            },
        );
    });
}
