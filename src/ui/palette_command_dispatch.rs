use crate::app::commands::AppCommand;
use crate::config::Config;
use crate::explorer::ExplorerState;

use super::{
    command_palette::CommandId, editor_command::EditorCommand, explorer_view::EditorViewState,
    sidebar_state,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_palette_command(
    command_id: CommandId,
    editor_view: &mut EditorViewState,
    explorer: &mut ExplorerState,
    sidebar: &mut sidebar_state::SidebarUiState,
    config: &mut Config,
    active_tab_index: usize,
    tab_count: usize,
    commands: &mut Vec<AppCommand>,
) {
    if let Some(command) = EditorCommand::from_palette(command_id) {
        let outcome = editor_view.dispatch_editor_command(command, Some(&explorer.root));
        if outcome.open_file_finder {
            explorer.open_finder();
            sidebar.open = true;
        }
        return;
    }

    match command_id {
        CommandId::OpenWorkspace => {
            commands.push(AppCommand::PickOpenProject);
        }
        CommandId::NewTab => {
            commands.push(AppCommand::NewTerminalTab);
        }
        CommandId::ToggleTerminal => {
            commands.push(AppCommand::NewTerminalTab);
        }
        CommandId::CloseTab => {
            commands.push(AppCommand::CloseTab(active_tab_index));
        }
        CommandId::NextTab => {
            if tab_count > 0 {
                commands.push(AppCommand::SwitchTab((active_tab_index + 1) % tab_count));
            }
        }
        CommandId::PrevTab => {
            if tab_count > 0 {
                commands.push(AppCommand::SwitchTab(
                    active_tab_index.checked_sub(1).unwrap_or(tab_count - 1),
                ));
            }
        }
        CommandId::ToggleSidebar => {
            commands.push(AppCommand::ToggleSidebar);
        }
        CommandId::ToggleFps => {
            commands.push(AppCommand::ToggleFps);
        }
        CommandId::ToggleEffects => {
            config.effects.enabled = !config.effects.enabled;
            commands.push(AppCommand::ApplyConfig(config.clone()));
        }
        CommandId::Save
        | CommandId::Undo
        | CommandId::Redo
        | CommandId::SelectAll
        | CommandId::Cut
        | CommandId::Copy
        | CommandId::Paste
        | CommandId::DeleteLine
        | CommandId::DuplicateLine
        | CommandId::MoveLineUp
        | CommandId::MoveLineDown
        | CommandId::FormatDocument
        | CommandId::RenameSymbol
        | CommandId::GoToDefinition
        | CommandId::ShowHover
        | CommandId::CodeActions
        | CommandId::DocumentSymbols
        | CommandId::Find
        | CommandId::FindReplace
        | CommandId::FindReferences
        | CommandId::WorkspaceSymbols
        | CommandId::ProjectSearch
        | CommandId::RunTask
        | CommandId::FindFile
        | CommandId::ToggleMarkdownMode
        | CommandId::MarkdownSource
        | CommandId::MarkdownPreview
        | CommandId::MarkdownSplit
        | CommandId::Stacker(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply_for_test(
        command_id: CommandId,
        active_tab_index: usize,
        tab_count: usize,
    ) -> Vec<AppCommand> {
        let mut editor_view = EditorViewState::default();
        let mut explorer = ExplorerState::default();
        let mut sidebar = sidebar_state::SidebarUiState::default();
        let mut config = Config::default();
        let mut commands = Vec::new();

        apply_palette_command(
            command_id,
            &mut editor_view,
            &mut explorer,
            &mut sidebar,
            &mut config,
            active_tab_index,
            tab_count,
            &mut commands,
        );

        commands
    }

    #[test]
    fn open_workspace_palette_command_emits_project_picker() {
        let commands = apply_for_test(CommandId::OpenWorkspace, 0, 1);

        assert!(matches!(&commands[..], [AppCommand::PickOpenProject]));
    }

    #[test]
    fn toggle_terminal_palette_command_uses_terminal_tab_command() {
        let commands = apply_for_test(CommandId::ToggleTerminal, 0, 1);

        assert!(matches!(&commands[..], [AppCommand::NewTerminalTab]));
    }

    #[test]
    fn previous_tab_palette_command_wraps_to_last_tab() {
        let commands = apply_for_test(CommandId::PrevTab, 0, 3);

        assert!(matches!(&commands[..], [AppCommand::SwitchTab(2)]));
    }

    #[test]
    fn sidebar_palette_command_emits_app_toggle() {
        let commands = apply_for_test(CommandId::ToggleSidebar, 0, 1);

        assert!(matches!(&commands[..], [AppCommand::ToggleSidebar]));
    }

    #[test]
    fn fps_palette_command_emits_app_toggle() {
        let commands = apply_for_test(CommandId::ToggleFps, 0, 1);

        assert!(matches!(&commands[..], [AppCommand::ToggleFps]));
    }

    #[test]
    fn effects_palette_command_emits_updated_config() {
        let commands = apply_for_test(CommandId::ToggleEffects, 0, 1);

        assert!(matches!(
            &commands[..],
            [AppCommand::ApplyConfig(config)] if !config.effects.enabled
        ));
    }
}
