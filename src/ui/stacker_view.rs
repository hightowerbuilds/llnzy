use crate::stacker::{
    document::StackerDocumentEditor, draft::StackerDraft, queue::QueuedPrompt, StackerPrompt,
};

use super::{stacker_state::PendingStackerDraftSwitch, STACKER_PROMPT_EDITOR_ID};

mod actions;
mod editor_panel;
mod layout;
mod modals;
mod prompts;
mod toolbar;

pub(crate) const DEFAULT_EDITOR_FONT_SIZE: f32 = 16.0;

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
    web_editor_rect: &mut Option<egui::Rect>,
    queued_prompts: &mut Vec<QueuedPrompt>,
) {
    *web_editor_rect = None;

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

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Stacker")
                .size(20.0)
                .color(layout::HEADING_COLOR),
        );
        ui.label(
            egui::RichText::new("Prompt editor")
                .size(12.0)
                .color(layout::DIM),
        );
    });

    ui.add_space(10.0);

    let available_h = ui.available_height();
    let list_h = (available_h * 0.24).clamp(120.0, 220.0);

    prompts::render_prompt_list_panel(
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

    let editor_h = (ui.available_height() - layout::EDITOR_BOTTOM_GAP).max(1.0);
    editor_panel::render_prompt_editor_panel(
        ui,
        editor_h,
        prompts,
        editor,
        draft,
        pending_switch,
        editing,
        dirty,
        editor_font_size,
        web_editor_rect,
    );

    modals::render_discard_draft_modal(ui.ctx(), prompts, editor, draft, pending_switch, editing);
    modals::render_delete_prompt_modal(
        ui.ctx(),
        prompts,
        editor,
        draft,
        editing,
        dirty,
        pending_delete,
    );
}
