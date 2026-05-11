use crate::stacker::{
    commands::{
        execute_stacker_command_at, stacker_command_descriptor, stacker_command_registry,
        stacker_editor_command, StackerCommandId, StackerEditorCommand,
    },
    draft::StackerDraft,
    import_prompts, merge_unique_prompts, new_prompt, promote_inbox_prompt, prompt_label,
    session::StackerSession,
    stacker_path, StackerPrompt,
};

use super::{
    actions::start_scratch_prompt,
    layout::{small, MAX_EDITOR_FONT_SIZE, MIN_EDITOR_FONT_SIZE, NOTE_TEXT},
    PendingStackerDraftSwitch,
};
use crate::ui::stacker_cursor;

#[allow(clippy::too_many_arguments)]
pub(super) fn render_editor_toolbar(
    ui: &mut egui::Ui,
    editor_id: egui::Id,
    prompts: &mut Vec<StackerPrompt>,
    inbox_prompts: &mut Vec<StackerPrompt>,
    editor: &mut StackerSession,
    draft: &mut StackerDraft,
    pending_switch: &mut Option<PendingStackerDraftSwitch>,
    editing: &mut Option<usize>,
    dirty: &mut bool,
    editor_font_size: &mut f32,
) {
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
            } else if let Some(id) = draft.active_inbox_id().map(str::to_string) {
                if let Some(inbox_idx) = inbox_prompts
                    .iter()
                    .position(|prompt| prompt.id.as_deref() == Some(id.as_str()))
                {
                    let mut prompt = inbox_prompts[inbox_idx].clone();
                    prompt.text = editor.text().trim().to_string();
                    prompt.label = prompt_label(&prompt.text);
                    prompt.category.clear();
                    match promote_inbox_prompt(&prompt) {
                        Ok(saved) => {
                            inbox_prompts.remove(inbox_idx);
                            prompts.push(saved);
                            let saved_idx = prompts.len() - 1;
                            *editing = Some(saved_idx);
                            if let Some(prompt) = prompts.get(saved_idx) {
                                draft.load_saved_prompt(saved_idx, prompt.text.clone());
                            }
                            *dirty = true;
                        }
                        Err(err) => log::warn!("failed to promote inbox prompt {id}: {err}"),
                    }
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

fn import_prompt_file(prompts: &mut Vec<StackerPrompt>, dirty: &mut bool) {
    let Some(path) = stacker_path() else { return };
    let import_path = path.with_extension("import.json");
    if let Ok(imported) = import_prompts(&import_path) {
        if merge_unique_prompts(prompts, imported) > 0 {
            *dirty = true;
        }
    }
}

fn apply_editor_command(
    ctx: &egui::Context,
    editor_id: egui::Id,
    editor: &mut StackerSession,
    draft: &mut StackerDraft,
    command: StackerEditorCommand,
) -> bool {
    let selection =
        stacker_cursor::current_selection(ctx, editor_id, editor.selection(), editor.char_count());
    let outcome = execute_stacker_command_at(editor, selection, command);
    stacker_cursor::store_document_selection(ctx, editor_id, editor, outcome.selection);
    if outcome.changed {
        draft.record_current_text(editor.text());
    }
    outcome.changed
}
