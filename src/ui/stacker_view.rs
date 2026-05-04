use crate::stacker::{
    commands::{
        execute_stacker_command_at, stacker_command_descriptor, stacker_command_registry,
        stacker_editor_command, StackerCommandId, StackerEditorCommand,
    },
    document::StackerDocumentEditor,
    draft::StackerDraft,
    formatting::maybe_continue_list,
    import_prompts, merge_unique_prompts, new_prompt, prompt_label,
    queue::{self, QueuedPrompt},
    stacker_path, StackerPrompt,
};

use super::{
    stacker_cursor,
    stacker_state::{PendingStackerDraftSwitch, StackerUiState},
    STACKER_PROMPT_EDITOR_ID,
};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SavedPromptRowAction {
    Open,
    Edit,
    Delete,
}

fn small(text: &str) -> egui::RichText {
    egui::RichText::new(text).size(S)
}

fn stacker_editor_font(font_size: f32) -> egui::FontId {
    egui::FontId::new(font_size, egui::FontFamily::Name(ATKINSON.into()))
}

pub(crate) fn apply_registered_stacker_command(
    ctx: &egui::Context,
    stacker: &mut StackerUiState,
    command_id: StackerCommandId,
) -> bool {
    apply_editor_command(
        ctx,
        egui::Id::new(STACKER_PROMPT_EDITOR_ID),
        &mut stacker.editor,
        &mut stacker.draft,
        stacker_editor_command(command_id),
    )
}

/// Render the Stacker (prompt queue) view -- minimalist flat-list design.
///
/// Takes ownership of mutable string state for closure-friendliness,
/// writing values back through the mutable references on return.
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_stacker_view(
    ui: &mut egui::Ui,
    prompts: &mut Vec<StackerPrompt>,
    editor: &mut StackerDocumentEditor,
    draft: &mut StackerDraft,
    pending_switch: &mut Option<PendingStackerDraftSwitch>,
    pending_delete: &mut Option<usize>,
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
    if matches!(
        *pending_switch,
        Some(PendingStackerDraftSwitch::SavedPrompt(idx)) if idx >= prompts.len()
    ) {
        *pending_switch = None;
    }
    if matches!(*pending_delete, Some(idx) if idx >= prompts.len()) {
        *pending_delete = None;
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

    render_prompt_list_panel(
        ui,
        list_h,
        prompts,
        editing,
        editor,
        draft,
        pending_switch,
        pending_delete,
        queued_prompts,
    );

    ui.add_space(12.0);

    let editor_h = (ui.available_height() - EDITOR_BOTTOM_GAP).max(1.0);
    render_prompt_editor_panel(
        ui,
        editor_h,
        prompts,
        editor,
        draft,
        pending_switch,
        editing,
        dirty,
        editor_font_size,
    );

    render_discard_draft_modal(ui.ctx(), prompts, editor, draft, pending_switch, editing);
    render_delete_prompt_modal(
        ui.ctx(),
        prompts,
        editor,
        draft,
        editing,
        dirty,
        pending_delete,
    );
}

#[allow(clippy::too_many_arguments)]
fn render_prompt_list_panel(
    ui: &mut egui::Ui,
    height: f32,
    prompts: &mut Vec<StackerPrompt>,
    editing: &mut Option<usize>,
    editor: &mut StackerDocumentEditor,
    draft: &mut StackerDraft,
    pending_switch: &mut Option<PendingStackerDraftSwitch>,
    pending_delete: &mut Option<usize>,
    queued_prompts: &mut Vec<QueuedPrompt>,
) {
    queue::sanitize_prompt_queue(queued_prompts);

    egui::Frame::none()
        .fill(PANEL_BG)
        .rounding(egui::Rounding::same(4.0))
        .inner_margin(egui::Margin::same(10.0))
        .show(ui, |ui| {
            ui.set_height(height);
            ui.horizontal_wrapped(|ui| {
                let queue_w = 380.0_f32.min((ui.available_width() * 0.5).max(260.0));
                ui.vertical(|ui| {
                    ui.set_width(queue_w);
                    render_queue_bank(ui, queue_w, height, queued_prompts);
                });

                ui.add_space(10.0);

                ui.vertical(|ui| {
                    render_saved_prompt_list(
                        ui,
                        height,
                        prompts,
                        editing,
                        editor,
                        draft,
                        pending_switch,
                        pending_delete,
                        queued_prompts,
                    );
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
    prompts: &mut Vec<StackerPrompt>,
    editing: &mut Option<usize>,
    editor: &mut StackerDocumentEditor,
    draft: &mut StackerDraft,
    pending_switch: &mut Option<PendingStackerDraftSwitch>,
    pending_delete: &mut Option<usize>,
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
        let prompt_w = (ui.available_width() - 294.0).max(40.0);
        ui.add_sized([prompt_w, 18.0], header_label("Prompt"));
        ui.add_sized([48.0, 18.0], header_label("Chars"));
        ui.add_sized([104.0, 18.0], header_label("Queue"));
        ui.add_sized([44.0, 18.0], header_label("Edit"));
        ui.add_sized([56.0, 18.0], header_label("Delete"));
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
                let mut row_action = None;
                let mut child_button_clicked = false;
                let row_resp = egui::Frame::none()
                    .fill(if selected { ROW_HOVER } else { ROW_BG })
                    .rounding(egui::Rounding::same(3.0))
                    .inner_margin(egui::Margin::symmetric(8.0, 5.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let title_w = (ui.available_width() - 286.0).max(40.0);
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
                                child_button_clicked = true;
                                queue::add_prompt(queued_prompts, prompt);
                            }
                            if ui
                                .add(
                                    egui::Button::new(egui::RichText::new("Edit").size(12.0))
                                        .min_size(egui::vec2(44.0, 22.0)),
                                )
                                .clicked()
                            {
                                child_button_clicked = true;
                                row_action = Some(SavedPromptRowAction::Edit);
                            }
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Delete")
                                            .size(12.0)
                                            .color(egui::Color32::from_rgb(245, 125, 125)),
                                    )
                                    .min_size(egui::vec2(56.0, 22.0)),
                                )
                                .clicked()
                            {
                                child_button_clicked = true;
                                row_action = Some(SavedPromptRowAction::Delete);
                            }
                        });
                    })
                    .response;

                if row_action.is_none() && !child_button_clicked && row_resp.clicked() {
                    row_action = Some(SavedPromptRowAction::Open);
                }

                match row_action {
                    Some(SavedPromptRowAction::Open | SavedPromptRowAction::Edit) => {
                        request_load_saved_prompt(
                            ui.ctx(),
                            prompts,
                            editor,
                            draft,
                            pending_switch,
                            editing,
                            i,
                        );
                    }
                    Some(SavedPromptRowAction::Delete) => {
                        *pending_delete = Some(i);
                    }
                    None => {}
                }
            }
        });
}

fn render_prompt_editor_panel(
    ui: &mut egui::Ui,
    height: f32,
    prompts: &mut Vec<StackerPrompt>,
    editor: &mut StackerDocumentEditor,
    draft: &mut StackerDraft,
    pending_switch: &mut Option<PendingStackerDraftSwitch>,
    editing: &mut Option<usize>,
    dirty: &mut bool,
    editor_font_size: &mut f32,
) {
    let editor_id = egui::Id::new(STACKER_PROMPT_EDITOR_ID);

    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), height),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.add_space(8.0);
                ui.spacing_mut().item_spacing.x = 4.0;
                for command in stacker_command_registry() {
                    if matches!(
                        command.id,
                        StackerCommandId::Clear | StackerCommandId::Undo | StackerCommandId::Redo
                    ) {
                        continue;
                    }
                    if stacker_toolbar_button(ui, command).clicked() {
                        apply_editor_command(
                            ui.ctx(),
                            editor_id,
                            editor,
                            draft,
                            stacker_editor_command(command.id),
                        );
                    }
                }
                ui.add_space(8.0);
                if ui
                    .add_enabled(
                        editor.can_undo(),
                        egui::Button::new(small(
                            stacker_command_descriptor(StackerCommandId::Undo).toolbar_label,
                        ))
                        .min_size(egui::vec2(48.0, 24.0)),
                    )
                    .clicked()
                {
                    apply_editor_command(
                        ui.ctx(),
                        editor_id,
                        editor,
                        draft,
                        stacker_editor_command(StackerCommandId::Undo),
                    );
                }
                if ui
                    .add_enabled(
                        editor.can_redo(),
                        egui::Button::new(small(
                            stacker_command_descriptor(StackerCommandId::Redo).toolbar_label,
                        ))
                        .min_size(egui::vec2(48.0, 24.0)),
                    )
                    .clicked()
                {
                    apply_editor_command(
                        ui.ctx(),
                        editor_id,
                        editor,
                        draft,
                        stacker_editor_command(StackerCommandId::Redo),
                    );
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
                    if draft.is_dirty() {
                        *pending_switch = Some(PendingStackerDraftSwitch::Scratch);
                    } else {
                        start_scratch_prompt(ui.ctx(), editor, draft, editing);
                    }
                }
                if ui.button(small("Reset Cache")).clicked() {
                    stacker_cursor::reset_text_edit_state(ui.ctx(), editor_id);
                }
                ui.add_space(8.0);
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
                        !editor.text().trim().is_empty(),
                        egui::Button::new(small(save_label)),
                    )
                    .clicked()
                {
                    if let Some(idx) = *editing {
                        if let Some(prompt) = prompts.get_mut(idx) {
                            prompt.text = editor.text().trim().to_string();
                            prompt.label = prompt_label(&prompt.text);
                            prompt.category.clear();
                            draft.load_saved_prompt(idx, prompt.text.clone());
                            *dirty = true;
                        }
                    } else if let Some(prompt) = new_prompt(editor.text(), "") {
                        prompts.push(prompt);
                        *editing = Some(prompts.len() - 1);
                        if let Some(idx) = *editing {
                            if let Some(prompt) = prompts.get(idx) {
                                draft.load_saved_prompt(idx, prompt.text.clone());
                            }
                        }
                        *dirty = true;
                    }
                }
            });

            render_editor_status(ui, editor, draft, *editing);
            ui.add_space(6.0);
            render_text_edit_prompt(ui, editor_id, editor, draft, *editor_font_size);
        },
    );
}

fn render_text_edit_prompt(
    ui: &mut egui::Ui,
    editor_id: egui::Id,
    editor: &mut StackerDocumentEditor,
    draft: &mut StackerDraft,
    editor_font_size: f32,
) {
    let note_h = ui.available_height().max(1.0);
    egui::Frame::none()
        .fill(NOTE_BG)
        .rounding(egui::Rounding::same(3.0))
        .inner_margin(egui::Margin::same(NOTE_PADDING))
        .show(ui, |ui| {
            ui.set_height(note_h);
            ui.scope(|ui| {
                let before_edit = editor.text().to_string();
                let before_selection = editor.selection();
                ui.visuals_mut().extreme_bg_color = egui::Color32::TRANSPARENT;
                ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                ui.visuals_mut().widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                ui.visuals_mut().widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                ui.visuals_mut().widgets.inactive.bg_stroke = egui::Stroke::NONE;
                ui.visuals_mut().widgets.hovered.bg_stroke = egui::Stroke::NONE;
                ui.visuals_mut().widgets.active.bg_stroke = egui::Stroke::NONE;
                ui.set_min_size(egui::vec2(ui.available_width(), ui.available_height()));
                let mut output = egui::TextEdit::multiline(editor.text_mut_for_widget())
                    .id(editor_id)
                    .desired_rows(32)
                    .desired_width(f32::INFINITY)
                    .hint_text("Write your prompt here...")
                    .font(stacker_editor_font(editor_font_size))
                    .text_color(NOTE_TEXT)
                    .frame(false)
                    .show(ui);

                if let Some(cursor_range) = output.cursor_range {
                    let cursor_range = cursor_range.as_ccursor_range();
                    let mut selection = stacker_cursor::selection_from_cursor_range(cursor_range);
                    if output.response.changed() {
                        let cursor_idx = cursor_range.primary.index;
                        if let Some(new_cursor_idx) = maybe_continue_list(
                            &before_edit,
                            editor.text_mut_for_widget(),
                            cursor_idx,
                        ) {
                            selection =
                                crate::stacker::input::StackerSelection::collapsed(new_cursor_idx);
                            output.state.cursor.set_char_range(Some(
                                egui::text::CCursorRange::one(egui::text::CCursor::new(
                                    new_cursor_idx,
                                )),
                            ));
                            output.state.clone().store(ui.ctx(), editor_id);
                        }
                    }
                    if editor.record_widget_change(before_edit, before_selection, selection) {
                        draft.record_current_text(editor.text().to_string());
                    }
                }
            });
        });
}

fn stacker_toolbar_button(
    ui: &mut egui::Ui,
    command: &crate::stacker::commands::StackerCommandDescriptor,
) -> egui::Response {
    let min_w = match command.id {
        StackerCommandId::OrderedList | StackerCommandId::Heading1 => 32.0,
        StackerCommandId::CodeBlock | StackerCommandId::ChecklistItem => 38.0,
        _ => 28.0,
    };
    let size = if command.id == StackerCommandId::UnorderedList {
        16.0
    } else {
        13.0
    };
    ui.add(
        egui::Button::new(
            egui::RichText::new(command.toolbar_label)
                .size(size)
                .strong()
                .color(NOTE_TEXT),
        )
        .min_size(egui::vec2(min_w, 24.0)),
    )
    .on_hover_text(command.tooltip)
}

fn render_editor_status(
    ui: &mut egui::Ui,
    editor: &StackerDocumentEditor,
    draft: &StackerDraft,
    editing: Option<usize>,
) {
    let selected = editor.selection().start.abs_diff(editor.selection().end);
    let words = editor.text().split_whitespace().count();
    let lines = editor.text().lines().count().max(1);
    let source = match editing {
        Some(idx) if draft.active_prompt_index() == Some(idx) => format!("Prompt {}", idx + 1),
        Some(idx) => format!("Prompt {}", idx + 1),
        None => "Scratch".to_string(),
    };
    let dirty = if draft.is_dirty() { "Unsaved" } else { "Saved" };

    ui.horizontal_wrapped(|ui| {
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(source)
                .size(11.0)
                .color(egui::Color32::from_rgb(150, 155, 166)),
        );
        ui.label(
            egui::RichText::new(dirty)
                .size(11.0)
                .color(if draft.is_dirty() {
                    egui::Color32::from_rgb(229, 192, 123)
                } else {
                    MUTED
                }),
        );
        ui.label(
            egui::RichText::new(format!("{} chars", editor.char_count()))
                .size(11.0)
                .color(MUTED),
        );
        ui.label(
            egui::RichText::new(format!("{words} words"))
                .size(11.0)
                .color(MUTED),
        );
        ui.label(
            egui::RichText::new(format!("{lines} lines"))
                .size(11.0)
                .color(MUTED),
        );
        if selected > 0 {
            ui.label(
                egui::RichText::new(format!("{selected} selected"))
                    .size(11.0)
                    .color(MUTED),
            );
        }
    });
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

fn request_load_saved_prompt(
    ctx: &egui::Context,
    prompts: &[StackerPrompt],
    editor: &mut StackerDocumentEditor,
    draft: &mut StackerDraft,
    pending_switch: &mut Option<PendingStackerDraftSwitch>,
    editing: &mut Option<usize>,
    index: usize,
) {
    if draft.active_prompt_index() == Some(index) {
        stacker_cursor::store_document_selection(
            ctx,
            egui::Id::new(STACKER_PROMPT_EDITOR_ID),
            editor,
            editor.selection(),
        );
    } else if draft.switching_to_saved_prompt_would_discard_changes(index) {
        *pending_switch = Some(PendingStackerDraftSwitch::SavedPrompt(index));
    } else {
        load_saved_prompt_into_editor(ctx, prompts, editor, draft, editing, index);
    }
}

fn render_delete_prompt_modal(
    ctx: &egui::Context,
    prompts: &mut Vec<StackerPrompt>,
    editor: &mut StackerDocumentEditor,
    draft: &mut StackerDraft,
    editing: &mut Option<usize>,
    dirty: &mut bool,
    pending_delete: &mut Option<usize>,
) {
    let Some(index) = *pending_delete else {
        return;
    };
    let Some(prompt) = prompts.get(index) else {
        *pending_delete = None;
        return;
    };

    let mut confirm = false;
    let mut cancel = false;
    let currently_editing = draft.active_prompt_index() == Some(index);
    let title = truncate_line(&prompt.label, 54);
    egui::Window::new("Delete saved prompt?")
        .id(egui::Id::new("stacker_delete_prompt_modal"))
        .fixed_pos(egui::pos2(
            ctx.screen_rect().center().x - 180.0,
            ctx.screen_rect().center().y - 64.0,
        ))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.set_width(360.0);
            ui.label(
                egui::RichText::new(format!("Delete \"{title}\"? This cannot be undone."))
                    .size(13.0)
                    .color(egui::Color32::from_rgb(210, 215, 225)),
            );
            if currently_editing && draft.is_dirty() {
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(
                        "The open editor has unsaved changes for this prompt. Deleting it will discard that draft.",
                    )
                    .size(12.0)
                    .color(egui::Color32::from_rgb(229, 192, 123)),
                );
            }
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Delete prompt")
                                .size(12.0)
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(180, 50, 50)),
                    )
                    .clicked()
                {
                    confirm = true;
                }
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Cancel")
                                .size(12.0)
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(50, 52, 62)),
                    )
                    .clicked()
                {
                    cancel = true;
                }
            });
            if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                cancel = true;
            }
        });

    if confirm {
        delete_saved_prompt(ctx, prompts, editor, draft, editing, dirty, index);
        *pending_delete = None;
    } else if cancel {
        *pending_delete = None;
    }
}

fn delete_saved_prompt(
    ctx: &egui::Context,
    prompts: &mut Vec<StackerPrompt>,
    editor: &mut StackerDocumentEditor,
    draft: &mut StackerDraft,
    editing: &mut Option<usize>,
    dirty: &mut bool,
    index: usize,
) {
    if index >= prompts.len() {
        return;
    }
    prompts.remove(index);
    *dirty = true;

    match *editing {
        Some(active) if active == index => {
            start_scratch_prompt(ctx, editor, draft, editing);
        }
        Some(active) if active > index => {
            *editing = Some(active - 1);
            draft.shift_saved_prompt_index_after_delete(index);
        }
        _ => {
            draft.shift_saved_prompt_index_after_delete(index);
        }
    }
}

fn render_discard_draft_modal(
    ctx: &egui::Context,
    prompts: &[StackerPrompt],
    editor: &mut StackerDocumentEditor,
    draft: &mut StackerDraft,
    pending_switch: &mut Option<PendingStackerDraftSwitch>,
    editing: &mut Option<usize>,
) {
    let Some(target) = *pending_switch else {
        return;
    };

    let mut discard = false;
    let mut cancel = false;
    egui::Window::new("Discard changes?")
        .id(egui::Id::new("stacker_discard_draft_modal"))
        .fixed_pos(egui::pos2(
            ctx.screen_rect().center().x - 170.0,
            ctx.screen_rect().center().y - 58.0,
        ))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.set_width(340.0);
            ui.label(
                egui::RichText::new("This draft has unsaved changes. Switching will discard them.")
                    .size(13.0)
                    .color(egui::Color32::from_rgb(210, 215, 225)),
            );
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Discard changes")
                                .size(12.0)
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(180, 50, 50)),
                    )
                    .clicked()
                {
                    discard = true;
                }
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Cancel")
                                .size(12.0)
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(50, 52, 62)),
                    )
                    .clicked()
                {
                    cancel = true;
                }
            });
            if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                cancel = true;
            }
        });

    if discard {
        match target {
            PendingStackerDraftSwitch::Scratch => {
                start_scratch_prompt(ctx, editor, draft, editing);
            }
            PendingStackerDraftSwitch::SavedPrompt(index) => {
                load_saved_prompt_into_editor(ctx, prompts, editor, draft, editing, index);
            }
        }
        *pending_switch = None;
    } else if cancel {
        *pending_switch = None;
    }
}

fn load_saved_prompt_into_editor(
    ctx: &egui::Context,
    prompts: &[StackerPrompt],
    editor: &mut StackerDocumentEditor,
    draft: &mut StackerDraft,
    editing: &mut Option<usize>,
    index: usize,
) {
    let Some(prompt) = prompts.get(index) else {
        return;
    };
    *editing = Some(index);
    editor.set_text(prompt.text.clone());
    draft.load_saved_prompt(index, prompt.text.clone());
    let cursor = editor.char_count();
    stacker_cursor::store_document_cursor(
        ctx,
        egui::Id::new(STACKER_PROMPT_EDITOR_ID),
        editor,
        cursor,
    );
}

fn start_scratch_prompt(
    ctx: &egui::Context,
    editor: &mut StackerDocumentEditor,
    draft: &mut StackerDraft,
    editing: &mut Option<usize>,
) {
    *editing = None;
    editor.clear();
    draft.start_scratch();
    stacker_cursor::reset_text_edit_state(ctx, egui::Id::new(STACKER_PROMPT_EDITOR_ID));
}

fn apply_editor_command(
    ctx: &egui::Context,
    editor_id: egui::Id,
    editor: &mut StackerDocumentEditor,
    draft: &mut StackerDraft,
    command: StackerEditorCommand,
) -> bool {
    let selection =
        stacker_cursor::current_selection(ctx, editor_id, editor.selection(), editor.char_count());
    let outcome = execute_stacker_command_at(editor, selection, command);
    stacker_cursor::store_document_selection(ctx, editor_id, editor, outcome.selection);
    if outcome.changed {
        draft.record_current_text(editor.text().to_string());
    }
    outcome.changed
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
