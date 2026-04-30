use crate::stacker::{
    export_prompts, import_prompts, merge_unique_prompts, new_prompt, prompt_label, stacker_path,
    StackerPrompt,
};

const S: f32 = 14.0;
const MUTED: egui::Color32 = egui::Color32::from_rgb(130, 130, 145);
const HEADING_COLOR: egui::Color32 = egui::Color32::from_rgb(200, 200, 210);
const DIM: egui::Color32 = egui::Color32::from_rgb(90, 92, 105);
const ROW_BG: egui::Color32 = egui::Color32::from_rgb(30, 30, 30);
const ROW_HOVER: egui::Color32 = egui::Color32::from_rgb(42, 42, 42);
const PANEL_BG: egui::Color32 = egui::Color32::from_rgb(28, 28, 28);
const NOTE_BG: egui::Color32 = PANEL_BG;
const NOTE_TEXT: egui::Color32 = egui::Color32::from_rgb(240, 248, 255);
const NOTE_PADDING: f32 = 34.0;
const EDITOR_BOTTOM_GAP: f32 = 20.0;
const PROMPT_EDITOR_ID: &str = "stacker_prompt_editor";

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
    _saved_edit_idx: &mut Option<usize>,
    clipboard_copy: &mut Option<String>,
    _prompt_bar_visible: &mut bool,
    _prompt_bar_views: &mut u8,
) {
    if let Some(idx) = *editing {
        if idx >= prompts.len() {
            *editing = None;
            edit_text.clear();
        }
    }
    category_input.clear();
    filter_category.clear();

    // ── Header actions ──
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Stacker")
                .size(20.0)
                .color(HEADING_COLOR),
        );
        ui.label(egui::RichText::new("Prompt editor").size(12.0).color(DIM));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(
                    egui::Button::new(egui::RichText::new("Export").size(12.0).color(DIM))
                        .frame(false),
                )
                .clicked()
            {
                if let Some(path) = stacker_path() {
                    let export_path = path.with_extension("export.json");
                    let _ = export_prompts(prompts, &export_path);
                }
            }
            if ui
                .add(
                    egui::Button::new(egui::RichText::new("Import").size(12.0).color(DIM))
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

    ui.add_space(10.0);

    let available_h = ui.available_height();
    let list_h = (available_h * 0.24).clamp(120.0, 220.0);

    render_prompt_list_panel(
        ui,
        list_h,
        prompts,
        search,
        editing,
        input,
        dirty,
        clipboard_copy,
    );

    ui.add_space(12.0);

    let editor_h = (ui.available_height() - EDITOR_BOTTOM_GAP).max(1.0);
    render_prompt_editor_panel(ui, editor_h, prompts, input, editing, dirty);
}

#[allow(clippy::too_many_arguments)]
fn render_prompt_list_panel(
    ui: &mut egui::Ui,
    height: f32,
    prompts: &mut Vec<StackerPrompt>,
    search: &mut String,
    editing: &mut Option<usize>,
    input: &mut String,
    dirty: &mut bool,
    _clipboard_copy: &mut Option<String>,
) {
    let search_lower = search.to_lowercase();
    let visible: Vec<usize> = (0..prompts.len())
        .filter(|&i| {
            let p = &prompts[i];
            let search_ok = search.is_empty()
                || p.text.to_lowercase().contains(&search_lower)
                || p.label.to_lowercase().contains(&search_lower);
            search_ok
        })
        .collect();

    egui::Frame::none()
        .fill(PANEL_BG)
        .rounding(egui::Rounding::same(4.0))
        .inner_margin(egui::Margin::same(10.0))
        .show(ui, |ui| {
            ui.set_height(height);

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("Prompts ({})", prompts.len()))
                        .size(13.0)
                        .color(HEADING_COLOR),
                );
                ui.add_space(16.0);
                ui.add(
                    egui::TextEdit::singleline(search)
                        .desired_width(190.0)
                        .hint_text("Search"),
                );
            });
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.add_sized([ui.available_width() - 72.0, 18.0], header_label("Prompt"));
                ui.add_sized([54.0, 18.0], header_label("Chars"));
            });
            ui.separator();

            if prompts.is_empty() {
                ui.add_space(20.0);
                ui.label(small("No prompts yet.").color(DIM));
            } else if visible.is_empty() {
                ui.add_space(20.0);
                ui.label(small("No prompts match the current filter.").color(DIM));
            }

            egui::ScrollArea::vertical()
                .id_salt("stacker_prompt_list")
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 3.0;

                    for &i in &visible {
                        let prompt = &prompts[i];
                        let selected = *editing == Some(i);
                        let row_resp = egui::Frame::none()
                            .fill(if selected { ROW_HOVER } else { ROW_BG })
                            .rounding(egui::Rounding::same(3.0))
                            .inner_margin(egui::Margin::symmetric(8.0, 5.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let title_w = (ui.available_width() - 70.0).max(220.0);
                                    ui.add_sized(
                                        [title_w, 20.0],
                                        egui::Label::new(
                                            egui::RichText::new(truncate_line(&prompt.label, 70))
                                                .size(13.0)
                                                .color(egui::Color32::from_rgb(210, 210, 216)),
                                        )
                                        .sense(egui::Sense::click()),
                                    );
                                    ui.add_sized(
                                        [54.0, 20.0],
                                        egui::Label::new(
                                            egui::RichText::new(
                                                prompt.text.chars().count().to_string(),
                                            )
                                            .size(12.0)
                                            .color(MUTED),
                                        ),
                                    );
                                });
                            })
                            .response;

                        if row_resp.clicked() {
                            *editing = Some(i);
                            *input = prompt.text.clone();
                        }
                    }
                });
        });
    let _ = dirty;
}

fn render_prompt_editor_panel(
    ui: &mut egui::Ui,
    height: f32,
    prompts: &mut Vec<StackerPrompt>,
    input: &mut String,
    editing: &mut Option<usize>,
    dirty: &mut bool,
) {
    let editor_id = ui.make_persistent_id(PROMPT_EDITOR_ID);

    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), height),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.horizontal(|ui| {
                let title = if let Some(idx) = *editing {
                    prompts
                        .get(idx)
                        .map(|prompt| prompt.label.as_str())
                        .unwrap_or("Selected prompt")
                } else {
                    "New prompt"
                };
                ui.label(egui::RichText::new(title).size(15.0).color(HEADING_COLOR));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(small("New")).clicked() {
                        *editing = None;
                        input.clear();
                    }
                    if ui.button(small("Reset Cache")).clicked() {
                        reset_prompt_editor_cache(ui.ctx(), editor_id);
                    }
                    let save_label = if editing.is_some() {
                        "Save"
                    } else {
                        "Save Prompt"
                    };
                    if ui
                        .add_enabled(
                            !input.trim().is_empty(),
                            egui::Button::new(small(save_label)),
                        )
                        .clicked()
                    {
                        if let Some(idx) = *editing {
                            if let Some(prompt) = prompts.get_mut(idx) {
                                prompt.text = input.trim().to_string();
                                prompt.label = prompt_label(&prompt.text);
                                prompt.category.clear();
                                *dirty = true;
                            }
                        } else if let Some(prompt) = new_prompt(input, "") {
                            prompts.push(prompt);
                            *editing = Some(prompts.len() - 1);
                            *dirty = true;
                        }
                    }
                });
            });

            ui.add_space(10.0);
            let note_h = ui.available_height().max(1.0);
            egui::Frame::none()
                .fill(NOTE_BG)
                .rounding(egui::Rounding::same(3.0))
                .inner_margin(egui::Margin::same(NOTE_PADDING))
                .show(ui, |ui| {
                    ui.set_height(note_h);
                    ui.scope(|ui| {
                        ui.visuals_mut().extreme_bg_color = egui::Color32::TRANSPARENT;
                        ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                        ui.visuals_mut().widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                        ui.visuals_mut().widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                        ui.visuals_mut().widgets.inactive.bg_stroke = egui::Stroke::NONE;
                        ui.visuals_mut().widgets.hovered.bg_stroke = egui::Stroke::NONE;
                        ui.visuals_mut().widgets.active.bg_stroke = egui::Stroke::NONE;
                        ui.add_sized(
                            [ui.available_width(), ui.available_height()],
                            egui::TextEdit::multiline(input)
                                .id(editor_id)
                                .desired_rows(32)
                                .desired_width(f32::INFINITY)
                                .hint_text("Write your prompt here...")
                                .font(egui::FontId::monospace(S))
                                .text_color(NOTE_TEXT),
                        );
                    });
                });
        },
    );
}

fn reset_prompt_editor_cache(ctx: &egui::Context, editor_id: egui::Id) {
    egui::text_edit::TextEditState::default().store(ctx, editor_id);
    ctx.memory_mut(|memory| memory.request_focus(editor_id));
    ctx.request_repaint();
}

fn header_label(text: &str) -> egui::Label {
    egui::Label::new(egui::RichText::new(text).size(11.0).color(DIM).strong())
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
                            let btn = ui.add(
                                egui::Button::new(
                                    egui::RichText::new(preview)
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
