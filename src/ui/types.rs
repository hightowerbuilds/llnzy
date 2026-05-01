use std::time::Instant;

use crate::app::commands::AppCommand;
use crate::workspace::TabKind;

pub const SIDEBAR_WIDTH: f32 = 200.0;
pub const BUMPER_WIDTH: f32 = 20.0;

/// Which view is active.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveView {
    Home,
    Shells,
    Explorer,
    Stacker,
    Sketch,
    Appearances,
    Settings,
}

/// Which settings tab is active.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsTab {
    Themes,
    Background,
    Text,
    Editor,
    Workspace,
}

/// A ghost-text animation that floats up and fades when a prompt is copied.
pub struct CopyGhost {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub created: Instant,
}

pub(crate) const GHOST_DURATION_SECS: f32 = 0.9;
pub(crate) const GHOST_FLOAT_PX: f32 = 50.0;

/// A pending close confirmation for unsaved buffers.
pub enum PendingClose {
    /// Asking about a single tab (tab_index, file_name).
    Tab(usize, String),
    /// Asking about window close (list of modified tab indices and file names).
    Window(Vec<(usize, String)>),
}

/// Response from the save prompt dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SavePromptResponse {
    Save,
    DontSave,
    Cancel,
}

#[derive(Default)]
pub struct UiFrameOutput {
    pub commands: Vec<AppCommand>,
    pub save_prompt_response: Option<SavePromptResponse>,
}

#[derive(Clone)]
pub struct UiTabInfo {
    pub title: String,
    pub kind: TabKind,
    pub exited: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiTabPaneInfo {
    pub kind: TabKind,
    pub buffer_idx: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JoinedTabs {
    pub primary: usize,
    pub secondary: usize,
    pub ratio: f32,
}

impl JoinedTabs {
    pub const MIN_RATIO: f32 = 0.18;
    pub const MAX_RATIO: f32 = 0.82;

    pub fn new(primary: usize, secondary: usize) -> Self {
        Self {
            primary,
            secondary,
            ratio: 0.5,
        }
    }

    pub fn contains(self, idx: usize) -> bool {
        self.primary == idx || self.secondary == idx
    }

    pub fn clamped(self) -> Self {
        Self {
            ratio: self.ratio.clamp(Self::MIN_RATIO, Self::MAX_RATIO),
            ..self
        }
    }
}
