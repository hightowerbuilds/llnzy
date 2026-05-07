use winit::keyboard::{Key, KeyCode, ModifiersState, PhysicalKey};

use crate::app::commands::AppCommand;
use crate::keybindings::primary_modifier;

pub fn app_zoom_shortcut_command(
    logical_key: &Key,
    physical_key: PhysicalKey,
    modifiers: ModifiersState,
) -> Option<AppCommand> {
    if !primary_modifier(modifiers) || modifiers.alt_key() {
        return None;
    }

    match physical_key {
        PhysicalKey::Code(KeyCode::Equal) | PhysicalKey::Code(KeyCode::NumpadAdd) => {
            Some(AppCommand::ZoomIn)
        }
        PhysicalKey::Code(KeyCode::Minus) | PhysicalKey::Code(KeyCode::NumpadSubtract) => {
            Some(AppCommand::ZoomOut)
        }
        PhysicalKey::Code(KeyCode::Digit0) | PhysicalKey::Code(KeyCode::Numpad0) => {
            Some(AppCommand::ZoomReset)
        }
        _ => match logical_key {
            Key::Character(ch) if ch.as_str() == "+" || ch.as_str() == "=" => {
                Some(AppCommand::ZoomIn)
            }
            Key::Character(ch) if ch.as_str() == "-" => Some(AppCommand::ZoomOut),
            Key::Character(ch) if ch.as_str() == "0" => Some(AppCommand::ZoomReset),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn app_zoom_shortcuts_use_physical_plus_minus_keys() {
        assert!(matches!(
            app_zoom_shortcut_command(&ch("="), PhysicalKey::Code(KeyCode::Equal), primary_mods()),
            Some(AppCommand::ZoomIn)
        ));
        assert!(matches!(
            app_zoom_shortcut_command(&ch("-"), PhysicalKey::Code(KeyCode::Minus), primary_mods()),
            Some(AppCommand::ZoomOut)
        ));
    }

    #[test]
    fn app_zoom_shortcuts_block_plain_plus_minus() {
        assert!(app_zoom_shortcut_command(
            &ch("="),
            PhysicalKey::Code(KeyCode::Equal),
            ModifiersState::empty()
        )
        .is_none());
    }
}
