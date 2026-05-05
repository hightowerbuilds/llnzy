use crate::editor::recovery;

use super::super::explorer_view::EditorViewState;

impl EditorViewState {
    pub(super) fn command_save(&mut self) {
        let Some(buf) = self.editor.buffers.get_mut(self.editor.active) else {
            return;
        };
        match buf.save() {
            Ok(()) => {
                let _ = recovery::clear_buffer_snapshot(buf);
                self.status_msg = Some("Saved".to_string());
                self.lsp_did_save();
                self.request_hints_and_lenses();
            }
            Err(e) => self.status_msg = Some(save_failed_status(&e)),
        }
    }
}

fn save_failed_status(error: &str) -> String {
    format!("Save failed: {error}")
}
