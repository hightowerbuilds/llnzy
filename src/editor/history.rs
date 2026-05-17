use std::time::Instant;

use super::buffer::Position;

/// A single edit operation, sufficient to undo or redo.
#[derive(Clone, Debug)]
pub struct EditOp {
    /// Position where the edit starts.
    pub start: Position,
    /// Position where the old content ended (before the edit).
    pub end_before: Position,
    /// Position where the new content ends (after the edit).
    pub end_after: Position,
    /// Text that was removed (empty for pure insertion).
    pub old_text: String,
    /// Text that was inserted (empty for pure deletion).
    pub new_text: String,
}

/// Linear undo/redo history with coalescing of rapid edits.
pub struct UndoHistory {
    /// Past operations (most recent at the end).
    undo_stack: Vec<EditOp>,
    /// Undone operations available for redo (most recent at the end).
    redo_stack: Vec<EditOp>,
    /// Maximum undo depth.
    max_depth: usize,
    /// Soft cap on the total bytes of `old_text + new_text` retained across
    /// the undo stack. When exceeded, oldest entries are dropped.
    max_bytes: usize,
    /// Running total of `old_text + new_text` bytes in the undo stack.
    undo_bytes: usize,
    /// Timestamp of the last push, for coalescing.
    last_push: Instant,
    /// Index in undo_stack that corresponds to the saved state.
    /// None if the saved state is no longer in the history.
    saved_at: Option<usize>,
}

/// Maximum time between edits that can be coalesced into a single undo entry.
const COALESCE_WINDOW_MS: u128 = 800;

/// Default soft cap on undo-stack byte size. A 10MB paste produces one ~20MB
/// op (old_text + new_text); the count cap of 1000 alone could let undo
/// retain tens of GB worth of text, so we also bound by total bytes.
const DEFAULT_MAX_UNDO_BYTES: usize = 64 * 1024 * 1024;

fn op_bytes(op: &EditOp) -> usize {
    op.old_text.len() + op.new_text.len()
}

impl UndoHistory {
    pub fn new() -> Self {
        Self::with_depth(1000)
    }

    pub fn with_depth(max_depth: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_depth,
            max_bytes: DEFAULT_MAX_UNDO_BYTES,
            undo_bytes: 0,
            last_push: Instant::now() - std::time::Duration::from_secs(60),
            saved_at: Some(0), // empty buffer is the saved state
        }
    }

    /// Push a new edit operation. Clears the redo stack.
    pub fn push(&mut self, op: EditOp) {
        self.redo_stack.clear();
        self.undo_bytes += op_bytes(&op);
        self.undo_stack.push(op);
        self.last_push = Instant::now();

        // Enforce depth limit
        if self.undo_stack.len() > self.max_depth {
            let excess = self.undo_stack.len() - self.max_depth;
            let drained = self.undo_stack.drain(0..excess);
            for evicted in drained {
                self.undo_bytes = self.undo_bytes.saturating_sub(op_bytes(&evicted));
            }
            // Adjust saved_at
            self.saved_at = self.saved_at.and_then(|s| s.checked_sub(excess));
        }

        // Enforce byte cap by dropping oldest entries. Always keep at least
        // one op so the most recent edit can be undone even if it alone
        // exceeds the cap.
        let mut dropped = 0usize;
        while self.undo_bytes > self.max_bytes && self.undo_stack.len() > 1 {
            let evicted = self.undo_stack.remove(0);
            self.undo_bytes = self.undo_bytes.saturating_sub(op_bytes(&evicted));
            dropped += 1;
        }
        if dropped > 0 {
            self.saved_at = self.saved_at.and_then(|s| s.checked_sub(dropped));
        }
    }

    /// Try to coalesce a character insertion with the previous operation.
    /// Returns true if coalesced, false if a new op should be pushed.
    pub fn try_coalesce_insert(&mut self, pos: Position, text: &str, end_pos: Position) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_push).as_millis();

        if elapsed > COALESCE_WINDOW_MS {
            return false;
        }

        let Some(prev) = self.undo_stack.last_mut() else {
            return false;
        };

        // Only coalesce if:
        // 1. Previous was also a pure insertion (no deletion)
        // 2. This insert continues immediately after the previous one
        // 3. Neither contains a newline (newlines break undo groups)
        if !prev.old_text.is_empty() {
            return false;
        }
        if prev.end_after != pos {
            return false;
        }
        if prev.new_text.contains('\n') || text.contains('\n') {
            return false;
        }

        // Coalesce: extend the previous op
        prev.new_text.push_str(text);
        prev.end_after = end_pos;
        self.undo_bytes += text.len();
        self.last_push = now;
        self.redo_stack.clear();
        true
    }

    /// Undo the last operation. Returns the operation to reverse.
    pub fn undo(&mut self) -> Option<EditOp> {
        let op = self.undo_stack.pop()?;
        self.undo_bytes = self.undo_bytes.saturating_sub(op_bytes(&op));
        self.redo_stack.push(op.clone());
        Some(op)
    }

    /// Redo the last undone operation. Returns the operation to reapply.
    pub fn redo(&mut self) -> Option<EditOp> {
        let op = self.redo_stack.pop()?;
        self.undo_bytes += op_bytes(&op);
        self.undo_stack.push(op.clone());
        Some(op)
    }

    /// Mark the current state as "saved" (for modified tracking).
    pub fn mark_saved(&mut self) {
        self.saved_at = Some(self.undo_stack.len());
    }

    /// Whether the current state matches the last saved state.
    pub fn is_at_saved(&self) -> bool {
        self.saved_at == Some(self.undo_stack.len())
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clear all history.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.undo_bytes = 0;
        self.saved_at = Some(0);
    }
}

impl Default for UndoHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn insert_op(line: usize, col: usize, text: &str) -> EditOp {
        let end_col = col + text.len();
        EditOp {
            start: Position::new(line, col),
            end_before: Position::new(line, col),
            end_after: Position::new(line, end_col),
            old_text: String::new(),
            new_text: text.to_string(),
        }
    }

    fn delete_op(line: usize, start_col: usize, end_col: usize, old: &str) -> EditOp {
        EditOp {
            start: Position::new(line, start_col),
            end_before: Position::new(line, end_col),
            end_after: Position::new(line, start_col),
            old_text: old.to_string(),
            new_text: String::new(),
        }
    }

    #[test]
    fn push_and_undo() {
        let mut h = UndoHistory::new();
        h.push(insert_op(0, 0, "hello"));
        assert!(h.can_undo());
        let op = h.undo().unwrap();
        assert_eq!(op.new_text, "hello");
        assert!(!h.can_undo());
    }

    #[test]
    fn undo_then_redo() {
        let mut h = UndoHistory::new();
        h.push(insert_op(0, 0, "hello"));
        h.undo();
        assert!(h.can_redo());
        let op = h.redo().unwrap();
        assert_eq!(op.new_text, "hello");
        assert!(!h.can_redo());
    }

    #[test]
    fn push_clears_redo() {
        let mut h = UndoHistory::new();
        h.push(insert_op(0, 0, "a"));
        h.undo();
        assert!(h.can_redo());
        h.push(insert_op(0, 0, "b"));
        assert!(!h.can_redo());
    }

    #[test]
    fn depth_limit_enforced() {
        let mut h = UndoHistory::with_depth(3);
        h.push(insert_op(0, 0, "a"));
        h.push(insert_op(0, 1, "b"));
        h.push(insert_op(0, 2, "c"));
        h.push(insert_op(0, 3, "d"));
        // oldest ("a") should be dropped
        assert_eq!(h.undo_stack.len(), 3);
        let first = &h.undo_stack[0];
        assert_eq!(first.new_text, "b");
    }

    #[test]
    fn coalesce_consecutive_inserts() {
        let mut h = UndoHistory::new();
        h.push(insert_op(0, 0, "h"));
        // Immediate follow-up at the next position
        let coalesced = h.try_coalesce_insert(Position::new(0, 1), "e", Position::new(0, 2));
        assert!(coalesced);
        assert_eq!(h.undo_stack.len(), 1);
        assert_eq!(h.undo_stack[0].new_text, "he");
    }

    #[test]
    fn no_coalesce_after_newline() {
        let mut h = UndoHistory::new();
        h.push(insert_op(0, 0, "a"));
        let coalesced = h.try_coalesce_insert(Position::new(0, 1), "\n", Position::new(1, 0));
        assert!(!coalesced);
    }

    #[test]
    fn no_coalesce_non_adjacent() {
        let mut h = UndoHistory::new();
        h.push(insert_op(0, 0, "a"));
        // Position gap: insert at col 5 instead of col 1
        let coalesced = h.try_coalesce_insert(Position::new(0, 5), "b", Position::new(0, 6));
        assert!(!coalesced);
    }

    #[test]
    fn saved_state_tracking() {
        let mut h = UndoHistory::new();
        assert!(h.is_at_saved());
        h.push(insert_op(0, 0, "a"));
        assert!(!h.is_at_saved());
        h.mark_saved();
        assert!(h.is_at_saved());
        h.push(insert_op(0, 1, "b"));
        assert!(!h.is_at_saved());
        h.undo();
        assert!(h.is_at_saved());
    }

    #[test]
    fn byte_cap_drops_oldest_entries() {
        let mut h = UndoHistory::with_depth(1000);
        // Force a small cap so we can exercise the eviction path with a few
        // ops instead of allocating tens of MiB in a test.
        h.max_bytes = 32;
        h.push(insert_op(0, 0, "aaaaaaaaaa")); // 10 bytes
        h.push(insert_op(0, 10, "bbbbbbbbbb")); // 20 bytes total
        h.push(insert_op(0, 20, "cccccccccc")); // 30 bytes total
        assert_eq!(h.undo_stack.len(), 3);
        // This push exceeds the 32-byte cap; the oldest ("aaa…") should go.
        h.push(insert_op(0, 30, "dddddddddd")); // would push to 40 bytes
        assert_eq!(h.undo_stack.len(), 3);
        assert_eq!(h.undo_stack[0].new_text, "bbbbbbbbbb");
        // Cap is exceeded after a single very large op, but we always retain
        // the most recent so the user can still undo it.
        h.clear();
        h.push(insert_op(0, 0, "x".repeat(64).as_str()));
        assert_eq!(h.undo_stack.len(), 1);
    }

    #[test]
    fn delete_op_undo_redo() {
        let mut h = UndoHistory::new();
        h.push(delete_op(0, 0, 5, "hello"));
        let undone = h.undo().unwrap();
        assert_eq!(undone.old_text, "hello");
        let redone = h.redo().unwrap();
        assert_eq!(redone.old_text, "hello");
    }
}
