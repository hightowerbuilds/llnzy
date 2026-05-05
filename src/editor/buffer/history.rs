use crate::editor::history::EditOp;

use super::model::content_hash;
use super::{Buffer, BufferEdit, Position};

impl Buffer {
    /// Undo the last edit. Returns the cursor position to restore.
    pub fn undo(&mut self) -> Option<Position> {
        let op = self.history.undo()?;
        self.apply_inverse(&op);
        self.last_edit = Some(BufferEdit {
            start: op.start,
            old_end: op.end_after,
            new_end: op.end_before,
            new_text: op.old_text.clone(),
        });
        self.modified = content_hash(&self.rope) != self.saved_hash;
        Some(op.start)
    }

    /// Redo the last undone edit. Returns the cursor position to restore.
    pub fn redo(&mut self) -> Option<Position> {
        let op = self.history.redo()?;
        self.apply_forward(&op);
        self.last_edit = Some(BufferEdit {
            start: op.start,
            old_end: op.end_before,
            new_end: op.end_after,
            new_text: op.new_text.clone(),
        });
        self.modified = content_hash(&self.rope) != self.saved_hash;
        Some(op.end_after)
    }

    fn apply_inverse(&mut self, op: &EditOp) {
        let start_idx = self.pos_to_char(op.start);
        // Remove what was inserted.
        if !op.new_text.is_empty() {
            let end_idx = start_idx + op.new_text.chars().count();
            let end_idx = end_idx.min(self.rope.len_chars());
            if start_idx < end_idx {
                self.rope.remove(start_idx..end_idx);
            }
        }
        // Re-insert what was deleted.
        if !op.old_text.is_empty() {
            self.rope.insert(start_idx, &op.old_text);
        }
    }

    fn apply_forward(&mut self, op: &EditOp) {
        let start_idx = self.pos_to_char(op.start);
        // Remove what was there before.
        if !op.old_text.is_empty() {
            let end_idx = start_idx + op.old_text.chars().count();
            let end_idx = end_idx.min(self.rope.len_chars());
            if start_idx < end_idx {
                self.rope.remove(start_idx..end_idx);
            }
        }
        // Insert the new text.
        if !op.new_text.is_empty() {
            self.rope.insert(start_idx, &op.new_text);
        }
    }
}
