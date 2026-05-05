use std::path::Path;

use crate::editor::keymap::KeyAction;
use crate::editor::syntax::SyntaxEngine;

use super::super::editor_folding::{self, FoldingCommand};
use super::super::explorer_view::EditorViewState;
use super::key_actions::key_action_commands;
use super::types::{EditorCommand, EditorCommandOutcome};

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
            EditorCommand::ToggleLineComment => {
                outcome.changed_buffer = self.command_toggle_line_comment();
            }
            EditorCommand::ToggleBlockComment => {
                outcome.changed_buffer = self.command_toggle_block_comment();
            }
            EditorCommand::JumpToMatchingBracket => self.command_jump_to_matching_bracket(),
            EditorCommand::FoldCurrent => self.command_apply_folding(FoldingCommand::FoldCurrent),
            EditorCommand::UnfoldCurrent => {
                self.command_apply_folding(FoldingCommand::UnfoldCurrent)
            }
            EditorCommand::FoldAll => self.command_apply_folding(FoldingCommand::FoldAll),
            EditorCommand::UnfoldAll => self.command_apply_folding(FoldingCommand::UnfoldAll),
            EditorCommand::FormatDocument => self.format_document(),
            EditorCommand::RenameSymbol => self.open_rename_input(),
            EditorCommand::GoToDefinition => self.request_goto_definition(),
            EditorCommand::ShowHover => self.request_hover(),
            EditorCommand::RequestCompletion => self.request_completion(),
            EditorCommand::CodeActions => self.request_code_actions(),
            EditorCommand::DocumentSymbols => self.request_document_symbols(),
            EditorCommand::Find => {
                if self.editor_search.active && !self.editor_search.replace_mode {
                    self.editor_search.close();
                } else {
                    self.editor_search.open_find();
                    self.editor_search.mark_dirty();
                }
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

    pub(super) fn with_active_buf_view(
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

    fn command_jump_to_matching_bracket(&mut self) {
        let mut status = None;
        self.with_active_buf_view(|buf, view| {
            if let Some((at, matching)) = buf.matching_bracket(view.cursor.pos) {
                view.cursor.clear_selection();
                view.cursor.pos = if view.cursor.pos == matching {
                    at
                } else {
                    matching
                };
                view.cursor.desired_col = None;
            } else {
                status = Some("No matching bracket".to_string());
            }
        });
        self.status_msg = status;
    }

    fn command_apply_folding(&mut self, command: FoldingCommand) {
        let mut status = None;
        self.with_active_buf_view(|buf, view| {
            if matches!(
                command,
                FoldingCommand::UnfoldCurrent | FoldingCommand::UnfoldAll
            ) {
                editor_folding::apply_folding_command(view, &[], buf.line_count(), command);
                return;
            }

            let Some(tree) = view.tree.as_ref() else {
                status = Some("No foldable syntax tree for this file".to_string());
                return;
            };
            let syntax = SyntaxEngine::default();
            let foldable_ranges = syntax.foldable_ranges(tree);
            if foldable_ranges.is_empty()
                && matches!(
                    command,
                    FoldingCommand::FoldCurrent | FoldingCommand::FoldAll
                )
            {
                status = Some("No foldable ranges for this file".to_string());
                return;
            }
            editor_folding::apply_folding_command(
                view,
                &foldable_ranges,
                buf.line_count(),
                command,
            );
        });
        self.status_msg = status;
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
        self.recovery_dirty = true;
        self.active_snippet = None;
    }
}
