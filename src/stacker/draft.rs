#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StackerDraftSource {
    Scratch,
    SavedPrompt(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StackerDraft {
    source: StackerDraftSource,
    original_text: String,
    current_text: String,
}

impl StackerDraft {
    pub fn new() -> Self {
        Self {
            source: StackerDraftSource::Scratch,
            original_text: String::new(),
            current_text: String::new(),
        }
    }

    pub fn start_scratch(&mut self) {
        self.source = StackerDraftSource::Scratch;
        self.original_text.clear();
        self.current_text.clear();
    }

    pub fn load_saved_prompt(&mut self, index: usize, text: impl Into<String>) {
        let text = text.into();
        self.source = StackerDraftSource::SavedPrompt(index);
        self.original_text = text.clone();
        self.current_text = text;
    }

    pub fn record_current_text(&mut self, text: impl Into<String>) -> bool {
        let text = text.into();
        let changed = self.current_text != text;
        self.current_text = text;
        changed
    }

    pub fn is_dirty(&self) -> bool {
        self.current_text != self.original_text
    }

    pub fn is_scratch(&self) -> bool {
        self.source == StackerDraftSource::Scratch
    }

    pub fn active_prompt_index(&self) -> Option<usize> {
        match self.source {
            StackerDraftSource::Scratch => None,
            StackerDraftSource::SavedPrompt(index) => Some(index),
        }
    }

    pub fn source(&self) -> &StackerDraftSource {
        &self.source
    }

    pub fn original_text(&self) -> &str {
        &self.original_text
    }

    pub fn current_text(&self) -> &str {
        &self.current_text
    }

    pub fn switching_to_saved_prompt_would_discard_changes(&self, index: usize) -> bool {
        self.is_dirty() && self.source != StackerDraftSource::SavedPrompt(index)
    }

    pub fn switching_to_scratch_would_discard_changes(&self) -> bool {
        self.is_dirty() && self.source != StackerDraftSource::Scratch
    }

    pub fn shift_saved_prompt_index_after_delete(&mut self, deleted_index: usize) {
        let StackerDraftSource::SavedPrompt(index) = self.source else {
            return;
        };
        if index > deleted_index {
            self.source = StackerDraftSource::SavedPrompt(index - 1);
        }
    }
}

impl Default for StackerDraft {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scratch_dirty_tracks_current_text() {
        let mut draft = StackerDraft::new();

        assert!(draft.is_scratch());
        assert!(!draft.is_dirty());
        assert_eq!(draft.active_prompt_index(), None);

        assert!(draft.record_current_text("new unsaved prompt"));

        assert!(draft.is_dirty());
        assert_eq!(draft.original_text(), "");
        assert_eq!(draft.current_text(), "new unsaved prompt");
    }

    #[test]
    fn switching_saved_prompt_detects_discarded_changes() {
        let mut draft = StackerDraft::new();
        draft.load_saved_prompt(0, "first saved prompt");
        draft.record_current_text("edited first prompt");

        assert!(draft.is_dirty());
        assert!(!draft.switching_to_saved_prompt_would_discard_changes(0));
        assert!(draft.switching_to_saved_prompt_would_discard_changes(1));

        draft.load_saved_prompt(1, "second saved prompt");

        assert!(!draft.is_dirty());
        assert_eq!(draft.active_prompt_index(), Some(1));
        assert_eq!(draft.current_text(), "second saved prompt");
    }

    #[test]
    fn start_scratch_clears_dirty_scratch() {
        let mut draft = StackerDraft::new();
        draft.record_current_text("scratch text");

        assert!(draft.is_dirty());

        draft.start_scratch();

        assert!(draft.is_scratch());
        assert!(!draft.is_dirty());
        assert_eq!(draft.active_prompt_index(), None);
        assert_eq!(draft.current_text(), "");
    }

    #[test]
    fn unchanged_saved_prompt_is_not_dirty() {
        let mut draft = StackerDraft::new();

        draft.load_saved_prompt(3, "saved prompt");

        assert_eq!(draft.active_prompt_index(), Some(3));
        assert_eq!(draft.original_text(), "saved prompt");
        assert_eq!(draft.current_text(), "saved prompt");
        assert!(!draft.is_dirty());
        assert!(!draft.switching_to_saved_prompt_would_discard_changes(4));
        assert!(!draft.switching_to_scratch_would_discard_changes());
    }

    #[test]
    fn deleting_saved_prompt_before_active_prompt_shifts_source_index() {
        let mut draft = StackerDraft::new();
        draft.load_saved_prompt(3, "saved prompt");
        draft.record_current_text("edited saved prompt");

        draft.shift_saved_prompt_index_after_delete(1);

        assert_eq!(draft.active_prompt_index(), Some(2));
        assert!(draft.is_dirty());
        assert_eq!(draft.original_text(), "saved prompt");
        assert_eq!(draft.current_text(), "edited saved prompt");
    }

    #[test]
    fn deleting_saved_prompt_after_active_prompt_keeps_source_index() {
        let mut draft = StackerDraft::new();
        draft.load_saved_prompt(1, "saved prompt");

        draft.shift_saved_prompt_index_after_delete(3);

        assert_eq!(draft.active_prompt_index(), Some(1));
    }
}
