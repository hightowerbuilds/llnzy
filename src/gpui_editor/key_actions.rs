use super::*;

pub(crate) fn bind_editor_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, None),
        KeyBinding::new("delete", Delete, None),
        KeyBinding::new("enter", Enter, None),
        KeyBinding::new("tab", Tab, None),
        KeyBinding::new("shift-tab", ShiftTab, None),
        KeyBinding::new("left", Left, None),
        KeyBinding::new("right", Right, None),
        KeyBinding::new("up", Up, None),
        KeyBinding::new("down", Down, None),
        KeyBinding::new("shift-left", SelectLeft, None),
        KeyBinding::new("shift-right", SelectRight, None),
        KeyBinding::new("shift-up", SelectUp, None),
        KeyBinding::new("shift-down", SelectDown, None),
        KeyBinding::new("alt-left", WordLeft, None),
        KeyBinding::new("alt-right", WordRight, None),
        KeyBinding::new("shift-alt-left", SelectWordLeft, None),
        KeyBinding::new("shift-alt-right", SelectWordRight, None),
        KeyBinding::new("home", Home, None),
        KeyBinding::new("end", End, None),
        KeyBinding::new("shift-home", SelectHome, None),
        KeyBinding::new("shift-end", SelectEnd, None),
        KeyBinding::new("cmd-left", LineStart, None),
        KeyBinding::new("cmd-right", LineEnd, None),
        KeyBinding::new("shift-cmd-left", SelectLineStart, None),
        KeyBinding::new("shift-cmd-right", SelectLineEnd, None),
        KeyBinding::new("ctrl-a", LineStart, None),
        KeyBinding::new("ctrl-e", LineEnd, None),
        KeyBinding::new("cmd-up", DocumentStart, None),
        KeyBinding::new("cmd-down", DocumentEnd, None),
        KeyBinding::new("shift-cmd-up", SelectDocumentStart, None),
        KeyBinding::new("shift-cmd-down", SelectDocumentEnd, None),
        KeyBinding::new("pageup", PageUp, None),
        KeyBinding::new("pagedown", PageDown, None),
        KeyBinding::new("shift-pageup", SelectPageUp, None),
        KeyBinding::new("shift-pagedown", SelectPageDown, None),
        KeyBinding::new("alt-backspace", DeleteWordBackward, None),
        KeyBinding::new("alt-delete", DeleteWordForward, None),
        KeyBinding::new("cmd-backspace", DeleteToLineStart, None),
        KeyBinding::new("cmd-delete", DeleteToLineEnd, None),
        KeyBinding::new("ctrl-k", DeleteToLineEnd, None),
        KeyBinding::new("cmd-a", SelectAll, None),
        KeyBinding::new("cmd-d", SelectWord, None),
        KeyBinding::new("cmd-l", SelectLine, None),
        KeyBinding::new("cmd-shift-k", DeleteLine, None),
        KeyBinding::new("cmd-shift-d", DuplicateLineOrSelection, None),
        KeyBinding::new("alt-up", MoveLineUp, None),
        KeyBinding::new("alt-down", MoveLineDown, None),
        KeyBinding::new("cmd-/", ToggleLineComment, None),
        KeyBinding::new("cmd-v", Paste, None),
        KeyBinding::new("cmd-c", Copy, None),
        KeyBinding::new("cmd-x", Cut, None),
        KeyBinding::new("cmd-s", Save, None),
        KeyBinding::new("cmd-z", Undo, None),
        KeyBinding::new("cmd-shift-z", Redo, None),
        KeyBinding::new("cmd-f", Find, None),
        KeyBinding::new("cmd-g", FindNext, None),
        KeyBinding::new("shift-cmd-g", FindPrevious, None),
        KeyBinding::new("ctrl-g", GoToLine, None),
        KeyBinding::new("escape", CloseFind, None),
    ]);
}

impl EditorPrototype {
    pub(super) fn scroll_by_lines(&mut self, delta: isize, cx: &mut Context<Self>) {
        if self.scroll_active_by_lines_without_notify(delta) {
            cx.notify();
        }
    }

    pub(super) fn move_left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::Left,
                extend: false,
            },
            cx,
        );
    }

    pub(super) fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::Left,
                extend: true,
            },
            cx,
        );
    }

    pub(super) fn move_right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::Right,
                extend: false,
            },
            cx,
        );
    }

    pub(super) fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::Right,
                extend: true,
            },
            cx,
        );
    }

    pub(super) fn move_up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        if self.move_lsp_panel_selection(-1, cx) {
            return;
        }
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::Up,
                extend: false,
            },
            cx,
        );
    }

    pub(super) fn select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::Up,
                extend: true,
            },
            cx,
        );
    }

    pub(super) fn move_down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        if self.move_lsp_panel_selection(1, cx) {
            return;
        }
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::Down,
                extend: false,
            },
            cx,
        );
    }

    pub(super) fn select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::Down,
                extend: true,
            },
            cx,
        );
    }

    pub(super) fn move_word_left(&mut self, _: &WordLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::WordLeft,
                extend: false,
            },
            cx,
        );
    }

    pub(super) fn select_word_left(
        &mut self,
        _: &SelectWordLeft,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::WordLeft,
                extend: true,
            },
            cx,
        );
    }

    pub(super) fn move_word_right(
        &mut self,
        _: &WordRight,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::WordRight,
                extend: false,
            },
            cx,
        );
    }

    pub(super) fn select_word_right(
        &mut self,
        _: &SelectWordRight,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::WordRight,
                extend: true,
            },
            cx,
        );
    }

    pub(super) fn move_home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::SmartLineStart,
                extend: false,
            },
            cx,
        );
    }

    pub(super) fn select_home(&mut self, _: &SelectHome, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::SmartLineStart,
                extend: true,
            },
            cx,
        );
    }

    pub(super) fn move_end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::LineEnd,
                extend: false,
            },
            cx,
        );
    }

    pub(super) fn select_end(&mut self, _: &SelectEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::LineEnd,
                extend: true,
            },
            cx,
        );
    }

    pub(super) fn move_line_start(
        &mut self,
        _: &LineStart,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::LineStart,
                extend: false,
            },
            cx,
        );
    }

    pub(super) fn select_line_start(
        &mut self,
        _: &SelectLineStart,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::LineStart,
                extend: true,
            },
            cx,
        );
    }

    pub(super) fn move_line_end(&mut self, _: &LineEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::LineEnd,
                extend: false,
            },
            cx,
        );
    }

    pub(super) fn select_line_end(
        &mut self,
        _: &SelectLineEnd,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::LineEnd,
                extend: true,
            },
            cx,
        );
    }

    pub(super) fn move_document_start(
        &mut self,
        _: &DocumentStart,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::DocumentStart,
                extend: false,
            },
            cx,
        );
    }

    pub(super) fn select_document_start(
        &mut self,
        _: &SelectDocumentStart,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::DocumentStart,
                extend: true,
            },
            cx,
        );
    }

    pub(super) fn move_document_end(
        &mut self,
        _: &DocumentEnd,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::DocumentEnd,
                extend: false,
            },
            cx,
        );
    }

    pub(super) fn select_document_end(
        &mut self,
        _: &SelectDocumentEnd,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::DocumentEnd,
                extend: true,
            },
            cx,
        );
    }

    pub(super) fn page_up(&mut self, _: &PageUp, _: &mut Window, cx: &mut Context<Self>) {
        self.page_up_impl(false, cx);
    }

    pub(super) fn page_down(&mut self, _: &PageDown, _: &mut Window, cx: &mut Context<Self>) {
        self.page_down_impl(false, cx);
    }

    pub(super) fn select_page_up(
        &mut self,
        _: &SelectPageUp,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.page_up_impl(true, cx);
    }

    pub(super) fn select_page_down(
        &mut self,
        _: &SelectPageDown,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.page_down_impl(true, cx);
    }

    fn page_up_impl(&mut self, extend: bool, cx: &mut Context<Self>) {
        if self.editor.is_empty() {
            self.scroll_by_lines(-(self.visible_line_limit() as isize), cx);
            return;
        }
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::PageUp,
                extend,
            },
            cx,
        );
    }

    fn page_down_impl(&mut self, extend: bool, cx: &mut Context<Self>) {
        self.dispatch_editor_command(
            EditorCommand::Move {
                motion: EditorMotion::PageDown,
                extend,
            },
            cx,
        );
        if self.editor.is_empty() {
            self.scroll_by_lines(self.visible_line_limit() as isize, cx);
        }
    }

    pub(super) fn backspace(&mut self, _: &Backspace, _: &mut Window, cx: &mut Context<Self>) {
        if self.rename_active {
            self.pop_lsp_rename_text(cx);
            return;
        }
        if self.editor_search.active {
            self.pop_search_text(cx);
            return;
        }
        self.dispatch_editor_command(EditorCommand::Delete(EditorDeleteTarget::BackwardChar), cx);
    }

    pub(super) fn delete(&mut self, _: &Delete, _: &mut Window, cx: &mut Context<Self>) {
        if self.rename_active {
            self.pop_lsp_rename_text(cx);
            return;
        }
        if self.editor_search.active {
            return;
        }
        self.dispatch_editor_command(EditorCommand::Delete(EditorDeleteTarget::ForwardChar), cx);
    }

    pub(super) fn delete_word_backward(
        &mut self,
        _: &DeleteWordBackward,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dispatch_editor_command(EditorCommand::Delete(EditorDeleteTarget::BackwardWord), cx);
    }

    pub(super) fn delete_word_forward(
        &mut self,
        _: &DeleteWordForward,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dispatch_editor_command(EditorCommand::Delete(EditorDeleteTarget::ForwardWord), cx);
    }

    pub(super) fn delete_to_line_start(
        &mut self,
        _: &DeleteToLineStart,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dispatch_editor_command(EditorCommand::Delete(EditorDeleteTarget::ToLineStart), cx);
    }

    pub(super) fn delete_to_line_end(
        &mut self,
        _: &DeleteToLineEnd,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dispatch_editor_command(EditorCommand::Delete(EditorDeleteTarget::ToLineEnd), cx);
    }

    pub(super) fn enter(&mut self, _: &Enter, _: &mut Window, cx: &mut Context<Self>) {
        if self.accept_lsp_panel_selection(cx) {
            return;
        }
        if self.rename_active {
            self.submit_lsp_rename(cx);
            return;
        }
        if self.go_to_line_active {
            self.submit_go_to_line(cx);
            return;
        }
        if self.editor_search.active {
            self.move_search_focus(EditorSearchDirection::Next, cx);
            return;
        }
        self.dispatch_editor_command(EditorCommand::Enter, cx);
    }

    pub(super) fn tab(&mut self, _: &Tab, _: &mut Window, cx: &mut Context<Self>) {
        if self.accept_lsp_panel_selection(cx) {
            return;
        }
        if self.rename_active {
            return;
        }
        if self.go_to_line_active {
            return;
        }
        if self.editor_search.active {
            self.toggle_search_input_target(cx);
            return;
        }
        self.dispatch_editor_command(EditorCommand::Indent { outdent: false }, cx);
    }

    pub(super) fn shift_tab(&mut self, _: &ShiftTab, _: &mut Window, cx: &mut Context<Self>) {
        if self.move_lsp_panel_selection(-1, cx) {
            return;
        }
        if self.rename_active {
            return;
        }
        if self.go_to_line_active {
            return;
        }
        if self.editor_search.active {
            self.toggle_search_input_target(cx);
            return;
        }
        self.dispatch_editor_command(EditorCommand::Indent { outdent: true }, cx);
    }

    pub(super) fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        if self.go_to_line_active {
            return;
        }
        if self.editor_search.active {
            return;
        }
        self.dispatch_editor_command(EditorCommand::Select(EditorSelectTarget::All), cx);
    }

    pub(super) fn select_word(&mut self, _: &SelectWord, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(EditorCommand::Select(EditorSelectTarget::Word), cx);
    }

    pub(super) fn select_line(&mut self, _: &SelectLine, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(EditorCommand::Select(EditorSelectTarget::Line), cx);
    }

    pub(super) fn delete_line(&mut self, _: &DeleteLine, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(EditorCommand::DeleteLine, cx);
    }

    pub(super) fn duplicate_line_or_selection_action(
        &mut self,
        _: &DuplicateLineOrSelection,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dispatch_editor_command(EditorCommand::DuplicateLineOrSelection, cx);
    }

    pub(super) fn move_line_up(&mut self, _: &MoveLineUp, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(EditorCommand::MoveLine(EditorLineMove::Up), cx);
    }

    pub(super) fn move_line_down(
        &mut self,
        _: &MoveLineDown,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dispatch_editor_command(EditorCommand::MoveLine(EditorLineMove::Down), cx);
    }

    pub(super) fn toggle_line_comment_action(
        &mut self,
        _: &ToggleLineComment,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dispatch_editor_command(EditorCommand::ToggleLineComment, cx);
    }

    pub(super) fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(EditorCommand::Copy, cx);
    }

    pub(super) fn cut(&mut self, _: &Cut, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(EditorCommand::Cut, cx);
    }

    pub(super) fn paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
        if self.rename_active {
            if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                self.push_lsp_rename_text(&text.replace(['\n', '\r'], ""), cx);
            }
            return;
        }
        if self.editor_search.active {
            if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                self.push_search_text(&text.replace('\n', " "), cx);
            }
            return;
        }
        self.dispatch_editor_command(EditorCommand::Paste, cx);
    }

    pub(super) fn save(&mut self, _: &Save, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(EditorCommand::Save, cx);
    }

    pub(super) fn undo(&mut self, _: &Undo, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(EditorCommand::Undo, cx);
    }

    pub(super) fn redo(&mut self, _: &Redo, _: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_editor_command(EditorCommand::Redo, cx);
    }

    pub(super) fn find(&mut self, _: &Find, window: &mut Window, cx: &mut Context<Self>) {
        self.open_find(window, cx);
    }

    pub(super) fn find_next(&mut self, _: &FindNext, _: &mut Window, cx: &mut Context<Self>) {
        self.move_search_focus(EditorSearchDirection::Next, cx);
    }

    pub(super) fn find_previous(
        &mut self,
        _: &FindPrevious,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_search_focus(EditorSearchDirection::Previous, cx);
    }

    pub(super) fn close_find_action(
        &mut self,
        _: &CloseFind,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_go_to_line(cx);
        self.close_lsp_rename(cx);
        self.close_lsp_panel(cx);
        self.close_find(cx);
    }

    pub(super) fn go_to_line(&mut self, _: &GoToLine, window: &mut Window, cx: &mut Context<Self>) {
        self.open_go_to_line(window, cx);
    }
}
