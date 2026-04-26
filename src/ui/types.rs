use std::time::Instant;

pub const SIDEBAR_WIDTH: f32 = 200.0;

/// Which view is active.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveView {
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
