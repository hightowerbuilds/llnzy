use super::stacker_view;
use super::types::CopyGhost;
use crate::stacker::{
    document::StackerDocumentEditor,
    draft::StackerDraft,
    load_inbox_prompts, load_saved_prompts, load_stacker_queue, persist_prompt_library,
    queue::{self, QueuedPrompt},
    save_stacker_queue, StackerPrompt,
};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use winit::event_loop::EventLoopProxy;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PendingStackerDraftSwitch {
    Scratch,
    SavedPrompt(usize),
    InboxPrompt(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PendingStackerPromptDelete {
    Saved(usize),
    Inbox(String),
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
    pub inbox_prompts: Vec<StackerPrompt>,
    pub editor: StackerDocumentEditor,
    pub draft: StackerDraft,
    pub pending_draft_switch: Option<PendingStackerDraftSwitch>,
    pub pending_prompt_delete: Option<PendingStackerPromptDelete>,
    pub editing: Option<usize>,
    pub edit_text: String,
    pub dirty: bool,
    pub copy_ghosts: Vec<CopyGhost>,
    pub editor_font_size: f32,
    pub web_editor_rect: Option<egui::Rect>,
    pub queued_prompts: Vec<QueuedPrompt>,
    pub prompt_view_mode: StackerPromptViewMode,
    last_persisted_prompts: Vec<StackerPrompt>,
    last_persisted_queue: Vec<QueuedPrompt>,
    inbox_watcher: Option<StackerInboxWatcher>,
    inbox_watch_error: Option<String>,
}

impl Default for StackerUiState {
    fn default() -> Self {
        Self {
            prompts: Vec::new(),
            inbox_prompts: Vec::new(),
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
            last_persisted_prompts: Vec::new(),
            last_persisted_queue: Vec::new(),
            inbox_watcher: None,
            inbox_watch_error: None,
        }
    }
}

impl StackerUiState {
    pub fn load() -> Self {
        let prompts = load_saved_prompts();
        let inbox_prompts = load_inbox_prompts();
        let mut queued_prompts = load_stacker_queue();
        queue::sanitize_prompt_queue(&mut queued_prompts);
        Self {
            last_persisted_prompts: prompts.clone(),
            prompts,
            inbox_prompts,
            last_persisted_queue: queued_prompts.clone(),
            queued_prompts,
            ..Default::default()
        }
    }

    pub fn persist_if_dirty(&mut self) {
        if self.dirty {
            persist_prompt_library(&mut self.prompts, &self.last_persisted_prompts);
            self.last_persisted_prompts = self.prompts.clone();
            self.dirty = false;
        }
        queue::sanitize_prompt_queue(&mut self.queued_prompts);
        if self.queued_prompts != self.last_persisted_queue {
            save_stacker_queue(&self.queued_prompts);
            self.last_persisted_queue = self.queued_prompts.clone();
        }
    }

    pub fn refresh_inbox(&mut self) {
        self.inbox_prompts = load_inbox_prompts();
        if let Some(id) = self.draft.active_inbox_id() {
            if !self
                .inbox_prompts
                .iter()
                .any(|prompt| prompt.id.as_deref() == Some(id))
            {
                self.draft.start_scratch();
                self.editor.clear();
            }
        }
    }

    pub fn ensure_inbox_watcher(&mut self, proxy: EventLoopProxy<crate::UserEvent>) {
        let Some(paths) = crate::platform::paths::current_paths() else {
            return;
        };
        let inbox_dir = paths.prompts_inbox_dir();
        if self
            .inbox_watcher
            .as_ref()
            .is_some_and(|watcher| watcher.matches_dir(&inbox_dir))
        {
            return;
        }
        match StackerInboxWatcher::new(inbox_dir, proxy) {
            Ok(watcher) => {
                self.inbox_watcher = Some(watcher);
                self.inbox_watch_error = None;
            }
            Err(error) => {
                self.inbox_watcher = None;
                self.inbox_watch_error = Some(error.clone());
                log::warn!("Failed to watch Stacker inbox: {error}");
            }
        }
    }

    pub fn poll_inbox_watcher(&mut self) -> bool {
        let changed = self
            .inbox_watcher
            .as_mut()
            .is_some_and(StackerInboxWatcher::poll);
        if changed {
            self.refresh_inbox();
        }
        changed
    }
}

struct StackerInboxWatcher {
    _watcher: RecommendedWatcher,
    event_rx: Receiver<notify::Result<Event>>,
    inbox_dir: PathBuf,
}

impl StackerInboxWatcher {
    fn new(inbox_dir: PathBuf, proxy: EventLoopProxy<crate::UserEvent>) -> Result<Self, String> {
        std::fs::create_dir_all(&inbox_dir)
            .map_err(|err| format!("Failed to create {}: {err}", inbox_dir.display()))?;
        let (tx, rx) = mpsc::channel();
        let wake_path = inbox_dir.clone();
        let mut watcher = notify::recommended_watcher(move |event| {
            let _ = tx.send(event);
            let _ = proxy.send_event(crate::UserEvent::FileChanged(wake_path.clone()));
        })
        .map_err(|err| format!("Failed to create inbox watcher: {err}"))?;
        watcher
            .watch(&inbox_dir, RecursiveMode::NonRecursive)
            .map_err(|err| format!("Failed to watch {}: {err}", inbox_dir.display()))?;
        Ok(Self {
            _watcher: watcher,
            event_rx: rx,
            inbox_dir,
        })
    }

    fn matches_dir(&self, dir: &std::path::Path) -> bool {
        self.inbox_dir == dir
    }

    fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                Ok(event) => {
                    if event
                        .paths
                        .iter()
                        .any(|path| path.extension().and_then(|s| s.to_str()) == Some("md"))
                    {
                        changed = true;
                    }
                }
                Err(err) => log::warn!("Stacker inbox watcher error: {err}"),
            }
        }
        changed
    }
}
