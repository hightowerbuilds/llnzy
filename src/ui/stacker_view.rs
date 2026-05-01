use crate::stacker::{
    formatting::{apply_list_prefix, char_to_byte_idx, maybe_continue_list, ListButtonKind},
    import_prompts, merge_unique_prompts, new_prompt, prompt_label,
    queue::{self, QueuedPrompt},
    stacker_path, StackerPrompt,
};

use super::STACKER_PROMPT_EDITOR_ID;

const S: f32 = 14.0;
pub(crate) const DEFAULT_EDITOR_FONT_SIZE: f32 = 16.0;
const MIN_EDITOR_FONT_SIZE: f32 = 12.0;
const MAX_EDITOR_FONT_SIZE: f32 = 24.0;
const ATKINSON: &str = "Atkinson Hyperlegible";
const MUTED: egui::Color32 = egui::Color32::from_rgb(130, 130, 145);
const HEADING_COLOR: egui::Color32 = egui::Color32::from_rgb(200, 200, 210);
const DIM: egui::Color32 = egui::Color32::from_rgb(90, 92, 105);
const ROW_BG: egui::Color32 = egui::Color32::from_rgb(30, 30, 30);
const ROW_HOVER: egui::Color32 = egui::Color32::from_rgb(42, 42, 42);
const PANEL_BG: egui::Color32 = egui::Color32::from_rgb(28, 28, 28);
const NOTE_BG: egui::Color32 = PANEL_BG;
const NOTE_TEXT: egui::Color32 = egui::Color32::from_rgb(240, 248, 255);
const QUEUE_GREEN: egui::Color32 = egui::Color32::from_rgb(106, 255, 144);
const NOTE_PADDING: f32 = 34.0;
const EDITOR_BOTTOM_GAP: f32 = 20.0;

fn small(text: &str) -> egui::RichText {
    egui::RichText::new(text).size(S)
}

fn stacker_editor_font(font_size: f32) -> egui::FontId {
    egui::FontId::new(font_size, egui::FontFamily::Name(ATKINSON.into()))
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
    editing: &mut Option<usize>,
    edit_text: &mut String,
    dirty: &mut bool,
    _saved_edit_idx: &mut Option<usize>,
    editor_font_size: &mut f32,
    queued_prompts: &mut Vec<QueuedPrompt>,
) {
    if let Some(idx) = *editing {
        if idx >= prompts.len() {
            *editing = None;
            edit_text.clear();
        }
    }

    // ── Header actions ──
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Stacker")
                .size(20.0)
                .color(HEADING_COLOR),
        );
        ui.label(egui::RichText::new("Prompt editor").size(12.0).color(DIM));
    });

    ui.add_space(10.0);

    let available_h = ui.available_height();
    let list_h = (available_h * 0.24).clamp(120.0, 220.0);

    render_prompt_list_panel(ui, list_h, prompts, editing, input, queued_prompts);

    ui.add_space(12.0);

    let editor_h = (ui.available_height() - EDITOR_BOTTOM_GAP).max(1.0);
    render_prompt_editor_panel(
        ui,
        editor_h,
        prompts,
        input,
        editing,
        dirty,
        editor_font_size,
    );
}

#[allow(clippy::too_many_arguments)]
fn render_prompt_list_panel(
    ui: &mut egui::Ui,
    height: f32,
    prompts: &mut Vec<StackerPrompt>,
    editing: &mut Option<usize>,
    input: &mut String,
    queued_prompts: &mut Vec<QueuedPrompt>,
) {
    queue::sanitize_prompt_queue(queued_prompts);

    egui::Frame::none()
        .fill(PANEL_BG)
        .rounding(egui::Rounding::same(4.0))
        .inner_margin(egui::Margin::same(10.0))
        .show(ui, |ui| {
            ui.set_height(height);
            ui.horizontal(|ui| {
                let queue_w = 380.0_f32.min((ui.available_width() * 0.5).max(260.0));
                ui.vertical(|ui| {
                    ui.set_width(queue_w);
                    render_queue_bank(ui, queue_w, height, queued_prompts);
                });

                ui.add_space(10.0);

                ui.vertical(|ui| {
                    render_saved_prompt_list(ui, height, prompts, editing, input, queued_prompts);
                });
            });
        });
}

fn render_queue_bank(ui: &mut egui::Ui, width: f32, height: f32, queued_prompts: &[QueuedPrompt]) {
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(22, 24, 22))
        .rounding(egui::Rounding::same(3.0))
        .inner_margin(egui::Margin::symmetric(8.0, 7.0))
        .show(ui, |ui| {
            ui.set_width(width);
            ui.set_height(height - 20.0);
            ui.label(
                egui::RichText::new(format!(
                    "QUEUE {}/{}",
                    queued_prompts.len(),
                    queue::MAX_QUEUE_PROMPTS
                ))
                .size(11.0)
                .strong()
                .color(QUEUE_GREEN),
            );
            ui.add_space(6.0);

            if queued_prompts.is_empty() {
                ui.label(egui::RichText::new("EMPTY").size(12.0).color(DIM));
                return;
            }

            egui::ScrollArea::vertical()
                .id_salt("stacker_prompt_queue_bank")
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    for prompt in queued_prompts {
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(26, 32, 26))
                            .rounding(egui::Rounding::same(3.0))
                            .inner_margin(egui::Margin::symmetric(7.0, 5.0))
                            .show(ui, |ui| {
                                ui.add_sized(
                                    [ui.available_width(), 18.0],
                                    egui::Label::new(
                                        egui::RichText::new(
                                            truncate_line(&prompt.label, 46).to_uppercase(),
                                        )
                                        .size(12.0)
                                        .strong()
                                        .color(QUEUE_GREEN),
                                    ),
                                );
                            });
                    }
                });
        });
}

#[allow(clippy::too_many_arguments)]
fn render_saved_prompt_list(
    ui: &mut egui::Ui,
    height: f32,
    prompts: &[StackerPrompt],
    editing: &mut Option<usize>,
    input: &mut String,
    queued_prompts: &mut Vec<QueuedPrompt>,
) {
    ui.set_height(height - 20.0);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("Saved prompts ({})", prompts.len()))
                .size(13.0)
                .color(HEADING_COLOR),
        );
    });
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        ui.add_sized([ui.available_width() - 178.0, 18.0], header_label("Prompt"));
        ui.add_sized([48.0, 18.0], header_label("Chars"));
        ui.add_sized([104.0, 18.0], header_label("Queue"));
    });
    ui.separator();

    if prompts.is_empty() {
        ui.add_space(20.0);
        ui.label(small("No prompts yet.").color(DIM));
    }

    egui::ScrollArea::vertical()
        .id_salt("stacker_prompt_list")
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 3.0;

            for i in 0..prompts.len() {
                let prompt = &prompts[i];
                let selected = *editing == Some(i);
                let row_resp = egui::Frame::none()
                    .fill(if selected { ROW_HOVER } else { ROW_BG })
                    .rounding(egui::Rounding::same(3.0))
                    .inner_margin(egui::Margin::symmetric(8.0, 5.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let title_w = (ui.available_width() - 170.0).max(160.0);
                            ui.add_sized(
                                [title_w, 20.0],
                                egui::Label::new(
                                    egui::RichText::new(truncate_line(&prompt.label, 64))
                                        .size(13.0)
                                        .color(egui::Color32::from_rgb(210, 210, 216)),
                                )
                                .sense(egui::Sense::click()),
                            );
                            ui.add_sized(
                                [48.0, 20.0],
                                egui::Label::new(
                                    egui::RichText::new(prompt.text.chars().count().to_string())
                                        .size(12.0)
                                        .color(MUTED),
                                ),
                            );

                            let already_queued = queue::contains_prompt(queued_prompts, prompt);
                            let can_add =
                                queued_prompts.len() < queue::MAX_QUEUE_PROMPTS && !already_queued;
                            let label = if already_queued {
                                "Queued"
                            } else {
                                "Add to queue"
                            };
                            if ui
                                .add_enabled(
                                    can_add,
                                    egui::Button::new(egui::RichText::new(label).size(12.0))
                                        .min_size(egui::vec2(104.0, 22.0)),
                                )
                                .clicked()
                            {
                                queue::add_prompt(queued_prompts, prompt);
                            }
                        });
                    })
                    .response;

                if row_resp.clicked() {
                    *editing = Some(i);
                    *input = prompt.text.clone();
                }
            }
        });
}

fn render_prompt_editor_panel(
    ui: &mut egui::Ui,
    height: f32,
    prompts: &mut Vec<StackerPrompt>,
    input: &mut String,
    editing: &mut Option<usize>,
    dirty: &mut bool,
    editor_font_size: &mut f32,
) {
    let editor_id = egui::Id::new(STACKER_PROMPT_EDITOR_ID);

    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), height),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.spacing_mut().item_spacing.x = 4.0;
                let bold = egui::Button::new(
                    egui::RichText::new("B")
                        .size(13.0)
                        .strong()
                        .color(NOTE_TEXT),
                )
                .min_size(egui::vec2(28.0, 24.0));
                if ui.add(bold).on_hover_text("Bold selected text").clicked() {
                    apply_bold_markup(ui.ctx(), editor_id, input);
                }
                let unordered = egui::Button::new(
                    egui::RichText::new("•")
                        .size(16.0)
                        .strong()
                        .color(NOTE_TEXT),
                )
                .min_size(egui::vec2(28.0, 24.0));
                if ui
                    .add(unordered)
                    .on_hover_text("Make unordered list")
                    .clicked()
                {
                    apply_list_markup(ui.ctx(), editor_id, input, ListButtonKind::Unordered);
                }
                let ordered = egui::Button::new(
                    egui::RichText::new("1.")
                        .size(13.0)
                        .strong()
                        .color(NOTE_TEXT),
                )
                .min_size(egui::vec2(32.0, 24.0));
                if ui
                    .add(ordered)
                    .on_hover_text("Make numbered list")
                    .clicked()
                {
                    apply_list_markup(ui.ctx(), editor_id, input, ListButtonKind::Ordered);
                }
                ui.add_space(8.0);
                if ui
                    .add(egui::Button::new(small("-")).min_size(egui::vec2(28.0, 24.0)))
                    .on_hover_text("Make prompt text smaller")
                    .clicked()
                {
                    *editor_font_size =
                        (*editor_font_size - 1.0).clamp(MIN_EDITOR_FONT_SIZE, MAX_EDITOR_FONT_SIZE);
                }
                if ui
                    .add(egui::Button::new(small("+")).min_size(egui::vec2(28.0, 24.0)))
                    .on_hover_text("Make prompt text bigger")
                    .clicked()
                {
                    *editor_font_size =
                        (*editor_font_size + 1.0).clamp(MIN_EDITOR_FONT_SIZE, MAX_EDITOR_FONT_SIZE);
                }
                ui.add_space(8.0);
                if ui.button(small("New")).clicked() {
                    *editing = None;
                    input.clear();
                }
                if ui.button(small("Reset Cache")).clicked() {
                    reset_prompt_editor_cache(ui.ctx(), editor_id);
                }
                if ui.button(small("Import")).clicked() {
                    import_prompt_file(prompts, dirty);
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

            ui.add_space(8.0);

            let note_h = ui.available_height().max(1.0);
            egui::Frame::none()
                .fill(NOTE_BG)
                .rounding(egui::Rounding::same(3.0))
                .inner_margin(egui::Margin::same(NOTE_PADDING))
                .show(ui, |ui| {
                    ui.set_height(note_h);
                    ui.scope(|ui| {
                        let before_edit = input.clone();
                        ui.visuals_mut().extreme_bg_color = egui::Color32::TRANSPARENT;
                        ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                        ui.visuals_mut().widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                        ui.visuals_mut().widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                        ui.visuals_mut().widgets.inactive.bg_stroke = egui::Stroke::NONE;
                        ui.visuals_mut().widgets.hovered.bg_stroke = egui::Stroke::NONE;
                        ui.visuals_mut().widgets.active.bg_stroke = egui::Stroke::NONE;
                        ui.set_min_size(egui::vec2(ui.available_width(), ui.available_height()));
                        let mut output = egui::TextEdit::multiline(input)
                            .id(editor_id)
                            .desired_rows(32)
                            .desired_width(f32::INFINITY)
                            .hint_text("Write your prompt here...")
                            .font(stacker_editor_font(*editor_font_size))
                            .text_color(NOTE_TEXT)
                            .frame(false)
                            .show(ui);

                        if output.response.changed() {
                            if let Some(cursor_range) = output.cursor_range {
                                let cursor_idx = cursor_range.as_ccursor_range().primary.index;
                                if let Some(new_cursor_idx) =
                                    maybe_continue_list(&before_edit, input, cursor_idx)
                                {
                                    output.state.cursor.set_char_range(Some(
                                        egui::text::CCursorRange::one(egui::text::CCursor::new(
                                            new_cursor_idx,
                                        )),
                                    ));
                                    output.state.clone().store(ui.ctx(), editor_id);
                                }
                            }
                        }
                    });
                });
        },
    );
}

fn import_prompt_file(prompts: &mut Vec<StackerPrompt>, dirty: &mut bool) {
    let Some(path) = stacker_path() else { return };
    let import_path = path.with_extension("import.json");
    if let Ok(imported) = import_prompts(&import_path) {
        if merge_unique_prompts(prompts, imported) > 0 {
            *dirty = true;
        }
    }
}

fn apply_bold_markup(ctx: &egui::Context, editor_id: egui::Id, input: &mut String) -> bool {
    let Some(mut state) = egui::text_edit::TextEditState::load(ctx, editor_id) else {
        return false;
    };
    let Some(range) = state.cursor.char_range() else {
        return false;
    };
    let [start, end] = range.sorted();
    let start = start.index;
    let end = end.index;
    if start == end {
        let insert_at = char_to_byte_idx(input, start);
        input.insert_str(insert_at, "****");
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::one(
                egui::text::CCursor::new(start + 2),
            )));
    } else {
        let start_byte = char_to_byte_idx(input, start);
        let end_byte = char_to_byte_idx(input, end);
        input.insert_str(end_byte, "**");
        input.insert_str(start_byte, "**");
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(start + 2),
                egui::text::CCursor::new(end + 2),
            )));
    }
    state.store(ctx, editor_id);
    ctx.memory_mut(|memory| memory.request_focus(editor_id));
    ctx.request_repaint();
    true
}

fn apply_list_markup(
    ctx: &egui::Context,
    editor_id: egui::Id,
    input: &mut String,
    kind: ListButtonKind,
) -> bool {
    let Some(mut state) = egui::text_edit::TextEditState::load(ctx, editor_id) else {
        return false;
    };
    let range = state
        .cursor
        .char_range()
        .unwrap_or_else(|| egui::text::CCursorRange::one(egui::text::CCursor::new(0)));
    let [start, end] = range.sorted();
    let start = start.index;
    let end = end.index;
    let (new_text, new_cursor) = apply_list_prefix(input, start, end, kind);
    *input = new_text;
    state
        .cursor
        .set_char_range(Some(egui::text::CCursorRange::one(
            egui::text::CCursor::new(new_cursor),
        )));
    state.store(ctx, editor_id);
    ctx.memory_mut(|memory| memory.request_focus(editor_id));
    ctx.request_repaint();
    true
}

fn reset_prompt_editor_cache(ctx: &egui::Context, editor_id: egui::Id) {
    egui::text_edit::TextEditState::default().store(ctx, editor_id);
    ctx.memory_mut(|memory| memory.request_focus(editor_id));
    ctx.request_repaint();
}

fn header_label(text: &str) -> egui::Label {
    egui::Label::new(egui::RichText::new(text).size(11.0).color(DIM).strong())
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
