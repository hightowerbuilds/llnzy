use crate::stacker::{
    document::StackerDocumentEditor, draft::StackerDraft, draft::StackerDraftSource,
    formatting::maybe_continue_list, StackerPrompt,
};

use super::{
    layout::{stacker_editor_font, MUTED, NOTE_BG, NOTE_PADDING, NOTE_TEXT},
    toolbar::render_editor_toolbar,
    PendingStackerDraftSwitch, STACKER_PROMPT_EDITOR_ID,
};
use crate::ui::stacker_cursor;

#[allow(clippy::too_many_arguments)]
pub(super) fn render_prompt_editor_panel(
    ui: &mut egui::Ui,
    height: f32,
    prompts: &mut Vec<StackerPrompt>,
    inbox_prompts: &mut Vec<StackerPrompt>,
    editor: &mut StackerDocumentEditor,
    draft: &mut StackerDraft,
    pending_switch: &mut Option<PendingStackerDraftSwitch>,
    editing: &mut Option<usize>,
    dirty: &mut bool,
    editor_font_size: &mut f32,
    web_editor_rect: &mut Option<egui::Rect>,
) {
    let editor_id = egui::Id::new(STACKER_PROMPT_EDITOR_ID);

    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), height),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            render_editor_toolbar(
                ui,
                editor_id,
                prompts,
                inbox_prompts,
                editor,
                draft,
                pending_switch,
                editing,
                dirty,
                editor_font_size,
            );

            render_editor_status(ui, editor, draft, *editing);
            ui.add_space(6.0);
            render_text_edit_prompt(
                ui,
                editor_id,
                editor,
                draft,
                *editor_font_size,
                web_editor_rect,
            );
        },
    );
}

fn render_text_edit_prompt(
    ui: &mut egui::Ui,
    editor_id: egui::Id,
    editor: &mut StackerDocumentEditor,
    draft: &mut StackerDraft,
    editor_font_size: f32,
    web_editor_rect: &mut Option<egui::Rect>,
) {
    let note_h = ui.available_height().max(1.0);
    let frame_output = egui::Frame::none()
        .fill(NOTE_BG)
        .rounding(egui::Rounding::same(3.0))
        .inner_margin(egui::Margin::same(NOTE_PADDING))
        .show(ui, |ui| {
            ui.set_height(note_h);
            ui.scope(|ui| {
                let before_edit = editor.text().to_string();
                let before_selection = editor.selection();
                let editor_width = ui.available_width().max(1.0);
                ui.visuals_mut().extreme_bg_color = egui::Color32::TRANSPARENT;
                ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                ui.visuals_mut().widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                ui.visuals_mut().widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                ui.visuals_mut().widgets.inactive.bg_stroke = egui::Stroke::NONE;
                ui.visuals_mut().widgets.hovered.bg_stroke = egui::Stroke::NONE;
                ui.visuals_mut().widgets.active.bg_stroke = egui::Stroke::NONE;
                ui.set_min_size(egui::vec2(editor_width, ui.available_height()));
                let mut output = egui::TextEdit::multiline(editor.text_mut_for_widget())
                    .id(editor_id)
                    .desired_rows(32)
                    .desired_width(editor_width)
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
    let clipped_rect = frame_output.response.rect.intersect(ui.clip_rect());
    *web_editor_rect = clipped_rect.is_positive().then_some(clipped_rect);
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
    let source = match draft.source() {
        StackerDraftSource::SavedPrompt(idx) => format!("Prompt {}", idx + 1),
        StackerDraftSource::InboxPrompt(_) => "Agent prompt".to_string(),
        StackerDraftSource::Scratch => match editing {
            Some(idx) => format!("Prompt {}", idx + 1),
            None => "Scratch".to_string(),
        },
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
