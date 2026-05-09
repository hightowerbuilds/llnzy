use std::path::Path;

use super::explorer_view::EditorViewState;
use super::git_state::{GitPanel, GitUiState};
use super::git_view_detail::render_detail_panel;
use super::git_view_log::render_log_panel;
use super::git_view_worktree::render_worktree_panel;
use crate::app::commands::AppCommand;
use crate::git::{GitErrorKind, GitFileState, GitHeadState, GitSnapshot};
use crate::path_utils::file_name_or_display;

pub(super) const PANEL: egui::Color32 = egui::Color32::from_rgb(38, 39, 40);
pub(super) const PANEL_DARK: egui::Color32 = egui::Color32::from_rgb(26, 27, 28);
pub(super) const TEXT: egui::Color32 = egui::Color32::from_rgb(232, 235, 232);
pub(super) const DIM: egui::Color32 = egui::Color32::from_rgb(150, 156, 152);
pub(super) const GREEN: egui::Color32 = egui::Color32::from_rgb(106, 255, 144);
pub(super) const RED: egui::Color32 = egui::Color32::from_rgb(255, 122, 122);
pub(super) const BLUE: egui::Color32 = egui::Color32::from_rgb(115, 182, 255);
pub(super) const YELLOW: egui::Color32 = egui::Color32::from_rgb(240, 205, 95);

pub(crate) fn render_git_view_ui(
    ui: &mut egui::Ui,
    state: &mut GitUiState,
    project_root: &Path,
    active_editor_file: Option<std::path::PathBuf>,
    editor_state: Option<&mut EditorViewState>,
    commands: &mut Vec<AppCommand>,
) {
    state.poll();
    state.set_active_editor_file(active_editor_file);
    state.ensure_loaded(project_root);
    if matches!(state.active_panel, GitPanel::CommitLog) && state.detail_expanded {
        state.ensure_detail_loaded();
    }
    if state.loading || state.detail_loading {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(80));
    }
    if state.watching_repo() {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(250));
    }

    render_header(ui, state);

    if let Some(error) = &state.error {
        render_empty_state(ui, state.error_kind, error, project_root);
        return;
    }

    let Some(snapshot) = state.snapshot.take() else {
        render_loading_state(ui, state, project_root);
        return;
    };

    ui.add_space(8.0);
    let content_rect = ui.available_rect_before_wrap();
    let mut content_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(content_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    content_ui.set_clip_rect(content_rect);
    match state.active_panel {
        GitPanel::CommitLog => {
            render_commit_log_dashboard(&mut content_ui, state, &snapshot, editor_state, commands)
        }
        GitPanel::Readme => render_readme_dashboard(&mut content_ui, state, &snapshot),
    }
    state.snapshot = Some(snapshot);
}

fn render_header(ui: &mut egui::Ui, state: &mut GitUiState) {
    egui::Frame::none()
        .fill(PANEL_DARK)
        .inner_margin(egui::Margin::symmetric(12.0, 8.0))
        .show(ui, |ui| {
            let has_snapshot = state.snapshot.is_some();
            ui.horizontal(|ui| {
                render_panel_button(ui, state, GitPanel::CommitLog, "Commit Log");
                render_panel_button(ui, state, GitPanel::Readme, "README");
                ui.add_space(10.0);
                if has_snapshot {
                    ui.add(
                        egui::TextEdit::singleline(&mut state.filter)
                            .desired_width(260.0)
                            .hint_text("Search commits"),
                    );
                    let mut all_branches = state.log_options.all_branches;
                    if ui.checkbox(&mut all_branches, "All branches").changed() {
                        state.set_all_branches(all_branches);
                    }
                    let mut first_parent = state.log_options.first_parent;
                    if ui.checkbox(&mut first_parent, "First parent").changed() {
                        state.set_first_parent(first_parent);
                    }
                    let mut active_file = state.active_file_history;
                    if ui.checkbox(&mut active_file, "Active file").changed() {
                        state.set_active_file_history(active_file);
                    }
                }
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let snapshot = state.snapshot.as_ref();
                let repo_name = snapshot
                    .and_then(|s| s.repo_root.file_name())
                    .and_then(|name| name.to_str())
                    .unwrap_or("Git");
                ui.label(
                    egui::RichText::new(repo_name)
                        .size(22.0)
                        .strong()
                        .color(TEXT),
                )
                .on_hover_text(
                    snapshot
                        .map(|s| s.repo_root.display().to_string())
                        .unwrap_or_default(),
                );
                ui.add_space(8.0);
                if let Some(snapshot) = snapshot {
                    badge(ui, branch_label(snapshot), BLUE);
                    render_repository_state_badges(ui, snapshot);
                    badge(
                        ui,
                        if snapshot.is_dirty { "DIRTY" } else { "CLEAN" },
                        if snapshot.is_dirty { YELLOW } else { GREEN },
                    );
                    if snapshot.ahead > 0 || snapshot.behind > 0 {
                        badge(
                            ui,
                            format!("+{} -{}", snapshot.ahead, snapshot.behind),
                            YELLOW,
                        );
                    }
                    badge(ui, format!("{} commits", snapshot.commits.len()), DIM);
                    if let Some(path) = &state.log_options.file_path {
                        badge(ui, format!("file {}", file_name_or_display(path)), BLUE);
                    }
                    if let Some(error) = &state.repo_watch_error {
                        badge(ui, "WATCH OFF", YELLOW).on_hover_text(error.as_str());
                    }
                }
            });
        });
}

fn render_panel_button(ui: &mut egui::Ui, state: &mut GitUiState, panel: GitPanel, label: &str) {
    let active = state.active_panel == panel;
    let fill = if active {
        egui::Color32::BLACK
    } else {
        egui::Color32::from_rgb(44, 46, 48)
    };
    let color = if active { GREEN } else { TEXT };
    let stroke = if active {
        egui::Stroke::new(1.0, GREEN)
    } else {
        egui::Stroke::new(1.0, egui::Color32::from_white_alpha(24))
    };
    if ui
        .add(
            egui::Button::new(egui::RichText::new(label).size(13.0).strong().color(color))
                .fill(fill)
                .stroke(stroke)
                .rounding(egui::Rounding::same(4.0))
                .min_size(egui::vec2(96.0, 30.0)),
        )
        .clicked()
    {
        state.active_panel = panel;
    }
}

fn render_empty_state(
    ui: &mut egui::Ui,
    error_kind: Option<GitErrorKind>,
    error: &str,
    project_root: &Path,
) {
    ui.add_space(18.0);
    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new(git_error_title(error_kind))
                .size(18.0)
                .color(TEXT),
        );
        ui.add_space(4.0);
        ui.label(egui::RichText::new(error).size(13.0).color(DIM));
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(project_root.display().to_string())
                .size(12.0)
                .color(DIM),
        );
    });
}

fn render_loading_state(ui: &mut egui::Ui, state: &GitUiState, project_root: &Path) {
    ui.add_space(18.0);
    ui.vertical(|ui| {
        let text = if state.loading { "Loading Git" } else { "Git" };
        ui.label(egui::RichText::new(text).size(18.0).color(TEXT));
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(project_root.display().to_string())
                .size(12.0)
                .color(DIM),
        );
    });
}

fn render_commit_log_dashboard(
    ui: &mut egui::Ui,
    state: &mut GitUiState,
    snapshot: &GitSnapshot,
    mut editor_state: Option<&mut EditorViewState>,
    commands: &mut Vec<AppCommand>,
) {
    let available_h = ui.available_height();
    let detail_h = 206.0_f32.min((available_h * 0.42).max(150.0));
    let top_height = (available_h - detail_h - 10.0).max(180.0);
    ui.horizontal(|ui| {
        ui.set_height(top_height);
        let worktree_width = 160.0_f32.min((ui.available_width() * 0.18).max(140.0));
        ui.allocate_ui_with_layout(
            egui::vec2(worktree_width, top_height),
            egui::Layout::top_down(egui::Align::Min),
            |ui| render_worktree_panel(ui, snapshot, editor_state.as_deref_mut(), commands),
        );
        ui.separator();
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), top_height),
            egui::Layout::top_down(egui::Align::Min),
            |ui| render_log_panel(ui, state, snapshot),
        );
    });
    ui.add_space(10.0);
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), ui.available_height().max(1.0)),
        egui::Layout::top_down(egui::Align::Min),
        |ui| render_detail_panel(ui, state, snapshot, editor_state, commands),
    );
}

fn render_readme_dashboard(ui: &mut egui::Ui, state: &mut GitUiState, snapshot: &GitSnapshot) {
    state.ensure_readme_loaded();
    panel_header(ui, "README");
    ui.add_space(8.0);
    egui::Frame::none()
        .fill(PANEL)
        .inner_margin(egui::Margin::same(12.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                badge(ui, branch_label(snapshot), BLUE);
                badge(ui, snapshot.repo_root.display().to_string(), DIM);
            });
            ui.add_space(8.0);
            if let Some(error) = &state.readme_error {
                ui.label(egui::RichText::new(error).size(13.0).color(DIM));
                return;
            }
            let Some(text) = &state.readme_text else {
                muted(ui, "Loading README");
                return;
            };
            egui::ScrollArea::vertical()
                .id_salt("git_readme_scroll")
                .auto_shrink([false, false])
                .max_height(ui.available_height().max(1.0))
                .show(ui, |ui| {
                    for line in text.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("# ") {
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new(trimmed.trim_start_matches("# "))
                                    .size(22.0)
                                    .strong()
                                    .color(TEXT),
                            );
                        } else if trimmed.starts_with("## ") {
                            ui.add_space(5.0);
                            ui.label(
                                egui::RichText::new(trimmed.trim_start_matches("## "))
                                    .size(17.0)
                                    .strong()
                                    .color(TEXT),
                            );
                        } else if trimmed.starts_with("```") {
                            ui.separator();
                        } else if trimmed.is_empty() {
                            ui.add_space(6.0);
                        } else {
                            ui.label(egui::RichText::new(line).size(12.0).monospace().color(TEXT));
                        }
                    }
                });
        });
}

fn git_error_title(error_kind: Option<GitErrorKind>) -> &'static str {
    match error_kind {
        Some(GitErrorKind::GitMissing) => "Git is unavailable",
        Some(GitErrorKind::NotRepository) => "No repository",
        Some(GitErrorKind::BareRepository) => "Bare repository",
        Some(GitErrorKind::CommandFailed) | None => "Git error",
    }
}

pub(super) fn panel_header(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(12.0)
            .strong()
            .color(egui::Color32::from_rgb(190, 196, 192)),
    );
}

pub(super) fn open_repo_file(
    snapshot: &GitSnapshot,
    path: &Path,
    editor_state: Option<&mut EditorViewState>,
    commands: &mut Vec<AppCommand>,
) {
    let Some(editor_state) = editor_state else {
        return;
    };
    let path = snapshot.repo_root.join(path);
    if !path.is_file() {
        return;
    }
    if let Ok(buffer_id) = editor_state.open_file(path.clone()) {
        commands.push(AppCommand::OpenCodeFile { path, buffer_id });
    }
}

pub(super) fn badge(
    ui: &mut egui::Ui,
    text: impl Into<String>,
    color: egui::Color32,
) -> egui::Response {
    ui.add(
        egui::Label::new(
            egui::RichText::new(text.into())
                .size(11.0)
                .strong()
                .color(color),
        )
        .selectable(false),
    )
}

pub(super) fn muted(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(12.0).color(DIM));
}

pub(super) fn wrapped_label(
    ui: &mut egui::Ui,
    text: &str,
    size: f32,
    color: egui::Color32,
) -> egui::Response {
    ui.add(
        egui::Label::new(egui::RichText::new(text).size(size).color(color))
            .wrap()
            .selectable(false),
    )
}

fn branch_label(snapshot: &GitSnapshot) -> String {
    snapshot
        .branch
        .clone()
        .or_else(|| {
            snapshot
                .head_oid
                .as_ref()
                .map(|oid| format!("detached {}", short_display(oid)))
        })
        .unwrap_or_else(|| "no HEAD".to_string())
}

fn render_repository_state_badges(ui: &mut egui::Ui, snapshot: &GitSnapshot) {
    let state = &snapshot.repository_state;
    match state.head {
        GitHeadState::Detached => {
            badge(ui, "DETACHED", YELLOW);
        }
        GitHeadState::Unborn => {
            badge(ui, "NO COMMITS", YELLOW);
        }
        GitHeadState::Unknown | GitHeadState::Branch => {}
    }
    if state.is_shallow {
        badge(ui, "SHALLOW", YELLOW);
    }
    if state.is_large {
        badge(ui, "LARGE REPO", YELLOW);
    }
}

pub(super) fn state_color(state: GitFileState) -> egui::Color32 {
    match state {
        GitFileState::Added | GitFileState::Untracked => GREEN,
        GitFileState::Modified | GitFileState::Renamed | GitFileState::TypeChanged => YELLOW,
        GitFileState::Deleted | GitFileState::Unknown => RED,
        GitFileState::Ignored | GitFileState::Unmodified => DIM,
    }
}

pub(super) fn short_display(oid: &str) -> String {
    oid.chars().take(7).collect()
}
