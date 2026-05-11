use crate::config::Config;
use crate::editor::syntax::SyntaxEngine;
use crate::editor::BufferView;
use crate::stacker::{
    draft::StackerDraft, queue::QueuedPrompt, session::StackerSession, StackerPrompt,
};

use super::{
    stacker_state::{PendingStackerDraftSwitch, PendingStackerPromptDelete, StackerPromptViewMode},
    STACKER_PROMPT_EDITOR_ID,
};

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
    inbox_prompts: &mut Vec<StackerPrompt>,
    editor: &mut StackerSession,
    draft: &mut StackerDraft,
    pending_switch: &mut Option<PendingStackerDraftSwitch>,
    pending_delete: &mut Option<PendingStackerPromptDelete>,
    editing: &mut Option<usize>,
    edit_text: &mut String,
    dirty: &mut bool,
    _saved_edit_idx: &mut Option<usize>,
    editor_font_size: &mut f32,
    prompt_editor_rect: &mut Option<egui::Rect>,
    prompt_editor_anchor: &mut Option<(std::sync::Arc<egui::Galley>, egui::Pos2)>,
    queued_prompts: &mut Vec<QueuedPrompt>,
    prompt_view_mode: &mut StackerPromptViewMode,
    config: &Config,
    prose_view: &mut BufferView,
    prose_syntax: &SyntaxEngine,
) {
    *prompt_editor_rect = None;
    *prompt_editor_anchor = None;

    if let Some(idx) = *editing {
        if idx >= prompts.len() {
            *editing = None;
            edit_text.clear();
        }
    }
    if pending_switch
        .as_ref()
        .is_some_and(|target| matches!(target, PendingStackerDraftSwitch::SavedPrompt(idx) if *idx >= prompts.len()))
    {
        *pending_switch = None;
    }
    if pending_switch.as_ref().is_some_and(|target| {
        matches!(
            target,
            PendingStackerDraftSwitch::InboxPrompt(id)
                if !inbox_prompts.iter().any(|prompt| prompt.id.as_deref() == Some(id.as_str()))
        )
    }) {
        *pending_switch = None;
    }
    if pending_delete.as_ref().is_some_and(
        |target| matches!(target, PendingStackerPromptDelete::Saved(idx) if *idx >= prompts.len()),
    ) {
        *pending_delete = None;
    }
    if pending_delete.as_ref().is_some_and(|target| {
        matches!(
            target,
            PendingStackerPromptDelete::Inbox(id)
                if !inbox_prompts.iter().any(|prompt| prompt.id.as_deref() == Some(id.as_str()))
        )
    }) {
        *pending_delete = None;
    }

    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.add_space(12.0);
        ui.label(
            egui::RichText::new("Stacker")
                .size(20.0)
                .color(layout::HEADING_COLOR),
        );
        ui.add_space(10.0);
        if ui
            .add_sized(
                [104.0, 24.0],
                egui::Button::new(egui::RichText::new(prompt_view_mode.toggle_label()).size(12.0)),
            )
            .on_hover_text("Switch saved prompts view")
            .clicked()
        {
            prompt_view_mode.toggle();
        }
    });

    ui.add_space(10.0);

    let available_h = ui.available_height();
    let list_h = (available_h * 0.34).clamp(160.0, 300.0);

    prompts::render_prompt_list_panel(
        ui,
        list_h,
        prompts,
        inbox_prompts,
        editing,
        editor,
        draft,
        pending_switch,
        pending_delete,
        queued_prompts,
        *prompt_view_mode,
    );

    ui.add_space(12.0);

    let editor_h = (ui.available_height() - layout::EDITOR_BOTTOM_GAP).max(1.0);
    editor_panel::render_prompt_editor_panel(
        ui,
        editor_h,
        prompts,
        inbox_prompts,
        editor,
        draft,
        pending_switch,
        editing,
        dirty,
        editor_font_size,
        prompt_editor_rect,
        prompt_editor_anchor,
        config,
        prose_view,
        prose_syntax,
    );

    modals::render_discard_draft_modal(
        ui.ctx(),
        prompts,
        inbox_prompts,
        editor,
        draft,
        pending_switch,
        editing,
    );
    modals::render_delete_prompt_modal(
        ui.ctx(),
        prompts,
        inbox_prompts,
        editor,
        draft,
        editing,
        dirty,
        pending_delete,
    );
}
