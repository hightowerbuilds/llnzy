use crate::*;

impl App {
    pub(super) fn handle_user_event(&mut self, event: UserEvent) {
        match event {
            UserEvent::PtyOutput => self.request_redraw(),
            UserEvent::LspMessage => self.request_redraw(),
            UserEvent::FileChanged(_) => self.request_redraw(),
            #[cfg(target_os = "macos")]
            UserEvent::StackerInputClientInsertText {
                text,
                replacement_utf16,
            } => {
                self.apply_stacker_input_client_insert_text(text, replacement_utf16);
            }
            #[cfg(target_os = "macos")]
            UserEvent::StackerInputClientSetMarkedText {
                text,
                marked_internal_utf16,
                replacement_utf16,
            } => {
                self.apply_stacker_input_client_set_marked_text(
                    text,
                    marked_internal_utf16,
                    replacement_utf16,
                );
            }
            #[cfg(target_os = "macos")]
            UserEvent::StackerInputClientUnmarkText => {
                self.apply_stacker_input_client_unmark_text();
            }
            #[cfg(target_os = "macos")]
            UserEvent::StackerInputClientDoCommand { selector_name } => {
                self.apply_stacker_input_client_do_command(&selector_name);
            }
            #[cfg(target_os = "macos")]
            UserEvent::MenuCommand(command_id) => {
                self.handle_platform_menu_command(&command_id);
            }
        }
    }
}
