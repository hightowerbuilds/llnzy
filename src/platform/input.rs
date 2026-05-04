use winit::event::{ElementState, KeyEvent};
use winit::keyboard::ModifiersState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimaryShortcutModifier {
    Command,
    Control,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextInputCapability {
    Ime,
    DeadKeys,
    Compose,
    AltGr,
    NativeTextControl,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlatformInputIntent {
    AppShortcut(String),
    TerminalInput(Vec<u8>),
    TextInput(String),
    MouseReport(Vec<u8>),
    FocusChange,
}

pub fn current_primary_shortcut_modifier() -> PrimaryShortcutModifier {
    if cfg!(target_os = "macos") {
        PrimaryShortcutModifier::Command
    } else {
        PrimaryShortcutModifier::Control
    }
}

pub fn primary_modifier_pressed(modifiers: ModifiersState) -> bool {
    match current_primary_shortcut_modifier() {
        PrimaryShortcutModifier::Command => modifiers.super_key(),
        PrimaryShortcutModifier::Control => modifiers.control_key(),
    }
}

pub fn text_input_capabilities() -> Vec<TextInputCapability> {
    if cfg!(target_os = "macos") {
        vec![
            TextInputCapability::Ime,
            TextInputCapability::DeadKeys,
            TextInputCapability::NativeTextControl,
        ]
    } else if cfg!(target_os = "windows") {
        vec![
            TextInputCapability::Ime,
            TextInputCapability::DeadKeys,
            TextInputCapability::AltGr,
            TextInputCapability::NativeTextControl,
        ]
    } else {
        vec![
            TextInputCapability::Ime,
            TextInputCapability::DeadKeys,
            TextInputCapability::Compose,
            TextInputCapability::AltGr,
        ]
    }
}

pub fn keyboard_intent(
    event: &KeyEvent,
    modifiers: ModifiersState,
    app_cursor: bool,
) -> Option<PlatformInputIntent> {
    if event.state != ElementState::Pressed {
        return None;
    }

    if let Some(text) = event.text.as_ref() {
        let text = text.as_str();
        if !text.is_empty()
            && !modifiers.control_key()
            && !modifiers.alt_key()
            && !modifiers.super_key()
            && crate::input::text_should_use_paste_path(text)
        {
            return Some(PlatformInputIntent::TextInput(text.to_string()));
        }
    }

    crate::input::encode_key(event, modifiers, app_cursor).map(PlatformInputIntent::TerminalInput)
}

pub fn is_modifier_only_key(event: &KeyEvent) -> bool {
    crate::input::is_modifier_only_key(event)
}

pub fn paste_like_text_input(text: &str, modifiers: ModifiersState) -> Option<PlatformInputIntent> {
    if !modifiers.control_key()
        && !modifiers.alt_key()
        && !modifiers.super_key()
        && crate::input::text_should_use_paste_path(text)
    {
        Some(PlatformInputIntent::TextInput(text.to_string()))
    } else {
        None
    }
}

pub fn mouse_report_intent(
    button: u8,
    col: usize,
    row: usize,
    press: bool,
    sgr: bool,
    modifiers: &ModifiersState,
) -> PlatformInputIntent {
    PlatformInputIntent::MouseReport(crate::input::encode_mouse(
        button, col, row, press, sgr, modifiers,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_modifier_matches_current_platform() {
        let mods = if cfg!(target_os = "macos") {
            ModifiersState::SUPER
        } else {
            ModifiersState::CONTROL
        };

        assert!(primary_modifier_pressed(mods));
    }

    #[test]
    fn paste_like_text_input_becomes_text_intent_without_shortcut_modifiers() {
        assert_eq!(
            paste_like_text_input("hello", ModifiersState::empty()),
            Some(PlatformInputIntent::TextInput("hello".to_string()))
        );
        assert_eq!(
            paste_like_text_input("hello", ModifiersState::CONTROL),
            None
        );
    }

    #[test]
    fn mouse_report_intent_wraps_encoded_mouse_bytes() {
        assert_eq!(
            mouse_report_intent(0, 5, 10, true, true, &ModifiersState::empty()),
            PlatformInputIntent::MouseReport(b"\x1b[<0;6;11M".to_vec())
        );
    }
}
