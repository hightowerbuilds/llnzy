use super::stacker_view;
use super::types::CopyGhost;
use crate::stacker::{
    document::StackerDocumentEditor,
    draft::StackerDraft,
    load_stacker_prompts, load_stacker_queue,
    queue::{self, QueuedPrompt},
    save_stacker_prompts, save_stacker_queue, StackerPrompt,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PendingStackerDraftSwitch {
    Scratch,
    SavedPrompt(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StackerPromptViewMode {
    List,
    Thumbnails,
}

impl StackerPromptViewMode {
    pub fn toggle(&mut self) {
        *self = match self {
            Self::List => Self::Thumbnails,
            Self::Thumbnails => Self::List,
        };
    }

    pub fn toggle_label(self) -> &'static str {
        match self {
            Self::List => "Thumbnails",
            Self::Thumbnails => "List",
        }
    }
}

pub struct StackerUiState {
    pub prompts: Vec<StackerPrompt>,
    pub editor: StackerDocumentEditor,
    pub draft: StackerDraft,
    pub pending_draft_switch: Option<PendingStackerDraftSwitch>,
    pub pending_prompt_delete: Option<usize>,
    pub editing: Option<usize>,
    pub edit_text: String,
    pub dirty: bool,
    pub copy_ghosts: Vec<CopyGhost>,
    pub editor_font_size: f32,
    pub web_editor_rect: Option<egui::Rect>,
    pub queued_prompts: Vec<QueuedPrompt>,
    pub prompt_view_mode: StackerPromptViewMode,
    last_persisted_queue: Vec<QueuedPrompt>,
}

impl Default for StackerUiState {
    fn default() -> Self {
        Self {
            prompts: Vec::new(),
            editor: StackerDocumentEditor::new(),
            draft: StackerDraft::new(),
            pending_draft_switch: None,
            pending_prompt_delete: None,
            editing: None,
            edit_text: String::new(),
            dirty: false,
            copy_ghosts: Vec::new(),
            editor_font_size: stacker_view::DEFAULT_EDITOR_FONT_SIZE,
            web_editor_rect: None,
            queued_prompts: Vec::new(),
            prompt_view_mode: StackerPromptViewMode::List,
            last_persisted_queue: Vec::new(),
        }
    }
}

impl StackerUiState {
    pub fn load() -> Self {
        let mut queued_prompts = load_stacker_queue();
        queue::sanitize_prompt_queue(&mut queued_prompts);
        Self {
            prompts: load_stacker_prompts(),
            last_persisted_queue: queued_prompts.clone(),
            queued_prompts,
            ..Default::default()
        }
    }

    pub fn persist_if_dirty(&mut self) {
        if self.dirty {
            save_stacker_prompts(&self.prompts);
            self.dirty = false;
        }
        queue::sanitize_prompt_queue(&mut self.queued_prompts);
        if self.queued_prompts != self.last_persisted_queue {
            save_stacker_queue(&self.queued_prompts);
            self.last_persisted_queue = self.queued_prompts.clone();
        }
    }
}
