use crate::stacker::{
    export_prompts, import_prompts, merge_unique_prompts, new_prompt, stacker_path, StackerPrompt,
};

const S: f32 = 14.0;
const MUTED: egui::Color32 = egui::Color32::from_rgb(130, 130, 145);
const TAG_COLOR: egui::Color32 = egui::Color32::from_rgb(100, 155, 220);
const HEADING_COLOR: egui::Color32 = egui::Color32::from_rgb(200, 200, 210);
const DIM: egui::Color32 = egui::Color32::from_rgb(90, 92, 105);
const ROW_HOVER: egui::Color32 = egui::Color32::from_rgb(32, 34, 44);

/// Prompt bar visibility bit flags.
pub(crate) const BAR_VIEW_SHELL: u8 = 0b01;
pub(crate) const BAR_VIEW_EDITOR: u8 = 0b10;

fn small(text: &str) -> egui::RichText {
    egui::RichText::new(text).size(S)
}

/// Render the Stacker (prompt queue) view -- minimalist flat-list design.
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
    prompt_bar_visible: &mut bool,
    prompt_bar_views: &mut u8,
) {
    // ── Title ──
    ui.label(
        egui::RichText::new("Stacker")
            .size(20.0)
            .color(HEADING_COLOR),
    );
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new("Prompt queue manager")
            .size(12.0)
            .color(DIM),
    );
    ui.add_space(16.0);

    // ── Compact input area (no group/border) ──
    ui.add(
        egui::TextEdit::multiline(input)
            .desired_rows(3)
            .desired_width(f32::INFINITY)
            .hint_text("Type or paste a prompt...")
            .font(egui::TextStyle::Monospace)
            .text_color(egui::Color32::from_rgb(200, 200, 210)),
    );
    ui.add_space(6.0);

    ui.horizontal(|ui| {
        ui.label(small("Category:").color(MUTED));
        ui.add(
            egui::TextEdit::singleline(category_input)
                .desired_width(120.0)
                .hint_text("optional"),
        );
        ui.add_space(12.0);
        if ui
            .add_enabled(
                !input.trim().is_empty(),
                egui::Button::new(small("Add").color(egui::Color32::WHITE)),
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

    ui.add_space(14.0);

    // ── Search + filter + import/export ──
    let mut filter_cat = std::mem::take(filter_category);
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(search)
                .desired_width(180.0)
                .hint_text("Search..."),
        );
        ui.add_space(10.0);

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
            let display = if filter_cat.is_empty() {
                "All"
            } else {
                &filter_cat
            };
            egui::ComboBox::from_id_salt("stacker_cat_filter")
                .selected_text(display)
                .width(100.0)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(filter_cat.is_empty(), "All").clicked() {
                        filter_cat.clear();
                    }
                    for cat in &categories {
                        if ui.selectable_label(filter_cat == *cat, cat).clicked() {
                            filter_cat = cat.clone();
                        }
                    }
                });
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(
                    egui::Button::new(egui::RichText::new("Export").size(11.0).color(DIM))
                        .frame(false),
                )
                .clicked()
            {
                if let Some(path) = stacker_path() {
                    let export_path = path.with_extension("export.json");
                    let _ = export_prompts(prompts, &export_path);
                }
            }
            ui.label(egui::RichText::new("|").size(11.0).color(DIM));
            if ui
                .add(
                    egui::Button::new(egui::RichText::new("Import").size(11.0).color(DIM))
                        .frame(false),
                )
                .clicked()
            {
                if let Some(path) = stacker_path() {
                    let import_path = path.with_extension("export.json");
                    if let Ok(imported) = import_prompts(&import_path) {
                        if merge_unique_prompts(prompts, imported) > 0 {
                            *dirty = true;
                        }
                    }
                }
            }
        });
    });

    ui.add_space(6.0);

    // Thin separator
    ui.scope(|ui| {
        ui.visuals_mut().widgets.noninteractive.bg_stroke =
            egui::Stroke::new(0.5, egui::Color32::from_rgb(50, 52, 65));
        ui.separator();
    });

    ui.add_space(6.0);

    // ── Prompt bar toggle ──
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("{} prompts", prompts.len()))
                .size(12.0)
                .color(DIM),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut editor_on = *prompt_bar_views & BAR_VIEW_EDITOR != 0;
            let mut shell_on = *prompt_bar_views & BAR_VIEW_SHELL != 0;

            if ui.add(egui::Checkbox::new(&mut editor_on, "")).changed() {
                if editor_on {
                    *prompt_bar_views |= BAR_VIEW_EDITOR;
                } else {
                    *prompt_bar_views &= !BAR_VIEW_EDITOR;
                }
            }
            ui.label(egui::RichText::new("Editor").size(11.0).color(DIM));

            if ui.add(egui::Checkbox::new(&mut shell_on, "")).changed() {
                if shell_on {
                    *prompt_bar_views |= BAR_VIEW_SHELL;
                } else {
                    *prompt_bar_views &= !BAR_VIEW_SHELL;
                }
            }
            ui.label(egui::RichText::new("Shell").size(11.0).color(DIM));

            if ui
                .add(egui::Checkbox::new(prompt_bar_visible, ""))
                .changed()
            {}
            ui.label(egui::RichText::new("Prompt bar:").size(11.0).color(DIM));
        });
    });

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

    if prompts.is_empty() {
        ui.add_space(20.0);
        ui.label(small("No prompts yet.").color(DIM));
    } else if visible.is_empty() {
        ui.add_space(20.0);
        ui.label(small("No prompts match the current filter.").color(DIM));
    }

    let mut remove_idx: Option<usize> = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;

            for &i in &visible {
                let prompt = &prompts[i];
                let is_editing = *editing == Some(i);

                // Each prompt is a flat row with hover highlight
                let row_frame = egui::Frame::none()
                    .inner_margin(egui::Margin::symmetric(10.0, 7.0))
                    .rounding(egui::Rounding::same(3.0));

                let resp = row_frame.show(ui, |ui| {
                    if is_editing {
                        // Inline edit mode
                        ui.add(
                            egui::TextEdit::multiline(edit_text)
                                .desired_rows(3)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace)
                                .text_color(egui::Color32::from_rgb(200, 200, 210)),
                        );
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if ui.button(small("Save")).clicked() {
                                *saved_edit_idx = *editing;
                                *editing = None;
                            }
                            if ui.button(small("Cancel")).clicked() {
                                *editing = None;
                                edit_text.clear();
                            }
                        });
                    } else {
                        ui.horizontal(|ui| {
                            // Truncated text preview
                            let preview = truncate_line(&prompt.text, 80);
                            ui.label(
                                egui::RichText::new(preview)
                                    .size(13.0)
                                    .color(egui::Color32::from_rgb(190, 190, 200))
                                    .monospace(),
                            );

                            // Category tag on the right
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    // Action buttons (dimmed, appear on row)
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                egui::RichText::new("x").size(11.0).color(DIM),
                                            )
                                            .frame(false),
                                        )
                                        .on_hover_text("Delete")
                                        .clicked()
                                    {
                                        remove_idx = Some(i);
                                    }

                                    if ui
                                        .add(
                                            egui::Button::new(
                                                egui::RichText::new("edit").size(11.0).color(DIM),
                                            )
                                            .frame(false),
                                        )
                                        .clicked()
                                    {
                                        *editing = Some(i);
                                        *edit_text = prompt.text.clone();
                                    }

                                    if ui
                                        .add(
                                            egui::Button::new(
                                                egui::RichText::new("copy").size(11.0).color(DIM),
                                            )
                                            .frame(false),
                                        )
                                        .clicked()
                                    {
                                        *clipboard_copy = Some(prompt.text.clone());
                                    }

                                    if !prompt.category.is_empty() {
                                        ui.label(
                                            egui::RichText::new(&prompt.category)
                                                .size(11.0)
                                                .color(TAG_COLOR),
                                        );
                                    }
                                },
                            );
                        });
                    }
                });

                // Hover highlight on the row
                if resp.response.hovered() {
                    ui.painter().rect_filled(
                        resp.response.rect,
                        egui::Rounding::same(3.0),
                        ROW_HOVER,
                    );
                }
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

/// Render the prompt queue bar (thin horizontal strip above footer).
/// Returns `Some(text)` if a prompt was clicked (to copy to clipboard).
pub(crate) fn render_prompt_bar(ctx: &egui::Context, prompts: &[StackerPrompt]) -> Option<String> {
    let mut copied: Option<String> = None;

    egui::TopBottomPanel::bottom("prompt_queue_bar")
        .exact_height(24.0)
        .frame(
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(22, 23, 30))
                .inner_margin(egui::Margin::symmetric(8.0, 2.0)),
        )
        .show(ctx, |ui| {
            egui::ScrollArea::horizontal()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;
                        for prompt in prompts {
                            let preview = truncate_line(&prompt.text, 40);
                            let label_text = if prompt.category.is_empty() {
                                preview.clone()
                            } else {
                                format!("{} [{}]", preview, prompt.category)
                            };
                            let btn = ui.add(
                                egui::Button::new(
                                    egui::RichText::new(label_text)
                                        .size(11.0)
                                        .color(egui::Color32::from_rgb(140, 142, 155)),
                                )
                                .frame(false),
                            );
                            if btn.clicked() {
                                copied = Some(prompt.text.clone());
                            }
                            if btn.hovered() {
                                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                        }
                    });
                });
        });

    copied
}

/// Truncate to first line, capped at `max_chars`.
fn truncate_line(text: &str, max_chars: usize) -> String {
    let first_line = text.lines().next().unwrap_or("");
    if first_line.len() > max_chars {
        format!("{}...", &first_line[..max_chars])
    } else if text.lines().count() > 1 {
        format!("{}...", first_line)
    } else {
        first_line.to_string()
    }
}
