use crate::error_log::{ErrorLog, ErrorPanel};
use crate::layout::ScreenLayout;
use crate::session::{Rect as PaneRect, Session};

pub type EguiRenderCallback<'a> =
    &'a mut dyn FnMut(&wgpu::Device, &wgpu::Queue, &wgpu::TextureView, egui_wgpu::ScreenDescriptor);

#[derive(Clone, Copy)]
pub struct TerminalPane<'a> {
    pub terminal: &'a Session,
    pub tab_id: u64,
    pub rect: PaneRect,
    pub active: bool,
}

pub struct RenderRequest<'a> {
    /// The terminal session to render, if the active tab is a terminal.
    pub terminal: Option<&'a Session>,
    /// Unique ID for the active tab (used for text cache management).
    pub tab_id: u64,
    /// Optional explicit terminal panes for joined-tab rendering.
    pub terminal_panes: &'a [TerminalPane<'a>],
    pub tab_titles: &'a [(String, bool)],
    pub selection_rects: &'a [(f32, f32, f32, f32, [f32; 4])],
    pub search_rects: &'a [(f32, f32, f32, f32, [f32; 4])],
    pub search_bar: Option<(&'a str, &'a str)>,
    pub error_panel: Option<(&'a ErrorPanel, &'a ErrorLog)>,
    pub visual_bell: bool,
    pub screen_layout: &'a ScreenLayout,
    pub egui_render: Option<EguiRenderCallback<'a>>,
    /// When false, saved effect settings remain intact but the frame renders clean.
    pub effects_enabled: bool,
    /// When true, egui renders to the scene texture so post-processing
    /// shaders (bloom, CRT) affect the active UI view.
    pub apply_effects_to_ui: bool,
    /// UV rect [left, top, right, bottom] restricting CRT effects.
    /// `None` means fullscreen (no masking).
    pub effects_mask: Option<[f32; 4]>,
}
