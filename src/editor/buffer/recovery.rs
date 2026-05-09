use ropey::Rope;

use crate::editor::history::UndoHistory;
use crate::text_utils::normalize_crlf_to_lf;

use super::model::content_hash;
use super::Buffer;

impl Buffer {
    pub fn restore_unsaved_text(&mut self, text: &str) {
        let normalized = normalize_crlf_to_lf(text);
        self.rope = Rope::from_str(normalized.as_ref());
        self.modified = content_hash(&self.rope) != self.saved_hash;
        self.history = UndoHistory::new();
    }
}
