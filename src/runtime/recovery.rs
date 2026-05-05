use llnzy::editor::recovery;
use llnzy::editor::BufferId;

use crate::App;

impl App {
    pub(crate) fn save_editor_recovery_snapshots(&mut self) {
        let Some(ui) = &self.ui else {
            return;
        };

        let mut failed = 0usize;
        for buffer in &ui.editor_view.editor.buffers {
            if let Err(err) = recovery::save_or_clear_buffer_snapshot(buffer) {
                failed += 1;
                self.error_log.warn(err);
            }
        }

        if failed > 0 {
            self.error_log.warn(format!(
                "Editor recovery snapshot save failed for {failed} buffer(s)"
            ));
        }
    }

    pub(crate) fn clear_editor_recovery_snapshot_for_buffer(&mut self, buffer_id: BufferId) {
        let Some(ui) = &self.ui else {
            return;
        };
        let Some(buffer) = ui.editor_view.editor.buffer_for_id(buffer_id) else {
            return;
        };
        if let Err(err) = recovery::clear_buffer_snapshot(buffer) {
            self.error_log.warn(err);
        }
    }
}
