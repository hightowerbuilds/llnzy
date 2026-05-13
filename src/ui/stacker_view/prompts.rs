use crate::stacker::{
    draft::StackerDraft,
    promote_inbox_prompt,
    queue::{self, QueuedPrompt},
    session::StackerSession,
    StackerPrompt,
};
use crate::text_utils::truncate_chars;

use super::{
    actions::{request_load_inbox_prompt, request_load_saved_prompt},
    layout::{
        header_label, small, truncate_line, DIM, HEADING_COLOR, MUTED, PANEL_BG, QUEUE_GREEN,
        ROW_BG, ROW_HOVER,
    },
    PendingStackerDraftSwitch, PendingStackerPromptDelete, StackerPromptViewMode,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SavedPromptRowAction {
    Open,
    Edit,
    Delete,
}

const THUMBNAIL_CARD_WIDTH: f32 = 276.0;
const THUMBNAIL_PREVIEW_WIDTH: f32 = 142.0;
const THUMBNAIL_PREVIEW_HEIGHT: f32 = 112.0;
const THUMBNAIL_ACTION_WIDTH: f32 = THUMBNAIL_CARD_WIDTH - 16.0;
const THUMBNAIL_BUTTON_WIDTH: f32 = 82.0;
const THUMBNAIL_BUTTON_HEIGHT: f32 = 24.0;
const THUMBNAIL_ROW_HEIGHT: f32 = THUMBNAIL_PREVIEW_HEIGHT + 32.0;
const QUEUED_BUTTON_FILL: egui::Color32 = egui::Color32::from_rgb(24, 58, 32);
const QUEUED_BUTTON_STROKE: egui::Color32 = egui::Color32::from_rgb(63, 214, 99);
const QUEUED_BUTTON_TEXT: egui::Color32 = egui::Color32::from_rgb(95, 255, 130);
const DEFAULT_BUTTON_TEXT: egui::Color32 = egui::Color32::from_rgb(210, 210, 216);
const LIST_QUEUE_BUTTON_WIDTH: f32 = 104.0;
const LIST_EDIT_BUTTON_WIDTH: f32 = 44.0;
const LIST_DELETE_BUTTON_WIDTH: f32 = 56.0;
const LIST_CHARS_WIDTH: f32 = 48.0;

pub(super) struct PromptListContext<'a> {
    pub(super) prompts: &'a mut Vec<StackerPrompt>,
    pub(super) inbox_prompts: &'a mut Vec<StackerPrompt>,
    pub(super) editing: &'a mut Option<usize>,
    pub(super) editor: &'a mut StackerSession,
    pub(super) draft: &'a mut StackerDraft,
    pub(super) pending_switch: &'a mut Option<PendingStackerDraftSwitch>,
    pub(super) pending_delete: &'a mut Option<PendingStackerPromptDelete>,
    pub(super) queued_prompts: &'a mut Vec<QueuedPrompt>,
    pub(super) view_mode: StackerPromptViewMode,
}

pub(super) fn render_prompt_list_panel(
    ui: &mut egui::Ui,
    height: f32,
    mut state: PromptListContext<'_>,
) {
    queue::sanitize_prompt_queue(state.queued_prompts);

    egui::Frame::none()
        .fill(PANEL_BG)
        .rounding(egui::Rounding::same(4.0))
        .inner_margin(egui::Margin::same(10.0))
        .show(ui, |ui| {
            ui.set_height(height);
            ui.horizontal_wrapped(|ui| {
                let queue_w = 190.0_f32.min((ui.available_width() * 0.25).max(150.0));
                ui.vertical(|ui| {
                    ui.set_width(queue_w);
                    render_queue_bank(ui, queue_w, height, &*state.queued_prompts);
                });

                ui.add_space(10.0);

                ui.vertical(|ui| {
                    render_saved_prompt_panel(ui, height, &mut state);
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
                        let label = prompt.label.lines().next().unwrap_or("").to_uppercase();
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(26, 32, 26))
                            .rounding(egui::Rounding::same(3.0))
                            .inner_margin(egui::Margin::symmetric(7.0, 5.0))
                            .show(ui, |ui| {
                                ui.add_sized(
                                    [ui.available_width(), 18.0],
                                    egui::Label::new(
                                        egui::RichText::new(label)
                                            .size(12.0)
                                            .strong()
                                            .color(QUEUE_GREEN),
                                    )
                                    .truncate(),
                                )
                                .on_hover_text(&prompt.label);
                            });
                    }
                });
        });
}

fn render_saved_prompt_panel(ui: &mut egui::Ui, height: f32, state: &mut PromptListContext<'_>) {
    ui.set_height(height - 20.0);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("Saved prompts ({})", state.prompts.len()))
                .size(13.0)
                .color(HEADING_COLOR),
        );
    });
    ui.add_space(8.0);

    if state.prompts.is_empty() && state.inbox_prompts.is_empty() {
        ui.add_space(20.0);
        ui.label(small("No prompts yet.").color(DIM));
        return;
    }

    match state.view_mode {
        StackerPromptViewMode::List => render_saved_prompt_list(ui, state),
        StackerPromptViewMode::Thumbnails => render_saved_prompt_thumbnails(ui, state),
    }
}

fn render_saved_prompt_list(ui: &mut egui::Ui, state: &mut PromptListContext<'_>) {
    let prompts = &mut *state.prompts;
    let inbox_prompts = &mut *state.inbox_prompts;
    let editing = &mut *state.editing;
    let editor = &mut *state.editor;
    let draft = &mut *state.draft;
    let pending_switch = &mut *state.pending_switch;
    let pending_delete = &mut *state.pending_delete;
    let queued_prompts = &mut *state.queued_prompts;

    ui.horizontal(|ui| {
        ui.add_sized([LIST_QUEUE_BUTTON_WIDTH, 18.0], header_label("Queue"));
        ui.add_sized([LIST_EDIT_BUTTON_WIDTH, 18.0], header_label("Edit"));
        ui.add_sized([LIST_DELETE_BUTTON_WIDTH, 18.0], header_label("Delete"));
        ui.add_sized([LIST_CHARS_WIDTH, 18.0], header_label("Chars"));
        ui.add_sized(
            [ui.available_width().max(40.0), 18.0],
            header_label("Prompt"),
        );
    });
    ui.separator();

    egui::ScrollArea::vertical()
        .id_salt("stacker_prompt_list")
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 3.0;

            if !inbox_prompts.is_empty() {
                ui.label(
                    egui::RichText::new(format!("From agent ({})", inbox_prompts.len()))
                        .size(11.0)
                        .italics()
                        .color(MUTED),
                );
                ui.add_space(2.0);

                let mut i = 0;
                while i < inbox_prompts.len() {
                    let prompt = &inbox_prompts[i];
                    let selected = draft.active_inbox_id() == prompt.id.as_deref();
                    let mut row_action = None;
                    let mut queue_clicked = false;
                    let mut child_button_clicked = false;
                    let row_resp = egui::Frame::none()
                        .fill(if selected { ROW_HOVER } else { ROW_BG })
                        .rounding(egui::Rounding::same(3.0))
                        .inner_margin(egui::Margin::symmetric(8.0, 5.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let already_queued = queue::contains_prompt(queued_prompts, prompt);
                                let can_queue = already_queued
                                    || queued_prompts.len() < queue::MAX_QUEUE_PROMPTS;
                                let label = if already_queued {
                                    "Queued"
                                } else {
                                    "Add to queue"
                                };
                                if ui
                                    .add_enabled(
                                        can_queue,
                                        queue_button(label, already_queued, 12.0)
                                            .min_size(egui::vec2(LIST_QUEUE_BUTTON_WIDTH, 22.0)),
                                    )
                                    .clicked()
                                {
                                    child_button_clicked = true;
                                    queue_clicked = true;
                                }
                                if ui
                                    .add(
                                        egui::Button::new(egui::RichText::new("Edit").size(12.0))
                                            .min_size(egui::vec2(LIST_EDIT_BUTTON_WIDTH, 22.0)),
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
                                        .min_size(egui::vec2(LIST_DELETE_BUTTON_WIDTH, 22.0)),
                                    )
                                    .clicked()
                                {
                                    child_button_clicked = true;
                                    row_action = Some(SavedPromptRowAction::Delete);
                                }
                                ui.add_sized(
                                    [LIST_CHARS_WIDTH, 20.0],
                                    egui::Label::new(
                                        egui::RichText::new(
                                            prompt.text.chars().count().to_string(),
                                        )
                                        .size(12.0)
                                        .color(MUTED),
                                    ),
                                );
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new("•").size(13.0).color(QUEUE_GREEN),
                                    );
                                    ui.add_sized(
                                        [ui.available_width().max(40.0), 20.0],
                                        egui::Label::new(
                                            egui::RichText::new(truncate_line(&prompt.label, 64))
                                                .size(13.0)
                                                .italics()
                                                .color(egui::Color32::from_rgb(190, 198, 206)),
                                        )
                                        .sense(egui::Sense::click()),
                                    );
                                });
                            });
                        })
                        .response
                        .on_hover_text(inbox_hover_text(prompt));

                    if row_action.is_none() && !child_button_clicked && row_resp.clicked() {
                        row_action = Some(SavedPromptRowAction::Open);
                    }

                    if queue_clicked {
                        if let Some(saved_idx) = promote_inbox_at(prompts, inbox_prompts, i) {
                            if let Some(prompt) = prompts.get(saved_idx) {
                                if !queue::contains_prompt(queued_prompts, prompt) {
                                    queue::add_prompt(queued_prompts, prompt);
                                }
                            }
                            continue;
                        }
                    }

                    match row_action {
                        Some(SavedPromptRowAction::Open | SavedPromptRowAction::Edit) => {
                            request_load_inbox_prompt(
                                ui.ctx(),
                                inbox_prompts,
                                editor,
                                draft,
                                pending_switch,
                                editing,
                                i,
                            );
                        }
                        Some(SavedPromptRowAction::Delete) => {
                            if let Some(id) = inbox_prompts[i].id.clone() {
                                *pending_delete = Some(PendingStackerPromptDelete::Inbox(id));
                            }
                        }
                        None => {}
                    }
                    i += 1;
                }

                ui.add_space(5.0);
                ui.separator();
                ui.add_space(5.0);
            }

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
                            let already_queued = queue::contains_prompt(queued_prompts, prompt);
                            let can_toggle =
                                already_queued || queued_prompts.len() < queue::MAX_QUEUE_PROMPTS;
                            let label = if already_queued {
                                "Queued"
                            } else {
                                "Add to queue"
                            };
                            if ui
                                .add_enabled(
                                    can_toggle,
                                    queue_button(label, already_queued, 12.0)
                                        .min_size(egui::vec2(LIST_QUEUE_BUTTON_WIDTH, 22.0)),
                                )
                                .clicked()
                            {
                                child_button_clicked = true;
                                queue::toggle_prompt(queued_prompts, prompt);
                            }
                            if ui
                                .add(
                                    egui::Button::new(egui::RichText::new("Edit").size(12.0))
                                        .min_size(egui::vec2(LIST_EDIT_BUTTON_WIDTH, 22.0)),
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
                                    .min_size(egui::vec2(LIST_DELETE_BUTTON_WIDTH, 22.0)),
                                )
                                .clicked()
                            {
                                child_button_clicked = true;
                                row_action = Some(SavedPromptRowAction::Delete);
                            }
                            ui.add_sized(
                                [LIST_CHARS_WIDTH, 20.0],
                                egui::Label::new(
                                    egui::RichText::new(prompt.text.chars().count().to_string())
                                        .size(12.0)
                                        .color(MUTED),
                                ),
                            );
                            ui.add_sized(
                                [ui.available_width().max(40.0), 20.0],
                                egui::Label::new(
                                    egui::RichText::new(truncate_line(&prompt.label, 64))
                                        .size(13.0)
                                        .color(egui::Color32::from_rgb(210, 210, 216)),
                                )
                                .sense(egui::Sense::click()),
                            );
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
                        *pending_delete = Some(PendingStackerPromptDelete::Saved(i));
                    }
                    None => {}
                }
            }
        });
}

fn render_saved_prompt_thumbnails(ui: &mut egui::Ui, state: &mut PromptListContext<'_>) {
    let prompts = &mut *state.prompts;
    let inbox_prompts = &mut *state.inbox_prompts;
    let editing = &mut *state.editing;
    let editor = &mut *state.editor;
    let draft = &mut *state.draft;
    let pending_switch = &mut *state.pending_switch;
    let pending_delete = &mut *state.pending_delete;
    let queued_prompts = &mut *state.queued_prompts;

    egui::ScrollArea::horizontal()
        .id_salt("stacker_prompt_thumbnails")
        .auto_shrink([false; 2])
        .max_height(THUMBNAIL_ROW_HEIGHT)
        .show(ui, |ui| {
            apply_thumbnail_wheel_scroll(ui);
            ui.set_min_height(THUMBNAIL_ROW_HEIGHT);
            ui.spacing_mut().item_spacing = egui::vec2(10.0, 0.0);
            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                if !inbox_prompts.is_empty() {
                    ui.vertical(|ui| {
                        ui.set_width(96.0);
                        ui.label(
                            egui::RichText::new(format!("From agent ({})", inbox_prompts.len()))
                                .size(11.0)
                                .italics()
                                .color(MUTED),
                        );
                    });

                    let mut i = 0;
                    while i < inbox_prompts.len() {
                        let prompt = &inbox_prompts[i];
                        let selected = draft.active_inbox_id() == prompt.id.as_deref();
                        let mut row_action = None;
                        let mut queue_clicked = false;
                        let mut child_button_clicked = false;
                        let card_resp = egui::Frame::none()
                            .fill(if selected { ROW_HOVER } else { ROW_BG })
                            .rounding(egui::Rounding::same(4.0))
                            .inner_margin(egui::Margin::same(8.0))
                            .show(ui, |ui| {
                                ui.set_min_width(THUMBNAIL_ACTION_WIDTH);
                                ui.set_max_width(THUMBNAIL_ACTION_WIDTH);
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 10.0;
                                    render_prompt_thumbnail_preview(ui, prompt, selected);
                                    ui.with_layout(
                                        egui::Layout::top_down(egui::Align::RIGHT),
                                        |ui| {
                                            ui.set_width(THUMBNAIL_BUTTON_WIDTH);
                                            ui.spacing_mut().item_spacing.y = 6.0;
                                            ui.label(
                                                egui::RichText::new("• agent")
                                                    .size(11.0)
                                                    .italics()
                                                    .color(QUEUE_GREEN),
                                            );
                                            let already_queued =
                                                queue::contains_prompt(queued_prompts, prompt);
                                            let can_queue = already_queued
                                                || queued_prompts.len() < queue::MAX_QUEUE_PROMPTS;
                                            let queue_label =
                                                if already_queued { "Queued" } else { "Queue" };
                                            if ui
                                                .add_enabled(
                                                    can_queue,
                                                    queue_button(queue_label, already_queued, 11.0)
                                                        .min_size(egui::vec2(
                                                            THUMBNAIL_BUTTON_WIDTH,
                                                            THUMBNAIL_BUTTON_HEIGHT,
                                                        )),
                                                )
                                                .clicked()
                                            {
                                                child_button_clicked = true;
                                                queue_clicked = true;
                                            }
                                            if ui
                                                .add(sized_thumbnail_button(
                                                    "Edit",
                                                    egui::Color32::from_rgb(210, 210, 216),
                                                ))
                                                .clicked()
                                            {
                                                child_button_clicked = true;
                                                row_action = Some(SavedPromptRowAction::Edit);
                                            }
                                            if ui
                                                .add(sized_thumbnail_button(
                                                    "Delete",
                                                    egui::Color32::from_rgb(245, 125, 125),
                                                ))
                                                .clicked()
                                            {
                                                child_button_clicked = true;
                                                row_action = Some(SavedPromptRowAction::Delete);
                                            }
                                        },
                                    );
                                });
                            })
                            .response
                            .on_hover_text(inbox_hover_text(prompt));

                        if row_action.is_none() && !child_button_clicked && card_resp.clicked() {
                            row_action = Some(SavedPromptRowAction::Open);
                        }

                        if queue_clicked {
                            if let Some(saved_idx) = promote_inbox_at(prompts, inbox_prompts, i) {
                                if let Some(prompt) = prompts.get(saved_idx) {
                                    if !queue::contains_prompt(queued_prompts, prompt) {
                                        queue::add_prompt(queued_prompts, prompt);
                                    }
                                }
                                continue;
                            }
                        }

                        match row_action {
                            Some(SavedPromptRowAction::Open | SavedPromptRowAction::Edit) => {
                                request_load_inbox_prompt(
                                    ui.ctx(),
                                    inbox_prompts,
                                    editor,
                                    draft,
                                    pending_switch,
                                    editing,
                                    i,
                                );
                            }
                            Some(SavedPromptRowAction::Delete) => {
                                if let Some(id) = inbox_prompts[i].id.clone() {
                                    *pending_delete = Some(PendingStackerPromptDelete::Inbox(id));
                                }
                            }
                            None => {}
                        }
                        i += 1;
                    }

                    ui.separator();
                }

                for i in 0..prompts.len() {
                    let prompt = &prompts[i];
                    let selected = *editing == Some(i);
                    let mut row_action = None;
                    let mut child_button_clicked = false;
                    let card_resp = egui::Frame::none()
                        .fill(if selected { ROW_HOVER } else { ROW_BG })
                        .rounding(egui::Rounding::same(4.0))
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.set_min_width(THUMBNAIL_ACTION_WIDTH);
                            ui.set_max_width(THUMBNAIL_ACTION_WIDTH);
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 10.0;
                                render_prompt_thumbnail_preview(ui, prompt, selected);
                                ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
                                    ui.set_width(THUMBNAIL_BUTTON_WIDTH);
                                    ui.spacing_mut().item_spacing.y = 6.0;
                                    let already_queued =
                                        queue::contains_prompt(queued_prompts, prompt);
                                    let can_toggle = already_queued
                                        || queued_prompts.len() < queue::MAX_QUEUE_PROMPTS;
                                    let queue_label =
                                        if already_queued { "Queued" } else { "Queue" };
                                    if ui
                                        .add_enabled(
                                            can_toggle,
                                            queue_button(queue_label, already_queued, 11.0)
                                                .min_size(egui::vec2(
                                                    THUMBNAIL_BUTTON_WIDTH,
                                                    THUMBNAIL_BUTTON_HEIGHT,
                                                )),
                                        )
                                        .clicked()
                                    {
                                        child_button_clicked = true;
                                        queue::toggle_prompt(queued_prompts, prompt);
                                    }
                                    if ui
                                        .add(sized_thumbnail_button(
                                            "Edit",
                                            egui::Color32::from_rgb(210, 210, 216),
                                        ))
                                        .clicked()
                                    {
                                        child_button_clicked = true;
                                        row_action = Some(SavedPromptRowAction::Edit);
                                    }
                                    if ui
                                        .add(sized_thumbnail_button(
                                            "Delete",
                                            egui::Color32::from_rgb(245, 125, 125),
                                        ))
                                        .clicked()
                                    {
                                        child_button_clicked = true;
                                        row_action = Some(SavedPromptRowAction::Delete);
                                    }
                                });
                            });
                        })
                        .response
                        .on_hover_text(&prompt.label);

                    if row_action.is_none() && !child_button_clicked && card_resp.clicked() {
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
                            *pending_delete = Some(PendingStackerPromptDelete::Saved(i));
                        }
                        None => {}
                    }
                }
            });
        });
}

fn promote_inbox_at(
    prompts: &mut Vec<StackerPrompt>,
    inbox_prompts: &mut Vec<StackerPrompt>,
    inbox_index: usize,
) -> Option<usize> {
    let prompt = inbox_prompts.get(inbox_index)?.clone();
    let id = prompt.id.clone().unwrap_or_else(|| "<missing>".to_string());
    match promote_inbox_prompt(&prompt) {
        Ok(saved) => {
            inbox_prompts.remove(inbox_index);
            prompts.push(saved);
            Some(prompts.len() - 1)
        }
        Err(err) => {
            log::warn!("failed to promote inbox prompt {id}: {err}");
            None
        }
    }
}

fn inbox_hover_text(prompt: &StackerPrompt) -> String {
    match prompt.source_agent.as_deref() {
        Some(agent) if !agent.trim().is_empty() => {
            format!("Suggested by {agent}\n{}", prompt.label)
        }
        _ => prompt.label.clone(),
    }
}

fn sized_thumbnail_button(label: &str, color: egui::Color32) -> egui::Button<'_> {
    egui::Button::new(egui::RichText::new(label).size(11.0).color(color))
        .min_size(egui::vec2(THUMBNAIL_BUTTON_WIDTH, THUMBNAIL_BUTTON_HEIGHT))
}

fn queue_button(label: &str, queued: bool, text_size: f32) -> egui::Button<'_> {
    let button = egui::Button::new(egui::RichText::new(label).size(text_size).color(if queued {
        QUEUED_BUTTON_TEXT
    } else {
        DEFAULT_BUTTON_TEXT
    }));
    if queued {
        button
            .fill(QUEUED_BUTTON_FILL)
            .stroke(egui::Stroke::new(1.0, QUEUED_BUTTON_STROKE))
    } else {
        button
    }
}

fn apply_thumbnail_wheel_scroll(ui: &mut egui::Ui) {
    if !ui.rect_contains_pointer(ui.clip_rect()) {
        return;
    }

    let scroll_delta = ui.ctx().input(|input| input.smooth_scroll_delta);
    let horizontal_delta = scroll_delta.x - scroll_delta.y;
    if horizontal_delta.abs() <= f32::EPSILON {
        return;
    }

    ui.scroll_with_delta(egui::vec2(horizontal_delta, 0.0));
    ui.ctx().input_mut(|input| {
        input.smooth_scroll_delta = egui::Vec2::ZERO;
    });
}

fn render_prompt_thumbnail_preview(ui: &mut egui::Ui, prompt: &StackerPrompt, selected: bool) {
    let preview_size = egui::vec2(THUMBNAIL_PREVIEW_WIDTH, THUMBNAIL_PREVIEW_HEIGHT);
    ui.horizontal_centered(|ui| {
        render_prompt_thumbnail_preview_box(ui, prompt, selected, preview_size);
    });
}

fn render_prompt_thumbnail_preview_box(
    ui: &mut egui::Ui,
    prompt: &StackerPrompt,
    selected: bool,
    preview_size: egui::Vec2,
) {
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(24, 24, 26))
        .stroke(egui::Stroke::new(
            1.0,
            if selected {
                QUEUE_GREEN
            } else {
                egui::Color32::from_rgb(48, 48, 52)
            },
        ))
        .rounding(egui::Rounding::same(3.0))
        .inner_margin(egui::Margin::same(11.0))
        .show(ui, |ui| {
            ui.set_min_size(preview_size);
            ui.set_max_size(preview_size);
            let text = thumbnail_preview_text(prompt);
            ui.add_sized(
                [preview_size.x, preview_size.y],
                egui::Label::new(
                    egui::RichText::new(text)
                        .size(11.0)
                        .color(egui::Color32::from_rgb(210, 210, 216)),
                )
                .wrap(),
            );
        });
}

fn thumbnail_preview_text(prompt: &StackerPrompt) -> String {
    let source = if prompt.text.trim().is_empty() {
        prompt.label.as_str()
    } else {
        prompt.text.as_str()
    };

    let mut lines = Vec::new();
    for source_line in source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        for wrapped in wrap_thumbnail_line(source_line, 22) {
            lines.push(wrapped);
            if lines.len() >= 8 {
                break;
            }
        }
        if lines.len() >= 8 {
            break;
        }
    }

    if lines.is_empty() {
        lines.push(truncate_thumbnail_word(prompt.label.trim(), 22));
    }
    if lines.len() == 8 {
        if let Some(last) = lines.last_mut() {
            if !last.ends_with("...") {
                last.push_str("...");
            }
        }
    }
    lines.join("\n")
}

fn wrap_thumbnail_line(line: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in line.split_whitespace() {
        let word = truncate_thumbnail_word(word, max_chars);
        let next_len = current.chars().count() + usize::from(!current.is_empty()) + word.len();
        if !current.is_empty() && next_len > max_chars {
            lines.push(current);
            current = word;
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(&word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn truncate_thumbnail_word(word: &str, max_chars: usize) -> String {
    truncate_chars(word, max_chars).into_owned()
}
