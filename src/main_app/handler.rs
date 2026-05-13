use crate::*;

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.handle_resumed(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let native_hover_target = match &event {
            WindowEvent::HoveredFile(path) => {
                Some((path.clone(), self.native_file_drop_target(path)))
            }
            _ => None,
        };
        let native_cursor_target = match &event {
            WindowEvent::CursorMoved { position, .. } => self
                .ui
                .as_ref()
                .and_then(|ui| ui.drag_drop.hovered_native_files.first())
                .and_then(|path| self.native_file_drop_target_at(path, *position)),
            _ => None,
        };
        let terminal_ime_commit =
            matches!(&event, WindowEvent::Ime(Ime::Commit(_))) && self.active_session().is_some();
        let stacker_ime_commit =
            matches!(&event, WindowEvent::Ime(Ime::Commit(_))) && self.active_stacker_tab();
        let stacker_keyboard_text_commit = self.active_stacker_tab()
            && stacker_keyboard_text_fallback_candidate(&event, self.modifiers);

        if let WindowEvent::KeyboardInput {
            event: key_event, ..
        } = &event
        {
            if key_event.state == ElementState::Pressed {
                if let Some(command) = app_zoom_shortcut_command(
                    &key_event.logical_key,
                    key_event.physical_key,
                    self.modifiers,
                ) {
                    let mut sidebar_changed = false;
                    self.handle_app_command(command, &mut sidebar_changed);
                    return;
                }

                if let Some(action) = self.config.keybindings.match_key(key_event, self.modifiers) {
                    if let Some(command) =
                        app_command_for_keybinding(&action, self.active_tab, self.tabs.len())
                    {
                        let mut sidebar_changed = false;
                        if self.handle_app_command(command, &mut sidebar_changed) && sidebar_changed
                        {
                            self.recompute_layout();
                            self.resize_terminal_tabs();
                        }
                        return;
                    }
                }
            }

            if self.active_session().is_some() && key_event.state == ElementState::Pressed {
                if let Some(action) = self.config.keybindings.match_key(key_event, self.modifiers) {
                    match action {
                        Action::Copy => {
                            self.copy_selection();
                            return;
                        }
                        Action::Paste => {
                            self.do_paste();
                            return;
                        }
                        Action::SelectAll => {
                            self.do_select_all();
                            return;
                        }
                        _ => {}
                    }
                }
            }

            if self.active_stacker_tab() && key_event.state == ElementState::Pressed {
                if let Some(command) =
                    stacker_editor_shortcut(&key_event.logical_key, self.modifiers)
                {
                    self.apply_stacker_editor_command(command);
                    return;
                }

                if let Some(history_command) =
                    document_history_shortcut(&key_event.logical_key, self.modifiers)
                {
                    let handled = match history_command {
                        HistoryCommand::Undo => self.undo_stacker_editor(),
                        HistoryCommand::Redo => self.redo_stacker_editor(),
                    };
                    if handled {
                        return;
                    }
                }

                if let Some(action) = self.config.keybindings.match_key(key_event, self.modifiers) {
                    match action {
                        Action::Copy => {
                            self.copy_stacker_editor_selection();
                            return;
                        }
                        Action::Paste => {
                            self.do_paste();
                            return;
                        }
                        Action::SelectAll => {
                            self.select_all_stacker_editor();
                            return;
                        }
                        _ => {}
                    }
                }
            }

            if self.active_sketch_tab() && key_event.state == ElementState::Pressed {
                if let Some(history_command) =
                    document_history_shortcut(&key_event.logical_key, self.modifiers)
                {
                    let action = match history_command {
                        HistoryCommand::Undo => ExternalAction::Undo,
                        HistoryCommand::Redo => ExternalAction::Redo,
                    };
                    if self.dispatch_active_external_action(action).was_handled() {
                        return;
                    }
                }
            }

            if self.route_code_editor_keybinding(key_event) {
                return;
            }
        }

        if let WindowEvent::MouseInput {
            state: ElementState::Pressed,
            button: MouseButton::Left,
            ..
        } = &event
        {
            if let Some(tab_idx) = self.joined_tab_at_cursor() {
                if self
                    .tabs
                    .get(tab_idx)
                    .is_some_and(|tab| matches!(tab.content, TabContent::Terminal(_)))
                {
                    self.clear_egui_keyboard_focus();
                }
                if tab_idx != self.active_tab {
                    self.switch_tab(tab_idx);
                }
            }

            if let Some(tab_idx) = self.joined_terminal_tab_at_cursor() {
                self.clear_egui_keyboard_focus();
                if tab_idx != self.active_tab {
                    self.switch_tab(tab_idx);
                }
            }
        }

        if let WindowEvent::MouseWheel { delta, .. } = &event {
            if self.route_terminal_mouse_wheel(delta) {
                return;
            }
        }

        // Route events to egui first
        if let (Some(window), Some(ui)) = (&self.window, &mut self.ui) {
            let stacker_input_before_egui =
                stacker_ime_commit.then(|| ui.stacker.editor.text().to_string());
            let stacker_keyboard_text_before_egui =
                stacker_keyboard_text_commit.then(|| ui.stacker.editor.text().to_string());
            let response = ui.handle_event(window, &event);
            let terminal_should_receive_consumed_ime = terminal_ime_commit
                && !ui.captures_terminal_input()
                && !ui.ctx.wants_keyboard_input();
            let stacker_should_receive_consumed_ime = stacker_input_before_egui
                .as_ref()
                .is_some_and(|input_before| input_before == ui.stacker.editor.text());
            let stacker_prompt_editor_focused = ui.ctx.memory(|memory| {
                memory.has_focus(egui::Id::new(llnzy::ui::STACKER_PROMPT_EDITOR_ID))
            });
            let stacker_should_receive_consumed_key_text = stacker_keyboard_text_before_egui
                .as_ref()
                .is_some_and(|input_before| {
                    stacker_prompt_editor_focused && input_before == ui.stacker.editor.text()
                });
            match &event {
                WindowEvent::HoveredFile(path) => {
                    ui.drag_drop.hover_native_file(path.clone());
                    ui.drag_drop.active_target = native_hover_target.and_then(|(_, target)| target);
                    window.request_redraw();
                }
                WindowEvent::CursorMoved { .. }
                    if !ui.drag_drop.hovered_native_files.is_empty() =>
                {
                    ui.drag_drop.active_target = native_cursor_target;
                    window.request_redraw();
                }
                WindowEvent::HoveredFileCancelled => {
                    ui.drag_drop.cancel();
                    window.request_redraw();
                }
                _ => {}
            }
            // Sketch owns raw canvas input; do not let unconsumed pointer/text events leak
            // into the terminal while that workspace is active.
            if ui.captures_terminal_input() && terminal_input_event(&event) {
                self.request_redraw();
                return;
            }
            // If egui consumed a mouse/keyboard event, don't pass to terminal.
            // The footer and bumper are always visible, so any egui-consumed
            // event must be respected regardless of which view is active.
            if response
                && !terminal_should_receive_consumed_ime
                && !stacker_should_receive_consumed_ime
                && !stacker_should_receive_consumed_key_text
            {
                self.request_redraw();
                match &event {
                    WindowEvent::CloseRequested | WindowEvent::Resized(_) => {}
                    _ => return,
                }
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                // Check for unsaved CodeFile buffers before quitting.
                if self.begin_window_close() {
                    self.error_log.info("Close requested");
                    self.terminate_all_terminal_tabs_for_exit();
                    self.save_window_state();
                    event_loop.exit();
                }
            }

            WindowEvent::Focused(focused) => {
                if focused {
                    if self.active_stacker_tab() {
                        if let Some(window) = &self.window {
                            window.set_ime_allowed(true);
                        }
                    }
                    // If the window just regained focus while a Stacker pane
                    // is the active text-input target, the macOS first
                    // responder may have drifted back to the winit content
                    // view. Schedule a re-claim on the next frame's sync.
                    #[cfg(target_os = "macos")]
                    if self.stacker_visible_in_active_context() {
                        self.stacker_pending_focus = true;
                    }
                    self.request_redraw();
                }
            }

            WindowEvent::ScaleFactorChanged { .. } => {
                self.sync_window_metrics();
            }

            WindowEvent::Resized(_) => {
                self.sync_window_metrics();
            }

            WindowEvent::Moved(_) => {
                self.sync_window_metrics();
            }

            WindowEvent::RedrawRequested => {
                self.handle_redraw_requested(event_loop);
            }

            WindowEvent::ModifiersChanged(mods) => {
                self.modifiers = mods.state();
            }

            // --- Mouse wheel ---
            WindowEvent::MouseWheel { delta, .. } => {
                let _ = self.route_terminal_mouse_wheel(&delta);
            }

            // --- Mouse buttons ---
            WindowEvent::MouseInput { state, button, .. } => {
                self.handle_terminal_mouse_input(state, button);
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.handle_cursor_moved(position);
            }

            // --- Keyboard ---
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                if key_event.state != ElementState::Pressed {
                    return;
                }

                // When error panel is visible, arrow keys scroll it
                if self.error_panel.visible {
                    match &key_event.logical_key {
                        Key::Named(NamedKey::Escape) => {
                            self.error_panel.toggle();
                            self.request_redraw();
                            return;
                        }
                        Key::Named(NamedKey::ArrowUp) | Key::Named(NamedKey::PageUp) => {
                            self.error_panel.scroll_up();
                            self.request_redraw();
                            return;
                        }
                        Key::Named(NamedKey::ArrowDown) | Key::Named(NamedKey::PageDown) => {
                            self.error_panel.scroll_down();
                            self.request_redraw();
                            return;
                        }
                        _ => {} // Other keys pass through to terminal
                    }
                }

                // When search bar is active, route keys to search
                if self.search.active {
                    match &key_event.logical_key {
                        Key::Named(NamedKey::Escape) => {
                            self.search.close();
                            self.request_redraw();
                            return;
                        }
                        Key::Named(NamedKey::Enter) => {
                            if self.modifiers.shift_key() {
                                self.search.prev();
                            } else {
                                self.search.next();
                            }
                            // Scroll to focused match
                            if let Some(row) = self.search.focused_match().map(|m| m.row) {
                                if let Some(session) = self.active_session_mut() {
                                    session.terminal.scroll_to_search_row(row);
                                }
                            }
                            self.request_redraw();
                            return;
                        }
                        Key::Named(NamedKey::Backspace) => {
                            if let Some(terminal) = self
                                .tabs
                                .get(self.active_tab)
                                .and_then(|t| t.content.as_terminal())
                                .map(|s| &s.terminal)
                            {
                                self.search.pop_char(terminal);
                            }
                            self.request_redraw();
                            return;
                        }
                        _ => {
                            // Ctrl+R toggles regex mode
                            if self.modifiers.control_key() {
                                if let Key::Character(c) = &key_event.logical_key {
                                    if c.as_str() == "r" {
                                        self.search.toggle_regex();
                                        if let Some(terminal) = self
                                            .tabs
                                            .get(self.active_tab)
                                            .and_then(|t| t.content.as_terminal())
                                            .map(|s| &s.terminal)
                                        {
                                            self.search.update_matches(terminal);
                                        }
                                        self.request_redraw();
                                        return;
                                    }
                                }
                            }
                            // Type into search query
                            if let Some(ref text) = key_event.text {
                                for ch in text.chars() {
                                    if !ch.is_control() {
                                        if let Some(terminal) = self
                                            .tabs
                                            .get(self.active_tab)
                                            .and_then(|t| t.content.as_terminal())
                                            .map(|s| &s.terminal)
                                        {
                                            self.search.push_char(ch, terminal);
                                        }
                                    }
                                }
                                self.request_redraw();
                            }
                            return;
                        }
                    }
                }

                // Dispatch through keybinding registry
                if let Some(action) = self
                    .config
                    .keybindings
                    .match_key(&key_event, self.modifiers)
                {
                    if let Some(command) =
                        app_command_for_keybinding(&action, self.active_tab, self.tabs.len())
                    {
                        let mut sidebar_changed = false;
                        if self.handle_app_command(command, &mut sidebar_changed) && sidebar_changed
                        {
                            self.recompute_layout();
                            self.resize_terminal_tabs();
                        }
                        return;
                    }

                    match action {
                        Action::Search => {
                            if self.route_code_editor_command(CommandId::Find) {
                                return;
                            }
                            // Search only works on terminal tabs
                            if self.active_session().is_some() {
                                self.search.toggle();
                                self.request_redraw();
                            }
                        }
                        Action::Copy => {
                            if self.route_code_editor_command(CommandId::Copy) {
                                return;
                            }
                            if !self.copy_stacker_editor_selection() {
                                self.copy_selection();
                            }
                        }
                        Action::Paste => {
                            if self.route_code_editor_command(CommandId::Paste) {
                                return;
                            }
                            self.do_paste();
                        }
                        Action::SelectAll => {
                            if self.route_code_editor_command(CommandId::SelectAll) {
                                return;
                            }
                            if !self.select_all_stacker_editor() {
                                self.do_select_all();
                            }
                        }
                        Action::ToggleErrorPanel => {
                            self.error_panel.toggle();
                            self.request_redraw();
                        }
                        Action::CyclePaneForward | Action::CyclePaneBackward => {
                            // Pane cycling removed — these are no-ops now
                        }
                        Action::ScrollPageUp => {
                            if !self.mouse_reporting() {
                                if let Some(s) = self.active_session_mut() {
                                    s.terminal.scroll_page_up();
                                }
                                self.invalidate_and_redraw();
                            }
                        }
                        Action::ScrollPageDown => {
                            if !self.mouse_reporting() {
                                if let Some(s) = self.active_session_mut() {
                                    s.terminal.scroll_page_down();
                                }
                                self.invalidate_and_redraw();
                            }
                        }
                        Action::ToggleTerminalPanel => {
                            // Terminal panel in explorer removed — no-op
                        }
                        Action::NewWindow
                        | Action::NewTab
                        | Action::CloseTab
                        | Action::NextTab
                        | Action::PrevTab
                        | Action::SplitVertical
                        | Action::SplitHorizontal
                        | Action::ToggleFullscreen
                        | Action::ToggleEffects
                        | Action::ToggleFps
                        | Action::ToggleSidebar
                        | Action::ZoomIn
                        | Action::ZoomOut
                        | Action::ZoomReset
                        | Action::SwitchTab(_) => {}
                    }
                    return;
                }

                if self.active_stacker_tab()
                    && !self.modifiers.control_key()
                    && !self.modifiers.alt_key()
                    && !self.modifiers.super_key()
                {
                    match key_event.logical_key {
                        Key::Named(NamedKey::Backspace) => {
                            llnzy::external_input_trace::trace("stacker.winit_backspace", || {
                                "winit Backspace path fired".to_string()
                            });
                            let handled = self.delete_stacker_editor_backward();
                            llnzy::external_input_trace::trace("stacker.winit_backspace", || {
                                format!("handled={handled}")
                            });
                            return;
                        }
                        Key::Named(NamedKey::Delete) => {
                            llnzy::external_input_trace::trace("stacker.winit_delete", || {
                                "winit Delete path fired".to_string()
                            });
                            self.delete_stacker_editor_forward();
                            return;
                        }
                        _ => {}
                    }
                }

                // On macOS the NSTextInputClient subview owns text input for
                // Stacker (insertText: / setMarkedText: drive the session).
                // Routing key_event.text into the editor here would double
                // every keystroke, because AppKit's interpretKeyEvents:
                // dispatches insertText: on the subview in parallel with
                // winit's keyboard event.
                #[cfg(not(target_os = "macos"))]
                if let Some(ref text) = key_event.text {
                    let s = text.as_str();
                    if !s.is_empty()
                        && !self.modifiers.control_key()
                        && !self.modifiers.alt_key()
                        && !self.modifiers.super_key()
                        && self.append_text_to_stacker_editor(s)
                    {
                        llnzy::external_input_trace::trace("stacker.keyboard_text", || {
                            format!("chars={}", s.chars().count())
                        });
                        return;
                    }
                }

                // Only send raw keys to PTY if active tab is a terminal
                if self.active_session().is_some() {
                    if llnzy::platform::input::is_modifier_only_key(&key_event) {
                        return;
                    }

                    if self.terminal_selection_active() {
                        self.clear_terminal_selection();
                        self.request_redraw();
                    }

                    self.last_keypress = Instant::now();
                    let app_cursor = self.app_cursor();
                    if let Some(intent) = llnzy::platform::input::keyboard_intent(
                        &key_event,
                        self.modifiers,
                        app_cursor,
                    ) {
                        match intent {
                            llnzy::platform::input::PlatformInputIntent::TextInput(text) => {
                                self.write_text_to_active(&text);
                            }
                            llnzy::platform::input::PlatformInputIntent::TerminalInput(bytes) => {
                                self.write_to_active(&bytes);
                            }
                            _ => {}
                        }
                    }
                }
            }

            // IME (input method) events — handles composed text from
            // non-US keyboards, dead keys, and CJK input methods.
            WindowEvent::Ime(ime) => {
                // On macOS the NSTextInputClient subview owns IME composition
                // (setMarkedText: / unmarkText). Writing winit's Ime::Commit
                // text here would land a second copy after AppKit already
                // committed the marked range into the session.
                #[cfg(not(target_os = "macos"))]
                if let Ime::Commit(text) = &ime {
                    if self.append_text_to_stacker_editor(text) {
                        llnzy::external_input_trace::trace("stacker.ime_commit", || {
                            format!("chars={}", text.chars().count())
                        });
                        return;
                    }
                }

                if self.active_session().is_some() {
                    match ime {
                        Ime::Commit(text) => {
                            self.last_keypress = Instant::now();
                            if self.terminal_selection_active() {
                                self.clear_terminal_selection();
                            }
                            if self.search.active {
                                if let Some(terminal) = self
                                    .tabs
                                    .get(self.active_tab)
                                    .and_then(|t| t.content.as_terminal())
                                    .map(|s| &s.terminal)
                                {
                                    for ch in text.chars() {
                                        if !ch.is_control() {
                                            self.search.push_char(ch, terminal);
                                        }
                                    }
                                }
                                self.request_redraw();
                            } else {
                                self.write_text_to_active(&text);
                                self.request_redraw();
                            }
                        }
                        Ime::Preedit(_, _) => {}
                        Ime::Enabled | Ime::Disabled => {}
                    }
                }
            }

            // Drag-and-drop: insert escaped file path into terminal
            WindowEvent::DroppedFile(path) => {
                let target = self
                    .ui
                    .as_ref()
                    .and_then(|ui| ui.drag_drop.active_target.clone())
                    .or_else(|| self.native_file_drop_target(&path));
                if let (Some(ui), Some(target)) = (&mut self.ui, target) {
                    if let Some(command) =
                        ui.drag_drop.command_for_external_files(vec![path], target)
                    {
                        let mut sidebar_changed = false;
                        if self
                            .handle_app_command(AppCommand::DragDrop(command), &mut sidebar_changed)
                        {
                            if sidebar_changed {
                                self.recompute_layout();
                                self.resize_terminal_tabs();
                            }
                            self.request_redraw();
                        }
                    }
                }
            }

            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        self.handle_user_event(event);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.handle_about_to_wait(event_loop);
    }
}
