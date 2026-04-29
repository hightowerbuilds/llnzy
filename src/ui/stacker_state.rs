use super::stacker_view;
use super::types::{ActiveView, CopyGhost};
use crate::stacker::{load_stacker_prompts, save_stacker_prompts, StackerPrompt};
use crate::workspace::TabKind;

#[derive(Default)]
pub struct StackerUiState {
    pub prompts: Vec<StackerPrompt>,
    pub input: String,
    pub category_input: String,
    pub search: String,
    pub filter_category: String,
    pub editing: Option<usize>,
    pub edit_text: String,
    pub dirty: bool,
    pub prompt_bar_visible: bool,
    pub prompt_bar_views: u8,
    pub copy_ghosts: Vec<CopyGhost>,
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
    }

    pub fn render_prompt_bar(
        &self,
        ctx: &egui::Context,
        current_view: ActiveView,
        active_tab_kind: Option<TabKind>,
    ) -> Option<String> {
        if !self.prompt_bar_visible || self.prompts.is_empty() || current_view == ActiveView::Home {
            return None;
        }

        let show_bar = match active_tab_kind {
            Some(TabKind::Terminal) => self.prompt_bar_views & stacker_view::BAR_VIEW_SHELL != 0,
            Some(TabKind::CodeFile) => self.prompt_bar_views & stacker_view::BAR_VIEW_EDITOR != 0,
            _ => false,
        };
        if show_bar {
            stacker_view::render_prompt_bar(ctx, &self.prompts)
        } else {
            None
        }
    }
}
