use super::*;

#[test]
fn terminal_mouse_reporting_uses_local_selection_when_shift_is_held() {
    assert!(local_terminal_selection_requested(true, true, false));
}

#[test]
fn terminal_mouse_reporting_keeps_existing_local_selection_drag() {
    assert!(local_terminal_selection_requested(true, false, true));
}

#[test]
fn terminal_mouse_reporting_routes_normal_mouse_to_cli() {
    assert!(!local_terminal_selection_requested(true, false, false));
    assert!(!local_terminal_selection_requested(false, true, false));
}

#[test]
fn terminal_mouse_drag_starts_after_leaving_press_cell() {
    assert!(!terminal_mouse_drag_exceeded((4, 8), 4, 8));
    assert!(terminal_mouse_drag_exceeded((4, 8), 4, 9));
    assert!(terminal_mouse_drag_exceeded((4, 8), 5, 8));
}

fn primary_mods() -> ModifiersState {
    if cfg!(target_os = "macos") {
        ModifiersState::SUPER
    } else {
        ModifiersState::CONTROL
    }
}

fn ch(s: &str) -> Key {
    Key::Character(s.into())
}

#[test]
fn document_history_shortcut_maps_primary_z_to_undo() {
    assert_eq!(
        document_history_shortcut(&ch("z"), primary_mods()),
        Some(HistoryCommand::Undo)
    );
}

#[test]
fn document_history_shortcut_maps_primary_shift_z_to_redo() {
    assert_eq!(
        document_history_shortcut(&ch("z"), primary_mods() | ModifiersState::SHIFT),
        Some(HistoryCommand::Redo)
    );
}

#[test]
fn document_history_shortcut_maps_primary_y_to_redo() {
    assert_eq!(
        document_history_shortcut(&ch("y"), primary_mods()),
        Some(HistoryCommand::Redo)
    );
}

#[test]
fn document_history_shortcut_ignores_plain_z() {
    assert_eq!(
        document_history_shortcut(&ch("z"), ModifiersState::empty()),
        None
    );
}

#[test]
fn stacker_format_shortcuts_come_from_command_registry() {
    assert_eq!(
        stacker_editor_shortcut(&ch("b"), primary_mods()),
        Some(StackerCommandId::Bold)
    );
    assert_eq!(
        stacker_editor_shortcut(&ch("`"), primary_mods()),
        Some(StackerCommandId::InlineCode)
    );
}

#[test]
fn stacker_format_shortcuts_ignore_plain_text_keys() {
    assert_eq!(
        stacker_editor_shortcut(&ch("b"), ModifiersState::empty()),
        None
    );
}
