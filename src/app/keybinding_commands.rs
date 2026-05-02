use crate::app::commands::AppCommand;
use crate::keybindings::Action;

pub fn app_command_for_keybinding(
    action: &Action,
    active_tab_index: usize,
    tab_count: usize,
) -> Option<AppCommand> {
    match action {
        Action::NewTab | Action::SplitVertical | Action::SplitHorizontal => {
            Some(AppCommand::NewTerminalTab)
        }
        Action::CloseTab => Some(AppCommand::CloseTab(active_tab_index)),
        Action::NextTab => next_tab(active_tab_index, tab_count).map(AppCommand::SwitchTab),
        Action::PrevTab => previous_tab(active_tab_index, tab_count).map(AppCommand::SwitchTab),
        Action::ToggleFullscreen => Some(AppCommand::ToggleFullscreen),
        Action::ToggleEffects => Some(AppCommand::ToggleEffects),
        Action::ToggleFps => Some(AppCommand::ToggleFps),
        Action::ToggleSidebar => Some(AppCommand::ToggleSidebar),
        Action::SwitchTab(n) => Some(AppCommand::SwitchTab(n.saturating_sub(1) as usize)),
        Action::Copy
        | Action::Paste
        | Action::SelectAll
        | Action::Search
        | Action::ToggleErrorPanel
        | Action::CyclePaneForward
        | Action::CyclePaneBackward
        | Action::ScrollPageUp
        | Action::ScrollPageDown
        | Action::ToggleTerminalPanel
        | Action::ZoomIn
        | Action::ZoomOut
        | Action::ZoomReset => None,
    }
}

fn next_tab(active_tab_index: usize, tab_count: usize) -> Option<usize> {
    (tab_count > 0).then_some((active_tab_index + 1) % tab_count)
}

fn previous_tab(active_tab_index: usize, tab_count: usize) -> Option<usize> {
    if tab_count == 0 {
        None
    } else {
        Some(active_tab_index.checked_sub(1).unwrap_or(tab_count - 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_tab_wraps_to_zero() {
        assert!(matches!(
            app_command_for_keybinding(&Action::NextTab, 2, 3),
            Some(AppCommand::SwitchTab(0))
        ));
    }

    #[test]
    fn previous_tab_wraps_to_last() {
        assert!(matches!(
            app_command_for_keybinding(&Action::PrevTab, 0, 3),
            Some(AppCommand::SwitchTab(2))
        ));
    }

    #[test]
    fn legacy_split_keybindings_open_terminal_tabs() {
        assert!(matches!(
            app_command_for_keybinding(&Action::SplitVertical, 0, 1),
            Some(AppCommand::NewTerminalTab)
        ));
        assert!(matches!(
            app_command_for_keybinding(&Action::SplitHorizontal, 0, 1),
            Some(AppCommand::NewTerminalTab)
        ));
    }

    #[test]
    fn surface_specific_actions_stay_out_of_app_command_mapping() {
        assert!(app_command_for_keybinding(&Action::Copy, 0, 1).is_none());
        assert!(app_command_for_keybinding(&Action::Search, 0, 1).is_none());
        assert!(app_command_for_keybinding(&Action::ZoomIn, 0, 1).is_none());
    }
}
