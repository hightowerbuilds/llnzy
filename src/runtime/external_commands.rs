use llnzy::external_command::{
    CommandStatus, CommandTarget, ExternalAction, ExternalCommand, ExternalCommandResult,
    FocusPolicy, ResolvedTarget, SelectionPolicy, SurfaceKind, TextSelection,
};
use llnzy::external_input_trace;
use llnzy::input::text_should_use_paste_path;
use llnzy::stacker::commands::{execute_stacker_command_at, stacker_editor_command};
use llnzy::stacker::input::StackerSelection;
use llnzy::ui::command_palette::CommandId;
use llnzy::workspace::TabContent;

use crate::runtime::terminal::{
    current_stacker_selection, store_stacker_cursor, store_stacker_selection,
};
use crate::App;

impl App {
    pub(crate) fn dispatch_external_command(
        &mut self,
        command: ExternalCommand,
    ) -> ExternalCommandResult {
        let Some((tab_idx, target)) = self.resolve_external_target(command.target) else {
            return ExternalCommandResult::failed(
                command.id,
                CommandStatus::NoTarget,
                None,
                "No editable command target matched the request",
            );
        };

        let result = match target.surface {
            SurfaceKind::Stacker => self.dispatch_stacker_command(tab_idx, target, &command),
            SurfaceKind::CodeEditor => self.dispatch_code_editor_command(tab_idx, target, &command),
            SurfaceKind::Terminal => self.dispatch_terminal_command(tab_idx, target, &command),
        };

        external_input_trace::trace("external_command.dispatch", || {
            format!(
                "source={:?}, target={:?}, action={:?}, status={:?}, changed={}",
                command.source, target, command.action, result.status, result.changed
            )
        });
        result
    }

    pub(crate) fn dispatch_active_external_action(
        &mut self,
        action: ExternalAction,
    ) -> ExternalCommandResult {
        self.dispatch_external_command(ExternalCommand::internal(CommandTarget::ActiveTab, action))
    }

    fn resolve_external_target(&self, target: CommandTarget) -> Option<(usize, ResolvedTarget)> {
        match target {
            CommandTarget::FocusedSurface | CommandTarget::ActiveTab => {
                self.resolved_target_for_tab(self.active_tab)
            }
            CommandTarget::TabId(tab_id) | CommandTarget::Pane { tab_id } => self
                .tabs
                .iter()
                .position(|tab| tab.id == tab_id)
                .and_then(|idx| self.resolved_target_for_tab(idx)),
            CommandTarget::Surface(surface) => {
                let active = self.resolved_target_for_tab(self.active_tab);
                if active.is_some_and(|(_, resolved)| resolved.surface == surface) {
                    return active;
                }
                self.tabs
                    .iter()
                    .position(|tab| surface_matches(tab, surface))
                    .map(|idx| {
                        (
                            idx,
                            ResolvedTarget {
                                tab_id: self.tabs[idx].id,
                                surface,
                            },
                        )
                    })
            }
        }
    }

    fn resolved_target_for_tab(&self, tab_idx: usize) -> Option<(usize, ResolvedTarget)> {
        let tab = self.tabs.get(tab_idx)?;
        let surface = surface_for_content(&tab.content)?;
        Some((
            tab_idx,
            ResolvedTarget {
                tab_id: tab.id,
                surface,
            },
        ))
    }

    fn dispatch_stacker_command(
        &mut self,
        _tab_idx: usize,
        target: ResolvedTarget,
        command: &ExternalCommand,
    ) -> ExternalCommandResult {
        let Some(ui) = &mut self.ui else {
            return ExternalCommandResult::failed(
                command.id,
                CommandStatus::NoTarget,
                Some(target),
                "Stacker UI state is unavailable",
            );
        };

        let selection = stacker_selection_for_policy(ui, command.selection_policy);
        let mut changed = false;
        let mut selection_out = None;

        match &command.action {
            ExternalAction::InsertText { text } | ExternalAction::ReplaceSelection { text } => {
                if text.is_empty() {
                    return ExternalCommandResult::handled(command.id, target, false);
                }
                let outcome = ui.stacker.editor.insert_text(selection, text);
                changed = outcome.changed;
                store_stacker_cursor(ui, outcome.cursor);
                selection_out = Some(TextSelection {
                    start: outcome.cursor,
                    end: outcome.cursor,
                });
                external_input_trace::trace("stacker.external_insert_text", || {
                    format!(
                        "chars={}, selection={}..{}, cursor={}",
                        text.chars().count(),
                        selection.start,
                        selection.end,
                        outcome.cursor
                    )
                });
            }
            ExternalAction::SetSelection { start, end } => {
                let selection = StackerSelection {
                    start: *start,
                    end: *end,
                };
                store_stacker_selection(ui, selection);
                selection_out = Some(TextSelection {
                    start: selection.start,
                    end: selection.end,
                });
            }
            ExternalAction::SelectAll => {
                let selection = ui.stacker.editor.select_all();
                store_stacker_selection(ui, selection);
                selection_out = Some(TextSelection {
                    start: selection.start,
                    end: selection.end,
                });
            }
            ExternalAction::Copy => {
                if let Some(text) = ui.stacker.editor.selected_text(selection) {
                    if let Some(clipboard) = &mut self.clipboard {
                        let _ = clipboard.set_text(text);
                    }
                }
            }
            ExternalAction::Paste => {
                let text = self
                    .clipboard
                    .as_mut()
                    .and_then(|clipboard| clipboard.get_text().ok());
                let Some(text) = text else {
                    return ExternalCommandResult::handled(command.id, target, false);
                };
                let outcome = ui.stacker.editor.insert_text(selection, &text);
                changed = outcome.changed;
                store_stacker_cursor(ui, outcome.cursor);
                selection_out = Some(TextSelection {
                    start: outcome.cursor,
                    end: outcome.cursor,
                });
            }
            ExternalAction::Undo | ExternalAction::Redo => {
                changed = if matches!(command.action, ExternalAction::Undo) {
                    ui.stacker.editor.undo()
                } else {
                    ui.stacker.editor.redo()
                };
                let selection = ui.stacker.editor.selection();
                store_stacker_selection(ui, selection);
                selection_out = Some(TextSelection {
                    start: selection.start,
                    end: selection.end,
                });
            }
            ExternalAction::ApplyFormatting(command_id) => {
                let outcome = execute_stacker_command_at(
                    &mut ui.stacker.editor,
                    selection,
                    stacker_editor_command(*command_id),
                );
                changed = outcome.changed;
                store_stacker_selection(ui, outcome.selection);
                selection_out = Some(TextSelection {
                    start: outcome.selection.start,
                    end: outcome.selection.end,
                });
            }
            ExternalAction::Save | ExternalAction::Submit => {
                return ExternalCommandResult::failed(
                    command.id,
                    CommandStatus::UnsupportedAction,
                    Some(target),
                    "Stacker save and submit are still handled by Stacker UI state",
                );
            }
        }

        if changed {
            ui.stacker
                .draft
                .record_current_text(ui.stacker.editor.text().to_string());
            self.request_redraw();
        }
        if matches!(
            command.focus_policy,
            FocusPolicy::FocusTarget | FocusPolicy::FocusAfter
        ) {
            self.stacker_webview_pending_focus = true;
            self.request_redraw();
        }

        let mut result = ExternalCommandResult::handled(command.id, target, changed);
        result.selection = selection_out;
        result
    }

    fn dispatch_code_editor_command(
        &mut self,
        tab_idx: usize,
        target: ResolvedTarget,
        command: &ExternalCommand,
    ) -> ExternalCommandResult {
        let Some(buffer_id) = self.tabs.get(tab_idx).and_then(|tab| match tab.content {
            TabContent::CodeFile { buffer_id, .. } => Some(buffer_id),
            _ => None,
        }) else {
            return ExternalCommandResult::failed(
                command.id,
                CommandStatus::NoTarget,
                Some(target),
                "No code editor buffer matched the request",
            );
        };

        let Some(command_id) = editor_command_for_external_action(&command.action) else {
            return ExternalCommandResult::failed(
                command.id,
                CommandStatus::UnsupportedAction,
                Some(target),
                "Code editor does not support that external action yet",
            );
        };

        let clipboard_in = if matches!(command.action, ExternalAction::Paste) {
            self.clipboard
                .as_mut()
                .and_then(|clipboard| clipboard.get_text().ok())
        } else {
            None
        };

        let Some(ui) = self.ui.as_mut() else {
            return ExternalCommandResult::failed(
                command.id,
                CommandStatus::NoTarget,
                Some(target),
                "Code editor UI state is unavailable",
            );
        };
        if !ui.editor_view.editor.switch_to_id(buffer_id) {
            return ExternalCommandResult::failed(
                command.id,
                CommandStatus::NoTarget,
                Some(target),
                "Code editor buffer is unavailable",
            );
        }
        if matches!(command.action, ExternalAction::Paste) {
            ui.editor_view.clipboard_in = clipboard_in;
        }
        if !ui.dispatch_editor_command_id(command_id) {
            return ExternalCommandResult::failed(
                command.id,
                CommandStatus::UnsupportedAction,
                Some(target),
                "Code editor rejected the command",
            );
        }
        if let Some(text) = ui.editor_view.clipboard_out.take() {
            if let Some(clipboard) = &mut self.clipboard {
                let _ = clipboard.set_text(text);
            }
        }

        self.request_redraw();
        ExternalCommandResult::handled(command.id, target, true)
    }

    fn dispatch_terminal_command(
        &mut self,
        tab_idx: usize,
        target: ResolvedTarget,
        command: &ExternalCommand,
    ) -> ExternalCommandResult {
        match &command.action {
            ExternalAction::InsertText { text } | ExternalAction::ReplaceSelection { text } => {
                if text_should_use_paste_path(text) {
                    self.paste_text_to_terminal_tab(tab_idx, text);
                } else {
                    self.write_to_terminal_tab(tab_idx, text.as_bytes());
                }
                ExternalCommandResult::handled(command.id, target, !text.is_empty())
            }
            ExternalAction::Paste => {
                let text = self
                    .clipboard
                    .as_mut()
                    .and_then(|clipboard| clipboard.get_text().ok());
                let Some(text) = text else {
                    return ExternalCommandResult::handled(command.id, target, false);
                };
                self.paste_text_to_terminal_tab(tab_idx, &text);
                ExternalCommandResult::handled(command.id, target, !text.is_empty())
            }
            ExternalAction::Copy => {
                let text = self
                    .session_for_tab(tab_idx)
                    .and_then(|session| session.terminal.selected_text());
                let Some(text) = text else {
                    return ExternalCommandResult::handled(command.id, target, false);
                };
                if let Some(clipboard) = &mut self.clipboard {
                    let _ = clipboard.set_text(text);
                }
                ExternalCommandResult::handled(command.id, target, false)
            }
            ExternalAction::SelectAll => {
                if let Some(session) = self.session_for_tab_mut(tab_idx) {
                    session.terminal.select_all();
                    self.request_redraw();
                    return ExternalCommandResult::handled(command.id, target, true);
                }
                ExternalCommandResult::failed(
                    command.id,
                    CommandStatus::NoTarget,
                    Some(target),
                    "Terminal session is unavailable",
                )
            }
            ExternalAction::SetSelection { .. }
            | ExternalAction::Undo
            | ExternalAction::Redo
            | ExternalAction::ApplyFormatting(_)
            | ExternalAction::Save
            | ExternalAction::Submit => ExternalCommandResult::failed(
                command.id,
                CommandStatus::UnsupportedAction,
                Some(target),
                "Terminal does not support document editing actions",
            ),
        }
    }

    fn paste_text_to_terminal_tab(&mut self, tab_idx: usize, text: &str) {
        let bracketed = self
            .session_for_tab(tab_idx)
            .is_some_and(|session| session.terminal.bracketed_paste());
        external_input_trace::trace("terminal.external_paste_text", || {
            format!("chars={}, bracketed={}", text.chars().count(), bracketed)
        });
        if bracketed {
            let mut bytes = Vec::with_capacity(text.len() + 12);
            bytes.extend_from_slice(b"\x1b[200~");
            bytes.extend_from_slice(text.as_bytes());
            bytes.extend_from_slice(b"\x1b[201~");
            self.write_to_terminal_tab(tab_idx, &bytes);
        } else {
            self.write_to_terminal_tab(tab_idx, text.as_bytes());
        }
    }
}

fn surface_for_content(content: &TabContent) -> Option<SurfaceKind> {
    match content {
        TabContent::Stacker => Some(SurfaceKind::Stacker),
        TabContent::CodeFile { .. } => Some(SurfaceKind::CodeEditor),
        TabContent::Terminal(_) => Some(SurfaceKind::Terminal),
        _ => None,
    }
}

fn surface_matches(tab: &llnzy::workspace::WorkspaceTab, surface: SurfaceKind) -> bool {
    surface_for_content(&tab.content) == Some(surface)
}

fn stacker_selection_for_policy(
    ui: &llnzy::ui::UiState,
    policy: SelectionPolicy,
) -> StackerSelection {
    let current = current_stacker_selection(ui);
    match policy {
        SelectionPolicy::UseCurrentSelection | SelectionPolicy::ReplaceCurrentSelection => current,
        SelectionPolicy::SetSelectionBefore { start, end } => StackerSelection { start, end },
        SelectionPolicy::Append => {
            let end = ui.stacker.editor.char_count();
            StackerSelection::collapsed(end)
        }
        SelectionPolicy::Prepend => StackerSelection::collapsed(0),
    }
}

fn editor_command_for_external_action(action: &ExternalAction) -> Option<CommandId> {
    match action {
        ExternalAction::Save => Some(CommandId::Save),
        ExternalAction::Undo => Some(CommandId::Undo),
        ExternalAction::Redo => Some(CommandId::Redo),
        ExternalAction::SelectAll => Some(CommandId::SelectAll),
        ExternalAction::Copy => Some(CommandId::Copy),
        ExternalAction::Paste => Some(CommandId::Paste),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use llnzy::external_command::ExternalAction;

    #[test]
    fn editor_command_mapping_covers_common_text_actions() {
        assert_eq!(
            editor_command_for_external_action(&ExternalAction::Save),
            Some(CommandId::Save)
        );
        assert_eq!(
            editor_command_for_external_action(&ExternalAction::Undo),
            Some(CommandId::Undo)
        );
        assert_eq!(
            editor_command_for_external_action(&ExternalAction::Redo),
            Some(CommandId::Redo)
        );
        assert_eq!(
            editor_command_for_external_action(&ExternalAction::SelectAll),
            Some(CommandId::SelectAll)
        );
        assert_eq!(
            editor_command_for_external_action(&ExternalAction::Copy),
            Some(CommandId::Copy)
        );
        assert_eq!(
            editor_command_for_external_action(&ExternalAction::Paste),
            Some(CommandId::Paste)
        );
    }

    #[test]
    fn editor_command_mapping_rejects_surface_specific_actions() {
        assert_eq!(
            editor_command_for_external_action(&ExternalAction::InsertText {
                text: "hello".to_string()
            }),
            None
        );
    }
}
