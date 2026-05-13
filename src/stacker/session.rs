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
mod tests;
