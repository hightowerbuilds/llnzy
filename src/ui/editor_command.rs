use std::path::Path;

use crate::editor::keymap::KeyAction;
use crate::editor::MarkdownViewMode;

use super::command_palette::CommandId;
use super::explorer_view::EditorViewState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EditorCommand {
    Save,
    Undo,
    Redo,
    SelectAll,
    Cut,
    Copy,
    Paste,
    DeleteLine,
    DuplicateLine,
    MoveLineUp,
    MoveLineDown,
    FormatDocument,
    RenameSymbol,
    GoToDefinition,
    ShowHover,
    RequestCompletion,
    CodeActions,
    DocumentSymbols,
    Find,
    FindReplace,
    FindReferences,
    WorkspaceSymbols,
    ProjectSearch,
    RunTask,
    FindFile,
    ToggleMarkdownMode,
    SetMarkdownMode(MarkdownViewMode),
}

#[derive(Default)]
pub(crate) struct EditorCommandOutcome {
    pub open_file_finder: bool,
    pub changed_buffer: bool,
}

impl EditorCommand {
    pub(crate) fn from_palette(id: CommandId) -> Option<Self> {
        Some(match id {
            CommandId::Save => Self::Save,
            CommandId::Undo => Self::Undo,
            CommandId::Redo => Self::Redo,
            CommandId::SelectAll => Self::SelectAll,
            CommandId::Cut => Self::Cut,
            CommandId::Copy => Self::Copy,
            CommandId::Paste => Self::Paste,
            CommandId::DeleteLine => Self::DeleteLine,
            CommandId::DuplicateLine => Self::DuplicateLine,
            CommandId::MoveLineUp => Self::MoveLineUp,
            CommandId::MoveLineDown => Self::MoveLineDown,
            CommandId::FormatDocument => Self::FormatDocument,
            CommandId::RenameSymbol => Self::RenameSymbol,
            CommandId::GoToDefinition => Self::GoToDefinition,
            CommandId::ShowHover => Self::ShowHover,
            CommandId::CodeActions => Self::CodeActions,
            CommandId::DocumentSymbols => Self::DocumentSymbols,
            CommandId::Find => Self::Find,
            CommandId::FindReplace => Self::FindReplace,
            CommandId::FindReferences => Self::FindReferences,
            CommandId::WorkspaceSymbols => Self::WorkspaceSymbols,
            CommandId::ProjectSearch => Self::ProjectSearch,
            CommandId::RunTask => Self::RunTask,
            CommandId::FindFile => Self::FindFile,
            CommandId::ToggleMarkdownMode => Self::ToggleMarkdownMode,
            CommandId::MarkdownSource => Self::SetMarkdownMode(MarkdownViewMode::Source),
            CommandId::MarkdownPreview => Self::SetMarkdownMode(MarkdownViewMode::Preview),
            CommandId::MarkdownSplit => Self::SetMarkdownMode(MarkdownViewMode::Split),
            CommandId::OpenWorkspace
            | CommandId::ToggleTerminal
            | CommandId::ToggleSidebar
            | CommandId::NewTab
            | CommandId::CloseTab
            | CommandId::NextTab
            | CommandId::PrevTab
            | CommandId::ToggleEffects
            | CommandId::ToggleFps => return None,
        })
    }
}

impl EditorViewState {
    pub(crate) fn dispatch_editor_command(
        &mut self,
        command: EditorCommand,
        project_root: Option<&Path>,
    ) -> EditorCommandOutcome {
        let mut outcome = EditorCommandOutcome::default();
        match command {
            EditorCommand::Save => self.command_save(),
            EditorCommand::Undo => {
                outcome.changed_buffer = self.command_undo();
            }
            EditorCommand::Redo => {
                outcome.changed_buffer = self.command_redo();
            }
            EditorCommand::SelectAll => self.with_active_buf_view(|buf, view| {
                view.cursor.select_all(buf);
            }),
            EditorCommand::Cut => {
                outcome.changed_buffer = self.command_cut();
            }
            EditorCommand::Copy => self.command_copy(),
            EditorCommand::Paste => {
                outcome.changed_buffer = self.command_paste();
            }
            EditorCommand::DeleteLine => {
                outcome.changed_buffer = self.command_delete_line();
            }
            EditorCommand::DuplicateLine => {
                outcome.changed_buffer = self.command_duplicate_line();
            }
            EditorCommand::MoveLineUp => {
                outcome.changed_buffer = self.command_move_line_up();
            }
            EditorCommand::MoveLineDown => {
                outcome.changed_buffer = self.command_move_line_down();
            }
            EditorCommand::FormatDocument => self.format_document(),
            EditorCommand::RenameSymbol => self.open_rename_input(),
            EditorCommand::GoToDefinition => self.request_goto_definition(),
            EditorCommand::ShowHover => self.request_hover(),
            EditorCommand::RequestCompletion => self.request_completion(),
            EditorCommand::CodeActions => self.request_code_actions(),
            EditorCommand::DocumentSymbols => self.request_document_symbols(),
            EditorCommand::Find => {
                self.editor_search.open_find();
                self.editor_search.mark_dirty();
            }
            EditorCommand::FindReplace => {
                self.editor_search.open_replace();
                self.editor_search.mark_dirty();
            }
            EditorCommand::FindReferences => self.request_references(),
            EditorCommand::WorkspaceSymbols => self.open_workspace_symbols(),
            EditorCommand::ProjectSearch => self.project_search.open(),
            EditorCommand::RunTask => self.open_task_picker(project_root),
            EditorCommand::FindFile => outcome.open_file_finder = true,
            EditorCommand::ToggleMarkdownMode => self.with_active_view(|view| {
                view.markdown_mode = view.markdown_mode.cycle();
            }),
            EditorCommand::SetMarkdownMode(mode) => self.with_active_view(|view| {
                view.markdown_mode = mode;
            }),
        }

        if outcome.changed_buffer {
            self.after_command_buffer_edit();
        }
        outcome
    }

    pub(crate) fn dispatch_key_action_commands(
        &mut self,
        action: &KeyAction,
        project_root: Option<&Path>,
    ) -> EditorCommandOutcome {
        let mut outcome = EditorCommandOutcome::default();
        for command in key_action_commands(action) {
            let next = self.dispatch_editor_command(command, project_root);
            outcome.open_file_finder |= next.open_file_finder;
            outcome.changed_buffer |= next.changed_buffer;
        }
        outcome
    }

    fn with_active_buf_view(
        &mut self,
        f: impl FnOnce(&mut crate::editor::buffer::Buffer, &mut crate::editor::BufferView),
    ) {
        if self.editor.active < self.editor.buffers.len() {
            let active = self.editor.active;
            f(
                &mut self.editor.buffers[active],
                &mut self.editor.views[active],
            );
        }
    }

    fn with_active_view(&mut self, f: impl FnOnce(&mut crate::editor::BufferView)) {
        if let Some(view) = self.editor.views.get_mut(self.editor.active) {
            f(view);
        }
    }

    fn command_save(&mut self) {
        let Some(buf) = self.editor.buffers.get_mut(self.editor.active) else {
            return;
        };
        match buf.save() {
            Ok(()) => {
                self.status_msg = Some("Saved".to_string());
                self.lsp_did_save();
                self.request_hints_and_lenses();
            }
            Err(e) => self.status_msg = Some(save_failed_status(&e)),
        }
    }

    fn command_undo(&mut self) -> bool {
        let mut changed = false;
        self.with_active_buf_view(|buf, view| {
            if let Some(pos) = buf.undo() {
                view.cursor.clear_selection();
                view.cursor.pos = pos;
                view.cursor.desired_col = None;
                changed = true;
            }
        });
        changed
    }

    fn command_redo(&mut self) -> bool {
        let mut changed = false;
        self.with_active_buf_view(|buf, view| {
            if let Some(pos) = buf.redo() {
                view.cursor.clear_selection();
                view.cursor.pos = pos;
                view.cursor.desired_col = None;
                changed = true;
            }
        });
        changed
    }

    fn command_copy(&mut self) {
        let mut copied = None;
        self.with_active_buf_view(|buf, view| {
            copied = Some(if let Some((start, end)) = view.cursor.selection() {
                buf.text_range(start, end)
            } else {
                buf.line_text_for_copy(view.cursor.pos.line)
            });
        });
        self.clipboard_out = copied;
    }

    fn command_cut(&mut self) -> bool {
        let mut changed = false;
        let mut copied = None;
        self.status_msg = None;
        self.with_active_buf_view(|buf, view| {
            if let Some((start, end)) = view.cursor.selection() {
                copied = Some(buf.text_range(start, end));
                buf.delete(start, end);
                view.cursor.clear_selection();
                view.cursor.pos = start;
            } else {
                copied = Some(buf.line_text_for_copy(view.cursor.pos.line));
                view.cursor.pos = buf.delete_line(view.cursor.pos.line);
                view.cursor.clear_selection();
            }
            view.cursor.desired_col = None;
            changed = true;
        });
        self.clipboard_out = copied;
        changed
    }

    fn command_paste(&mut self) -> bool {
        let Some(text) = self.clipboard_in.take() else {
            return false;
        };
        let mut changed = false;
        self.status_msg = None;
        self.with_active_buf_view(|buf, view| {
            if let Some((start, end)) = view.cursor.selection() {
                buf.delete(start, end);
                view.cursor.clear_selection();
                view.cursor.pos = start;
            }
            let end_pos = buf.compute_end_pos_pub(view.cursor.pos, &text);
            buf.insert(view.cursor.pos, &text);
            view.cursor.pos = end_pos;
            view.cursor.desired_col = None;
            changed = true;
        });
        changed
    }

    fn command_delete_line(&mut self) -> bool {
        let mut changed = false;
        self.status_msg = None;
        self.with_active_buf_view(|buf, view| {
            view.cursor.pos = buf.delete_line(view.cursor.pos.line);
            view.cursor.clear_selection();
            view.cursor.desired_col = None;
            changed = true;
        });
        changed
    }

    fn command_duplicate_line(&mut self) -> bool {
        let mut changed = false;
        self.status_msg = None;
        self.with_active_buf_view(|buf, view| {
            view.cursor.pos = buf.duplicate_line(view.cursor.pos.line);
            view.cursor.clear_selection();
            view.cursor.desired_col = None;
            changed = true;
        });
        changed
    }

    fn command_move_line_up(&mut self) -> bool {
        let mut changed = false;
        self.status_msg = None;
        self.with_active_buf_view(|buf, view| {
            if let Some(pos) = buf.move_line_up(view.cursor.pos.line) {
                view.cursor.pos = pos;
                view.cursor.clear_selection();
                view.cursor.desired_col = None;
                changed = true;
            }
        });
        changed
    }

    fn command_move_line_down(&mut self) -> bool {
        let mut changed = false;
        self.status_msg = None;
        self.with_active_buf_view(|buf, view| {
            if let Some(pos) = buf.move_line_down(view.cursor.pos.line) {
                view.cursor.pos = pos;
                view.cursor.clear_selection();
                view.cursor.desired_col = None;
                changed = true;
            }
        });
        changed
    }

    fn open_rename_input(&mut self) {
        if self.rename_input.is_some() {
            return;
        }
        let Some((buf, view)) = self.editor.active_buf_view() else {
            return;
        };
        let pos = view.cursor.pos;
        let line = buf.line(pos.line);
        let chars: Vec<char> = line.chars().collect();
        let mut start = pos.col;
        let mut end = pos.col;
        while start > 0
            && chars
                .get(start - 1)
                .is_some_and(|c| c.is_alphanumeric() || *c == '_')
        {
            start -= 1;
        }
        while end < chars.len()
            && chars
                .get(end)
                .is_some_and(|c| c.is_alphanumeric() || *c == '_')
        {
            end += 1;
        }
        self.rename_input = Some(chars[start..end].iter().collect::<String>());
    }

    fn open_workspace_symbols(&mut self) {
        self.request_workspace_symbols("");
        self.workspace_symbols_popup = Some(Vec::new());
        self.workspace_symbols_selected = 0;
        self.workspace_symbols_query.clear();
    }

    fn open_task_picker(&mut self, project_root: Option<&Path>) {
        let Some(project_root) = project_root else {
            self.status_msg = Some("No project root for task detection".to_string());
            return;
        };
        let tasks = crate::tasks::detect_tasks(project_root);
        if tasks.is_empty() {
            self.status_msg = Some("No tasks detected in project".to_string());
        } else {
            self.task_picker = Some(tasks);
            self.task_picker_selected = 0;
        }
    }

    fn after_command_buffer_edit(&mut self) {
        let active = self.editor.active;
        if let Some(view) = self.editor.views.get_mut(active) {
            view.tree_dirty = true;
            view.folded_ranges.clear();
            if let Some(gutter) = &mut view.git_gutter {
                gutter.mark_dirty();
            }
        }
        self.last_edit = None;
        self.lsp_did_change();
        self.hover_text = None;
        if self.editor_search.active {
            self.editor_search.mark_dirty();
        }
        self.active_snippet = None;
    }
}

fn key_action_commands(action: &KeyAction) -> impl Iterator<Item = EditorCommand> + '_ {
    [
        (action.save, EditorCommand::Save),
        (action.undo, EditorCommand::Undo),
        (action.redo, EditorCommand::Redo),
        (action.select_all, EditorCommand::SelectAll),
        (action.cut, EditorCommand::Cut),
        (action.copy, EditorCommand::Copy),
        (action.paste, EditorCommand::Paste),
        (action.delete_line, EditorCommand::DeleteLine),
        (action.duplicate_line, EditorCommand::DuplicateLine),
        (action.move_line_up, EditorCommand::MoveLineUp),
        (action.move_line_down, EditorCommand::MoveLineDown),
        (action.goto_definition, EditorCommand::GoToDefinition),
        (action.request_hover, EditorCommand::ShowHover),
        (action.request_completion, EditorCommand::RequestCompletion),
        (action.format_document, EditorCommand::FormatDocument),
        (action.rename_symbol, EditorCommand::RenameSymbol),
        (action.code_actions, EditorCommand::CodeActions),
        (action.open_file_finder, EditorCommand::FindFile),
        (action.document_symbols, EditorCommand::DocumentSymbols),
        (action.workspace_symbols, EditorCommand::WorkspaceSymbols),
        (action.find_references, EditorCommand::FindReferences),
        (action.open_find, EditorCommand::Find),
        (action.open_find_replace, EditorCommand::FindReplace),
        (action.project_search, EditorCommand::ProjectSearch),
        (action.run_task, EditorCommand::RunTask),
    ]
    .into_iter()
    .filter_map(|(enabled, command)| enabled.then_some(command))
}

fn save_failed_status(error: &str) -> String {
    format!("Save failed: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::buffer::Buffer;
    use crate::editor::buffer::Position;
    use crate::editor::BufferView;

    fn state_with_text(text: &str) -> EditorViewState {
        let mut state = EditorViewState::default();
        let mut buffer = Buffer::empty();
        buffer.insert(Position::new(0, 0), text);
        state.editor.buffers.push(buffer);
        state.editor.views.push(BufferView::default());
        state
    }

    #[test]
    fn palette_save_maps_to_editor_save_command() {
        assert_eq!(
            EditorCommand::from_palette(CommandId::Save),
            Some(EditorCommand::Save)
        );
    }

    #[test]
    fn app_palette_commands_do_not_map_to_editor_commands() {
        assert_eq!(EditorCommand::from_palette(CommandId::NewTab), None);
        assert_eq!(EditorCommand::from_palette(CommandId::ToggleSidebar), None);
    }

    #[test]
    fn dispatch_find_opens_editor_search() {
        let mut state = state_with_text("hello");
        state.dispatch_editor_command(EditorCommand::Find, None);
        assert!(state.editor_search.active);
        assert!(!state.editor_search.replace_mode);
    }

    #[test]
    fn dispatch_toggle_markdown_mode_cycles_active_view() {
        let mut state = state_with_text("hello");
        state.dispatch_editor_command(EditorCommand::ToggleMarkdownMode, None);
        assert_eq!(
            state.editor.views[0].markdown_mode,
            MarkdownViewMode::Preview
        );
    }

    #[test]
    fn dispatch_cut_copies_selection_and_removes_text() {
        let mut state = state_with_text("hello world");
        state.editor.views[0].cursor.anchor = Some(Position::new(0, 0));
        state.editor.views[0].cursor.pos = Position::new(0, 5);

        let outcome = state.dispatch_editor_command(EditorCommand::Cut, None);

        assert!(outcome.changed_buffer);
        assert_eq!(state.clipboard_out.as_deref(), Some("hello"));
        assert_eq!(state.editor.buffers[0].text(), " world");
    }

    #[test]
    fn dispatch_select_all_selects_entire_buffer() {
        let mut state = state_with_text("hello\nworld");

        let outcome = state.dispatch_editor_command(EditorCommand::SelectAll, None);

        assert!(!outcome.changed_buffer);
        assert_eq!(
            state.editor.views[0].cursor.selection(),
            Some((Position::new(0, 0), Position::new(1, 5)))
        );
    }

    #[test]
    fn dispatch_copy_uses_active_selection() {
        let mut state = state_with_text("hello world");
        state.editor.views[0].cursor.anchor = Some(Position::new(0, 6));
        state.editor.views[0].cursor.pos = Position::new(0, 11);

        let outcome = state.dispatch_editor_command(EditorCommand::Copy, None);

        assert!(!outcome.changed_buffer);
        assert_eq!(state.clipboard_out.as_deref(), Some("world"));
        assert_eq!(state.editor.buffers[0].text(), "hello world");
    }

    #[test]
    fn dispatch_paste_replaces_active_selection() {
        let mut state = state_with_text("hello world");
        state.editor.views[0].cursor.anchor = Some(Position::new(0, 6));
        state.editor.views[0].cursor.pos = Position::new(0, 11);
        state.clipboard_in = Some("llnzy".to_string());

        let outcome = state.dispatch_editor_command(EditorCommand::Paste, None);

        assert!(outcome.changed_buffer);
        assert_eq!(state.editor.buffers[0].text(), "hello llnzy");
        assert_eq!(state.editor.views[0].cursor.pos, Position::new(0, 11));
        assert!(!state.editor.views[0].cursor.has_selection());
    }

    #[test]
    fn key_action_delete_line_routes_through_command_dispatch() {
        let mut state = state_with_text("one\ntwo\nthree");
        state.editor.views[0].cursor.pos = Position::new(1, 0);
        let action = KeyAction {
            delete_line: true,
            ..KeyAction::default()
        };

        let outcome = state.dispatch_key_action_commands(&action, None);

        assert!(outcome.changed_buffer);
        assert_eq!(state.editor.buffers[0].text(), "one\nthree");
    }

    #[test]
    fn key_action_copy_routes_without_marking_buffer_changed() {
        let mut state = state_with_text("alpha\nbeta");
        state.editor.views[0].cursor.pos = Position::new(1, 0);
        let action = KeyAction {
            copy: true,
            ..KeyAction::default()
        };

        let outcome = state.dispatch_key_action_commands(&action, None);

        assert!(!outcome.changed_buffer);
        assert_eq!(state.clipboard_out.as_deref(), Some("beta\n"));
    }

    #[test]
    fn dispatch_save_failure_keeps_buffer_dirty_and_reports_status() {
        let mut state = state_with_text("unsaved");
        let missing_parent =
            std::env::temp_dir().join(format!("llnzy-command-missing-{}", std::process::id()));
        state.editor.buffers[0].set_path(missing_parent.join("file.txt"));

        state.dispatch_editor_command(EditorCommand::Save, None);

        assert!(state.editor.buffers[0].is_modified());
        assert!(state
            .status_msg
            .as_deref()
            .is_some_and(|message| message.starts_with("Save failed: ")));
    }
}
