use std::path::Path;

use crate::editor::keymap::KeyAction;
use crate::editor::syntax::SyntaxEngine;
use crate::editor::MarkdownViewMode;
use crate::editor::{buffer::Position, BufferView};

use super::command_palette::CommandId;
use super::editor_folding::{self, FoldingCommand};
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
    ToggleLineComment,
    ToggleBlockComment,
    JumpToMatchingBracket,
    FoldCurrent,
    UnfoldCurrent,
    FoldAll,
    UnfoldAll,
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
            CommandId::ToggleLineComment => Self::ToggleLineComment,
            CommandId::ToggleBlockComment => Self::ToggleBlockComment,
            CommandId::JumpToMatchingBracket => Self::JumpToMatchingBracket,
            CommandId::FoldCurrent => Self::FoldCurrent,
            CommandId::UnfoldCurrent => Self::UnfoldCurrent,
            CommandId::FoldAll => Self::FoldAll,
            CommandId::UnfoldAll => Self::UnfoldAll,
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
            | CommandId::ToggleWordWrap
            | CommandId::ToggleEffects
            | CommandId::ToggleFps
            | CommandId::Stacker(_) => return None,
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

    fn command_toggle_line_comment(&mut self) -> bool {
        let mut changed = false;
        let mut status = None;
        self.with_active_buf_view(|buf, view| {
            let style = comment_style(view.lang_id, buf.path());
            if let Some(prefix) = style.line {
                let (start_line, end_line) = selected_line_range(view, buf);
                changed = toggle_line_comments_as_command(buf, start_line, end_line, prefix);
                if changed {
                    view.cursor.desired_col = None;
                }
            } else if let Some((open, close)) = style.block {
                let (start_line, end_line) = selected_line_range(view, buf);
                let before = buf.text();
                for line in (start_line..=end_line).rev() {
                    let start = Position::new(line, 0);
                    let end = Position::new(line, buf.line_len(line));
                    buf.toggle_block_comment(start, end, open, close);
                }
                changed = before != buf.text();
                if changed {
                    view.cursor.desired_col = None;
                }
            } else {
                status = Some("No comment style for this file".to_string());
            }
        });
        self.status_msg = status;
        changed
    }

    fn command_toggle_block_comment(&mut self) -> bool {
        let mut changed = false;
        let mut status = None;
        self.with_active_buf_view(|buf, view| {
            let style = comment_style(view.lang_id, buf.path());
            let Some((open, close)) = style.block else {
                status = Some("No block comment style for this file".to_string());
                return;
            };

            let before = buf.text();
            let had_selection = view.cursor.has_selection();
            let (start, end) = view.cursor.selection().unwrap_or_else(|| {
                let line = view.cursor.pos.line;
                (
                    Position::new(line, 0),
                    Position::new(line, buf.line_len(line)),
                )
            });
            let (new_start, new_end) = buf.toggle_block_comment(start, end, open, close);
            changed = before != buf.text();
            if changed {
                if had_selection {
                    view.cursor.anchor = Some(new_start);
                    view.cursor.pos = new_end;
                } else {
                    view.cursor.clear_selection();
                    view.cursor.pos = new_end;
                }
                view.cursor.desired_col = None;
            }
        });
        self.status_msg = status;
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
        (action.toggle_line_comment, EditorCommand::ToggleLineComment),
        (
            action.toggle_block_comment,
            EditorCommand::ToggleBlockComment,
        ),
        (
            action.jump_to_matching_bracket,
            EditorCommand::JumpToMatchingBracket,
        ),
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

#[derive(Clone, Copy)]
struct CommentStyle {
    line: Option<&'static str>,
    block: Option<(&'static str, &'static str)>,
}

fn selected_line_range(view: &BufferView, buf: &crate::editor::buffer::Buffer) -> (usize, usize) {
    if let Some((start, end)) = view.cursor.selection() {
        let mut end_line = end.line;
        if end.col == 0 && end.line > start.line {
            end_line -= 1;
        }
        (
            start.line.min(buf.line_count().saturating_sub(1)),
            end_line.min(buf.line_count().saturating_sub(1)),
        )
    } else {
        let line = view.cursor.pos.line.min(buf.line_count().saturating_sub(1));
        (line, line)
    }
}

fn toggle_line_comments_as_command(
    buf: &mut crate::editor::buffer::Buffer,
    start_line: usize,
    end_line: usize,
    prefix: &str,
) -> bool {
    if prefix.is_empty() || buf.line_count() == 0 {
        return false;
    }
    let end_line = end_line.min(buf.line_count().saturating_sub(1));
    if start_line > end_line {
        return false;
    }

    let mut any_content = false;
    let mut all_commented = true;
    for line_idx in start_line..=end_line {
        let line = buf.line(line_idx);
        if line.trim().is_empty() {
            continue;
        }
        any_content = true;
        let indent = line_indent(line);
        if !line[indent.len()..].starts_with(prefix) {
            all_commented = false;
            break;
        }
    }

    if !any_content {
        return false;
    }

    let replacement = (start_line..=end_line)
        .map(|line_idx| {
            let line = buf.line(line_idx);
            if line.trim().is_empty() {
                return line.to_string();
            }

            let indent = line_indent(line);
            let after_indent = &line[indent.len()..];
            if all_commented {
                let after_prefix = &after_indent[prefix.len()..];
                let after_prefix = after_prefix.strip_prefix(' ').unwrap_or(after_prefix);
                format!("{indent}{after_prefix}")
            } else {
                format!("{indent}{prefix} {after_indent}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let start = Position::new(start_line, 0);
    let end = Position::new(end_line, buf.line_len(end_line));
    if buf.text_range(start, end) == replacement {
        return false;
    }
    buf.replace(start, end, &replacement);
    true
}

fn line_indent(line: &str) -> &str {
    let trimmed = line.trim_start_matches(|c: char| c == ' ' || c == '\t');
    &line[..line.len() - trimmed.len()]
}

fn comment_style(lang_id: Option<&'static str>, path: Option<&Path>) -> CommentStyle {
    let ext = path
        .and_then(|p| p.extension())
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    let lang = lang_id.or_else(|| match ext.as_deref() {
        Some("rs") => Some("rust"),
        Some("js" | "mjs" | "cjs" | "jsx") => Some("javascript"),
        Some("ts" | "mts" | "cts") => Some("typescript"),
        Some("tsx") => Some("tsx"),
        Some("py" | "pyi") => Some("python"),
        Some("rb") => Some("ruby"),
        Some("go") => Some("go"),
        Some("c" | "h") => Some("c"),
        Some("cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx") => Some("cpp"),
        Some("java") => Some("java"),
        Some("kt" | "kts") => Some("kotlin"),
        Some("swift") => Some("swift"),
        Some("sql") => Some("sql"),
        Some("lua") => Some("lua"),
        Some("html" | "htm") => Some("html"),
        Some("css" | "scss") => Some("css"),
        Some("sh" | "bash" | "zsh") => Some("bash"),
        Some("toml") => Some("toml"),
        _ => None,
    });

    match lang {
        Some(
            "rust" | "javascript" | "typescript" | "tsx" | "go" | "c" | "cpp" | "java" | "kotlin"
            | "swift",
        ) => CommentStyle {
            line: Some("//"),
            block: Some(("/*", "*/")),
        },
        Some("python" | "ruby" | "bash" | "toml") => CommentStyle {
            line: Some("#"),
            block: None,
        },
        Some("sql" | "lua") => CommentStyle {
            line: Some("--"),
            block: None,
        },
        Some("html") => CommentStyle {
            line: None,
            block: Some(("<!--", "-->")),
        },
        Some("css") => CommentStyle {
            line: None,
            block: Some(("/*", "*/")),
        },
        _ => CommentStyle {
            line: Some("//"),
            block: Some(("/*", "*/")),
        },
    }
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

    fn state_with_text_path(text: &str, file_name: &str) -> EditorViewState {
        let mut state = state_with_text(text);
        let path = std::env::temp_dir().join(file_name);
        state.editor.buffers[0].set_path(path);
        state
    }

    fn state_with_rust_tree(text: &str) -> EditorViewState {
        let mut state = state_with_text_path(text, "main.rs");
        let mut syntax = SyntaxEngine::new();
        state.editor.views[0].lang_id = Some("rust");
        state.editor.views[0].tree = syntax.parse("rust", text);
        assert!(state.editor.views[0].tree.is_some());
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
    fn palette_comment_commands_map_to_editor_commands() {
        assert_eq!(
            EditorCommand::from_palette(CommandId::ToggleLineComment),
            Some(EditorCommand::ToggleLineComment)
        );
        assert_eq!(
            EditorCommand::from_palette(CommandId::ToggleBlockComment),
            Some(EditorCommand::ToggleBlockComment)
        );
        assert_eq!(
            EditorCommand::from_palette(CommandId::JumpToMatchingBracket),
            Some(EditorCommand::JumpToMatchingBracket)
        );
        assert_eq!(
            EditorCommand::from_palette(CommandId::FoldCurrent),
            Some(EditorCommand::FoldCurrent)
        );
        assert_eq!(
            EditorCommand::from_palette(CommandId::UnfoldCurrent),
            Some(EditorCommand::UnfoldCurrent)
        );
        assert_eq!(
            EditorCommand::from_palette(CommandId::FoldAll),
            Some(EditorCommand::FoldAll)
        );
        assert_eq!(
            EditorCommand::from_palette(CommandId::UnfoldAll),
            Some(EditorCommand::UnfoldAll)
        );
    }

    #[test]
    fn dispatch_find_toggles_editor_search() {
        let mut state = state_with_text("hello");
        state.dispatch_editor_command(EditorCommand::Find, None);
        assert!(state.editor_search.active);
        assert!(!state.editor_search.replace_mode);

        state.dispatch_editor_command(EditorCommand::Find, None);
        assert!(!state.editor_search.active);
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
    fn dispatch_toggle_line_comment_comments_selected_rust_lines() {
        let mut state = state_with_text_path("fn main() {}\nlet x = 1;", "main.rs");
        state.editor.views[0].cursor.anchor = Some(Position::new(0, 0));
        state.editor.views[0].cursor.pos = Position::new(1, 10);

        let outcome = state.dispatch_editor_command(EditorCommand::ToggleLineComment, None);

        assert!(outcome.changed_buffer);
        assert_eq!(
            state.editor.buffers[0].text(),
            "// fn main() {}\n// let x = 1;"
        );

        let undo = state.dispatch_editor_command(EditorCommand::Undo, None);
        assert!(undo.changed_buffer);
        assert_eq!(state.editor.buffers[0].text(), "fn main() {}\nlet x = 1;");
    }

    #[test]
    fn dispatch_toggle_line_comment_uses_python_hash_prefix() {
        let mut state = state_with_text_path("print('hi')", "script.py");

        let outcome = state.dispatch_editor_command(EditorCommand::ToggleLineComment, None);

        assert!(outcome.changed_buffer);
        assert_eq!(state.editor.buffers[0].text(), "# print('hi')");
    }

    #[test]
    fn dispatch_toggle_line_comment_uses_sql_dash_prefix() {
        let mut state = state_with_text_path("select * from users;", "query.sql");

        let outcome = state.dispatch_editor_command(EditorCommand::ToggleLineComment, None);

        assert!(outcome.changed_buffer);
        assert_eq!(state.editor.buffers[0].text(), "-- select * from users;");
    }

    #[test]
    fn dispatch_toggle_block_comment_wraps_selected_rust_text() {
        let mut state = state_with_text_path("let value = 1;", "main.rs");
        state.editor.views[0].cursor.anchor = Some(Position::new(0, 4));
        state.editor.views[0].cursor.pos = Position::new(0, 9);

        let outcome = state.dispatch_editor_command(EditorCommand::ToggleBlockComment, None);

        assert!(outcome.changed_buffer);
        assert_eq!(state.editor.buffers[0].text(), "let /*value*/ = 1;");
        assert_eq!(
            state.editor.views[0].cursor.selection(),
            Some((Position::new(0, 6), Position::new(0, 11)))
        );
    }

    #[test]
    fn dispatch_toggle_block_comment_reports_missing_style() {
        let mut state = state_with_text_path("print('hi')", "script.py");

        let outcome = state.dispatch_editor_command(EditorCommand::ToggleBlockComment, None);

        assert!(!outcome.changed_buffer);
        assert_eq!(
            state.status_msg.as_deref(),
            Some("No block comment style for this file")
        );
        assert_eq!(state.editor.buffers[0].text(), "print('hi')");
    }

    #[test]
    fn key_action_toggle_comment_routes_through_command_dispatch() {
        let mut state = state_with_text_path("puts 'hi'", "script.rb");
        let action = KeyAction {
            toggle_line_comment: true,
            ..KeyAction::default()
        };

        let outcome = state.dispatch_key_action_commands(&action, None);

        assert!(outcome.changed_buffer);
        assert_eq!(state.editor.buffers[0].text(), "# puts 'hi'");
    }

    #[test]
    fn dispatch_jump_to_matching_bracket_moves_cursor_to_pair() {
        let mut state = state_with_text("fn main() { call(1); }");
        state.editor.views[0].cursor.pos = Position::new(0, 10);
        state.editor.views[0].cursor.anchor = Some(Position::new(0, 0));

        let outcome = state.dispatch_editor_command(EditorCommand::JumpToMatchingBracket, None);

        assert!(!outcome.changed_buffer);
        assert_eq!(state.editor.views[0].cursor.pos, Position::new(0, 21));
        assert!(!state.editor.views[0].cursor.has_selection());
        assert_eq!(state.status_msg, None);
    }

    #[test]
    fn dispatch_jump_to_matching_bracket_reports_missing_pair() {
        let mut state = state_with_text("let value = 1;");
        state.editor.views[0].cursor.pos = Position::new(0, 4);

        let outcome = state.dispatch_editor_command(EditorCommand::JumpToMatchingBracket, None);

        assert!(!outcome.changed_buffer);
        assert_eq!(state.editor.views[0].cursor.pos, Position::new(0, 4));
        assert_eq!(state.status_msg.as_deref(), Some("No matching bracket"));
    }

    #[test]
    fn key_action_jump_to_matching_bracket_routes_through_command_dispatch() {
        let mut state = state_with_text("{\n    value\n}");
        let action = KeyAction {
            jump_to_matching_bracket: true,
            ..KeyAction::default()
        };

        let outcome = state.dispatch_key_action_commands(&action, None);

        assert!(!outcome.changed_buffer);
        assert_eq!(state.editor.views[0].cursor.pos, Position::new(2, 0));
    }

    #[test]
    fn dispatch_fold_current_folds_innermost_syntax_range() {
        let mut state = state_with_rust_tree(
            "fn main() {\n    if true {\n        println!(\"x\");\n    }\n}\n",
        );
        state.editor.views[0].cursor.pos = Position::new(2, 0);

        let outcome = state.dispatch_editor_command(EditorCommand::FoldCurrent, None);

        assert!(!outcome.changed_buffer);
        assert!(state.editor.views[0]
            .folded_ranges
            .iter()
            .any(|range| range.start_line == 1 && range.end_line >= 3));
        assert_eq!(state.status_msg, None);
    }

    #[test]
    fn dispatch_fold_all_and_unfold_all_update_active_view() {
        let mut state = state_with_rust_tree(
            "fn main() {\n    if true {\n        println!(\"x\");\n    }\n}\n",
        );

        let fold = state.dispatch_editor_command(EditorCommand::FoldAll, None);

        assert!(!fold.changed_buffer);
        assert!(!state.editor.views[0].folded_ranges.is_empty());

        let unfold = state.dispatch_editor_command(EditorCommand::UnfoldAll, None);

        assert!(!unfold.changed_buffer);
        assert!(state.editor.views[0].folded_ranges.is_empty());
    }

    #[test]
    fn dispatch_unfold_current_removes_covering_fold() {
        let mut state = state_with_rust_tree("fn main() {\n    println!(\"x\");\n}\n");
        state.editor.views[0]
            .folded_ranges
            .push(crate::editor::syntax::FoldRange {
                start_line: 0,
                end_line: 2,
            });
        state.editor.views[0].cursor.pos = Position::new(1, 0);

        let outcome = state.dispatch_editor_command(EditorCommand::UnfoldCurrent, None);

        assert!(!outcome.changed_buffer);
        assert!(state.editor.views[0].folded_ranges.is_empty());
    }

    #[test]
    fn dispatch_fold_current_reports_missing_tree() {
        let mut state = state_with_text("plain text");

        let outcome = state.dispatch_editor_command(EditorCommand::FoldCurrent, None);

        assert!(!outcome.changed_buffer);
        assert_eq!(
            state.status_msg.as_deref(),
            Some("No foldable syntax tree for this file")
        );
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
