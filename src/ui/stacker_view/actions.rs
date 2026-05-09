use crate::stacker::{document::StackerDocumentEditor, draft::StackerDraft, StackerPrompt};

use super::{PendingStackerDraftSwitch, STACKER_PROMPT_EDITOR_ID};
use crate::ui::stacker_cursor;

pub(super) fn request_load_saved_prompt(
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

pub(super) fn request_load_inbox_prompt(
    ctx: &egui::Context,
    inbox_prompts: &[StackerPrompt],
    editor: &mut StackerDocumentEditor,
    draft: &mut StackerDraft,
    pending_switch: &mut Option<PendingStackerDraftSwitch>,
    editing: &mut Option<usize>,
    index: usize,
) {
    let Some(prompt) = inbox_prompts.get(index) else {
        return;
    };
    let Some(id) = prompt.id.as_deref() else {
        return;
    };
    if draft.active_inbox_id() == Some(id) {
        stacker_cursor::store_document_selection(
            ctx,
            egui::Id::new(STACKER_PROMPT_EDITOR_ID),
            editor,
            editor.selection(),
        );
    } else if draft.switching_to_inbox_prompt_would_discard_changes(id) {
        *pending_switch = Some(PendingStackerDraftSwitch::InboxPrompt(id.to_string()));
    } else {
        load_inbox_prompt_into_editor(ctx, inbox_prompts, editor, draft, editing, index);
    }
}

pub(super) fn load_saved_prompt_into_editor(
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

pub(super) fn load_inbox_prompt_into_editor(
    ctx: &egui::Context,
    inbox_prompts: &[StackerPrompt],
    editor: &mut StackerDocumentEditor,
    draft: &mut StackerDraft,
    editing: &mut Option<usize>,
    index: usize,
) {
    let Some(prompt) = inbox_prompts.get(index) else {
        return;
    };
    let Some(id) = prompt.id.as_deref() else {
        return;
    };
    *editing = None;
    editor.set_text(prompt.text.clone());
    draft.load_inbox_prompt(id.to_string(), prompt.text.clone());
    let cursor = editor.char_count();
    stacker_cursor::store_document_cursor(
        ctx,
        egui::Id::new(STACKER_PROMPT_EDITOR_ID),
        editor,
        cursor,
    );
}

pub(super) fn start_scratch_prompt(
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
