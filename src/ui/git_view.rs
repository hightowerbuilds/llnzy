use std::path::Path;

use super::git_state::{GitPanel, GitUiState};
use crate::git::{
    file_state_label, GitCommitNode, GitErrorKind, GitFileState, GitHeadState, GitSnapshot,
    GitStatusEntry,
};

const BG: egui::Color32 = egui::Color32::from_rgb(30, 31, 32);
const PANEL: egui::Color32 = egui::Color32::from_rgb(38, 39, 40);
const PANEL_DARK: egui::Color32 = egui::Color32::from_rgb(26, 27, 28);
const TEXT: egui::Color32 = egui::Color32::from_rgb(232, 235, 232);
const DIM: egui::Color32 = egui::Color32::from_rgb(150, 156, 152);
const GREEN: egui::Color32 = egui::Color32::from_rgb(106, 255, 144);
const RED: egui::Color32 = egui::Color32::from_rgb(255, 122, 122);
const BLUE: egui::Color32 = egui::Color32::from_rgb(115, 182, 255);
const YELLOW: egui::Color32 = egui::Color32::from_rgb(240, 205, 95);

pub(crate) fn render_git_view(ctx: &egui::Context, state: &mut GitUiState, project_root: &Path) {
    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(BG)
                .inner_margin(egui::Margin::same(12.0)),
        )
        .show(ctx, |ui| {
            render_git_view_ui(ui, state, project_root);
        });
}

pub(crate) fn render_git_view_ui(ui: &mut egui::Ui, state: &mut GitUiState, project_root: &Path) {
    state.poll();
    state.ensure_loaded(project_root);
    if matches!(state.active_panel, GitPanel::CommitLog) && state.detail_expanded {
        state.ensure_detail_loaded();
    }
    if state.loading || state.detail_loading {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(80));
    }

    let snapshot = state.snapshot.clone();
    render_header(ui, state, snapshot.as_ref());

    if let Some(error) = &state.error {
        render_empty_state(ui, state.error_kind, error, project_root);
        return;
    }

    let Some(snapshot) = snapshot else {
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
        GitPanel::CommitLog => render_commit_log_dashboard(&mut content_ui, state, &snapshot),
        GitPanel::Readme => render_readme_dashboard(&mut content_ui, state, &snapshot),
    }
}

fn render_header(ui: &mut egui::Ui, state: &mut GitUiState, snapshot: Option<&GitSnapshot>) {
    egui::Frame::none()
        .fill(PANEL_DARK)
        .inner_margin(egui::Margin::symmetric(12.0, 8.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                render_panel_button(ui, state, GitPanel::CommitLog, "Commit Log");
                render_panel_button(ui, state, GitPanel::Readme, "README");
                ui.add_space(10.0);
                if snapshot.is_some() {
                    ui.add(
                        egui::TextEdit::singleline(&mut state.filter)
                            .desired_width(260.0)
                            .hint_text("Search commits"),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let text = if state.loading {
                        "Refreshing"
                    } else {
                        "Refresh"
                    };
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new(text).size(13.0).color(TEXT))
                                .fill(egui::Color32::from_rgb(48, 50, 52))
                                .rounding(egui::Rounding::same(4.0)),
                        )
                        .clicked()
                    {
                        state.refresh();
                    }
                });
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
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

fn render_commit_log_dashboard(ui: &mut egui::Ui, state: &mut GitUiState, snapshot: &GitSnapshot) {
    let available_h = ui.available_height();
    let detail_h = 206.0_f32.min((available_h * 0.42).max(150.0));
    let top_height = (available_h - detail_h - 10.0).max(180.0);
    ui.horizontal(|ui| {
        ui.set_height(top_height);
        let worktree_width = 160.0_f32.min((ui.available_width() * 0.18).max(140.0));
        ui.allocate_ui_with_layout(
            egui::vec2(worktree_width, top_height),
            egui::Layout::top_down(egui::Align::Min),
            |ui| render_worktree_panel(ui, snapshot),
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
        |ui| render_detail_panel(ui, state, snapshot),
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

fn render_worktree_panel(ui: &mut egui::Ui, snapshot: &GitSnapshot) {
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
                );
                render_status_group(
                    ui,
                    "Staged",
                    snapshot
                        .status
                        .iter()
                        .filter(|e| e.index != GitFileState::Unmodified && !e.conflicted),
                );
                render_status_group(
                    ui,
                    "Unstaged",
                    snapshot
                        .status
                        .iter()
                        .filter(|e| e.worktree != GitFileState::Unmodified && !e.conflicted),
                );
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

fn render_status_group<'a>(
    ui: &mut egui::Ui,
    title: &str,
    entries: impl Iterator<Item = &'a GitStatusEntry>,
) {
    let entries: Vec<&GitStatusEntry> = entries.collect();
    if entries.is_empty() {
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
            wrapped_label(ui, &entry.path.display().to_string(), 12.0, TEXT)
                .on_hover_text(entry.path.display().to_string());
        });
        if let Some(old_path) = &entry.old_path {
            wrapped_label(ui, &format!("from {}", old_path.display()), 11.0, DIM);
        }
    }
}

fn render_log_panel(ui: &mut egui::Ui, state: &mut GitUiState, snapshot: &GitSnapshot) {
    panel_header(ui, "Commit Log");
    ui.add_space(6.0);
    let commits = filtered_commits(snapshot, &state.filter);

    if commits.is_empty() {
        muted(ui, "No commits");
        return;
    }

    egui::ScrollArea::vertical()
        .id_salt("git_log_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for commit in commits {
                let selected = state.selected_commit.as_deref() == Some(commit.oid.as_str());
                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 30.0),
                    egui::Sense::click(),
                );
                if response.clicked() {
                    state.select_commit(commit.oid.clone());
                }
                paint_commit_row(ui, rect, &commit, selected);
            }
        });
}

fn filtered_commits(snapshot: &GitSnapshot, filter: &str) -> Vec<GitCommitNode> {
    let filter = filter.trim().to_lowercase();
    snapshot
        .commits
        .iter()
        .filter(|commit| {
            filter.is_empty()
                || commit.summary.to_lowercase().contains(&filter)
                || commit.oid.contains(&filter)
                || commit.author_name.to_lowercase().contains(&filter)
                || commit
                    .refs
                    .iter()
                    .any(|r| r.to_lowercase().contains(&filter))
        })
        .cloned()
        .collect()
}

fn paint_commit_row(ui: &egui::Ui, rect: egui::Rect, commit: &GitCommitNode, selected: bool) {
    let painter = ui.painter();
    let fill = if selected {
        egui::Color32::from_rgb(48, 56, 52)
    } else {
        PANEL
    };
    painter.rect_filled(
        rect.shrink2(egui::vec2(0.0, 2.0)),
        egui::Rounding::same(3.0),
        fill,
    );

    let graph_width = 92.0;
    let lane_step = 12.0;
    let top = rect.top() + 2.0;
    let bottom = rect.bottom() - 2.0;
    let center_y = rect.center().y;
    for lane in &commit.active_lanes {
        let x = rect.left() + 10.0 + *lane as f32 * lane_step;
        painter.line_segment(
            [egui::pos2(x, top), egui::pos2(x, bottom)],
            egui::Stroke::new(1.0, egui::Color32::from_rgb(88, 110, 98)),
        );
    }
    for edge in &commit.edges {
        if edge.from_lane != edge.to_lane {
            let from_x = rect.left() + 10.0 + edge.from_lane as f32 * lane_step;
            let to_x = rect.left() + 10.0 + edge.to_lane as f32 * lane_step;
            painter.line_segment(
                [egui::pos2(from_x, center_y), egui::pos2(to_x, bottom)],
                egui::Stroke::new(1.0, egui::Color32::from_rgb(88, 110, 98)),
            );
        }
    }
    let dot_x = rect.left() + 10.0 + commit.lane as f32 * lane_step;
    painter.circle_filled(
        egui::pos2(dot_x, center_y),
        if selected { 4.5 } else { 3.6 },
        if selected { GREEN } else { BLUE },
    );

    let text_x = rect.left() + graph_width;
    painter.text(
        egui::pos2(text_x, center_y),
        egui::Align2::LEFT_CENTER,
        &commit.short_oid,
        egui::FontId::monospace(12.0),
        DIM,
    );
    let summary_x = text_x + 62.0;
    let meta_x = (rect.right() - 150.0).max(summary_x + 120.0);
    let summary_chars = (((meta_x - summary_x - 10.0) / 7.0).max(12.0)) as usize;
    painter.text(
        egui::pos2(summary_x, center_y),
        egui::Align2::LEFT_CENTER,
        truncate(&commit.summary, summary_chars),
        egui::FontId::proportional(13.0),
        TEXT,
    );
    let meta = format!("{}  {}", commit.author_name, commit.relative_time);
    painter.text(
        egui::pos2(meta_x, center_y),
        egui::Align2::LEFT_CENTER,
        truncate(&meta, 28),
        egui::FontId::proportional(11.0),
        DIM,
    );
    let mut ref_x = summary_x + 360.0;
    if ref_x + 60.0 < meta_x {
        for label in commit.refs.iter().take(3) {
            let width = (label.len() as f32 * 6.2 + 12.0).min(140.0);
            if ref_x + width + 6.0 >= meta_x {
                break;
            }
            let pill = egui::Rect::from_min_size(
                egui::pos2(ref_x, center_y - 9.0),
                egui::vec2(width, 18.0),
            );
            painter.rect_filled(pill, egui::Rounding::same(3.0), egui::Color32::BLACK);
            painter.text(
                pill.center(),
                egui::Align2::CENTER_CENTER,
                truncate(label, 20),
                egui::FontId::proportional(10.5),
                GREEN,
            );
            ref_x += width + 5.0;
        }
    }
}

fn render_detail_panel(ui: &mut egui::Ui, state: &mut GitUiState, snapshot: &GitSnapshot) {
    panel_header(ui, "Commit Detail");
    ui.add_space(6.0);
    let Some(selected_oid) = state.selected_commit.clone() else {
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
        .unwrap_or_else(|| short_display(&selected_oid));

    egui::Frame::none()
        .fill(PANEL_DARK)
        .inner_margin(egui::Margin::symmetric(10.0, 7.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(truncate(&selected_summary, 110))
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
    let detail = state.selected_detail.clone();
    let Some(detail) = detail else {
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
                                ui.label(
                                    egui::RichText::new(path_display(&file.path))
                                        .size(12.0)
                                        .color(TEXT),
                                );
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

fn panel_header(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(12.0)
            .strong()
            .color(egui::Color32::from_rgb(190, 196, 192)),
    );
}

fn badge(ui: &mut egui::Ui, text: impl Into<String>, color: egui::Color32) {
    ui.add(
        egui::Label::new(
            egui::RichText::new(text.into())
                .size(11.0)
                .strong()
                .color(color),
        )
        .selectable(false),
    );
}

fn muted(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(12.0).color(DIM));
}

fn wrapped_label(ui: &mut egui::Ui, text: &str, size: f32, color: egui::Color32) -> egui::Response {
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
        GitHeadState::Detached => badge(ui, "DETACHED", YELLOW),
        GitHeadState::Unborn => badge(ui, "NO COMMITS", YELLOW),
        GitHeadState::Unknown | GitHeadState::Branch => {}
    }
    if state.is_shallow {
        badge(ui, "SHALLOW", YELLOW);
    }
    if state.is_large {
        badge(ui, "LARGE REPO", YELLOW);
    }
}

fn state_color(state: GitFileState) -> egui::Color32 {
    match state {
        GitFileState::Added | GitFileState::Untracked => GREEN,
        GitFileState::Modified | GitFileState::Renamed | GitFileState::TypeChanged => YELLOW,
        GitFileState::Deleted | GitFileState::Unknown => RED,
        GitFileState::Ignored | GitFileState::Unmodified => DIM,
    }
}

fn path_display(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| path.display().to_string())
}

fn short_display(oid: &str) -> String {
    oid.chars().take(7).collect()
}

fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max_chars.saturating_sub(3)).collect();
    out.push_str("...");
    out
}
