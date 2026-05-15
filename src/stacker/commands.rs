use super::{
    formatting::{apply_list_prefix, char_to_byte_idx, ListButtonKind},
    input::StackerSelection,
    session::StackerSession,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StackerCommandId {
    Bold,
    Italic,
    UnorderedList,
    OrderedList,
    Heading1,
    Heading2,
    Heading3,
    Blockquote,
    InlineCode,
    CodeBlock,
    ChecklistItem,
    Clear,
    Undo,
    Redo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StackerCommandDescriptor {
    pub id: StackerCommandId,
    pub name: &'static str,
    pub toolbar_label: &'static str,
    pub keybinding: &'static str,
    pub tooltip: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StackerEditorCommand {
    Bold,
    Italic,
    UnorderedList,
    OrderedList,
    Heading(u8),
    Blockquote,
    InlineCode,
    CodeBlock,
    ChecklistItem,
    Clear,
    LoadText(String),
    Undo,
    Redo,
}

const fn stacker_command_descriptor_value(id: StackerCommandId) -> StackerCommandDescriptor {
    match id {
        StackerCommandId::Bold => StackerCommandDescriptor {
            id,
            name: "Stacker: Bold",
            toolbar_label: "B",
            keybinding: "Cmd+B",
            tooltip: "Bold selected text",
        },
        StackerCommandId::Italic => StackerCommandDescriptor {
            id,
            name: "Stacker: Italic",
            toolbar_label: "I",
            keybinding: "Cmd+I",
            tooltip: "Italicize selected text",
        },
        StackerCommandId::UnorderedList => StackerCommandDescriptor {
            id,
            name: "Stacker: Unordered List",
            toolbar_label: "•",
            keybinding: "",
            tooltip: "Make unordered list",
        },
        StackerCommandId::OrderedList => StackerCommandDescriptor {
            id,
            name: "Stacker: Numbered List",
            toolbar_label: "1.",
            keybinding: "",
            tooltip: "Make numbered list",
        },
        StackerCommandId::Heading1 => StackerCommandDescriptor {
            id,
            name: "Stacker: Heading 1",
            toolbar_label: "H1",
            keybinding: "",
            tooltip: "Make H1 heading",
        },
        StackerCommandId::Heading2 => StackerCommandDescriptor {
            id,
            name: "Stacker: Heading 2",
            toolbar_label: "H2",
            keybinding: "",
            tooltip: "Make H2 heading",
        },
        StackerCommandId::Heading3 => StackerCommandDescriptor {
            id,
            name: "Stacker: Heading 3",
            toolbar_label: "H3",
            keybinding: "",
            tooltip: "Make H3 heading",
        },
        StackerCommandId::Blockquote => StackerCommandDescriptor {
            id,
            name: "Stacker: Quote",
            toolbar_label: ">",
            keybinding: "",
            tooltip: "Make quote",
        },
        StackerCommandId::InlineCode => StackerCommandDescriptor {
            id,
            name: "Stacker: Inline Code",
            toolbar_label: "`",
            keybinding: "Cmd+`",
            tooltip: "Inline code",
        },
        StackerCommandId::CodeBlock => StackerCommandDescriptor {
            id,
            name: "Stacker: Code Block",
            toolbar_label: "```",
            keybinding: "",
            tooltip: "Code block",
        },
        StackerCommandId::ChecklistItem => StackerCommandDescriptor {
            id,
            name: "Stacker: Checklist Item",
            toolbar_label: "[ ]",
            keybinding: "",
            tooltip: "Checklist item",
        },
        StackerCommandId::Clear => StackerCommandDescriptor {
            id,
            name: "Stacker: Clear Draft",
            toolbar_label: "Clear",
            keybinding: "",
            tooltip: "Clear the current draft",
        },
        StackerCommandId::Undo => StackerCommandDescriptor {
            id,
            name: "Stacker: Undo",
            toolbar_label: "Undo",
            keybinding: "Cmd+Z",
            tooltip: "Undo Stacker edit",
        },
        StackerCommandId::Redo => StackerCommandDescriptor {
            id,
            name: "Stacker: Redo",
            toolbar_label: "Redo",
            keybinding: "Cmd+Shift+Z",
            tooltip: "Redo Stacker edit",
        },
    }
}

const STACKER_COMMAND_BOLD: StackerCommandDescriptor =
    stacker_command_descriptor_value(StackerCommandId::Bold);
const STACKER_COMMAND_ITALIC: StackerCommandDescriptor =
    stacker_command_descriptor_value(StackerCommandId::Italic);
const STACKER_COMMAND_UNORDERED_LIST: StackerCommandDescriptor =
    stacker_command_descriptor_value(StackerCommandId::UnorderedList);
const STACKER_COMMAND_ORDERED_LIST: StackerCommandDescriptor =
    stacker_command_descriptor_value(StackerCommandId::OrderedList);
const STACKER_COMMAND_HEADING1: StackerCommandDescriptor =
    stacker_command_descriptor_value(StackerCommandId::Heading1);
const STACKER_COMMAND_HEADING2: StackerCommandDescriptor =
    stacker_command_descriptor_value(StackerCommandId::Heading2);
const STACKER_COMMAND_HEADING3: StackerCommandDescriptor =
    stacker_command_descriptor_value(StackerCommandId::Heading3);
const STACKER_COMMAND_BLOCKQUOTE: StackerCommandDescriptor =
    stacker_command_descriptor_value(StackerCommandId::Blockquote);
const STACKER_COMMAND_INLINE_CODE: StackerCommandDescriptor =
    stacker_command_descriptor_value(StackerCommandId::InlineCode);
const STACKER_COMMAND_CODE_BLOCK: StackerCommandDescriptor =
    stacker_command_descriptor_value(StackerCommandId::CodeBlock);
const STACKER_COMMAND_CHECKLIST_ITEM: StackerCommandDescriptor =
    stacker_command_descriptor_value(StackerCommandId::ChecklistItem);
const STACKER_COMMAND_CLEAR: StackerCommandDescriptor =
    stacker_command_descriptor_value(StackerCommandId::Clear);
const STACKER_COMMAND_UNDO: StackerCommandDescriptor =
    stacker_command_descriptor_value(StackerCommandId::Undo);
const STACKER_COMMAND_REDO: StackerCommandDescriptor =
    stacker_command_descriptor_value(StackerCommandId::Redo);

pub const STACKER_COMMANDS: &[StackerCommandDescriptor] = &[
    STACKER_COMMAND_BOLD,
    STACKER_COMMAND_ITALIC,
    STACKER_COMMAND_UNORDERED_LIST,
    STACKER_COMMAND_ORDERED_LIST,
    STACKER_COMMAND_HEADING1,
    STACKER_COMMAND_HEADING2,
    STACKER_COMMAND_HEADING3,
    STACKER_COMMAND_BLOCKQUOTE,
    STACKER_COMMAND_INLINE_CODE,
    STACKER_COMMAND_CODE_BLOCK,
    STACKER_COMMAND_CHECKLIST_ITEM,
    STACKER_COMMAND_CLEAR,
    STACKER_COMMAND_UNDO,
    STACKER_COMMAND_REDO,
];

pub fn stacker_command_registry() -> &'static [StackerCommandDescriptor] {
    STACKER_COMMANDS
}

pub fn stacker_command_descriptor(id: StackerCommandId) -> &'static StackerCommandDescriptor {
    match id {
        StackerCommandId::Bold => &STACKER_COMMAND_BOLD,
        StackerCommandId::Italic => &STACKER_COMMAND_ITALIC,
        StackerCommandId::UnorderedList => &STACKER_COMMAND_UNORDERED_LIST,
        StackerCommandId::OrderedList => &STACKER_COMMAND_ORDERED_LIST,
        StackerCommandId::Heading1 => &STACKER_COMMAND_HEADING1,
        StackerCommandId::Heading2 => &STACKER_COMMAND_HEADING2,
        StackerCommandId::Heading3 => &STACKER_COMMAND_HEADING3,
        StackerCommandId::Blockquote => &STACKER_COMMAND_BLOCKQUOTE,
        StackerCommandId::InlineCode => &STACKER_COMMAND_INLINE_CODE,
        StackerCommandId::CodeBlock => &STACKER_COMMAND_CODE_BLOCK,
        StackerCommandId::ChecklistItem => &STACKER_COMMAND_CHECKLIST_ITEM,
        StackerCommandId::Clear => &STACKER_COMMAND_CLEAR,
        StackerCommandId::Undo => &STACKER_COMMAND_UNDO,
        StackerCommandId::Redo => &STACKER_COMMAND_REDO,
    }
}

pub fn stacker_editor_command(id: StackerCommandId) -> StackerEditorCommand {
    match id {
        StackerCommandId::Bold => StackerEditorCommand::Bold,
        StackerCommandId::Italic => StackerEditorCommand::Italic,
        StackerCommandId::UnorderedList => StackerEditorCommand::UnorderedList,
        StackerCommandId::OrderedList => StackerEditorCommand::OrderedList,
        StackerCommandId::Heading1 => StackerEditorCommand::Heading(1),
        StackerCommandId::Heading2 => StackerEditorCommand::Heading(2),
        StackerCommandId::Heading3 => StackerEditorCommand::Heading(3),
        StackerCommandId::Blockquote => StackerEditorCommand::Blockquote,
        StackerCommandId::InlineCode => StackerEditorCommand::InlineCode,
        StackerCommandId::CodeBlock => StackerEditorCommand::CodeBlock,
        StackerCommandId::ChecklistItem => StackerEditorCommand::ChecklistItem,
        StackerCommandId::Clear => StackerEditorCommand::Clear,
        StackerCommandId::Undo => StackerEditorCommand::Undo,
        StackerCommandId::Redo => StackerEditorCommand::Redo,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StackerCommandOutcome {
    pub changed: bool,
    pub selection: StackerSelection,
}

pub fn execute_stacker_command(
    editor: &mut StackerSession,
    command: StackerEditorCommand,
) -> StackerCommandOutcome {
    execute_stacker_command_at(editor, editor.selection(), command)
}

pub fn execute_stacker_command_at(
    editor: &mut StackerSession,
    selection: StackerSelection,
    command: StackerEditorCommand,
) -> StackerCommandOutcome {
    match command {
        StackerEditorCommand::Bold => wrap_selection(editor, selection, "**", "**"),
        StackerEditorCommand::Italic => wrap_selection(editor, selection, "*", "*"),
        StackerEditorCommand::InlineCode => wrap_selection(editor, selection, "`", "`"),
        StackerEditorCommand::CodeBlock => wrap_selection(editor, selection, "```\n", "\n```"),
        StackerEditorCommand::UnorderedList => {
            apply_list(editor, selection, ListButtonKind::Unordered)
        }
        StackerEditorCommand::OrderedList => apply_list(editor, selection, ListButtonKind::Ordered),
        StackerEditorCommand::Heading(level) => {
            let level = level.clamp(1, 6) as usize;
            apply_line_prefix(editor, selection, &format!("{} ", "#".repeat(level)))
        }
        StackerEditorCommand::Blockquote => apply_line_prefix(editor, selection, "> "),
        StackerEditorCommand::ChecklistItem => apply_line_prefix(editor, selection, "- [ ] "),
        StackerEditorCommand::Clear => {
            let changed =
                editor.replace_all_with_history(String::new(), StackerSelection::collapsed(0));
            StackerCommandOutcome {
                changed,
                selection: editor.selection(),
            }
        }
        StackerEditorCommand::LoadText(text) => {
            let changed = editor.text() != text;
            editor.set_text(text);
            StackerCommandOutcome {
                changed,
                selection: editor.selection(),
            }
        }
        StackerEditorCommand::Undo => {
            let changed = editor.undo();
            StackerCommandOutcome {
                changed,
                selection: editor.selection(),
            }
        }
        StackerEditorCommand::Redo => {
            let changed = editor.redo();
            StackerCommandOutcome {
                changed,
                selection: editor.selection(),
            }
        }
    }
}

fn wrap_selection(
    editor: &mut StackerSession,
    selection: StackerSelection,
    prefix: &str,
    suffix: &str,
) -> StackerCommandOutcome {
    let selection = clamp_selection(editor.text(), selection).sorted();
    let prefix_len = prefix.chars().count();

    let next_selection = if selection.is_collapsed() {
        StackerSelection::collapsed(selection.start + prefix_len)
    } else {
        StackerSelection {
            start: selection.start + prefix_len,
            end: selection.end + prefix_len,
        }
    };

    let replacement = if selection.is_collapsed() {
        format!("{prefix}{suffix}")
    } else {
        let selected = slice_chars(editor.text(), selection.start, selection.end);
        format!("{prefix}{selected}{suffix}")
    };

    editor.replace_range(selection, &replacement, next_selection);
    StackerCommandOutcome {
        changed: true,
        selection: editor.selection(),
    }
}

fn apply_list(
    editor: &mut StackerSession,
    selection: StackerSelection,
    kind: ListButtonKind,
) -> StackerCommandOutcome {
    let selection = clamp_selection(editor.text(), selection).sorted();
    let (new_text, new_cursor) =
        apply_list_prefix(editor.text(), selection.start, selection.end, kind);
    let next_selection = StackerSelection::collapsed(new_cursor);
    let changed = editor.replace_all_with_history(new_text, next_selection);
    StackerCommandOutcome {
        changed,
        selection: editor.selection(),
    }
}

fn apply_line_prefix(
    editor: &mut StackerSession,
    selection: StackerSelection,
    prefix: &str,
) -> StackerCommandOutcome {
    let selection = clamp_selection(editor.text(), selection).sorted();
    let (new_text, next_selection) = prefix_selected_lines(editor.text(), selection, prefix);
    let changed = editor.replace_all_with_history(new_text, next_selection);
    StackerCommandOutcome {
        changed,
        selection: editor.selection(),
    }
}

fn prefix_selected_lines(
    input: &str,
    selection: StackerSelection,
    prefix: &str,
) -> (String, StackerSelection) {
    if input.is_empty() {
        let cursor = prefix.chars().count();
        return (prefix.to_string(), StackerSelection::collapsed(cursor));
    }

    let prefix_len = prefix.chars().count();
    let line_start = line_start_char(input, selection.start);
    let mut line_end = if selection.end > selection.start {
        selection.end
    } else {
        selection.start
    };
    if line_end > 0 && char_at(input, line_end.saturating_sub(1)) == Some('\n') {
        line_end = line_end.saturating_sub(1);
    }
    line_end = line_end_char(input, line_end);

    let mut output = String::new();
    let mut start_shift = 0usize;
    let mut end_shift = 0usize;
    let mut char_idx = 0usize;

    for segment in input.split_inclusive('\n') {
        let has_newline = segment.ends_with('\n');
        let line = segment.strip_suffix('\n').unwrap_or(segment);
        let segment_chars = segment.chars().count();
        let line_chars = line.chars().count();
        let overlaps = char_idx + line_chars >= line_start && char_idx <= line_end;
        let should_prefix = overlaps && (!line.trim().is_empty() || selection.is_collapsed());

        if should_prefix {
            output.push_str(prefix);
            if char_idx <= selection.start {
                start_shift += prefix_len;
            }
            if char_idx <= selection.end {
                end_shift += prefix_len;
            }
        }
        output.push_str(line);
        if has_newline {
            output.push('\n');
        }
        char_idx += segment_chars;
    }

    (
        output,
        StackerSelection {
            start: selection.start + start_shift,
            end: selection.end + end_shift,
        },
    )
}

fn clamp_selection(input: &str, selection: StackerSelection) -> StackerSelection {
    let char_count = input.chars().count();
    StackerSelection {
        start: selection.start.min(char_count),
        end: selection.end.min(char_count),
    }
}

fn slice_chars(input: &str, start: usize, end: usize) -> String {
    let start = char_to_byte_idx(input, start);
    let end = char_to_byte_idx(input, end);
    input[start..end].to_string()
}

fn line_start_char(text: &str, char_idx: usize) -> usize {
    let byte_idx = char_to_byte_idx(text, char_idx);
    text[..byte_idx]
        .rfind('\n')
        .map(|idx| text[..idx + 1].chars().count())
        .unwrap_or(0)
}

fn line_end_char(text: &str, char_idx: usize) -> usize {
    let byte_idx = char_to_byte_idx(text, char_idx);
    text[byte_idx..]
        .find('\n')
        .map(|idx| text[..byte_idx + idx].chars().count())
        .unwrap_or_else(|| text.chars().count())
}

fn char_at(text: &str, char_idx: usize) -> Option<char> {
    text.chars().nth(char_idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor_with(text: &str, selection: StackerSelection) -> StackerSession {
        let mut editor = StackerSession::new();
        editor.set_text(text);
        editor.set_selection(selection);
        editor
    }

    #[test]
    fn bold_wraps_selected_text_and_preserves_inner_selection() {
        let mut editor = editor_with("hello world", StackerSelection { start: 6, end: 11 });

        let outcome = execute_stacker_command(&mut editor, StackerEditorCommand::Bold);

        assert!(outcome.changed);
        assert_eq!(editor.text(), "hello **world**");
        assert_eq!(outcome.selection, StackerSelection { start: 8, end: 13 });
    }

    #[test]
    fn bold_inserts_marker_pair_at_collapsed_selection() {
        let mut editor = editor_with("hello ", StackerSelection::collapsed(6));

        let outcome = execute_stacker_command(&mut editor, StackerEditorCommand::Bold);

        assert_eq!(editor.text(), "hello ****");
        assert_eq!(outcome.selection, StackerSelection::collapsed(8));
    }

    #[test]
    fn unordered_list_reuses_list_prefix_behavior() {
        let mut editor = editor_with("alpha\nbeta", StackerSelection { start: 0, end: 10 });

        let outcome = execute_stacker_command(&mut editor, StackerEditorCommand::UnorderedList);

        assert!(outcome.changed);
        assert_eq!(editor.text(), "- alpha\n- beta");
        assert_eq!(outcome.selection, StackerSelection::collapsed(2));
    }

    #[test]
    fn inline_code_wraps_selection() {
        let mut editor = editor_with("use cargo test", StackerSelection { start: 4, end: 14 });

        let outcome = execute_stacker_command(&mut editor, StackerEditorCommand::InlineCode);

        assert_eq!(editor.text(), "use `cargo test`");
        assert_eq!(outcome.selection, StackerSelection { start: 5, end: 15 });
    }

    #[test]
    fn code_block_wraps_selection_on_separate_lines() {
        let mut editor = editor_with("println!();", StackerSelection { start: 0, end: 11 });

        let outcome = execute_stacker_command(&mut editor, StackerEditorCommand::CodeBlock);

        assert_eq!(editor.text(), "```\nprintln!();\n```");
        assert_eq!(outcome.selection, StackerSelection { start: 4, end: 15 });
    }

    #[test]
    fn checklist_prefixes_selected_lines() {
        let mut editor = editor_with("one\ntwo", StackerSelection { start: 0, end: 7 });

        let outcome = execute_stacker_command(&mut editor, StackerEditorCommand::ChecklistItem);

        assert_eq!(editor.text(), "- [ ] one\n- [ ] two");
        assert_eq!(outcome.selection, StackerSelection { start: 6, end: 19 });
    }

    #[test]
    fn blockquote_prefixes_current_line() {
        let mut editor = editor_with("one\ntwo", StackerSelection::collapsed(5));

        let outcome = execute_stacker_command(&mut editor, StackerEditorCommand::Blockquote);

        assert_eq!(editor.text(), "one\n> two");
        assert_eq!(outcome.selection, StackerSelection::collapsed(7));
    }

    #[test]
    fn clear_is_undoable() {
        let mut editor = editor_with("draft", StackerSelection::collapsed(5));

        let outcome = execute_stacker_command(&mut editor, StackerEditorCommand::Clear);

        assert!(outcome.changed);
        assert_eq!(editor.text(), "");

        let undo = execute_stacker_command(&mut editor, StackerEditorCommand::Undo);
        assert!(undo.changed);
        assert_eq!(editor.text(), "draft");
    }

    #[test]
    fn load_text_resets_document_without_history() {
        let mut editor = editor_with("draft", StackerSelection::collapsed(5));

        let outcome = execute_stacker_command(
            &mut editor,
            StackerEditorCommand::LoadText("saved".to_string()),
        );

        assert!(outcome.changed);
        assert_eq!(editor.text(), "saved");
        assert_eq!(outcome.selection, StackerSelection::collapsed(5));
        assert!(!execute_stacker_command(&mut editor, StackerEditorCommand::Undo).changed);
    }

    #[test]
    fn registry_maps_ids_to_editor_commands() {
        let ids = [
            StackerCommandId::Bold,
            StackerCommandId::Italic,
            StackerCommandId::UnorderedList,
            StackerCommandId::OrderedList,
            StackerCommandId::Heading1,
            StackerCommandId::Heading2,
            StackerCommandId::Heading3,
            StackerCommandId::Blockquote,
            StackerCommandId::InlineCode,
            StackerCommandId::CodeBlock,
            StackerCommandId::ChecklistItem,
            StackerCommandId::Clear,
            StackerCommandId::Undo,
            StackerCommandId::Redo,
        ];
        let registry = stacker_command_registry();

        assert_eq!(registry.len(), ids.len());
        for id in ids {
            assert!(registry.contains(stacker_command_descriptor(id)));
        }
        assert_eq!(
            stacker_editor_command(StackerCommandId::Heading1),
            StackerEditorCommand::Heading(1)
        );
        assert_eq!(
            stacker_editor_command(StackerCommandId::Heading2),
            StackerEditorCommand::Heading(2)
        );
        assert_eq!(
            stacker_editor_command(StackerCommandId::Heading3),
            StackerEditorCommand::Heading(3)
        );
    }
}
