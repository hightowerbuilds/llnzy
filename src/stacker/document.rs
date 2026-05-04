use super::input::{StackerEditOutcome, StackerInputEngine, StackerSelection};

const MAX_HISTORY: usize = 100;

#[derive(Clone, Debug, PartialEq, Eq)]
struct StackerDocumentSnapshot {
    text: String,
    selection: StackerSelection,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StackerDocumentEditor {
    text: String,
    selection: StackerSelection,
    undo_stack: Vec<StackerDocumentSnapshot>,
    redo_stack: Vec<StackerDocumentSnapshot>,
}

impl StackerDocumentEditor {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            selection: StackerSelection::collapsed(0),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn text_mut_for_widget(&mut self) -> &mut String {
        &mut self.text
    }

    pub fn char_count(&self) -> usize {
        self.text.chars().count()
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn selection(&self) -> StackerSelection {
        self.selection
    }

    pub fn set_selection(&mut self, selection: StackerSelection) {
        self.selection = self.clamp_selection(selection);
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.selection = StackerSelection::collapsed(self.char_count());
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn clear(&mut self) {
        self.set_text(String::new());
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn replace_selection(&mut self, replacement: &str) -> StackerEditOutcome {
        self.insert_text(self.selection, replacement)
    }

    pub fn insert_text(&mut self, selection: StackerSelection, text: &str) -> StackerEditOutcome {
        let before = self.snapshot_with_selection(selection);
        let outcome = StackerInputEngine::insert_text(&mut self.text, selection, text);
        if outcome.changed {
            self.commit_edit(before);
        }
        self.selection = StackerSelection::collapsed(outcome.cursor);
        outcome
    }

    pub fn replace_range(
        &mut self,
        selection: StackerSelection,
        replacement: &str,
        next_selection: StackerSelection,
    ) -> StackerEditOutcome {
        let before = self.snapshot_with_selection(selection);
        let outcome = StackerInputEngine::insert_text(&mut self.text, selection, replacement);
        if outcome.changed {
            self.commit_edit(before);
        }
        self.selection = self.clamp_selection(next_selection);
        StackerEditOutcome {
            cursor: self.selection.end,
            changed: outcome.changed,
        }
    }

    pub fn replace_all_with_history(
        &mut self,
        text: String,
        next_selection: StackerSelection,
    ) -> bool {
        if self.text == text {
            self.selection = self.clamp_selection(next_selection);
            return false;
        }
        let before = self.snapshot();
        self.text = text;
        self.selection = self.clamp_selection(next_selection);
        self.commit_edit(before);
        true
    }

    pub fn record_widget_change(
        &mut self,
        before_text: String,
        before_selection: StackerSelection,
        next_selection: StackerSelection,
    ) -> bool {
        let changed = self.text != before_text;
        if changed {
            self.push_undo(StackerDocumentSnapshot {
                selection: self.clamp_text_selection(before_selection, before_text.as_str()),
                text: before_text,
            });
            self.redo_stack.clear();
        }
        self.selection = self.clamp_selection(next_selection);
        changed
    }

    pub fn delete_backward(&mut self, selection: StackerSelection) -> StackerEditOutcome {
        let before = self.snapshot_with_selection(selection);
        let outcome = StackerInputEngine::delete_backward(&mut self.text, selection);
        if outcome.changed {
            self.commit_edit(before);
        }
        self.selection = StackerSelection::collapsed(outcome.cursor);
        outcome
    }

    pub fn delete_forward(&mut self, selection: StackerSelection) -> StackerEditOutcome {
        let before = self.snapshot_with_selection(selection);
        let outcome = StackerInputEngine::delete_forward(&mut self.text, selection);
        if outcome.changed {
            self.commit_edit(before);
        }
        self.selection = StackerSelection::collapsed(outcome.cursor);
        outcome
    }

    pub fn selected_text(&self, selection: StackerSelection) -> Option<String> {
        StackerInputEngine::selected_text(&self.text, selection)
    }

    pub fn select_all(&mut self) -> StackerSelection {
        let selection = StackerInputEngine::select_all(&self.text);
        self.selection = selection;
        selection
    }

    pub fn undo(&mut self) -> bool {
        let Some(snapshot) = self.undo_stack.pop() else {
            return false;
        };
        let current = self.snapshot();
        self.push_redo(current);
        self.restore(snapshot);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(snapshot) = self.redo_stack.pop() else {
            return false;
        };
        let current = self.snapshot();
        self.push_undo(current);
        self.restore(snapshot);
        true
    }

    fn snapshot(&self) -> StackerDocumentSnapshot {
        StackerDocumentSnapshot {
            text: self.text.clone(),
            selection: self.selection,
        }
    }

    fn snapshot_with_selection(&self, selection: StackerSelection) -> StackerDocumentSnapshot {
        StackerDocumentSnapshot {
            text: self.text.clone(),
            selection: self.clamp_selection(selection),
        }
    }

    fn restore(&mut self, snapshot: StackerDocumentSnapshot) {
        self.text = snapshot.text;
        self.selection = self.clamp_selection(snapshot.selection);
    }

    fn commit_edit(&mut self, before: StackerDocumentSnapshot) {
        self.push_undo(before);
        self.redo_stack.clear();
    }

    fn push_undo(&mut self, snapshot: StackerDocumentSnapshot) {
        self.undo_stack.push(snapshot);
        if self.undo_stack.len() > MAX_HISTORY {
            self.undo_stack.remove(0);
        }
    }

    fn push_redo(&mut self, snapshot: StackerDocumentSnapshot) {
        self.redo_stack.push(snapshot);
        if self.redo_stack.len() > MAX_HISTORY {
            self.redo_stack.remove(0);
        }
    }

    fn clamp_selection(&self, selection: StackerSelection) -> StackerSelection {
        self.clamp_text_selection(selection, &self.text)
    }

    fn clamp_text_selection(&self, selection: StackerSelection, text: &str) -> StackerSelection {
        let char_count = text.chars().count();
        StackerSelection {
            start: selection.start.min(char_count),
            end: selection.end.min(char_count),
        }
    }
}

impl Default for StackerDocumentEditor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_records_undo_and_redo() {
        let mut editor = StackerDocumentEditor::new();

        editor.insert_text(StackerSelection::collapsed(0), "hello");
        assert_eq!(editor.text(), "hello");

        assert!(editor.undo());
        assert_eq!(editor.text(), "");

        assert!(editor.redo());
        assert_eq!(editor.text(), "hello");
    }

    #[test]
    fn set_text_resets_history() {
        let mut editor = StackerDocumentEditor::new();
        editor.insert_text(StackerSelection::collapsed(0), "draft");
        editor.set_text("saved prompt");

        assert!(!editor.undo());
        assert_eq!(editor.text(), "saved prompt");
        assert_eq!(
            editor.selection(),
            StackerSelection::collapsed("saved prompt".chars().count())
        );
    }

    #[test]
    fn replace_all_records_single_history_entry() {
        let mut editor = StackerDocumentEditor::new();
        editor.set_text("one");

        assert!(
            editor.replace_all_with_history("one two".to_string(), StackerSelection::collapsed(7))
        );
        assert_eq!(editor.text(), "one two");

        assert!(editor.undo());
        assert_eq!(editor.text(), "one");
    }

    #[test]
    fn undo_redo_availability_tracks_history() {
        let mut editor = StackerDocumentEditor::new();
        assert!(editor.is_empty());
        assert!(!editor.can_undo());
        assert!(!editor.can_redo());

        editor.insert_text(StackerSelection::collapsed(0), "draft");
        assert!(!editor.is_empty());
        assert!(editor.can_undo());
        assert!(!editor.can_redo());

        assert!(editor.undo());
        assert!(!editor.can_undo());
        assert!(editor.can_redo());

        assert!(editor.redo());
        assert!(editor.can_undo());
        assert!(!editor.can_redo());
    }

    #[test]
    fn undo_restores_operation_selection_and_redo_restores_post_edit_selection() {
        let mut editor = StackerDocumentEditor::new();
        editor.set_text("hello world");
        editor.set_selection(StackerSelection::collapsed(0));

        editor.replace_range(
            StackerSelection { start: 6, end: 11 },
            "llnzy",
            StackerSelection::collapsed(11),
        );

        assert_eq!(editor.text(), "hello llnzy");
        assert_eq!(editor.selection(), StackerSelection::collapsed(11));

        assert!(editor.undo());
        assert_eq!(editor.text(), "hello world");
        assert_eq!(editor.selection(), StackerSelection { start: 6, end: 11 });

        assert!(editor.redo());
        assert_eq!(editor.text(), "hello llnzy");
        assert_eq!(editor.selection(), StackerSelection::collapsed(11));
    }

    #[test]
    fn replace_selection_uses_current_selection() {
        let mut editor = StackerDocumentEditor::new();
        editor.set_text("hello world");
        editor.set_selection(StackerSelection { start: 6, end: 11 });

        let outcome = editor.replace_selection("llnzy");

        assert!(outcome.changed);
        assert_eq!(outcome.cursor, 11);
        assert_eq!(editor.text(), "hello llnzy");
        assert_eq!(editor.selection(), StackerSelection::collapsed(11));
    }
}
