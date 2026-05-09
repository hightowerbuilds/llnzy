use crate::stacker::{
    archive_inbox_prompt, archive_saved_prompt, document::StackerDocumentEditor,
    draft::StackerDraft, StackerPrompt,
};

use super::{
    actions::{load_inbox_prompt_into_editor, load_saved_prompt_into_editor, start_scratch_prompt},
    layout::truncate_line,
    PendingStackerDraftSwitch, PendingStackerPromptDelete,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn render_delete_prompt_modal(
    ctx: &egui::Context,
    prompts: &mut Vec<StackerPrompt>,
    inbox_prompts: &mut Vec<StackerPrompt>,
    editor: &mut StackerDocumentEditor,
    draft: &mut StackerDraft,
    editing: &mut Option<usize>,
    dirty: &mut bool,
    pending_delete: &mut Option<PendingStackerPromptDelete>,
) {
    let Some(target) = pending_delete.clone() else {
        return;
    };
    let Some(prompt) = delete_target_prompt(&target, prompts, inbox_prompts) else {
        *pending_delete = None;
        return;
    };

    let mut confirm = false;
    let mut cancel = false;
    let currently_editing = match &target {
        PendingStackerPromptDelete::Saved(index) => draft.active_prompt_index() == Some(*index),
        PendingStackerPromptDelete::Inbox(id) => draft.active_inbox_id() == Some(id.as_str()),
    };
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
        match target {
            PendingStackerPromptDelete::Saved(index) => {
                delete_saved_prompt(ctx, prompts, editor, draft, editing, dirty, index);
            }
            PendingStackerPromptDelete::Inbox(id) => {
                delete_inbox_prompt(ctx, inbox_prompts, editor, draft, editing, &id);
            }
        }
        *pending_delete = None;
    } else if cancel {
        *pending_delete = None;
    }
}

fn delete_target_prompt<'a>(
    target: &PendingStackerPromptDelete,
    prompts: &'a [StackerPrompt],
    inbox_prompts: &'a [StackerPrompt],
) -> Option<&'a StackerPrompt> {
    match target {
        PendingStackerPromptDelete::Saved(index) => prompts.get(*index),
        PendingStackerPromptDelete::Inbox(id) => inbox_prompts
            .iter()
            .find(|prompt| prompt.id.as_deref() == Some(id.as_str())),
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
    if let Some(prompt) = prompts.get(index) {
        if prompt.id.is_some() {
            if let Err(err) = archive_saved_prompt(prompt) {
                log::warn!("failed to archive saved prompt before delete: {err}");
            }
        }
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

fn delete_inbox_prompt(
    ctx: &egui::Context,
    inbox_prompts: &mut Vec<StackerPrompt>,
    editor: &mut StackerDocumentEditor,
    draft: &mut StackerDraft,
    editing: &mut Option<usize>,
    id: &str,
) {
    let Some(index) = inbox_prompts
        .iter()
        .position(|prompt| prompt.id.as_deref() == Some(id))
    else {
        return;
    };
    let prompt = inbox_prompts[index].clone();
    if let Err(err) = archive_inbox_prompt(&prompt) {
        log::warn!("failed to archive inbox prompt {id}: {err}");
        return;
    }
    inbox_prompts.remove(index);
    if draft.active_inbox_id() == Some(id) {
        start_scratch_prompt(ctx, editor, draft, editing);
    }
}

pub(super) fn render_discard_draft_modal(
    ctx: &egui::Context,
    prompts: &[StackerPrompt],
    inbox_prompts: &[StackerPrompt],
    editor: &mut StackerDocumentEditor,
    draft: &mut StackerDraft,
    pending_switch: &mut Option<PendingStackerDraftSwitch>,
    editing: &mut Option<usize>,
) {
    let Some(target) = pending_switch.clone() else {
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
            PendingStackerDraftSwitch::InboxPrompt(id) => {
                if let Some(index) = inbox_prompts
                    .iter()
                    .position(|prompt| prompt.id.as_deref() == Some(id.as_str()))
                {
                    load_inbox_prompt_into_editor(
                        ctx,
                        inbox_prompts,
                        editor,
                        draft,
                        editing,
                        index,
                    );
                }
            }
        }
        *pending_switch = None;
    } else if cancel {
        *pending_switch = None;
    }
}
