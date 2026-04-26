use crate::stacker::{
    export_prompts, import_prompts, merge_unique_prompts, new_prompt, stacker_path, StackerPrompt,
};

const S: f32 = 16.0;

fn label(text: &str) -> egui::RichText {
    egui::RichText::new(text).size(S)
}

/// Render the Stacker (prompt queue) view.
///
/// Takes ownership of mutable string state for closure-friendliness,
/// writing values back through the mutable references on return.
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_stacker_view(
    ui: &mut egui::Ui,
    prompts: &mut Vec<StackerPrompt>,
    input: &mut String,
    category_input: &mut String,
    search: &mut String,
    filter_category: &mut String,
    editing: &mut Option<usize>,
    edit_text: &mut String,
    dirty: &mut bool,
    saved_edit_idx: &mut Option<usize>,
    clipboard_copy: &mut Option<String>,
) {
    ui.label(
        egui::RichText::new("Stacker — Prompt Queue")
            .size(22.0)
            .color(egui::Color32::WHITE),
    );
    ui.add_space(12.0);

    // ── Input area ──
    ui.group(|ui| {
        ui.label(label("New Prompt"));
        ui.add_space(4.0);

        ui.add(
            egui::TextEdit::multiline(input)
                .desired_rows(4)
                .desired_width(f32::INFINITY)
                .hint_text("Type or paste your prompt here...")
                .font(egui::TextStyle::Monospace),
        );
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label(label("Category:"));
            ui.add(
                egui::TextEdit::singleline(category_input)
                    .desired_width(150.0)
                    .hint_text("optional"),
            );
            ui.add_space(16.0);
            if ui
                .add_enabled(
                    !input.trim().is_empty(),
                    egui::Button::new(label("Save to Queue")),
                )
                .clicked()
            {
                if let Some(prompt) = new_prompt(input, category_input) {
                    prompts.push(prompt);
                    input.clear();
                    category_input.clear();
                    *dirty = true;
                }
            }
        });
    });

    ui.add_space(12.0);

    // ── Search + filter bar ──
    // Snapshot the filter category so closures don't fight over the &mut.
    let mut filter_cat = std::mem::take(filter_category);
    ui.horizontal(|ui| {
        ui.label(label("Search:"));
        ui.add(
            egui::TextEdit::singleline(search)
                .desired_width(200.0)
                .hint_text("filter prompts..."),
        );
        ui.add_space(16.0);

        let categories: Vec<String> = {
            let mut cats: Vec<String> = prompts
                .iter()
                .map(|p| p.category.clone())
                .filter(|c| !c.is_empty())
                .collect();
            cats.sort();
            cats.dedup();
            cats
        };
        if !categories.is_empty() {
            ui.label(label("Category:"));
            let display = if filter_cat.is_empty() {
                "All"
            } else {
                &filter_cat
            };
            egui::ComboBox::from_id_salt("stacker_cat_filter")
                .selected_text(display)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(filter_cat.is_empty(), "All")
                        .clicked()
                    {
                        filter_cat.clear();
                    }
                    for cat in &categories {
                        if ui
                            .selectable_label(filter_cat == *cat, cat)
                            .clicked()
                        {
                            filter_cat = cat.clone();
                        }
                    }
                });
        }

        ui.with_layout(
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                if ui.small_button("Export").clicked() {
                    if let Some(path) = stacker_path() {
                        let export_path = path.with_extension("export.json");
                        let _ = export_prompts(prompts, &export_path);
                    }
                }
                if ui.small_button("Import").clicked() {
                    if let Some(path) = stacker_path() {
                        let import_path = path.with_extension("export.json");
                        if let Ok(imported) = import_prompts(&import_path) {
                            if merge_unique_prompts(prompts, imported) > 0 {
                                *dirty = true;
                            }
                        }
                    }
                }
            },
        );
    });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    // ── Filtered prompt list ──
    let search_lower = search.to_lowercase();
    let visible: Vec<usize> = (0..prompts.len())
        .filter(|&i| {
            let p = &prompts[i];
            let cat_ok = filter_cat.is_empty() || p.category == filter_cat;
            let search_ok = search.is_empty()
                || p.text.to_lowercase().contains(&search_lower)
                || p.label.to_lowercase().contains(&search_lower)
                || p.category.to_lowercase().contains(&search_lower);
            cat_ok && search_ok
        })
        .collect();

    ui.label(
        egui::RichText::new(format!("Queue ({}/{})", visible.len(), prompts.len()))
            .size(18.0)
            .color(egui::Color32::WHITE),
    );
    ui.add_space(8.0);

    if prompts.is_empty() {
        ui.label(label("No prompts saved yet. Add one above."));
    } else if visible.is_empty() {
        ui.label(label("No prompts match the current filter."));
    }

    let mut remove_idx: Option<usize> = None;
    egui::ScrollArea::vertical().show(ui, |ui| {
        for &i in &visible {
            let prompt = &prompts[i];
            let is_editing = *editing == Some(i);

            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&prompt.label)
                            .size(15.0)
                            .color(egui::Color32::WHITE)
                            .strong(),
                    );
                    if !prompt.category.is_empty() {
                        ui.label(
                            egui::RichText::new(format!("[{}]", prompt.category))
                                .size(12.0)
                                .color(egui::Color32::from_rgb(120, 180, 255)),
                        );
                    }

                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if ui.small_button("Delete").clicked() {
                                remove_idx = Some(i);
                            }
                            if ui.button(label("Copy")).clicked() {
                                *clipboard_copy = Some(prompt.text.clone());
                            }
                            if !is_editing && ui.small_button("Edit").clicked() {
                                *editing = Some(i);
                                *edit_text = prompt.text.clone();
                            }
                        },
                    );
                });

                if is_editing {
                    ui.add(
                        egui::TextEdit::multiline(edit_text)
                            .desired_rows(4)
                            .desired_width(f32::INFINITY)
                            .font(egui::TextStyle::Monospace),
                    );
                    ui.horizontal(|ui| {
                        if ui.button(label("Save")).clicked() {
                            *saved_edit_idx = *editing;
                            *editing = None;
                        }
                        if ui.button(label("Cancel")).clicked() {
                            *editing = None;
                            edit_text.clear();
                        }
                    });
                } else {
                    let preview: String =
                        prompt.text.lines().take(3).collect::<Vec<_>>().join("\n");
                    ui.label(
                        egui::RichText::new(preview)
                            .size(13.0)
                            .color(egui::Color32::from_rgb(160, 160, 170))
                            .monospace(),
                    );
                }
            });
            ui.add_space(4.0);
        }
    });

    if let Some(idx) = remove_idx {
        prompts.remove(idx);
        *dirty = true;
        if *editing == Some(idx) {
            *editing = None;
        }
    }

    // Write back the filter category
    *filter_category = filter_cat;
}
