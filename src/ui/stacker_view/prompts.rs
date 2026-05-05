use crate::stacker::{
    document::StackerDocumentEditor,
    draft::StackerDraft,
    queue::{self, QueuedPrompt},
    StackerPrompt,
};

use super::{
    actions::request_load_saved_prompt,
    layout::{
        header_label, small, truncate_line, DIM, HEADING_COLOR, MUTED, PANEL_BG, QUEUE_GREEN,
        ROW_BG, ROW_HOVER,
    },
    PendingStackerDraftSwitch,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SavedPromptRowAction {
    Open,
    Edit,
    Delete,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_prompt_list_panel(
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
