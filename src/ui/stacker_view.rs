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

pub(crate) struct StackerViewContext<'a> {
    pub(crate) ui: &'a mut egui::Ui,
    pub(crate) prompts: &'a mut Vec<StackerPrompt>,
    pub(crate) inbox_prompts: &'a mut Vec<StackerPrompt>,
    pub(crate) editor: &'a mut StackerSession,
    pub(crate) draft: &'a mut StackerDraft,
    pub(crate) pending_switch: &'a mut Option<PendingStackerDraftSwitch>,
    pub(crate) pending_delete: &'a mut Option<PendingStackerPromptDelete>,
    pub(crate) editing: &'a mut Option<usize>,
    pub(crate) edit_text: &'a mut String,
    pub(crate) dirty: &'a mut bool,
    pub(crate) editor_font_size: &'a mut f32,
    pub(crate) prompt_editor_rect: &'a mut Option<egui::Rect>,
    pub(crate) prompt_editor_anchor: &'a mut Option<(std::sync::Arc<egui::Galley>, egui::Pos2)>,
    pub(crate) queued_prompts: &'a mut Vec<QueuedPrompt>,
    pub(crate) prompt_view_mode: &'a mut StackerPromptViewMode,
    pub(crate) config: &'a Config,
    pub(crate) prose_view: &'a mut BufferView,
    pub(crate) prose_syntax: &'a SyntaxEngine,
}

/// Render the Stacker (prompt queue) view -- minimalist flat-list design.
///
/// Takes ownership of mutable string state for closure-friendliness,
/// writing values back through the mutable references on return.
pub(crate) fn render_stacker_view(input: StackerViewContext<'_>) {
    let StackerViewContext {
        ui,
        prompts,
        inbox_prompts,
        editor,
        draft,
        pending_switch,
        pending_delete,
        editing,
        edit_text,
        dirty,
        editor_font_size,
        prompt_editor_rect,
        prompt_editor_anchor,
        queued_prompts,
        prompt_view_mode,
        config,
        prose_view,
        prose_syntax,
    } = input;

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
        prompts::PromptListContext {
            prompts: &mut *prompts,
            inbox_prompts: &mut *inbox_prompts,
            editing: &mut *editing,
            editor: &mut *editor,
            draft: &mut *draft,
            pending_switch: &mut *pending_switch,
            pending_delete: &mut *pending_delete,
            queued_prompts: &mut *queued_prompts,
            view_mode: *prompt_view_mode,
        },
    );

    ui.add_space(12.0);

    let editor_h = (ui.available_height() - layout::EDITOR_BOTTOM_GAP).max(1.0);
    editor_panel::render_prompt_editor_panel(
        ui,
        editor_h,
        editor_panel::EditorPanelContext {
            prompts: &mut *prompts,
            inbox_prompts: &mut *inbox_prompts,
            editor: &mut *editor,
            draft: &mut *draft,
            pending_switch: &mut *pending_switch,
            editing: &mut *editing,
            dirty: &mut *dirty,
            editor_font_size: &mut *editor_font_size,
            prompt_editor_rect: &mut *prompt_editor_rect,
            prompt_editor_anchor: &mut *prompt_editor_anchor,
            config,
            prose_view: &mut *prose_view,
            prose_syntax,
        },
    );

    modals::render_discard_draft_modal(
        ui.ctx(),
        &*prompts,
        &*inbox_prompts,
        &mut *editor,
        &mut *draft,
        &mut *pending_switch,
        &mut *editing,
    );
    modals::render_delete_prompt_modal(
        ui.ctx(),
        modals::DeletePromptModalContext {
            prompts: &mut *prompts,
            inbox_prompts: &mut *inbox_prompts,
            editor: &mut *editor,
            draft: &mut *draft,
            editing: &mut *editing,
            dirty: &mut *dirty,
            pending_delete: &mut *pending_delete,
        },
    );
}
