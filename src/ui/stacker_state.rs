use super::stacker_view;
use super::types::CopyGhost;
use crate::stacker::{
    load_stacker_prompts,
    queue::{self, QueuedPrompt},
    save_stacker_prompts, StackerPrompt,
};

pub struct StackerUiState {
    pub prompts: Vec<StackerPrompt>,
    pub input: String,
    pub editing: Option<usize>,
    pub edit_text: String,
    pub dirty: bool,
    pub copy_ghosts: Vec<CopyGhost>,
    pub editor_font_size: f32,
    pub queued_prompts: Vec<QueuedPrompt>,
}

impl Default for StackerUiState {
    fn default() -> Self {
        Self {
            prompts: Vec::new(),
            input: String::new(),
            editing: None,
            edit_text: String::new(),
            dirty: false,
            copy_ghosts: Vec::new(),
            editor_font_size: stacker_view::DEFAULT_EDITOR_FONT_SIZE,
            queued_prompts: Vec::new(),
        }
    }
}

impl StackerUiState {
    pub fn load() -> Self {
        Self {
            prompts: load_stacker_prompts(),
            ..Default::default()
        }
    }

    pub fn persist_if_dirty(&mut self) {
        if self.dirty {
            save_stacker_prompts(&self.prompts);
            self.dirty = false;
        }
        queue::sanitize_prompt_queue(&mut self.queued_prompts);
    }
}
