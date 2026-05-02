use std::path::PathBuf;

use super::explorer_view::EditorViewState;

pub(super) fn render_workspace_symbols_popup(
    ui: &mut egui::Ui,
    editor_state: &mut EditorViewState,
) {
    if editor_state.workspace_symbols_popup.is_none() {
        return;
    }

    let mut navigate_to: Option<(PathBuf, u32, u32)> = None;
    let mut dismiss = false;
    let mut query_changed = false;

    let symbols = editor_state.workspace_symbols_popup.as_ref().unwrap();
    let selected = editor_state.workspace_symbols_selected;

    egui::Window::new("Workspace Symbols")
        .id(egui::Id::new("workspace_symbols_panel"))
        .fixed_pos(egui::pos2(100.0, 40.0))
        .default_size(egui::Vec2::new(500.0, 350.0))
        .resizable(true)
        .show(ui.ctx(), |ui| {
            let mut query = editor_state.workspace_symbols_query.clone();
            let resp = ui.add(
                egui::TextEdit::singleline(&mut query)
                    .hint_text("Search symbols...")
                    .desired_width((ui.available_width() - 10.0).max(80.0))
                    .text_color(egui::Color32::WHITE)
                    .font(egui::TextStyle::Monospace),
            );
            resp.request_focus();
            if query != editor_state.workspace_symbols_query {
                editor_state.workspace_symbols_query = query;
                query_changed = true;
            }

            let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
            if escape {
                dismiss = true;
            }
            if enter && !symbols.is_empty() {
                let s = &symbols[selected];
                navigate_to = Some((s.path.clone(), s.line, s.col));
            }

            ui.separator();
            ui.label(
                egui::RichText::new(format!("{} symbols", symbols.len()))
                    .size(11.0)
                    .color(egui::Color32::from_rgb(150, 155, 170)),
            );

            egui::ScrollArea::vertical()
                .max_height(280.0)
                .show(ui, |ui| {
                    for (i, s) in symbols.iter().enumerate() {
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
                        let file_name = s.path.file_name().and_then(|n| n.to_str()).unwrap_or("?");

                        egui::Frame::none()
                            .fill(bg)
                            .inner_margin(egui::Margin::symmetric(4.0, 2.0))
                            .show(ui, |ui| {
                                let resp = ui
                                    .horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(&s.name)
                                                .size(12.0)
                                                .color(text_color)
                                                .monospace(),
                                        );
                                        ui.label(
                                            egui::RichText::new(&s.kind)
                                                .size(10.0)
                                                .color(egui::Color32::from_rgb(120, 130, 160)),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{}:{}",
                                                file_name,
                                                s.line + 1
                                            ))
                                            .size(10.0)
                                            .color(egui::Color32::from_rgb(100, 105, 120)),
                                        );
                                    })
                                    .response;
                                if resp.interact(egui::Sense::click()).clicked() {
                                    navigate_to = Some((s.path.clone(), s.line, s.col));
                                }
                            });
                    }
                });
        });

    let syms_len = editor_state
        .workspace_symbols_popup
        .as_ref()
        .map_or(0, |s| s.len());
    let down_pressed = ui.input(|i| i.key_pressed(egui::Key::ArrowDown));
    let up_pressed = ui.input(|i| i.key_pressed(egui::Key::ArrowUp));
    if down_pressed {
        editor_state.workspace_symbols_selected =
            (editor_state.workspace_symbols_selected + 1).min(syms_len.saturating_sub(1));
    }
    if up_pressed {
        editor_state.workspace_symbols_selected =
            editor_state.workspace_symbols_selected.saturating_sub(1);
    }

    if query_changed {
        let query = editor_state.workspace_symbols_query.clone();
        editor_state.request_workspace_symbols(&query);
        editor_state.workspace_symbols_selected = 0;
    }

    if dismiss {
        editor_state.workspace_symbols_popup = None;
    }
    if let Some((path, line, col)) = navigate_to {
        editor_state.workspace_symbols_popup = None;
        match editor_state.open_file(path) {
            Ok(idx) => {
                let view = &mut editor_state.editor.views[idx];
                view.cursor.pos = crate::editor::buffer::Position::new(line as usize, col as usize);
                view.cursor.clear_selection();
                view.cursor.desired_col = None;
                editor_state.status_msg = None;
            }
            Err(e) => editor_state.status_msg = Some(format!("Failed to open symbol: {e}")),
        }
    }
}

pub(super) fn render_references_popup(ui: &mut egui::Ui, editor_state: &mut EditorViewState) {
    if editor_state.references_popup.is_none() {
        return;
    }

    let mut navigate_to: Option<(PathBuf, u32, u32)> = None;
    let mut dismiss = false;

    let refs = editor_state.references_popup.as_ref().unwrap();
    let selected = editor_state.references_selected;

    egui::Window::new("References")
        .id(egui::Id::new("references_panel"))
        .fixed_pos(egui::pos2(100.0, 50.0))
        .default_size(egui::Vec2::new(500.0, 300.0))
        .resizable(true)
        .show(ui.ctx(), |ui| {
            ui.label(
                egui::RichText::new(format!("{} references", refs.len()))
                    .size(13.0)
                    .color(egui::Color32::WHITE),
            );
            ui.separator();

            let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
            let _down = ui.input(|i| i.key_pressed(egui::Key::ArrowDown));
            let _up = ui.input(|i| i.key_pressed(egui::Key::ArrowUp));

            if escape {
                dismiss = true;
            }
            if enter && !refs.is_empty() {
                let r = &refs[selected];
                navigate_to = Some((r.path.clone(), r.line, r.col));
            }

            egui::ScrollArea::vertical()
                .max_height(250.0)
                .show(ui, |ui| {
                    for (i, r) in refs.iter().enumerate() {
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
                        let file_name = r.path.file_name().and_then(|n| n.to_str()).unwrap_or("?");

                        egui::Frame::none()
                            .fill(bg)
                            .inner_margin(egui::Margin::symmetric(4.0, 2.0))
                            .show(ui, |ui| {
                                let resp = ui
                                    .horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{}:{}",
                                                file_name,
                                                r.line + 1
                                            ))
                                            .size(12.0)
                                            .color(egui::Color32::from_rgb(100, 180, 255))
                                            .monospace(),
                                        );
                                        ui.label(
                                            egui::RichText::new(&r.context)
                                                .size(12.0)
                                                .color(text_color)
                                                .monospace(),
                                        );
                                    })
                                    .response;
                                if resp.interact(egui::Sense::click()).clicked() {
                                    navigate_to = Some((r.path.clone(), r.line, r.col));
                                }
                            });
                    }
                });
        });

    let refs_len = editor_state
        .references_popup
        .as_ref()
        .map_or(0, |r| r.len());
    let down_pressed = ui.input(|i| i.key_pressed(egui::Key::ArrowDown));
    let up_pressed = ui.input(|i| i.key_pressed(egui::Key::ArrowUp));
    if down_pressed {
        editor_state.references_selected =
            (editor_state.references_selected + 1).min(refs_len.saturating_sub(1));
    }
    if up_pressed {
        editor_state.references_selected = editor_state.references_selected.saturating_sub(1);
    }

    if dismiss {
        editor_state.references_popup = None;
    }
    if let Some((path, line, col)) = navigate_to {
        editor_state.references_popup = None;
        match editor_state.open_file(path) {
            Ok(idx) => {
                let view = &mut editor_state.editor.views[idx];
                view.cursor.pos = crate::editor::buffer::Position::new(line as usize, col as usize);
                view.cursor.clear_selection();
                view.cursor.desired_col = None;
                editor_state.status_msg = None;
            }
            Err(e) => editor_state.status_msg = Some(format!("Failed to open reference: {e}")),
        }
    }
}
