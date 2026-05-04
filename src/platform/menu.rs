#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuCapability {
    NativeMenuBar,
    InWindowMenu,
    CommandPaletteOnly,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuCommandBinding {
    pub command_id: String,
    pub label: String,
    pub accelerator: Option<String>,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformMenuAction {
    NewTab,
    Save,
    CloseTab,
    Undo,
    Redo,
    Copy,
    Paste,
    SelectAll,
    Find,
    ToggleFullscreen,
    TabJoin,
    TabSeparate,
    TabSplit,
    TabRename,
    SplitVertical,
    SplitHorizontal,
    ToggleWordWrap,
    ToggleEffects,
    OpenProject,
    CloseProject,
}

pub const COMMAND_NEW_TAB: &str = "app.new-terminal-tab";
pub const COMMAND_SAVE: &str = "editor.save";
pub const COMMAND_CLOSE_TAB: &str = "app.close-tab";
pub const COMMAND_UNDO: &str = "edit.undo";
pub const COMMAND_REDO: &str = "edit.redo";
pub const COMMAND_COPY: &str = "edit.copy";
pub const COMMAND_PASTE: &str = "edit.paste";
pub const COMMAND_SELECT_ALL: &str = "edit.select-all";
pub const COMMAND_FIND: &str = "edit.find";
pub const COMMAND_TOGGLE_FULLSCREEN: &str = "app.toggle-fullscreen";
pub const COMMAND_TAB_JOIN: &str = "tab.join-next";
pub const COMMAND_TAB_SEPARATE: &str = "tab.separate";
pub const COMMAND_TAB_SPLIT: &str = "tab.split";
pub const COMMAND_TAB_RENAME: &str = "tab.rename";
pub const COMMAND_SPLIT_VERTICAL: &str = "view.split-vertical";
pub const COMMAND_SPLIT_HORIZONTAL: &str = "view.split-horizontal";
pub const COMMAND_TOGGLE_WORD_WRAP: &str = "editor.toggle-word-wrap";
pub const COMMAND_TOGGLE_EFFECTS: &str = "view.toggle-effects";
pub const COMMAND_OPEN_PROJECT: &str = "project.open";
pub const COMMAND_CLOSE_PROJECT: &str = "project.close";

pub fn command_id_for_native_action(action: PlatformMenuAction) -> &'static str {
    match action {
        PlatformMenuAction::NewTab => COMMAND_NEW_TAB,
        PlatformMenuAction::Save => COMMAND_SAVE,
        PlatformMenuAction::CloseTab => COMMAND_CLOSE_TAB,
        PlatformMenuAction::Undo => COMMAND_UNDO,
        PlatformMenuAction::Redo => COMMAND_REDO,
        PlatformMenuAction::Copy => COMMAND_COPY,
        PlatformMenuAction::Paste => COMMAND_PASTE,
        PlatformMenuAction::SelectAll => COMMAND_SELECT_ALL,
        PlatformMenuAction::Find => COMMAND_FIND,
        PlatformMenuAction::ToggleFullscreen => COMMAND_TOGGLE_FULLSCREEN,
        PlatformMenuAction::TabJoin => COMMAND_TAB_JOIN,
        PlatformMenuAction::TabSeparate => COMMAND_TAB_SEPARATE,
        PlatformMenuAction::TabSplit => COMMAND_TAB_SPLIT,
        PlatformMenuAction::TabRename => COMMAND_TAB_RENAME,
        PlatformMenuAction::SplitVertical => COMMAND_SPLIT_VERTICAL,
        PlatformMenuAction::SplitHorizontal => COMMAND_SPLIT_HORIZONTAL,
        PlatformMenuAction::ToggleWordWrap => COMMAND_TOGGLE_WORD_WRAP,
        PlatformMenuAction::ToggleEffects => COMMAND_TOGGLE_EFFECTS,
        PlatformMenuAction::OpenProject => COMMAND_OPEN_PROJECT,
        PlatformMenuAction::CloseProject => COMMAND_CLOSE_PROJECT,
    }
}

pub fn binding(
    action: PlatformMenuAction,
    label: impl Into<String>,
    accelerator: Option<&str>,
    enabled: bool,
) -> MenuCommandBinding {
    MenuCommandBinding {
        command_id: command_id_for_native_action(action).to_string(),
        label: label.into(),
        accelerator: accelerator.map(str::to_string),
        enabled,
    }
}

#[cfg(target_os = "macos")]
pub fn current_menu_capability() -> MenuCapability {
    MenuCapability::NativeMenuBar
}

#[cfg(not(target_os = "macos"))]
pub fn current_menu_capability() -> MenuCapability {
    MenuCapability::InWindowMenu
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_menu_actions_map_to_stable_command_ids() {
        assert_eq!(
            command_id_for_native_action(PlatformMenuAction::Save),
            COMMAND_SAVE
        );
        assert_eq!(
            command_id_for_native_action(PlatformMenuAction::OpenProject),
            COMMAND_OPEN_PROJECT
        );
        assert_eq!(
            command_id_for_native_action(PlatformMenuAction::ToggleWordWrap),
            COMMAND_TOGGLE_WORD_WRAP
        );
    }

    #[test]
    fn menu_binding_carries_command_metadata() {
        let binding = binding(PlatformMenuAction::Copy, "Copy", Some("Cmd+C"), true);

        assert_eq!(binding.command_id, COMMAND_COPY);
        assert_eq!(binding.label, "Copy");
        assert_eq!(binding.accelerator.as_deref(), Some("Cmd+C"));
        assert!(binding.enabled);
    }
}
