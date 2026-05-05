use crate::*;

impl App {
    pub(super) fn handle_user_event(&mut self, event: UserEvent) {
        match event {
            UserEvent::PtyOutput => self.request_redraw(),
            UserEvent::LspMessage => self.request_redraw(),
            UserEvent::FileChanged(_) => self.request_redraw(),
            UserEvent::StackerWebViewMessage(raw) => {
                self.apply_stacker_webview_message(raw);
            }
            #[cfg(target_os = "macos")]
            UserEvent::StackerNativeEdit(edit) => {
                self.apply_stacker_native_edit(edit);
            }
            #[cfg(target_os = "macos")]
            UserEvent::MenuCommand(command_id) => {
                self.handle_platform_menu_command(&command_id);
            }
        }
    }
}
