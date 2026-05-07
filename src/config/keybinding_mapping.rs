use std::collections::HashMap;

use crate::keybindings::{self, Action, KeyBindings};

pub(super) fn apply_keybindings(keybindings: &mut KeyBindings, overrides: HashMap<String, String>) {
    for (action_name, key_str) in overrides {
        let action = match action_name.as_str() {
            "copy" => Some(Action::Copy),
            "paste" => Some(Action::Paste),
            "select_all" => Some(Action::SelectAll),
            "search" | "find" => Some(Action::Search),
            "new_window" => Some(Action::NewWindow),
            "new_tab" => Some(Action::NewTab),
            "close_tab" => Some(Action::CloseTab),
            "next_tab" => Some(Action::NextTab),
            "prev_tab" => Some(Action::PrevTab),
            "split_vertical" => Some(Action::SplitVertical),
            "split_horizontal" => Some(Action::SplitHorizontal),
            "toggle_fullscreen" => Some(Action::ToggleFullscreen),
            "toggle_effects" => Some(Action::ToggleEffects),
            "toggle_fps" => Some(Action::ToggleFps),
            "toggle_error_panel" => Some(Action::ToggleErrorPanel),
            "scroll_page_up" => Some(Action::ScrollPageUp),
            "scroll_page_down" => Some(Action::ScrollPageDown),
            "zoom_in" => Some(Action::ZoomIn),
            "zoom_out" => Some(Action::ZoomOut),
            "zoom_reset" => Some(Action::ZoomReset),
            s if s.starts_with("switch_tab_") => s
                .strip_prefix("switch_tab_")
                .and_then(|n| n.parse::<u8>().ok())
                .filter(|n| (1..=9).contains(n))
                .map(Action::SwitchTab),
            _ => {
                log::warn!("Unknown keybinding action: {}", action_name);
                None
            }
        };
        if let (Some(action), Some(combo)) = (action, keybindings::parse_key_combo(&key_str)) {
            keybindings.set(action, combo);
        }
    }
}
