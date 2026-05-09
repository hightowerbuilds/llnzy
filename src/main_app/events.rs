use crate::*;

impl App {
    pub(super) fn handle_user_event(&mut self, event: UserEvent) {
        match event {
            UserEvent::PtyOutput => self.request_redraw(),
            UserEvent::LspMessage => self.request_redraw(),
            UserEvent::FileChanged(_) => self.request_redraw(),
            #[cfg(target_os = "macos")]
            UserEvent::StackerNativeTextChanged {
                kind,
                text,
                utf16_start,
                utf16_end,
            } => {
                self.apply_stacker_native_text_changed(kind, text, utf16_start, utf16_end);
            }
            #[cfg(target_os = "macos")]
            UserEvent::MenuCommand(command_id) => {
                self.handle_platform_menu_command(&command_id);
            }
        }
    }
}
