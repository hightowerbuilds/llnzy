use ropey::Rope;

use crate::editor::history::UndoHistory;

use super::model::content_hash;
use super::Buffer;

impl Buffer {
    pub fn restore_unsaved_text(&mut self, text: &str) {
        let normalized = text.replace("\r\n", "\n");
        self.rope = Rope::from_str(&normalized);
        self.modified = content_hash(&self.rope) != self.saved_hash;
        self.history = UndoHistory::new();
    }
}
