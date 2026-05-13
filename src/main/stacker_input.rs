use super::*;

impl App {
    pub(crate) fn active_stacker_tab(&self) -> bool {
        self.active_tab()
            .is_some_and(|tab| matches!(tab.content, TabContent::Stacker))
    }

    pub(crate) fn active_sketch_tab(&self) -> bool {
        self.active_tab()
            .is_some_and(|tab| matches!(tab.content, TabContent::Sketch))
    }

    pub(crate) fn clear_egui_keyboard_focus(&self) {
        if let Some(ui) = &self.ui {
            ui.ctx.memory_mut(|memory| memory.stop_text_input());
        }
    }

    pub(crate) fn stacker_visible_in_active_context(&self) -> bool {
        self.stacker_tab_in_active_context().is_some()
    }

    pub(crate) fn stacker_tab_in_active_context(&self) -> Option<usize> {
        if self.active_stacker_tab() {
            return Some(self.active_tab);
        }
        let Some(ui) = &self.ui else {
            return None;
        };
        let joined = active_joined_tabs(&self.tabs, self.active_tab, &ui.tab_groups)?;

        [joined.primary, joined.secondary].into_iter().find(|&idx| {
            self.tabs
                .get(idx)
                .is_some_and(|tab| matches!(tab.content, TabContent::Stacker))
        })
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn create_stacker_input_client(&mut self) {
        let Some(window) = &self.window else { return };
        if self.stacker_input_client.is_some() {
            return;
        }
        match StackerInputClient::new(window.as_ref(), self.proxy.clone()) {
            Ok(client) => {
                self.stacker_input_client = Some(client);
            }
            Err(err) => self.error_log.error(err),
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub(crate) fn create_stacker_input_client(&mut self) {}

    #[cfg(target_os = "macos")]
    pub(crate) fn sync_stacker_input_client(&mut self) {
        let active = self.stacker_visible_in_active_context();
        let focus_when_shown = self.active_stacker_tab();
        let Some(client) = &mut self.stacker_input_client else {
            return;
        };
        if !active {
            client.set_visible(false);
            self.stacker_pending_focus = false;
            return;
        }

        let Some(ui) = &self.ui else {
            client.set_visible(false);
            return;
        };
        let modal_open = ui.stacker.pending_draft_switch.is_some()
            || ui.stacker.pending_prompt_delete.is_some()
            || ui.pending_close.is_some();
        let Some(rect) = ui.stacker.prompt_editor_rect else {
            client.set_visible(false);
            return;
        };
        if modal_open {
            client.set_visible(false);
            return;
        }

        client.set_bounds(rect);
        client.set_state(
            ui.stacker.editor.text(),
            ui.stacker.editor.char_count(),
            ui.stacker.editor.selection(),
            ui.stacker.editor.marked_range(),
        );
        client.set_galley(ui.stacker.prompt_editor_anchor.clone());
        let became_visible = client.set_visible(true);
        let should_focus = focus_when_shown && (self.stacker_pending_focus || became_visible);
        if should_focus {
            client.focus();
            self.stacker_pending_focus = false;
        } else if focus_when_shown {
            client.ensure_focused();
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub(crate) fn sync_stacker_input_client(&mut self) {}

    #[cfg(target_os = "macos")]
    pub(crate) fn apply_stacker_input_client_insert_text(
        &mut self,
        text: String,
        replacement_utf16: Option<(usize, usize)>,
    ) {
        llnzy::external_input_trace::trace("stacker.insert_text_entry", || {
            format!("text={:?}, replacement={:?}", text, replacement_utf16)
        });
        if !text.is_empty()
            && text
                .chars()
                .all(|c| c.is_control() && c != '\n' && c != '\t')
        {
            llnzy::external_input_trace::trace("stacker.insert_text_dropped_control", || {
                format!("text={text:?}")
            });
            return;
        }
        if self.stacker_tab_in_active_context().is_none() {
            return;
        }
        let Some(ui) = &mut self.ui else { return };

        // Resolution order matches AppKit: existing marked range → explicit
        // replacement_range → current selection.
        let target = if let Some(marked) = ui.stacker.editor.marked_range() {
            marked
        } else if let Some(pair) = replacement_utf16 {
            // Only materialise the pre-edit text when we actually need it for
            // the UTF-16 → char conversion. For plain typing this branch is
            // never taken, saving an O(n) heap allocation per keystroke.
            let session_text = ui.stacker.editor.text();
            llnzy::stacker_input_client::utf16_pair_to_selection(session_text, pair)
        } else {
            ui.stacker.editor.selection()
        };
        ui.stacker.editor.unmark_text();
        let cursor_after = target.sorted().start + text.chars().count();
        ui.stacker
            .editor
            .replace_range(target, &text, StackerSelection::collapsed(cursor_after));
        store_stacker_selection(ui, ui.stacker.editor.selection());
        ui.stacker
            .draft
            .record_current_text(ui.stacker.editor.text());
        llnzy::external_input_trace::trace("stacker.input_client_insert", || {
            format!(
                "chars={}, cursor={cursor_after}",
                ui.stacker.editor.text().chars().count()
            )
        });
        self.request_redraw();
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn apply_stacker_input_client_set_marked_text(
        &mut self,
        text: String,
        marked_internal_utf16: (usize, usize),
        replacement_utf16: Option<(usize, usize)>,
    ) {
        llnzy::external_input_trace::trace("stacker.set_marked_text_entry", || {
            format!(
                "text={:?}, internal={:?}, replacement={:?}",
                text, marked_internal_utf16, replacement_utf16
            )
        });
        if self.stacker_tab_in_active_context().is_none() {
            return;
        }
        let Some(ui) = &mut self.ui else { return };

        let replacement = replacement_utf16.map(|pair| {
            // Borrow only when needed; avoids an O(n) alloc on the common path.
            llnzy::stacker_input_client::utf16_pair_to_selection(ui.stacker.editor.text(), pair)
        });

        // The internal selection's UTF-16 indices are relative to the new
        // marked text, so resolve against `text` itself.
        let internal =
            llnzy::stacker_input_client::utf16_pair_to_selection(&text, marked_internal_utf16);

        ui.stacker
            .editor
            .set_marked_text(&text, internal, replacement);
        store_stacker_selection(ui, ui.stacker.editor.selection());
        ui.stacker
            .draft
            .record_current_text(ui.stacker.editor.text());
        llnzy::external_input_trace::trace("stacker.input_client_set_marked", || {
            format!(
                "marked_chars={}, internal={}..{}",
                text.chars().count(),
                internal.start,
                internal.end
            )
        });
        self.request_redraw();
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn apply_stacker_input_client_unmark_text(&mut self) {
        let Some(ui) = &mut self.ui else { return };
        ui.stacker.editor.unmark_text();
        self.request_redraw();
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn apply_stacker_input_client_do_command(&mut self, selector_name: &str) {
        llnzy::external_input_trace::trace("stacker.do_command_entry", || {
            selector_name.to_string()
        });
        if self.stacker_tab_in_active_context().is_none() {
            return;
        }
        let Some(ui) = &mut self.ui else { return };
        let selection = ui.stacker.editor.selection();

        let handled = match selector_name {
            "insertNewline:" | "insertLineBreak:" | "insertParagraphSeparator:" => {
                ui.stacker
                    .editor
                    .replace_range(selection, "\n", StackerSelection::collapsed(0));
                true
            }
            "deleteBackward:" => {
                ui.stacker.editor.delete_backward(selection);
                true
            }
            "deleteForward:" => {
                ui.stacker.editor.delete_forward(selection);
                true
            }
            "selectAll:" => {
                ui.stacker.editor.select_all();
                true
            }
            "moveLeft:" => {
                let next = selection.sorted().start.saturating_sub(1);
                ui.stacker
                    .editor
                    .set_selection(StackerSelection::collapsed(next));
                true
            }
            "moveRight:" => {
                let next = selection
                    .sorted()
                    .end
                    .saturating_add(1)
                    .min(ui.stacker.editor.char_count());
                ui.stacker
                    .editor
                    .set_selection(StackerSelection::collapsed(next));
                true
            }
            other => {
                llnzy::external_input_trace::trace(
                    "stacker.input_client_unhandled_selector",
                    || other.to_string(),
                );
                false
            }
        };

        if handled {
            store_stacker_selection(ui, ui.stacker.editor.selection());
            ui.stacker
                .draft
                .record_current_text(ui.stacker.editor.text());
            self.request_redraw();
        }
    }
}

#[cfg(target_os = "macos")]
fn store_stacker_selection(ui: &mut UiState, selection: StackerSelection) {
    let editor_id = egui::Id::new(STACKER_PROMPT_EDITOR_ID);
    let ctx = ui.ctx.clone();
    stacker_cursor::store_document_selection(&ctx, editor_id, &mut ui.stacker.editor, selection);
}
