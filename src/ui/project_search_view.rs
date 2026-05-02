use std::path::{Path, PathBuf};
use std::time::Duration;

use super::explorer_view::EditorViewState;

pub(super) fn render_project_search(
    ui: &mut egui::Ui,
    editor_state: &mut EditorViewState,
    root: &Path,
) {
    if !editor_state.project_search.active {
        return;
    }

    editor_state.project_search.poll();
    if editor_state.project_search.is_searching() {
        ui.ctx().request_repaint_after(Duration::from_millis(50));
    }

    let mut navigate_to: Option<(PathBuf, usize, usize)> = None;
    let mut dismiss = false;
    let mut do_search = false;

    egui::Window::new("Project Search")
        .id(egui::Id::new("project_search_panel"))
        .fixed_pos(egui::pos2(80.0, 40.0))
        .default_size(egui::Vec2::new(550.0, 400.0))
        .resizable(true)
        .show(ui.ctx(), |ui| {
            ui.horizontal(|ui| {
                let mut query = editor_state.project_search.query.clone();
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut query)
                        .hint_text("Search in project...")
                        .desired_width((ui.available_width() - 100.0).max(80.0))
                        .text_color(egui::Color32::WHITE)
                        .font(egui::TextStyle::Monospace),
                );
                resp.request_focus();
                if query != editor_state.project_search.query {
                    editor_state.project_search.query = query;
                }

                let regex_bg = if editor_state.project_search.regex_mode {
                    egui::Color32::from_rgb(60, 100, 180)
                } else {
                    egui::Color32::from_rgb(50, 52, 62)
                };
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(".*")
                                .size(11.0)
                                .color(egui::Color32::WHITE),
                        )
                        .fill(regex_bg)
                        .min_size(egui::Vec2::new(28.0, 20.0)),
                    )
                    .clicked()
                {
                    editor_state.project_search.regex_mode =
                        !editor_state.project_search.regex_mode;
                }

                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Search")
                                .size(12.0)
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(40, 100, 200)),
                    )
                    .clicked()
                {
                    do_search = true;
                }
            });

            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                dismiss = true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                do_search = true;
            }

            ui.separator();

            if editor_state.project_search.is_searching() {
                ui.label(
                    egui::RichText::new("Searching...")
                        .size(12.0)
                        .color(egui::Color32::from_rgb(150, 155, 170)),
                );
            }

            if let Some(result) = &editor_state.project_search.result {
                ui.label(
                    egui::RichText::new(format!("{} matches", result.matches.len()))
                        .size(11.0)
                        .color(egui::Color32::from_rgb(150, 155, 170)),
                );

                let selected = editor_state.project_search.selected;
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        for (i, m) in result.matches.iter().enumerate() {
                            let bg = if i == selected {
                                egui::Color32::from_rgb(50, 80, 130)
                            } else {
                                egui::Color32::TRANSPARENT
                            };
                            let text_color = if i == selected {
                                egui::Color32::WHITE
                            } else {
                                egui::Color32::from_rgb(200, 205, 215)
                            };
                            let file_name =
                                m.path.file_name().and_then(|n| n.to_str()).unwrap_or("?");

                            egui::Frame::none()
                                .fill(bg)
                                .inner_margin(egui::Margin::symmetric(4.0, 1.0))
                                .show(ui, |ui| {
                                    let resp = ui
                                        .horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{}:{}",
                                                    file_name,
                                                    m.line + 1
                                                ))
                                                .size(11.0)
                                                .color(egui::Color32::from_rgb(100, 180, 255))
                                                .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(&m.line_text)
                                                    .size(11.0)
                                                    .color(text_color)
                                                    .monospace(),
                                            );
                                        })
                                        .response;
                                    if resp.interact(egui::Sense::click()).clicked() {
                                        navigate_to = Some((m.path.clone(), m.line, m.col));
                                    }
                                });
                        }
                    });
            }
        });

    let count = editor_state.project_search.match_count();
    if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
        editor_state.project_search.selected =
            (editor_state.project_search.selected + 1).min(count.saturating_sub(1));
    }
    if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
        editor_state.project_search.selected =
            editor_state.project_search.selected.saturating_sub(1);
    }

    if do_search {
        editor_state.project_search.search(root);
    }
    if dismiss {
        editor_state.project_search.close();
    }
    if let Some((path, line, col)) = navigate_to {
        editor_state.project_search.close();
        match editor_state.open_file(path) {
            Ok(buffer_id) => {
                let Some(idx) = editor_state.editor.index_for_id(buffer_id) else {
                    editor_state.status_msg =
                        Some("Opened search result buffer is missing".to_string());
                    return;
                };
                let view = &mut editor_state.editor.views[idx];
                view.cursor.pos = crate::editor::buffer::Position::new(line, col);
                view.cursor.clear_selection();
                view.cursor.desired_col = None;
                editor_state.status_msg = None;
            }
            Err(e) => editor_state.status_msg = Some(format!("Failed to open: {e}")),
        }
    }
}
