use crate::editor::search::EditorSearch;
use crate::editor::BufferView;

pub(super) fn render_search_bar(
    ui: &mut egui::Ui,
    search: &mut EditorSearch,
    buf: &mut crate::editor::buffer::Buffer,
    view: &mut BufferView,
) -> f32 {
    let bar_bg = egui::Color32::from_rgb(35, 37, 48);
    let btn_color = egui::Color32::from_rgb(100, 180, 255);
    let toggle_on = egui::Color32::from_rgb(60, 100, 180);
    let toggle_off = egui::Color32::from_rgb(50, 52, 62);
    let status_color = egui::Color32::from_rgb(150, 155, 170);

    let mut total_h = 0.0;

    let find_response = egui::Frame::none()
        .fill(bar_bg)
        .inner_margin(egui::Margin::symmetric(8.0, 4.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Find:")
                        .size(12.0)
                        .color(egui::Color32::from_rgb(180, 185, 200)),
                );

                let mut query = search.query.clone();
                let response = ui.add(
                    egui::TextEdit::singleline(&mut query)
                        .desired_width((ui.available_width() - 280.0).max(80.0))
                        .hint_text("Search...")
                        .text_color(egui::Color32::WHITE)
                        .font(egui::TextStyle::Monospace),
                );
                if !search.replace_mode || response.has_focus() {
                    response.request_focus();
                }
                if query != search.query {
                    search.query = query;
                    search.update_matches(buf);
                    search.focus_nearest(view.cursor.pos);
                }

                let case_label = if search.case_sensitive { "Aa" } else { "Aa" };
                let case_bg = if search.case_sensitive {
                    toggle_on
                } else {
                    toggle_off
                };
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(case_label)
                                .size(11.0)
                                .color(egui::Color32::WHITE),
                        )
                        .fill(case_bg)
                        .min_size(egui::Vec2::new(28.0, 20.0)),
                    )
                    .on_hover_text("Case sensitive")
                    .clicked()
                {
                    search.case_sensitive = !search.case_sensitive;
                    search.update_matches(buf);
                    search.focus_nearest(view.cursor.pos);
                }
                let word_bg = if search.whole_word {
                    toggle_on
                } else {
                    toggle_off
                };
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("W")
                                .size(11.0)
                                .color(egui::Color32::WHITE),
                        )
                        .fill(word_bg)
                        .min_size(egui::Vec2::new(28.0, 20.0)),
                    )
                    .on_hover_text("Whole word")
                    .clicked()
                {
                    search.whole_word = !search.whole_word;
                    search.update_matches(buf);
                    search.focus_nearest(view.cursor.pos);
                }
                let regex_bg = if search.regex_mode {
                    toggle_on
                } else {
                    toggle_off
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
                    .on_hover_text("Regex")
                    .clicked()
                {
                    search.regex_mode = !search.regex_mode;
                    search.update_matches(buf);
                    search.focus_nearest(view.cursor.pos);
                }

                ui.label(
                    egui::RichText::new(search.status())
                        .size(11.0)
                        .color(status_color),
                );

                if ui
                    .add(
                        egui::Button::new(egui::RichText::new("<").size(12.0).color(btn_color))
                            .fill(egui::Color32::TRANSPARENT)
                            .min_size(egui::Vec2::new(24.0, 20.0)),
                    )
                    .on_hover_text("Previous (Shift+Enter)")
                    .clicked()
                {
                    if let Some(pos) = search.prev() {
                        view.cursor.pos = pos;
                        view.cursor.clear_selection();
                        view.cursor.desired_col = None;
                    }
                }
                if ui
                    .add(
                        egui::Button::new(egui::RichText::new(">").size(12.0).color(btn_color))
                            .fill(egui::Color32::TRANSPARENT)
                            .min_size(egui::Vec2::new(24.0, 20.0)),
                    )
                    .on_hover_text("Next (Enter)")
                    .clicked()
                {
                    if let Some(pos) = search.next() {
                        view.cursor.pos = pos;
                        view.cursor.clear_selection();
                        view.cursor.desired_col = None;
                    }
                }

                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("x")
                                .size(12.0)
                                .color(egui::Color32::from_rgb(180, 180, 190)),
                        )
                        .fill(egui::Color32::TRANSPARENT)
                        .min_size(egui::Vec2::new(20.0, 20.0)),
                    )
                    .on_hover_text("Close (Escape)")
                    .clicked()
                {
                    search.close();
                }
            });
        });
    total_h += find_response.response.rect.height();

    if search.replace_mode && search.active {
        let replace_response = egui::Frame::none()
            .fill(bar_bg)
            .inner_margin(egui::Margin::symmetric(8.0, 4.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Replace:")
                            .size(12.0)
                            .color(egui::Color32::from_rgb(180, 185, 200)),
                    );

                    let mut replacement = search.replacement.clone();
                    ui.add(
                        egui::TextEdit::singleline(&mut replacement)
                            .desired_width((ui.available_width() - 180.0).max(80.0))
                            .hint_text("Replace with...")
                            .text_color(egui::Color32::WHITE)
                            .font(egui::TextStyle::Monospace),
                    );
                    if replacement != search.replacement {
                        search.replacement = replacement;
                    }

                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Replace").size(11.0).color(btn_color),
                            )
                            .fill(toggle_off)
                            .min_size(egui::Vec2::new(56.0, 20.0)),
                        )
                        .clicked()
                    {
                        if let Some(pos) = search.replace_current(buf) {
                            view.cursor.pos = pos;
                            view.cursor.clear_selection();
                            view.cursor.desired_col = None;
                            view.tree_dirty = true;
                        }
                    }

                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Replace All")
                                    .size(11.0)
                                    .color(btn_color),
                            )
                            .fill(toggle_off)
                            .min_size(egui::Vec2::new(80.0, 20.0)),
                        )
                        .clicked()
                    {
                        let count = search.replace_all(buf);
                        if count > 0 {
                            view.tree_dirty = true;
                        }
                    }
                });
            });
        total_h += replace_response.response.rect.height();
    }

    let ctx = ui.ctx().clone();
    ctx.input(|input| {
        if input.key_pressed(egui::Key::Escape) {
            search.close();
        } else if input.key_pressed(egui::Key::Enter) && !input.modifiers.shift {
            if let Some(pos) = search.next() {
                view.cursor.pos = pos;
                view.cursor.clear_selection();
                view.cursor.desired_col = None;
            }
        } else if input.key_pressed(egui::Key::Enter) && input.modifiers.shift {
            if let Some(pos) = search.prev() {
                view.cursor.pos = pos;
                view.cursor.clear_selection();
                view.cursor.desired_col = None;
            }
        }
    });

    total_h
}
