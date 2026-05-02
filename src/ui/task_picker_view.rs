use super::explorer_view::EditorViewState;

pub(super) fn render_task_picker(ui: &mut egui::Ui, editor_state: &mut EditorViewState) {
    if editor_state.task_picker.is_none() {
        return;
    }

    let mut selected_task: Option<crate::tasks::Task> = None;
    let mut dismiss = false;

    let tasks = editor_state.task_picker.as_ref().unwrap();
    let selected = editor_state.task_picker_selected;

    egui::Window::new("Run Task")
        .id(egui::Id::new("task_picker"))
        .fixed_pos(egui::pos2(
            ui.ctx().screen_rect().center().x - 180.0,
            ui.ctx().screen_rect().center().y - 100.0,
        ))
        .resizable(false)
        .show(ui.ctx(), |ui| {
            ui.label(
                egui::RichText::new("Select a task to run:")
                    .size(13.0)
                    .color(egui::Color32::WHITE),
            );
            ui.separator();

            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                dismiss = true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) && !tasks.is_empty() {
                selected_task = Some(tasks[selected].clone());
            }

            for (i, task) in tasks.iter().enumerate() {
                let bg = if i == selected {
                    egui::Color32::from_rgb(50, 80, 130)
                } else {
                    egui::Color32::TRANSPARENT
                };
                let text_color = if i == selected {
                    egui::Color32::WHITE
                } else {
                    egui::Color32::from_rgb(200, 205, 215)
                };

                egui::Frame::none()
                    .fill(bg)
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                    .show(ui, |ui| {
                        let resp =
                            ui.label(egui::RichText::new(&task.name).size(13.0).color(text_color));
                        if resp.interact(egui::Sense::click()).clicked() {
                            selected_task = Some(task.clone());
                        }
                    });
            }
        });

    let task_count = editor_state.task_picker.as_ref().map_or(0, |t| t.len());
    if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
        editor_state.task_picker_selected =
            (editor_state.task_picker_selected + 1).min(task_count.saturating_sub(1));
    }
    if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
        editor_state.task_picker_selected = editor_state.task_picker_selected.saturating_sub(1);
    }

    if dismiss {
        editor_state.task_picker = None;
    }
    if let Some(task) = selected_task {
        editor_state.pending_task = Some(task);
        editor_state.task_picker = None;
    }
}
