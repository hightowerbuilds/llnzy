use crate::sketch::{SketchElement, SketchPoint, SketchState, SketchTool};

use super::sketch_paint::screen_to_canvas;

pub(super) fn sketch_shortcuts(ui: &egui::Ui, sketch: &mut SketchState) {
    ui.input(|input| {
        if let Some(shortcut) = sketch_history_shortcut(
            input.modifiers,
            input.key_pressed(egui::Key::Z),
            input.key_pressed(egui::Key::Y),
        ) {
            apply_sketch_history_shortcut(sketch, shortcut, false);
        }
        if (input.key_pressed(egui::Key::Delete) || input.key_pressed(egui::Key::Backspace))
            && sketch.text_draft.is_none()
        {
            sketch.delete_selected();
        }
        if input.key_pressed(egui::Key::Escape) {
            if sketch.text_draft.is_some() {
                sketch.cancel_text_draft();
            } else {
                sketch.selected = None;
            }
        }
    });
}

/// Handle keyboard input when a text draft is active (inline text tool).
pub(super) fn handle_inline_text_input(ui: &egui::Ui, sketch: &mut SketchState) {
    let Some(draft) = &sketch.text_draft else {
        return;
    };
    let mut text = draft.text.clone();
    let mut commit = false;
    let mut cancel = false;

    ui.input(|input| {
        // Escape cancels
        if input.key_pressed(egui::Key::Escape) {
            cancel = true;
            return;
        }
        // Enter commits (without modifiers)
        if input.key_pressed(egui::Key::Enter) && !input.modifiers.shift {
            commit = true;
            return;
        }
        // Backspace removes last char
        if input.key_pressed(egui::Key::Backspace) {
            text.pop();
            return;
        }
        if let Some(shortcut) = sketch_history_shortcut(
            input.modifiers,
            input.key_pressed(egui::Key::Z),
            input.key_pressed(egui::Key::Y),
        ) {
            apply_sketch_history_shortcut(sketch, shortcut, true);
            return;
        }
        // Collect typed text from events
        let mut pasted_from_event = false;
        for event in &input.events {
            match event {
                egui::Event::Text(s) => text.push_str(s),
                egui::Event::Paste(s) => {
                    text.push_str(s);
                    pasted_from_event = true;
                }
                _ => {}
            }
        }
        if !pasted_from_event && input.modifiers.command && input.key_pressed(egui::Key::V) {
            if let Some(paste) = sketch.clipboard_in.as_deref() {
                text.push_str(paste);
            }
        }
    });

    if cancel {
        sketch.cancel_text_draft();
    } else if commit {
        sketch.update_text_draft(text);
        sketch.commit_text_draft();
    } else {
        sketch.update_text_draft(text);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SketchHistoryShortcut {
    Undo,
    Redo,
}

fn sketch_history_shortcut(
    modifiers: egui::Modifiers,
    z_pressed: bool,
    y_pressed: bool,
) -> Option<SketchHistoryShortcut> {
    if !modifiers.command {
        return None;
    }

    if z_pressed {
        return Some(if modifiers.shift {
            SketchHistoryShortcut::Redo
        } else {
            SketchHistoryShortcut::Undo
        });
    }

    y_pressed.then_some(SketchHistoryShortcut::Redo)
}

fn apply_sketch_history_shortcut(
    sketch: &mut SketchState,
    shortcut: SketchHistoryShortcut,
    cancel_text_draft_for_undo: bool,
) {
    match shortcut {
        SketchHistoryShortcut::Undo => {
            if cancel_text_draft_for_undo {
                sketch.cancel_text_draft();
            }
            sketch.undo();
        }
        SketchHistoryShortcut::Redo => {
            sketch.redo();
        }
    }
}

pub(super) fn handle_canvas_paste(
    ctx: &egui::Context,
    sketch: &mut SketchState,
    canvas_rect: egui::Rect,
) {
    if sketch.text_draft.is_some() || ctx.wants_keyboard_input() {
        return;
    }
    let paste = ctx.input(|input| {
        let pasted_event = input.events.iter().find_map(|event| {
            if let egui::Event::Paste(text) = event {
                Some(text.clone())
            } else {
                None
            }
        });
        if pasted_event.is_some() {
            pasted_event
        } else if input.modifiers.command && input.key_pressed(egui::Key::V) {
            sketch.clipboard_in.clone()
        } else {
            None
        }
    });
    let Some(text) = paste.filter(|text| !text.trim().is_empty()) else {
        return;
    };
    let point = ctx.input(|input| input.pointer.hover_pos()).map_or_else(
        || SketchPoint::new(canvas_rect.width() * 0.5, 72.0),
        |pos| {
            if canvas_rect.contains(pos) {
                screen_to_canvas(pos, canvas_rect)
            } else {
                SketchPoint::new(canvas_rect.width() * 0.5, 72.0)
            }
        },
    );
    sketch.paste_text_box(&text, point);
}

pub(super) fn handle_sketch_pointer(
    sketch: &mut SketchState,
    response: &egui::Response,
    canvas_rect: egui::Rect,
) {
    let Some(pointer_pos) = response.interact_pointer_pos() else {
        return;
    };
    let point = screen_to_canvas(pointer_pos, canvas_rect);
    let modifiers = response.ctx.input(|input| input.modifiers);

    match sketch.tool {
        SketchTool::Marker => {
            if response.drag_started() {
                sketch.begin_stroke(point);
            } else if response.dragged() {
                sketch.append_stroke_point(point);
            }
            if response.drag_stopped() {
                sketch.finish_stroke();
            }
        }
        SketchTool::Rectangle => {
            if response.drag_started() {
                sketch.begin_rectangle(point);
                sketch.update_rectangle_with_modifiers(point, modifiers.shift, modifiers.alt);
            } else if response.dragged() {
                sketch.update_rectangle_with_modifiers(point, modifiers.shift, modifiers.alt);
            }
            if response.drag_stopped() {
                sketch.update_rectangle_with_modifiers(point, modifiers.shift, modifiers.alt);
                sketch.finish_rectangle();
            }
        }
        SketchTool::Text => {
            if response.clicked() {
                // If there's already a text draft active, commit it first
                if sketch.text_draft.is_some() {
                    let draft_text = sketch.text_draft.as_ref().map(|d| d.text.clone());
                    if let Some(text) = draft_text {
                        sketch.update_text_draft(text);
                    }
                    sketch.commit_text_draft();
                }
                sketch.add_text_box(point);
            }
        }
        SketchTool::Select => {
            if let Some(handle) = sketch.selected_resize_handle_at(point) {
                response
                    .ctx
                    .set_cursor_icon(cursor_icon_for_resize_handle(handle));
            }
            if response.double_clicked() {
                if let Some(index) = sketch.hit_test(point) {
                    if matches!(
                        sketch.document.elements.get(index),
                        Some(SketchElement::Text(_))
                    ) {
                        sketch.edit_text_box(index);
                        return;
                    }
                }
            }
            if response.drag_started() {
                if let Some(handle) = sketch.selected_resize_handle_at(point) {
                    sketch.begin_resize_selected(handle, point);
                } else {
                    sketch.select_at(point);
                    sketch.begin_move_selected(point);
                }
            } else if response.dragged() {
                if sketch.resize_draft.is_some() {
                    sketch.update_resize_selected(point);
                } else {
                    sketch.update_move_selected(point);
                }
            }
            if response.drag_stopped() {
                sketch.finish_resize_selected();
                sketch.finish_move_selected();
            }
            if response.clicked() && sketch.selected_resize_handle_at(point).is_none() {
                sketch.select_at(point);
            }
        }
    }
}

fn cursor_icon_for_resize_handle(handle: crate::sketch::ResizeHandle) -> egui::CursorIcon {
    match handle {
        crate::sketch::ResizeHandle::TopLeft | crate::sketch::ResizeHandle::BottomRight => {
            egui::CursorIcon::ResizeNwSe
        }
        crate::sketch::ResizeHandle::TopRight | crate::sketch::ResizeHandle::BottomLeft => {
            egui::CursorIcon::ResizeNeSw
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command_modifiers(shift: bool) -> egui::Modifiers {
        egui::Modifiers {
            command: true,
            shift,
            ..Default::default()
        }
    }

    #[test]
    fn sketch_history_shortcuts_map_command_z_to_undo() {
        assert_eq!(
            sketch_history_shortcut(command_modifiers(false), true, false),
            Some(SketchHistoryShortcut::Undo)
        );
    }

    #[test]
    fn sketch_history_shortcuts_map_command_shift_z_to_redo() {
        assert_eq!(
            sketch_history_shortcut(command_modifiers(true), true, false),
            Some(SketchHistoryShortcut::Redo)
        );
    }

    #[test]
    fn sketch_history_shortcuts_map_command_y_to_redo() {
        assert_eq!(
            sketch_history_shortcut(command_modifiers(false), false, true),
            Some(SketchHistoryShortcut::Redo)
        );
    }

    #[test]
    fn sketch_history_shortcuts_ignore_z_without_command_modifier() {
        assert_eq!(
            sketch_history_shortcut(egui::Modifiers::default(), true, false),
            None
        );
    }
}
