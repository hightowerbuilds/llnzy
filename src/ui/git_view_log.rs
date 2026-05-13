use super::git_state::{GitSelectionMove, GitUiState};
use super::git_view::{muted, panel_header, BLUE, DIM, GREEN, PANEL, TEXT};
use crate::git::{GitCommitNode, GitSnapshot};
use crate::text_utils::{contains_case_insensitive, truncate_chars};

pub(super) fn render_log_panel(ui: &mut egui::Ui, state: &mut GitUiState, snapshot: &GitSnapshot) {
    panel_header(ui, "Commit Log");
    ui.add_space(6.0);
    let commits = filtered_commits(snapshot, &state.filter);

    if commits.is_empty() {
        muted(ui, "No commits");
        return;
    }
    handle_log_keyboard(ui, state, &commits);

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
                paint_commit_row(ui, rect, commit, selected);
            }
        });
}

fn handle_log_keyboard(ui: &mut egui::Ui, state: &mut GitUiState, commits: &[&GitCommitNode]) {
    let mut movement = None;
    let mut toggle_detail = false;
    ui.input(|input| {
        if input.key_pressed(egui::Key::ArrowUp) {
            movement = Some(GitSelectionMove::Previous);
        }
        if input.key_pressed(egui::Key::ArrowDown) {
            movement = Some(GitSelectionMove::Next);
        }
        if input.key_pressed(egui::Key::Home) {
            movement = Some(GitSelectionMove::First);
        }
        if input.key_pressed(egui::Key::End) {
            movement = Some(GitSelectionMove::Last);
        }
        if input.key_pressed(egui::Key::Enter) || input.key_pressed(egui::Key::Space) {
            toggle_detail = true;
        }
    });

    if let Some(movement) = movement {
        if let Some(oid) =
            commit_oid_after_move(commits, state.selected_commit.as_deref(), movement)
        {
            state.select_commit(oid.to_string());
        }
    }
    if toggle_detail {
        state.detail_expanded = !state.detail_expanded;
    }
}

fn commit_oid_after_move<'a>(
    commits: &'a [&GitCommitNode],
    selected_commit: Option<&str>,
    movement: GitSelectionMove,
) -> Option<&'a str> {
    if commits.is_empty() {
        return None;
    }

    let current = selected_commit
        .and_then(|selected| commits.iter().position(|commit| commit.oid == selected));
    let next = match (movement, current) {
        (GitSelectionMove::First, _) => 0,
        (GitSelectionMove::Last, _) => commits.len() - 1,
        (GitSelectionMove::Previous, Some(idx)) => idx.saturating_sub(1),
        (GitSelectionMove::Next, Some(idx)) => (idx + 1).min(commits.len() - 1),
        (GitSelectionMove::Previous | GitSelectionMove::Next, None) => 0,
    };
    Some(commits[next].oid.as_str())
}

fn filtered_commits<'a>(snapshot: &'a GitSnapshot, filter: &str) -> Vec<&'a GitCommitNode> {
    let filter = filter.trim();
    snapshot
        .commits
        .iter()
        .filter(|commit| {
            filter.is_empty()
                || contains_case_insensitive(&commit.summary, filter)
                || commit.oid.contains(filter)
                || contains_case_insensitive(&commit.author_name, filter)
                || commit
                    .refs
                    .iter()
                    .any(|r| contains_case_insensitive(r, filter))
        })
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
        truncate_chars(&commit.summary, summary_chars).as_ref(),
        egui::FontId::proportional(13.0),
        TEXT,
    );
    let meta = format!("{}  {}", commit.author_name, commit.relative_time);
    painter.text(
        egui::pos2(meta_x, center_y),
        egui::Align2::LEFT_CENTER,
        truncate_chars(&meta, 28).as_ref(),
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
                truncate_chars(label, 20).as_ref(),
                egui::FontId::proportional(10.5),
                GREEN,
            );
            ref_x += width + 5.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::git_view::short_display;
    use super::*;

    fn commit(oid: &str, summary: &str, author: &str) -> GitCommitNode {
        GitCommitNode {
            oid: oid.to_string(),
            short_oid: short_display(oid),
            summary: summary.to_string(),
            author_name: author.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn filtered_commits_borrows_matching_commits() {
        let snapshot = GitSnapshot {
            commits: vec![
                commit("abc1234", "Add command palette", "Ada"),
                commit("def5678", "Fix terminal resize", "Linus"),
            ],
            ..Default::default()
        };

        let matches = filtered_commits(&snapshot, "terminal");

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].oid, "def5678");
    }

    #[test]
    fn commit_oid_after_move_tracks_filtered_commit_refs() {
        let snapshot = GitSnapshot {
            commits: vec![
                commit("aaa1111", "First", "Ada"),
                commit("bbb2222", "Second", "Ada"),
                commit("ccc3333", "Third", "Ada"),
            ],
            ..Default::default()
        };
        let commits = snapshot.commits.iter().collect::<Vec<_>>();

        assert_eq!(
            commit_oid_after_move(&commits, Some("bbb2222"), GitSelectionMove::Next),
            Some("ccc3333")
        );
        assert_eq!(
            commit_oid_after_move(&commits, Some("bbb2222"), GitSelectionMove::Previous),
            Some("aaa1111")
        );
    }
}
