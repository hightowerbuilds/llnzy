use crate::*;

impl App {
    pub(super) fn handle_terminal_mouse_input(&mut self, state: ElementState, button: MouseButton) {
        if self.cursor_over_non_terminal_chrome() {
            self.request_redraw();
            return;
        }
        let (row, col) = self.pixel_to_grid(self.cursor_pos);

        if button == MouseButton::Right && state == ElementState::Pressed {
            if self.terminal_selection_active() {
                self.copy_selection();
            }
            return;
        }

        if button == MouseButton::Left
            && state == ElementState::Pressed
            && self.modifiers.super_key()
        {
            self.open_terminal_link_or_file_at(row, col);
            return;
        }

        let local_terminal_selection = local_terminal_selection_requested(
            self.mouse_reporting(),
            self.modifiers.shift_key(),
            self.terminal_selection_drag,
        );
        if self.mouse_reporting() && button == MouseButton::Left && !local_terminal_selection {
            match state {
                ElementState::Pressed => {
                    self.clear_terminal_selection();
                    self.mouse_pressed = true;
                    self.terminal_selection_drag = false;
                    self.terminal_pending_mouse_press = Some((row, col));
                    self.request_redraw();
                }
                ElementState::Released => {
                    let mut selection_changed = false;
                    if let Some((press_row, press_col)) = self.terminal_pending_mouse_press.take() {
                        let sgr = self.sgr_mouse();
                        let press = llnzy::platform::input::mouse_report_intent(
                            0,
                            press_col,
                            press_row,
                            true,
                            sgr,
                            &self.modifiers,
                        );
                        let release = llnzy::platform::input::mouse_report_intent(
                            0,
                            col,
                            row,
                            false,
                            sgr,
                            &self.modifiers,
                        );
                        if let llnzy::platform::input::PlatformInputIntent::MouseReport(press) =
                            press
                        {
                            self.write_to_active(&press);
                        }
                        if let llnzy::platform::input::PlatformInputIntent::MouseReport(release) =
                            release
                        {
                            self.write_to_active(&release);
                        }
                    } else if self.terminal_selection_drag
                        && self.update_active_terminal_selection(row, col)
                    {
                        selection_changed = true;
                        self.request_redraw();
                    }
                    if self.terminal_selection_drag || selection_changed {
                        self.copy_terminal_selection_on_release();
                    }
                    self.mouse_pressed = false;
                    self.terminal_selection_drag = false;
                }
            }
            return;
        }

        if button == MouseButton::Left {
            match state {
                ElementState::Pressed => {
                    self.terminal_selection_drag = local_terminal_selection;
                    let click_count = self.click_state.click(row, col);
                    match click_count {
                        2 => {
                            if let Some(session) = self.active_session_mut() {
                                session.terminal.select_word(row, col);
                            }
                        }
                        3 => {
                            if let Some(session) = self.active_session_mut() {
                                session.terminal.select_line(row);
                            }
                        }
                        _ => {
                            if let Some(session) = self.active_session_mut() {
                                session.terminal.start_selection(row, col);
                            }
                        }
                    }
                    self.mouse_pressed = true;
                    self.request_redraw();
                }
                ElementState::Released => {
                    if self.update_active_terminal_selection(row, col) {
                        self.request_redraw();
                    }
                    self.copy_terminal_selection_on_release();
                    self.mouse_pressed = false;
                    self.terminal_selection_drag = false;
                    self.terminal_pending_mouse_press = None;
                }
            }
        }
    }

    pub(super) fn handle_cursor_moved(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        self.cursor_pos = position;

        if self.mouse_pressed {
            let (row, col) = self.pixel_to_grid(position);
            if self.mouse_reporting() && !self.modifiers.shift_key() {
                if self.terminal_selection_drag {
                    if self.update_active_terminal_selection(row, col) {
                        self.request_redraw();
                    }
                } else if let Some(start) = self.terminal_pending_mouse_press {
                    if terminal_mouse_drag_exceeded(start, row, col) {
                        self.terminal_pending_mouse_press = None;
                        self.terminal_selection_drag = true;
                        if let Some(session) = self.active_session_mut() {
                            session.terminal.start_selection(start.0, start.1);
                            session.terminal.update_selection(row, col);
                        }
                        self.request_redraw();
                    }
                }
            } else if self.click_state.count() <= 1
                && self.update_active_terminal_selection(row, col)
            {
                self.request_redraw();
            }
        }
    }

    fn copy_terminal_selection_on_release(&mut self) {
        let Some(text) = copy_on_select_payload(
            self.config.terminal.copy_on_select,
            self.active_session()
                .and_then(|session| session.terminal.selected_text()),
        ) else {
            return;
        };

        if let Err(error) = self.clipboard.set_text(text) {
            log::warn!("copy-on-select failed: {error}");
        }
    }

    fn open_terminal_link_or_file_at(&mut self, row: usize, col: usize) {
        if let Some(session) = self.active_session() {
            let line_text = {
                let (cols, _) = session.terminal.size();
                (0..cols)
                    .map(|c| session.terminal.cell_char(row, c))
                    .collect::<String>()
            };
            let file_loc = parse_file_location(&line_text, col);

            if let Some((path, line, col_num)) = file_loc {
                if let Some(ui) = &mut self.ui {
                    match ui.editor_view.open_file(path) {
                        Ok(buffer_id) => {
                            let Some(idx) = ui.editor_view.editor.index_for_id(buffer_id) else {
                                return;
                            };
                            let view = &mut ui.editor_view.editor.views[idx];
                            view.cursor.pos = llnzy::editor::buffer::Position::new(
                                line.saturating_sub(1),
                                col_num.saturating_sub(1),
                            );
                            view.cursor.clear_selection();
                            view.cursor.desired_col = None;
                            ui.active_view = ActiveView::Shells;
                        }
                        Err(e) => {
                            self.error_log.error(format!("Cannot open file: {e}"));
                        }
                    }
                    self.request_redraw();
                }
                return;
            }

            let url = session
                .terminal
                .cell_hyperlink(row, col)
                .or_else(|| {
                    let line_text = session.terminal.row_text(row);
                    llnzy::terminal::detect_urls(&line_text)
                        .into_iter()
                        .find(|(start, end, _)| col >= *start && col < *end)
                        .map(|(_, _, url)| url)
                })
                .or_else(|| {
                    let text = session.terminal.word_at(row, col);
                    if text.starts_with("http://") || text.starts_with("https://") {
                        Some(text)
                    } else {
                        None
                    }
                });
            if let Some(url) = url {
                if let Err(error) = llnzy::platform::open::open_url(url) {
                    log::warn!("Failed to open terminal URL: {error}");
                }
            }
        }
    }
}

fn copy_on_select_payload(enabled: bool, selected_text: Option<String>) -> Option<String> {
    if !enabled {
        return None;
    }
    selected_text.filter(|text| !text.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_on_select_payload_requires_enabled_nonempty_selection() {
        assert_eq!(
            copy_on_select_payload(false, Some("text".to_string())),
            None
        );
        assert_eq!(copy_on_select_payload(true, None), None);
        assert_eq!(copy_on_select_payload(true, Some(String::new())), None);
        assert_eq!(
            copy_on_select_payload(true, Some("selected".to_string())),
            Some("selected".to_string())
        );
    }
}
