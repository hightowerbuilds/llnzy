use winit::event_loop::ActiveEventLoop;

use llnzy::ui::{PendingClose, SavePromptResponse};

use crate::App;

impl App {
    pub(crate) fn handle_save_prompt_response(
        &mut self,
        response: Option<SavePromptResponse>,
        event_loop: &ActiveEventLoop,
    ) -> bool {
        let Some(response) = response else {
            return false;
        };

        let pending = self.ui.as_mut().and_then(|u| u.pending_close.take());
        match (response, pending) {
            (SavePromptResponse::Save, Some(PendingClose::Tab(file))) => {
                match self.save_code_buffer_for_close(file.buffer_id) {
                    Ok(()) => {
                        self.clear_save_prompt_error();
                        self.force_close_tab_id(file.tab_id);
                    }
                    Err(e) => {
                        if let Some(ui) = &mut self.ui {
                            ui.pending_close = Some(PendingClose::Tab(file));
                        }
                        self.report_save_failure(e);
                    }
                }
                true
            }
            (SavePromptResponse::DontSave, Some(PendingClose::Tab(file))) => {
                self.clear_save_prompt_error();
                self.clear_editor_recovery_snapshot_for_buffer(file.buffer_id);
                self.force_close_tab_id(file.tab_id);
                true
            }
            (SavePromptResponse::Save, Some(PendingClose::Window(tabs))) => {
                match self.save_modified_tabs_for_close(&tabs) {
                    Ok(()) => {
                        self.clear_save_prompt_error();
                        self.terminate_all_terminal_tabs_for_exit();
                        self.save_window_state();
                        event_loop.exit();
                    }
                    Err(e) => {
                        let still_modified = self.modified_code_tabs();
                        if let Some(ui) = &mut self.ui {
                            ui.pending_close =
                                Some(PendingClose::Window(if still_modified.is_empty() {
                                    tabs
                                } else {
                                    still_modified
                                }));
                        }
                        self.report_save_failure(e);
                    }
                }
                true
            }
            (SavePromptResponse::DontSave, Some(PendingClose::Window(files))) => {
                self.clear_save_prompt_error();
                for file in files {
                    self.clear_editor_recovery_snapshot_for_buffer(file.buffer_id);
                }
                self.terminate_all_terminal_tabs_for_exit();
                self.save_window_state();
                event_loop.exit();
                true
            }
            (SavePromptResponse::Cancel, pending) => {
                drop(pending);
                self.clear_save_prompt_error();
                true
            }
            _ => false,
        }
    }

    fn clear_save_prompt_error(&mut self) {
        if let Some(ui) = &mut self.ui {
            ui.save_prompt_error = None;
        }
    }
}
