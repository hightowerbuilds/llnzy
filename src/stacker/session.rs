//! Editor-buffer-backed Stacker session.
//!
//! `StackerSession` owns the active prompt document via the editor's
//! rope-backed `Buffer { kind: Prose }` and tracks selection / undo / redo
//! on top of it. It is the single source of truth for prompt text — the
//! toolbar, command palette, formatting commands, and the macOS
//! `LlnzyStackerInputClient` (`NSTextInputClient` subview) all operate
//! against this session.
//!
//! Internally the session pairs the buffer's rope-based history with a
//! parallel selection-history stack, so undo/redo restore the
//! `StackerSelection` that was active at edit time.
//!
//! `sync_to_view` and `sync_from_view` bridge the session's flat-char
//! selection model to the editor view's `(line, col)` `BufferView`, so
//! Stacker is rendered through the same `editor_host::render_prose_editor`
//! path as the source code editor.

use crate::editor::buffer::{Buffer, BufferKind, Position};

use super::input::{
    normalize_input_text, StackerEditOutcome, StackerInputEngine, StackerSelection,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SelectionPair {
    before: StackerSelection,
    after: StackerSelection,
}

pub struct StackerSession {
    buffer: Buffer,
    selection: StackerSelection,
    text_cache: String,
    /// Monotonically incrementing counter. Bumped on every text mutation so
    /// downstream caches (galley, status counts) can detect changes with an
    /// O(1) integer comparison instead of an O(n) string comparison.
    revision: u64,
    /// Cached word/line counts — recomputed only when `revision` changes,
    /// not on every frame.
    word_count: usize,
    line_count: usize,
    /// Selection metadata kept in lockstep with `buffer`'s undo stack.
    /// Pushed on every mutating edit, popped on undo into `selection_redo`.
    selection_history: Vec<SelectionPair>,
    /// Selection metadata kept in lockstep with `buffer`'s redo stack.
    selection_redo: Vec<SelectionPair>,
    /// Range of text currently held as IME composition / dictation
    /// preview. Set by `set_marked_text`, cleared by `unmark_text`. The
    /// macOS `NSTextInputClient` protocol uses this to render the
    /// composition underline and to resolve incoming refinement edits.
    marked_range: Option<StackerSelection>,
}

impl StackerSession {
    pub fn new() -> Self {
        let buffer = Buffer::empty_prose();
        let text_cache = buffer.text();
        Self {
            buffer,
            selection: StackerSelection::collapsed(0),
            revision: 0,
            word_count: 0,
            line_count: 1,
            text_cache,
            selection_history: Vec::new(),
            selection_redo: Vec::new(),
            marked_range: None,
        }
    }

    pub fn kind(&self) -> BufferKind {
        self.buffer.kind()
    }

    pub fn text(&self) -> &str {
        &self.text_cache
    }

    pub fn buffer(&self) -> &crate::editor::buffer::Buffer {
        &self.buffer
    }

    pub fn buffer_mut(&mut self) -> &mut crate::editor::buffer::Buffer {
        &mut self.buffer
    }

    pub fn char_count(&self) -> usize {
        self.buffer.len_chars()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Monotonically increasing counter; bumped on every text mutation.
    /// Use this for O(1) cache invalidation instead of O(n) text comparison.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn word_count(&self) -> usize {
        self.word_count
    }

    pub fn line_count(&self) -> usize {
        self.line_count
    }

    pub fn selection(&self) -> StackerSelection {
        self.selection
    }

    pub fn set_selection(&mut self, selection: StackerSelection) {
        self.selection = self.clamp_selection(selection);
    }

    /// Mirror the session's flat-char selection into a `BufferView`'s
    /// `(line, col)` cursor + anchor. Call before each editor-host render
    /// frame so the editor view paints the correct caret and selection.
    ///
    /// Marked range is intentionally not propagated — the editor view does
    /// not yet understand IME composition; the prose render path consults
    /// `marked_range()` separately for underline drawing.
    pub fn sync_to_view(&self, view: &mut crate::editor::BufferView) {
        let sel = self.selection;
        let end_pos = self.buffer.char_to_pos(sel.end);
        let anchor_pos = if sel.is_collapsed() {
            None
        } else {
            Some(self.buffer.char_to_pos(sel.start))
        };
        view.cursor.pos = end_pos;
        view.cursor.anchor = anchor_pos;
        view.cursor.desired_col = None;
        view.cursor.extra_cursors.clear();
    }

    /// Read the editor view's cursor + anchor back into the session's
    /// selection. Call after the editor host has run for the frame so
    /// pointer-driven cursor moves (the only mutation source in prose
    /// mode, since `prose_mode = true` short-circuits keyboard handling)
    /// reach the session's selection state.
    pub fn sync_from_view(&mut self, view: &crate::editor::BufferView) {
        let end = self.buffer.pos_to_char(view.cursor.pos);
        let start = view
            .cursor
            .anchor
            .map(|a| self.buffer.pos_to_char(a))
            .unwrap_or(end);
        let next = StackerSelection { start, end };
        if next != self.selection {
            self.set_selection(next);
        }
    }

    /// Replace the entire document and reset history. Used when loading a
    /// saved prompt — the previous draft's history shouldn't bleed into the
    /// newly opened prompt.
    pub fn set_text(&mut self, text: impl Into<String>) {
        let text = text.into();
        // Rebuild the buffer fresh so the prior undo stack is discarded.
        self.buffer = Buffer::empty_prose();
        if !text.is_empty() {
            self.buffer.insert(Position::default(), &text);
        }
        self.refresh_text_cache();
        self.selection = StackerSelection::collapsed(self.char_count());
        self.selection_history.clear();
        self.selection_redo.clear();
        self.marked_range = None;
    }

    pub fn clear(&mut self) {
        self.set_text(String::new());
    }

    pub fn can_undo(&self) -> bool {
        !self.selection_history.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.selection_redo.is_empty()
    }

    pub fn replace_selection(&mut self, replacement: &str) -> StackerEditOutcome {
        self.insert_text(self.selection, replacement)
    }

    pub fn insert_text(&mut self, selection: StackerSelection, text: &str) -> StackerEditOutcome {
        let normalized = normalize_input_text(text);
        let sel = self.clamp_selection(selection).sorted();

        if normalized.is_empty() {
            // Match StackerInputEngine: no edit, no history push, cursor
            // collapses to the start of the (clamped, sorted) selection.
            self.selection = StackerSelection::collapsed(sel.start);
            return StackerEditOutcome {
                cursor: sel.start,
                changed: false,
            };
        }

        let before_selection = sel;
        let cursor_after = sel.start + normalized.chars().count();
        let after_selection = StackerSelection::collapsed(cursor_after);

        self.apply_replace(sel, &normalized);
        self.push_history(before_selection, after_selection);
        self.selection = after_selection;

        StackerEditOutcome {
            cursor: cursor_after,
            changed: true,
        }
    }

    pub fn replace_range(
        &mut self,
        selection: StackerSelection,
        replacement: &str,
        next_selection: StackerSelection,
    ) -> StackerEditOutcome {
        let normalized = normalize_input_text(replacement);
        let sel = self.clamp_selection(selection).sorted();

        let changed = if normalized.is_empty() && sel.is_collapsed() {
            false
        } else {
            self.apply_replace(sel, &normalized);
            true
        };

        // Clamp `next_selection` against the post-edit text length so callers
        // that pass cursors aimed at the new content (e.g., wrapping a
        // selection in a marker pair and pointing the cursor between the
        // markers) don't get clipped by the pre-edit length.
        let next = self.clamp_selection(next_selection);
        if changed {
            self.push_history(sel, next);
        }
        self.selection = next;
        StackerEditOutcome {
            cursor: next.end,
            changed,
        }
    }

    pub fn replace_all_with_history(
        &mut self,
        text: String,
        next_selection: StackerSelection,
    ) -> bool {
        let next = clamp_to_text_len(next_selection, &text);
        if self.text_cache == text {
            self.selection = next;
            return false;
        }
        let before_len = self.char_count();
        let before_selection = StackerSelection {
            start: 0,
            end: before_len,
        };
        let whole = StackerSelection {
            start: 0,
            end: before_len,
        };
        self.apply_replace(whole, &text);
        // Selection-history "before" is the prior session selection so undo
        // restores the pre-replace caret/selection state.
        let _ = before_selection;
        self.push_history(self.previous_selection_for_replace_all(), next);
        self.selection = next;
        true
    }

    /// Returns the selection that should be restored on undo of a
    /// replace-all operation. We use the session's current selection at
    /// the moment of the replace, matching the snapshot-based editor's
    /// behavior of capturing pre-edit selection.
    fn previous_selection_for_replace_all(&self) -> StackerSelection {
        // We've just mutated the buffer — but the snapshot we want is the
        // selection that was active before. Callers of replace_all are
        // expected to set the session selection correctly first; if not,
        // we fall back to a collapsed cursor at start.
        self.selection
    }

    pub fn delete_backward(&mut self, selection: StackerSelection) -> StackerEditOutcome {
        let sel = self.clamp_selection(selection).sorted();

        if !sel.is_collapsed() {
            let before_selection = sel;
            let after_selection = StackerSelection::collapsed(sel.start);
            self.apply_replace(sel, "");
            self.push_history(before_selection, after_selection);
            self.selection = after_selection;
            return StackerEditOutcome {
                cursor: sel.start,
                changed: true,
            };
        }

        if sel.start == 0 {
            self.selection = StackerSelection::collapsed(0);
            return StackerEditOutcome {
                cursor: 0,
                changed: false,
            };
        }

        let range = StackerSelection {
            start: sel.start - 1,
            end: sel.start,
        };
        let before_selection = range;
        let after_selection = StackerSelection::collapsed(range.start);
        self.apply_replace(range, "");
        self.push_history(before_selection, after_selection);
        self.selection = after_selection;

        StackerEditOutcome {
            cursor: range.start,
            changed: true,
        }
    }

    pub fn delete_forward(&mut self, selection: StackerSelection) -> StackerEditOutcome {
        let sel = self.clamp_selection(selection).sorted();

        if !sel.is_collapsed() {
            let before_selection = sel;
            let after_selection = StackerSelection::collapsed(sel.start);
            self.apply_replace(sel, "");
            self.push_history(before_selection, after_selection);
            self.selection = after_selection;
            return StackerEditOutcome {
                cursor: sel.start,
                changed: true,
            };
        }

        let total = self.char_count();
        if sel.start >= total {
            self.selection = StackerSelection::collapsed(total);
            return StackerEditOutcome {
                cursor: total,
                changed: false,
            };
        }

        let range = StackerSelection {
            start: sel.start,
            end: sel.start + 1,
        };
        let before_selection = range;
        let after_selection = StackerSelection::collapsed(range.start);
        self.apply_replace(range, "");
        self.push_history(before_selection, after_selection);
        self.selection = after_selection;

        StackerEditOutcome {
            cursor: range.start,
            changed: true,
        }
    }

    pub fn selected_text(&self, selection: StackerSelection) -> Option<String> {
        StackerInputEngine::selected_text(&self.text_cache, selection)
    }

    pub fn select_all(&mut self) -> StackerSelection {
        let sel = StackerSelection {
            start: 0,
            end: self.char_count(),
        };
        self.selection = sel;
        sel
    }

    /// Returns the range currently held as IME composition / dictation
    /// preview, or `None` if no composition is active.
    pub fn marked_range(&self) -> Option<StackerSelection> {
        self.marked_range
    }

    /// Replace the current marked range (or, if none, `replacement_range`,
    /// or the active selection if both are absent) with `text` and mark
    /// the inserted text as the new composition. `marked_internal_selection`
    /// is the cursor position relative to the start of the new marked
    /// text — the OS uses this to position the composition caret while
    /// the user refines their input.
    ///
    /// Implements the AppKit `setMarkedText:selectedRange:replacementRange:`
    /// contract used by `NSTextInputClient` for IME and dictation.
    pub fn set_marked_text(
        &mut self,
        text: &str,
        marked_internal_selection: StackerSelection,
        replacement_range: Option<StackerSelection>,
    ) {
        let target_range = self
            .marked_range
            .or(replacement_range)
            .unwrap_or(self.selection)
            .sorted();

        let normalized = normalize_input_text(text);
        let marked_len = normalized.chars().count();
        let internal = marked_internal_selection.sorted();
        let internal_clamped = StackerSelection {
            start: internal.start.min(marked_len),
            end: internal.end.min(marked_len),
        };
        let next_selection = StackerSelection {
            start: target_range.start + internal_clamped.start,
            end: target_range.start + internal_clamped.end,
        };

        // The replace path normalizes line endings again, but normalize here
        // first so the marked-length math matches what actually lands.
        self.replace_range(target_range, &normalized, next_selection);

        self.marked_range = if marked_len == 0 {
            None
        } else {
            Some(StackerSelection {
                start: target_range.start,
                end: target_range.start + marked_len,
            })
        };
    }

    /// Commit any active marked range as final text. The text itself stays
    /// in place — `unmark_text` only clears the composition tracking, so
    /// future edits no longer treat the previously-marked region specially.
    pub fn unmark_text(&mut self) {
        self.marked_range = None;
    }

    pub fn undo(&mut self) -> bool {
        let Some(pair) = self.selection_history.pop() else {
            return false;
        };
        if self.buffer.undo().is_none() {
            // Histories desynced — push the pair back to keep state coherent.
            self.selection_history.push(pair);
            return false;
        }
        self.refresh_text_cache();
        self.selection_redo.push(pair);
        self.selection = self.clamp_selection(pair.before);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(pair) = self.selection_redo.pop() else {
            return false;
        };
        if self.buffer.redo().is_none() {
            self.selection_redo.push(pair);
            return false;
        }
        self.refresh_text_cache();
        self.selection_history.push(pair);
        self.selection = self.clamp_selection(pair.after);
        true
    }

    /// Apply a char-offset range replacement to the underlying buffer.
    fn apply_replace(&mut self, range: StackerSelection, replacement: &str) {
        let start_pos = self.buffer.char_to_pos(range.start);
        let end_pos = self.buffer.char_to_pos(range.end);
        if range.is_collapsed() {
            if !replacement.is_empty() {
                self.buffer.insert(start_pos, replacement);
            }
        } else if replacement.is_empty() {
            self.buffer.delete(start_pos, end_pos);
        } else {
            self.buffer.replace(start_pos, end_pos, replacement);
        }
        self.refresh_text_cache();
    }

    /// Rebuild `text_cache` from the rope and update derived counters.
    /// Called whenever the buffer changes so per-frame reads are O(1).
    fn refresh_text_cache(&mut self) {
        self.text_cache = self.buffer.text();
        self.word_count = self.text_cache.split_whitespace().count();
        self.line_count = self.text_cache.lines().count().max(1);
        self.revision = self.revision.wrapping_add(1);
    }

    fn push_history(&mut self, before: StackerSelection, after: StackerSelection) {
        self.selection_history.push(SelectionPair { before, after });
        self.selection_redo.clear();
    }

    fn clamp_selection(&self, selection: StackerSelection) -> StackerSelection {
        // char_count() delegates to ropey::Rope::len_chars(), which is O(1).
        let total = self.char_count();
        StackerSelection {
            start: selection.start.min(total),
            end: selection.end.min(total),
        }
    }
}

impl Default for StackerSession {
    fn default() -> Self {
        Self::new()
    }
}

fn clamp_to_text_len(selection: StackerSelection, text: &str) -> StackerSelection {
    let char_count = text.chars().count();
    StackerSelection {
        start: selection.start.min(char_count),
        end: selection.end.min(char_count),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_session_uses_prose_buffer() {
        let session = StackerSession::new();
        assert_eq!(session.kind(), BufferKind::Prose);
        assert!(session.is_empty());
        assert!(!session.can_undo());
        assert!(!session.can_redo());
    }

    #[test]
    fn insert_records_undo_and_redo() {
        let mut session = StackerSession::new();
        session.insert_text(StackerSelection::collapsed(0), "hello");
        assert_eq!(session.text(), "hello");

        assert!(session.undo());
        assert_eq!(session.text(), "");

        assert!(session.redo());
        assert_eq!(session.text(), "hello");
    }

    #[test]
    fn set_text_resets_history() {
        let mut session = StackerSession::new();
        session.insert_text(StackerSelection::collapsed(0), "draft");
        session.set_text("saved prompt");

        assert!(!session.can_undo());
        assert!(!session.undo());
        assert_eq!(session.text(), "saved prompt");
        assert_eq!(
            session.selection(),
            StackerSelection::collapsed("saved prompt".chars().count())
        );
    }

    #[test]
    fn replace_all_records_single_history_entry() {
        let mut session = StackerSession::new();
        session.set_text("one");

        assert!(session
            .replace_all_with_history("one two".to_string(), StackerSelection::collapsed(7),));
        assert_eq!(session.text(), "one two");

        assert!(session.undo());
        assert_eq!(session.text(), "one");
    }

    #[test]
    fn undo_redo_availability_tracks_history() {
        let mut session = StackerSession::new();
        assert!(session.is_empty());
        assert!(!session.can_undo());
        assert!(!session.can_redo());

        session.insert_text(StackerSelection::collapsed(0), "draft");
        assert!(!session.is_empty());
        assert!(session.can_undo());
        assert!(!session.can_redo());

        assert!(session.undo());
        assert!(!session.can_undo());
        assert!(session.can_redo());

        assert!(session.redo());
        assert!(session.can_undo());
        assert!(!session.can_redo());
    }

    #[test]
    fn undo_restores_operation_selection_and_redo_restores_post_edit_selection() {
        let mut session = StackerSession::new();
        session.set_text("hello world");
        session.set_selection(StackerSelection::collapsed(0));

        session.replace_range(
            StackerSelection { start: 6, end: 11 },
            "llnzy",
            StackerSelection::collapsed(11),
        );

        assert_eq!(session.text(), "hello llnzy");
        assert_eq!(session.selection(), StackerSelection::collapsed(11));

        assert!(session.undo());
        assert_eq!(session.text(), "hello world");
        assert_eq!(session.selection(), StackerSelection { start: 6, end: 11 });

        assert!(session.redo());
        assert_eq!(session.text(), "hello llnzy");
        assert_eq!(session.selection(), StackerSelection::collapsed(11));
    }

    #[test]
    fn replace_selection_uses_current_selection() {
        let mut session = StackerSession::new();
        session.set_text("hello world");
        session.set_selection(StackerSelection { start: 6, end: 11 });

        let outcome = session.replace_selection("llnzy");

        assert!(outcome.changed);
        assert_eq!(outcome.cursor, 11);
        assert_eq!(session.text(), "hello llnzy");
        assert_eq!(session.selection(), StackerSelection::collapsed(11));
    }

    #[test]
    fn delete_backward_at_doc_start_is_noop() {
        let mut session = StackerSession::new();
        session.set_text("abc");
        session.set_selection(StackerSelection::collapsed(0));

        let outcome = session.delete_backward(StackerSelection::collapsed(0));

        assert!(!outcome.changed);
        assert_eq!(session.text(), "abc");
        assert!(!session.can_undo());
    }

    #[test]
    fn delete_forward_at_doc_end_is_noop() {
        let mut session = StackerSession::new();
        session.set_text("abc");
        let total = session.char_count();
        session.set_selection(StackerSelection::collapsed(total));

        let outcome = session.delete_forward(StackerSelection::collapsed(total));

        assert!(!outcome.changed);
        assert_eq!(session.text(), "abc");
    }

    #[test]
    fn selected_text_returns_substring() {
        let mut session = StackerSession::new();
        session.set_text("hello world");

        let selected = session.selected_text(StackerSelection { start: 6, end: 11 });
        assert_eq!(selected.as_deref(), Some("world"));
    }

    #[test]
    fn select_all_spans_full_text() {
        let mut session = StackerSession::new();
        session.set_text("abcdef");

        let sel = session.select_all();
        assert_eq!(sel, StackerSelection { start: 0, end: 6 });
        assert_eq!(session.selection(), sel);
    }

    #[test]
    fn insert_normalizes_crlf_to_lf() {
        let mut session = StackerSession::new();
        session.insert_text(StackerSelection::collapsed(0), "one\r\ntwo\rthree");
        assert_eq!(session.text(), "one\ntwo\nthree");
    }

    #[test]
    fn unicode_selection_round_trips_via_undo() {
        let mut session = StackerSession::new();
        session.set_text("héllo wörld");
        session.set_selection(StackerSelection { start: 6, end: 11 });

        session.replace_range(
            StackerSelection { start: 6, end: 11 },
            "llnzy",
            StackerSelection::collapsed(11),
        );

        assert_eq!(session.text(), "héllo llnzy");
        assert!(session.undo());
        assert_eq!(session.text(), "héllo wörld");
        assert_eq!(session.selection(), StackerSelection { start: 6, end: 11 });
    }

    #[test]
    fn delete_backward_with_selection_records_undo() {
        let mut session = StackerSession::new();
        session.set_text("hello world");
        session.set_selection(StackerSelection { start: 6, end: 11 });

        let outcome = session.delete_backward(StackerSelection { start: 6, end: 11 });

        assert!(outcome.changed);
        assert_eq!(session.text(), "hello ");
        assert_eq!(session.selection(), StackerSelection::collapsed(6));

        assert!(session.undo());
        assert_eq!(session.text(), "hello world");
        assert_eq!(session.selection(), StackerSelection { start: 6, end: 11 });
    }

    #[test]
    fn fresh_session_has_no_marked_range() {
        let session = StackerSession::new();
        assert!(session.marked_range().is_none());
    }

    #[test]
    fn set_marked_text_at_collapsed_cursor_inserts_and_marks() {
        let mut session = StackerSession::new();
        session.set_text("hello ");
        session.set_selection(StackerSelection::collapsed(6));

        // First setMarkedText: no replacement_range, no existing marked
        // range → composition replaces the current selection.
        session.set_marked_text("wo", StackerSelection::collapsed(2), None);

        assert_eq!(session.text(), "hello wo");
        assert_eq!(
            session.marked_range(),
            Some(StackerSelection { start: 6, end: 8 })
        );
        assert_eq!(session.selection(), StackerSelection::collapsed(8));
    }

    #[test]
    fn second_set_marked_text_replaces_previous_marked_content() {
        let mut session = StackerSession::new();
        session.set_text("hello ");
        session.set_selection(StackerSelection::collapsed(6));
        session.set_marked_text("wo", StackerSelection::collapsed(2), None);
        assert_eq!(session.text(), "hello wo");

        // IME refines composition: replace marked "wo" with "wor".
        session.set_marked_text("wor", StackerSelection::collapsed(3), None);

        assert_eq!(session.text(), "hello wor");
        assert_eq!(
            session.marked_range(),
            Some(StackerSelection { start: 6, end: 9 })
        );
        assert_eq!(session.selection(), StackerSelection::collapsed(9));
    }

    #[test]
    fn unmark_text_commits_composition_in_place() {
        let mut session = StackerSession::new();
        session.set_text("hello ");
        session.set_selection(StackerSelection::collapsed(6));
        session.set_marked_text("world", StackerSelection::collapsed(5), None);
        assert!(session.marked_range().is_some());

        session.unmark_text();

        assert_eq!(session.text(), "hello world");
        assert!(session.marked_range().is_none());
        assert_eq!(session.selection(), StackerSelection::collapsed(11));
    }

    #[test]
    fn set_marked_text_with_replacement_range_overrides_when_unmarked() {
        let mut session = StackerSession::new();
        session.set_text("hello world");
        session.set_selection(StackerSelection::collapsed(0));

        // Insert composition over the explicit range "world".
        session.set_marked_text(
            "llnzy",
            StackerSelection::collapsed(5),
            Some(StackerSelection { start: 6, end: 11 }),
        );

        assert_eq!(session.text(), "hello llnzy");
        assert_eq!(
            session.marked_range(),
            Some(StackerSelection { start: 6, end: 11 })
        );
    }

    #[test]
    fn empty_set_marked_text_clears_marked_range() {
        let mut session = StackerSession::new();
        session.set_text("hello ");
        session.set_selection(StackerSelection::collapsed(6));
        session.set_marked_text("wo", StackerSelection::collapsed(2), None);
        assert_eq!(session.text(), "hello wo");

        // IME aborts composition: setMarkedText: with empty text replaces
        // the marked range with nothing, leaving an unmarked document.
        session.set_marked_text("", StackerSelection::collapsed(0), None);

        assert_eq!(session.text(), "hello ");
        assert!(session.marked_range().is_none());
    }

    #[test]
    fn set_text_clears_marked_range() {
        let mut session = StackerSession::new();
        session.set_text("hello ");
        session.set_selection(StackerSelection::collapsed(6));
        session.set_marked_text("wo", StackerSelection::collapsed(2), None);
        assert!(session.marked_range().is_some());

        session.set_text("brand new");

        assert!(session.marked_range().is_none());
    }

    #[test]
    fn marked_range_tracks_internal_selection_within_composition() {
        let mut session = StackerSession::new();
        session.set_text("");
        session.set_selection(StackerSelection::collapsed(0));

        // Composition with the cursor in the middle of the marked text.
        session.set_marked_text("héllo", StackerSelection::collapsed(2), None);

        assert_eq!(session.text(), "héllo");
        assert_eq!(
            session.marked_range(),
            Some(StackerSelection { start: 0, end: 5 })
        );
        assert_eq!(session.selection(), StackerSelection::collapsed(2));
    }

    #[test]
    fn sync_to_view_mirrors_collapsed_selection() {
        use crate::editor::BufferView;
        let mut session = StackerSession::new();
        session.set_text("hello\nworld");
        session.set_selection(StackerSelection::collapsed(8));

        let mut view = BufferView::default();
        session.sync_to_view(&mut view);

        assert_eq!(view.cursor.pos.line, 1);
        assert_eq!(view.cursor.pos.col, 2);
        assert!(view.cursor.anchor.is_none());
    }

    #[test]
    fn sync_to_view_mirrors_range_selection() {
        use crate::editor::BufferView;
        let mut session = StackerSession::new();
        session.set_text("hello\nworld");
        session.set_selection(StackerSelection { start: 2, end: 8 });

        let mut view = BufferView::default();
        session.sync_to_view(&mut view);

        assert_eq!(view.cursor.pos.line, 1);
        assert_eq!(view.cursor.pos.col, 2);
        let anchor = view.cursor.anchor.expect("anchor present for range");
        assert_eq!(anchor.line, 0);
        assert_eq!(anchor.col, 2);
    }

    #[test]
    fn sync_from_view_writes_collapsed_back() {
        use crate::editor::buffer::Position;
        use crate::editor::BufferView;
        let mut session = StackerSession::new();
        session.set_text("hello\nworld");
        session.set_selection(StackerSelection::collapsed(0));

        let mut view = BufferView::default();
        view.cursor.pos = Position::new(1, 3);
        view.cursor.anchor = None;

        session.sync_from_view(&view);
        assert_eq!(session.selection(), StackerSelection::collapsed(9));
    }

    #[test]
    fn sync_from_view_writes_range_back() {
        use crate::editor::buffer::Position;
        use crate::editor::BufferView;
        let mut session = StackerSession::new();
        session.set_text("hello\nworld");
        session.set_selection(StackerSelection::collapsed(0));

        let mut view = BufferView::default();
        view.cursor.pos = Position::new(1, 3);
        view.cursor.anchor = Some(Position::new(0, 2));

        session.sync_from_view(&view);
        assert_eq!(session.selection(), StackerSelection { start: 2, end: 9 });
    }

    #[test]
    fn sync_round_trip_is_stable() {
        use crate::editor::BufferView;
        let mut session = StackerSession::new();
        session.set_text("first line\nsecond line\nthird");
        session.set_selection(StackerSelection { start: 6, end: 18 });

        let mut view = BufferView::default();
        session.sync_to_view(&mut view);
        // No mutation; reading back should be a no-op.
        let before = session.selection();
        session.sync_from_view(&view);
        assert_eq!(session.selection(), before);
    }

    #[test]
    fn insert_via_session_then_sync_places_view_cursor_after_text() {
        use crate::editor::BufferView;
        let mut session = StackerSession::new();
        session.insert_text(StackerSelection::collapsed(0), "hello\nwor");
        // Session selection should be collapsed at end of inserted text (9 chars).
        assert_eq!(session.selection(), StackerSelection::collapsed(9));

        let mut view = BufferView::default();
        session.sync_to_view(&mut view);
        assert_eq!(view.cursor.pos.line, 1);
        assert_eq!(view.cursor.pos.col, 3);
        assert!(view.cursor.anchor.is_none());
    }

    #[test]
    fn delete_via_session_then_sync_moves_view_cursor_back() {
        use crate::editor::BufferView;
        let mut session = StackerSession::new();
        session.set_text("hello\nworld");
        session.set_selection(StackerSelection::collapsed(8));

        session.delete_backward(session.selection());
        assert_eq!(session.selection(), StackerSelection::collapsed(7));

        let mut view = BufferView::default();
        session.sync_to_view(&mut view);
        assert_eq!(view.cursor.pos.line, 1);
        assert_eq!(view.cursor.pos.col, 1);
    }

    #[test]
    fn mouse_drag_simulated_on_view_then_sync_updates_session() {
        use crate::editor::buffer::Position;
        use crate::editor::BufferView;
        let mut session = StackerSession::new();
        session.set_text("first line\nsecond line\nthird");
        session.set_selection(StackerSelection::collapsed(0));

        // Simulate a drag selection: click at (0, 6), drag to (1, 6).
        let mut view = BufferView::default();
        view.cursor.anchor = Some(Position::new(0, 6));
        view.cursor.pos = Position::new(1, 6);

        session.sync_from_view(&view);
        // 0,6 = char 6 ; 1,6 = char 11 (newline) + 6 = char 17.
        assert_eq!(session.selection(), StackerSelection { start: 6, end: 17 });
    }

    #[test]
    fn sync_round_trip_through_session_mutation_preserves_selection() {
        use crate::editor::BufferView;
        let mut session = StackerSession::new();
        session.insert_text(StackerSelection::collapsed(0), "abcdef");

        let mut view = BufferView::default();
        session.sync_to_view(&mut view);
        let view_pos_before = view.cursor.pos;

        // No view-side change; sync back must not move the session selection.
        session.sync_from_view(&view);
        assert_eq!(session.selection(), StackerSelection::collapsed(6));

        // And re-sync to view yields the same view position.
        session.sync_to_view(&mut view);
        assert_eq!(view.cursor.pos, view_pos_before);
    }

    #[test]
    fn sync_to_view_clears_extra_cursors_and_desired_col() {
        use crate::editor::buffer::Position;
        use crate::editor::cursor::CursorRange;
        use crate::editor::BufferView;
        let mut session = StackerSession::new();
        session.set_text("hello");
        session.set_selection(StackerSelection::collapsed(3));

        let mut view = BufferView::default();
        view.cursor.desired_col = Some(40);
        view.cursor.extra_cursors.push(CursorRange {
            pos: Position::new(0, 0),
            anchor: None,
        });

        session.sync_to_view(&mut view);
        assert_eq!(view.cursor.desired_col, None);
        assert!(view.cursor.extra_cursors.is_empty());
    }
}
