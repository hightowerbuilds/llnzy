pub mod command_palette;
mod editor_view;
mod explorer_view;
mod footer;
mod home_view;
mod overlays;
mod settings_tabs;
mod sidebar;
mod sketch_view;
mod stacker_view;
pub mod tab_bar;
pub mod types;

use std::time::Instant;
use winit::window::Window;

use crate::config::Config;
use crate::explorer::ExplorerState;
use crate::sketch::{save_default_document, save_named_sketch, SketchState};
use crate::stacker::{
    apply_prompt_edit, load_stacker_prompts, save_stacker_prompts, StackerPrompt,
};

pub use footer::FooterAction;
pub use types::{ActiveView, CopyGhost, PendingClose, SavePromptResponse, SettingsTab, BUMPER_WIDTH, SIDEBAR_WIDTH};

/// State for the egui-driven UI overlay.
pub struct UiState {
    pub ctx: egui::Context,
    pub winit_state: egui_winit::State,
    pub wgpu_renderer: egui_wgpu::Renderer,
    pub active_view: ActiveView,
    pub settings_tab: SettingsTab,
    /// How much vertical space the footer occupies (for terminal content layout)
    pub footer_height: f32,
    /// Config changes from the settings panel, to be applied by main loop
    pub pending_config: Option<Config>,
    /// Text copied to clipboard by Stacker (main loop applies it)
    pub clipboard_text: Option<String>,
    pub sidebar_open: bool,
    /// Actual sidebar panel width (tracked dynamically for layout).
    pub sidebar_actual_width: f32,
    // Debug overlay
    pub show_fps: bool,
    frame_times: std::collections::VecDeque<f32>,
    pub perf_stats: crate::editor::perf::PerfStats,
    // Stacker state
    pub stacker_prompts: Vec<StackerPrompt>,
    pub stacker_input: String,
    pub stacker_category_input: String,
    pub stacker_search: String,
    pub stacker_filter_category: String, // empty = show all
    pub stacker_editing: Option<usize>,  // index of prompt being edited
    pub stacker_edit_text: String,
    pub stacker_dirty: bool, // needs save to disk
    pub prompt_bar_visible: bool,  // whether the prompt queue bar is shown
    pub prompt_bar_views: u8,      // bit 0 = shell, bit 1 = editor
    pub copy_ghosts: Vec<CopyGhost>,
    // Sketch state
    pub sketch: SketchState,
    // Explorer state
    pub explorer: ExplorerState,
    // Editor view state (cursor, scroll, etc.)
    pub editor_view: explorer_view::EditorViewState,
    // Tab renaming
    pub editing_tab: Option<usize>,
    pub editing_tab_text: String,
    pub saved_tab_name: Option<(usize, String)>, // (tab_index, new_name) to apply after render
    pub last_tab_click: Option<(usize, Instant)>, // (tab_index, time) for double-click detection
    // Sketch canvas rect in physical pixels (for CRT mask)
    pub sketch_canvas_px: Option<[f32; 4]>,
    // Tab context for rendering interaction
    pub tab_count: usize,
    pub active_tab_index: usize,
    // Terminal panel (shown below editor in Explorer view)
    pub terminal_panel_open: bool,
    pub terminal_panel_ratio: f32,
    // Command palette
    pub palette: command_palette::PaletteState,
    pub palette_command: Option<command_palette::CommandId>,
    // Recent projects for Home screen
    pub recent_projects: Vec<std::path::PathBuf>,
    /// Project path to open (set by Home screen, applied by main loop)
    pub open_project: Option<std::path::PathBuf>,
    /// Footer action from the last render (consumed by main loop)
    pub footer_action: Option<FooterAction>,
    /// Active tab kind (set by main loop before render, used by footer highlighting)
    pub active_tab_kind: Option<crate::workspace::TabKind>,
    /// Tab bar action from the last render (consumed by main loop)
    pub tab_bar_action: Option<tab_bar::TabBarAction>,
    /// Workspace tabs reference for egui tab bar rendering (set by main loop before render)
    pub tab_names: Vec<(String, bool)>,
    /// Split view state: (right_tab_index, divider_ratio 0.0-1.0).
    /// When Some, the active tab renders on the left, the split tab on the right.
    pub split_view: Option<(usize, f32)>,
    /// Workspace to launch (set by Home screen or Settings, consumed by main loop).
    pub launch_workspace: Option<crate::workspace_store::SavedWorkspace>,
    /// Pending close confirmation for unsaved buffers.
    pub pending_close: Option<PendingClose>,
    /// Save prompt response from the last render (consumed by main loop).
    pub save_prompt_response: Option<SavePromptResponse>,
}

impl UiState {
    pub fn new(
        window: &Window,
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let ctx = egui::Context::default();

        // Style: dark theme with our terminal aesthetic
        let mut style = egui::Style {
            visuals: egui::Visuals::dark(),
            ..Default::default()
        };
        style.visuals.window_rounding = egui::Rounding::same(4.0);
        style.visuals.button_frame = true;
        ctx.set_style(style);

        let viewport_id = ctx.viewport_id();
        let winit_state = egui_winit::State::new(
            ctx.clone(),
            viewport_id,
            window,
            None, // native_pixels_per_point — auto-detect
            None, // max_texture_side
            None, // max_image_side
        );

        let wgpu_renderer = egui_wgpu::Renderer::new(
            device,
            surface_format,
            None, // depth format
            1,    // sample count
            false,
        );

        let stacker_prompts = load_stacker_prompts();

        UiState {
            ctx,
            winit_state,
            wgpu_renderer,
            active_view: ActiveView::Home,
            settings_tab: SettingsTab::Themes,
            footer_height: 36.0,
            pending_config: None,
            clipboard_text: None,
            sidebar_open: false,
            sidebar_actual_width: SIDEBAR_WIDTH,
            show_fps: false,
            frame_times: std::collections::VecDeque::with_capacity(120),
            perf_stats: crate::editor::perf::PerfStats::default(),
            stacker_prompts,
            stacker_input: String::new(),
            stacker_category_input: String::new(),
            stacker_search: String::new(),
            stacker_filter_category: String::new(),
            stacker_editing: None,
            stacker_edit_text: String::new(),
            stacker_dirty: false,
            prompt_bar_visible: false,
            prompt_bar_views: 0,
            copy_ghosts: Vec::new(),
            sketch: SketchState::load_default(),
            explorer: ExplorerState::new(),
            editor_view: explorer_view::EditorViewState::default(),
            editing_tab: None,
            editing_tab_text: String::new(),
            saved_tab_name: None,
            last_tab_click: None,
            sketch_canvas_px: None,
            tab_count: 0,
            active_tab_index: 0,
            terminal_panel_open: false,
            terminal_panel_ratio: 0.35,
            palette: command_palette::PaletteState::default(),
            palette_command: None,
            recent_projects: crate::explorer::load_recent_projects(),
            open_project: None,
            footer_action: None,
            active_tab_kind: None,
            tab_bar_action: None,
            tab_names: Vec::new(),
            split_view: None,
            launch_workspace: None,
            pending_close: None,
            save_prompt_response: None,
        }
    }

    /// Pass a winit event to egui. Returns true if egui consumed it.
    pub fn handle_event(&mut self, window: &Window, event: &winit::event::WindowEvent) -> bool {
        let response = self.winit_state.on_window_event(window, event);
        response.consumed
    }

    /// Whether the wgpu terminal content is completely covered.
    /// True when an overlay is showing or the active tab is not a terminal.
    pub fn settings_open(&self) -> bool {
        // Overlay views always cover the terminal
        if matches!(self.active_view, ActiveView::Home | ActiveView::Appearances | ActiveView::Settings) {
            return true;
        }
        // Non-terminal tabs cover the terminal content area
        !matches!(self.active_tab_kind, Some(crate::workspace::TabKind::Terminal))
    }

    pub fn toggle_terminal_panel(&mut self) {
        self.terminal_panel_open = !self.terminal_panel_open;
    }

    pub fn captures_terminal_input(&self) -> bool {
        matches!(self.active_tab_kind, Some(crate::workspace::TabKind::Sketch))
    }

    pub fn toggle_sidebar(&mut self) {
        self.sidebar_open = !self.sidebar_open;
    }

    /// Total width consumed by sidebar UI (bumper is always visible).
    pub fn sidebar_width(&self) -> f32 {
        if self.sidebar_open {
            self.sidebar_actual_width + BUMPER_WIDTH
        } else {
            BUMPER_WIDTH
        }
    }

    /// Start editing a tab's name. Initializes the edit state with the current or default name.
    pub fn start_editing_tab(&mut self, tab_index: usize, current_name: Option<&str>) {
        self.editing_tab = Some(tab_index);
        self.editing_tab_text = current_name.unwrap_or("").to_string();
    }

    /// Cancel tab editing without saving.
    pub fn cancel_editing_tab(&mut self) {
        self.editing_tab = None;
        self.editing_tab_text.clear();
    }

    /// Update tab context (called before rendering).
    pub fn set_tab_context(&mut self, tab_count: usize, active_tab_index: usize) {
        self.tab_count = tab_count;
        self.active_tab_index = active_tab_index;
    }

    /// Retrieve and clear any pending tab name change.
    pub fn take_saved_tab_name(&mut self) -> Option<(usize, String)> {
        self.saved_tab_name.take()
    }

    /// Run the egui frame and render. Returns the clipping info for the footer.
    /// Call this AFTER rendering the terminal content to the swapchain.
    /// Take pending config changes, if any.
    pub fn take_config(&mut self) -> Option<Config> {
        self.pending_config.take()
    }

    /// Record a frame time for the FPS overlay.
    pub fn record_frame_time(&mut self, dt: f32) {
        if self.frame_times.len() >= 120 {
            self.frame_times.pop_front();
        }
        self.frame_times.push_back(dt);
    }

    /// Run the egui frame and render to the swapchain.
    pub fn render(
        &mut self,
        window: &Window,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        screen_desc: egui_wgpu::ScreenDescriptor,
        config: &Config,
    ) {
        let raw_input = self.winit_state.take_egui_input(window);

        // Extract state to avoid borrowing self inside the closure
        let current_view = self.active_view;
        let footer_height = self.footer_height;
        let settings_tab = self.settings_tab;
        let mut nav_target: Option<ActiveView> = None;
        let mut footer_action_out: Option<FooterAction> = None;
        let active_tab_kind = self.active_tab_kind;
        let mut config_clone = config.clone();
        let mut clipboard_copy: Option<String> = None;

        // Stacker state — extract for closure
        let mut stacker_prompts = std::mem::take(&mut self.stacker_prompts);
        let mut stacker_input = std::mem::take(&mut self.stacker_input);
        let mut stacker_category_input = std::mem::take(&mut self.stacker_category_input);
        let mut stacker_search = std::mem::take(&mut self.stacker_search);
        let mut stacker_filter_category = std::mem::take(&mut self.stacker_filter_category);
        let mut stacker_editing = self.stacker_editing;
        let mut stacker_edit_text = std::mem::take(&mut self.stacker_edit_text);
        let mut stacker_dirty = self.stacker_dirty;
        let mut prompt_bar_visible = self.prompt_bar_visible;
        let mut prompt_bar_views = self.prompt_bar_views;
        let mut saved_edit_idx: Option<usize> = None;
        let mut copy_ghosts = std::mem::take(&mut self.copy_ghosts);
        let mut sketch = std::mem::take(&mut self.sketch);
        let mut sketch_canvas_px: Option<[f32; 4]> = None;
        let mut explorer = std::mem::take(&mut self.explorer);
        let mut editor_view = std::mem::take(&mut self.editor_view);
        let _terminal_panel_open = self.terminal_panel_open;
        let _terminal_panel_ratio = self.terminal_panel_ratio;
        let mut palette = std::mem::take(&mut self.palette);
        let mut palette_command: Option<command_palette::CommandId> = None;
        let recent_projects = self.recent_projects.clone();
        let mut open_project: Option<std::path::PathBuf> = None;
        let mut launch_workspace: Option<crate::workspace_store::SavedWorkspace> = None;
        let show_fps = self.show_fps;
        let fps_info = if show_fps && !self.frame_times.is_empty() {
            let avg_dt: f32 = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
            let fps = if avg_dt > 0.0 { 1.0 / avg_dt } else { 0.0 };
            Some((fps, avg_dt * 1000.0))
        } else {
            None
        };
        let perf_summary = if show_fps { Some(self.perf_stats.summary()) } else { None };

        let mut pending_close = self.pending_close.take();
        let mut save_prompt_response: Option<SavePromptResponse> = None;

        let mut sidebar_open = self.sidebar_open;
        let mut sidebar_panel_width = self.sidebar_actual_width;
        let mut editing_tab = self.editing_tab;
        let mut editing_tab_text = std::mem::take(&mut self.editing_tab_text);
        let tab_count = self.tab_count;
        let active_tab_index = self.active_tab_index;
        let tab_names = std::mem::take(&mut self.tab_names);
        let mut last_tab_click = self.last_tab_click.take();
        let mut saved_tab_name_out: Option<(usize, String)> = None;
        let mut tab_switch: Option<usize> = None;
        let mut tab_close: Option<usize> = None;
        let mut tab_split_right: Option<usize> = None;
        let mut tab_unsplit = false;
        let mut tab_close_others: Option<usize> = None;
        let mut tab_close_to_right: Option<usize> = None;
        let split_view = self.split_view;

        let full_output = self.ctx.run(raw_input, |ctx| {
            // ── Theme-derived colors ──
            let bg = config_clone.colors.background;
            let fg = config_clone.colors.foreground;
            let cursor_c = config_clone.colors.cursor;
            let chrome_bg = egui::Color32::from_rgb(
                (bg[0] as f32 * 0.65) as u8,
                (bg[1] as f32 * 0.65) as u8,
                (bg[2] as f32 * 0.65) as u8,
            );
            let active_btn = egui::Color32::from_rgb(
                (cursor_c[0] as f32 * 0.4) as u8,
                (cursor_c[1] as f32 * 0.4) as u8,
                (cursor_c[2] as f32 * 0.4) as u8,
            );
            let text_color = egui::Color32::from_rgb(fg[0], fg[1], fg[2]);

            // ── Footer ──
            let active_singleton = active_tab_kind.and_then(|k| {
                if matches!(k, crate::workspace::TabKind::Stacker | crate::workspace::TabKind::Sketch) {
                    Some(k)
                } else {
                    None
                }
            });
            if let Some(action) = footer::render_footer(
                ctx, footer_height, current_view, active_singleton, active_tab_kind, chrome_bg, active_btn, text_color,
            ) {
                match action {
                    footer::FooterAction::ShowOverlay(view) => {
                        nav_target = Some(view);
                    }
                    footer::FooterAction::OpenSingletonTab(kind) => {
                        footer_action_out = Some(footer::FooterAction::OpenSingletonTab(kind));
                    }
                    footer::FooterAction::NewTerminalTab => {
                        footer_action_out = Some(footer::FooterAction::NewTerminalTab);
                    }
                }
            }

            // ── Prompt queue bar (above footer, below content) ──
            if prompt_bar_visible && !stacker_prompts.is_empty() && current_view != ActiveView::Home {
                let show_bar = match active_tab_kind {
                    Some(crate::workspace::TabKind::Terminal) => {
                        prompt_bar_views & stacker_view::BAR_VIEW_SHELL != 0
                    }
                    Some(crate::workspace::TabKind::CodeFile) => {
                        prompt_bar_views & stacker_view::BAR_VIEW_EDITOR != 0
                    }
                    _ => false,
                };
                if show_bar {
                    if let Some(text) = stacker_view::render_prompt_bar(ctx, &stacker_prompts) {
                        clipboard_copy = Some(text);
                    }
                }
            }

            // ── Tab bar (egui) ──
            if !tab_names.is_empty() && current_view != ActiveView::Home {
                egui::TopBottomPanel::top("workspace_tab_bar")
                    .exact_height(30.0)
                    .frame(
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(
                                (bg[0] as f32 * 0.4) as u8,
                                (bg[1] as f32 * 0.4) as u8,
                                (bg[2] as f32 * 0.4) as u8,
                            ))
                            .inner_margin(egui::Margin::symmetric(4.0, 0.0)),
                    )
                    .show(ctx, |ui| {
                        ui.horizontal_centered(|ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            for (i, (name, _is_active)) in tab_names.iter().enumerate() {
                                let active = i == active_tab_index;
                                let tab_bg = if active {
                                    egui::Color32::from_rgb(50, 80, 140)
                                } else {
                                    egui::Color32::from_rgb(30, 32, 40)
                                };
                                let txt_color = if active {
                                    egui::Color32::WHITE
                                } else {
                                    egui::Color32::from_rgb(160, 165, 180)
                                };
                                let frame_resp = egui::Frame::none()
                                    .fill(tab_bg)
                                    .rounding(egui::Rounding { nw: 4.0, ne: 4.0, sw: 0.0, se: 0.0 })
                                    .inner_margin(egui::Margin::symmetric(10.0, 4.0))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            // Modified indicator dot
                                            if *_is_active {
                                                // _is_active here is the tab_names bool -- repurpose as modified flag
                                            }

                                            let label = ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(name).size(12.0).color(txt_color),
                                                ).sense(egui::Sense::click()),
                                            );
                                            if label.clicked() { tab_switch = Some(i); }

                                            ui.add_space(6.0);
                                            let x_color = if active {
                                                egui::Color32::from_rgb(200, 200, 210)
                                            } else {
                                                egui::Color32::from_rgb(100, 105, 115)
                                            };
                                            let x_btn = ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new("x").size(11.0).color(x_color),
                                                ).sense(egui::Sense::click()),
                                            );
                                            if x_btn.clicked() { tab_close = Some(i); }
                                            if x_btn.hovered() {
                                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                            }
                                        });
                                    });

                                // Right-click context menu
                                frame_resp.response.context_menu(|ui| {
                                    if split_view.is_some() {
                                        if ui.button("Unsplit").clicked() {
                                            tab_unsplit = true;
                                            ui.close_menu();
                                        }
                                    } else {
                                        if ui.button("Split Right").clicked() {
                                            tab_split_right = Some(i);
                                            ui.close_menu();
                                        }
                                    }
                                    ui.separator();
                                    if ui.button("Close").clicked() {
                                        tab_close = Some(i);
                                        ui.close_menu();
                                    }
                                    if ui.button("Close Others").clicked() {
                                        tab_close_others = Some(i);
                                        ui.close_menu();
                                    }
                                    if ui.button("Close to the Right").clicked() {
                                        tab_close_to_right = Some(i);
                                        ui.close_menu();
                                    }
                                });
                            }
                        });
                    });
            }

            // ── Sidebar ──
            let sidebar_result = sidebar::render_sidebar(
                ctx, sidebar_open, chrome_bg, bg, text_color,
                &mut explorer, &mut editor_view,
            );
            sidebar_open = sidebar_result.open;
            sidebar_panel_width = sidebar_result.panel_width;

            // ── Home view ──
            if current_view == ActiveView::Home {
                let action = home_view::render_home_view(ctx, &recent_projects);
                if action.nav_target.is_some() { nav_target = action.nav_target; }
                if action.open_project.is_some() { open_project = action.open_project; }
                if action.launch_workspace.is_some() { launch_workspace = action.launch_workspace; }
            }

            // ── Tab content views (rendered when Home overlay is not active) ──
            if current_view != ActiveView::Home {
                use crate::workspace::TabKind;
                match active_tab_kind {
                    Some(TabKind::Stacker) => {
                        egui::CentralPanel::default()
                            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(20, 20, 26)).inner_margin(egui::Margin::same(20.0)))
                            .show(ctx, |ui| {
                                stacker_view::render_stacker_view(
                                    ui, &mut stacker_prompts, &mut stacker_input,
                                    &mut stacker_category_input, &mut stacker_search,
                                    &mut stacker_filter_category, &mut stacker_editing,
                                    &mut stacker_edit_text, &mut stacker_dirty,
                                    &mut saved_edit_idx, &mut clipboard_copy,
                                    &mut prompt_bar_visible, &mut prompt_bar_views,
                                );
                            });
                    }
                    Some(TabKind::CodeFile) => {
                        egui::CentralPanel::default()
                            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(bg[0], bg[1], bg[2])).inner_margin(egui::Margin::same(20.0)))
                            .show(ctx, |ui| {
                                explorer_view::render_explorer_view(ui, &mut explorer, &mut editor_view, &config_clone);
                            });
                    }
                    Some(TabKind::Sketch) => {
                        let sketch_bg = egui::Color32::from_rgb(bg[0], bg[1], bg[2]);
                        let sketch_appearance = sketch_view::SketchAppearance { canvas_bg: sketch_bg, text_color, active_btn };
                        let mut canvas_rect_out = None;
                        egui::CentralPanel::default()
                            .frame(egui::Frame::none().fill(sketch_bg).inner_margin(egui::Margin::same(14.0)))
                            .show(ctx, |ui| {
                                canvas_rect_out = Some(sketch_view::render_sketch_view(ctx, ui, &mut sketch, &sketch_appearance));
                            });
                        if let Some(rect) = canvas_rect_out {
                            let ppp = ctx.pixels_per_point();
                            sketch_canvas_px = Some([rect.left() * ppp, rect.top() * ppp, rect.right() * ppp, rect.bottom() * ppp]);
                        }
                    }
                    Some(TabKind::Appearances) => {
                        egui::CentralPanel::default()
                            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(20, 20, 26)).inner_margin(egui::Margin::same(20.0)))
                            .show(ctx, |ui| {
                                egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                                    settings_tabs::render_themes_tab(ui, &mut config_clone);
                                    ui.add_space(24.0); ui.separator(); ui.add_space(16.0);
                                    settings_tabs::render_background_tab(ui, &mut config_clone);
                                    ui.add_space(24.0); ui.separator(); ui.add_space(16.0);
                                    settings_tabs::render_text_tab(ui, &mut config_clone);
                                });
                            });
                    }
                    Some(TabKind::Settings) => {
                        egui::CentralPanel::default()
                            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(20, 20, 26)).inner_margin(egui::Margin::same(20.0)))
                            .show(ctx, |ui| {
                                egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                                    settings_tabs::render_editor_tab(ui, &mut config_clone);
                                    ui.add_space(24.0); ui.separator(); ui.add_space(16.0);
                                    // Workspace builder returns an action if user clicks Launch
                                    let _ws_action = settings_tabs::render_workspace_tab(ui);
                                    // TODO: handle workspace launch action in main loop
                                });
                            });
                    }
                    Some(TabKind::Terminal) => {
                        // Terminal content rendered by wgpu — no egui panel needed
                    }
                    None => {
                        // No tabs — empty background
                        egui::CentralPanel::default()
                            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(bg[0], bg[1], bg[2])))
                            .show(ctx, |_ui| {});
                    }
                }
            }

            // ── Overlays ──
            overlays::render_copy_ghosts(ctx, &mut copy_ghosts);
            palette_command = overlays::render_command_palette(ctx, &mut palette);
            if let Some((fps, ms)) = fps_info {
                overlays::render_fps_overlay(ctx, fps, ms, perf_summary.as_deref());
            }
            // Save prompt dialog (rendered on top of everything)
            if let Some(ref pc) = pending_close {
                save_prompt_response = overlays::render_save_prompt(ctx, pc);
            }
        });

        // Apply inline edit after egui releases its temporary borrows.
        if let Some(idx) = saved_edit_idx {
            if apply_prompt_edit(&mut stacker_prompts, idx, &stacker_edit_text) {
                stacker_dirty = true;
            }
            stacker_edit_text.clear();
        }

        // Persist to disk when dirty
        if stacker_dirty {
            save_stacker_prompts(&stacker_prompts);
        }

        // Apply state changes
        self.settings_tab = settings_tab;
        self.stacker_prompts = stacker_prompts;
        self.stacker_input = stacker_input;
        self.stacker_category_input = stacker_category_input;
        self.stacker_search = stacker_search;
        self.stacker_filter_category = stacker_filter_category;
        self.stacker_editing = stacker_editing;
        self.stacker_edit_text = stacker_edit_text;
        self.stacker_dirty = false;
        self.prompt_bar_visible = prompt_bar_visible;
        self.prompt_bar_views = prompt_bar_views;
        self.copy_ghosts = copy_ghosts;
        if sketch.is_dirty() {
            // Always auto-save to the default scratch file
            let _ = save_default_document(&sketch.document);
            // Also save to the named sketch file if one is active
            if let Some(name) = &sketch.active_sketch_name {
                let _ = save_named_sketch(name, &sketch.document);
            }
            sketch.mark_saved();
        }
        self.sketch = sketch;
        self.sidebar_open = sidebar_open;
        self.sidebar_actual_width = sidebar_panel_width;
        self.explorer = explorer;
        self.editor_view = editor_view;
        self.palette = palette;
        self.palette_command = palette_command;
        self.open_project = open_project;
        self.launch_workspace = launch_workspace;
        self.sketch_canvas_px = sketch_canvas_px;

        // Restore tab editing state
        self.editing_tab = editing_tab;
        self.editing_tab_text = editing_tab_text;
        self.saved_tab_name = saved_tab_name_out;
        self.last_tab_click = last_tab_click;

        self.footer_action = footer_action_out;
        let has_tab_action = tab_switch.is_some() || tab_close.is_some()
            || tab_split_right.is_some() || tab_unsplit
            || tab_close_others.is_some() || tab_close_to_right.is_some();
        self.tab_bar_action = if has_tab_action {
            Some(tab_bar::TabBarAction {
                switch_to: tab_switch,
                close_tab: tab_close,
                split_right: tab_split_right,
                unsplit: tab_unsplit,
                close_others: tab_close_others,
                close_to_right: tab_close_to_right,
            })
        } else {
            None
        };
        self.pending_close = pending_close;
        self.save_prompt_response = save_prompt_response;

        if let Some(view) = nav_target {
            self.active_view = view;
        }
        if let Some(text) = clipboard_copy {
            self.clipboard_text = Some(text);
        }
        // Editor clipboard: copy/cut puts text here for the main loop
        if let Some(text) = self.editor_view.clipboard_out.take() {
            self.clipboard_text = Some(text);
        }

        // Push config changes when a settings surface is active
        if matches!(
            active_tab_kind,
            Some(crate::workspace::TabKind::Appearances | crate::workspace::TabKind::Settings)
        ) {
            self.pending_config = Some(config_clone);
        }

        self.winit_state
            .handle_platform_output(window, full_output.platform_output);

        let tris = self
            .ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        for (id, image_delta) in &full_output.textures_delta.set {
            self.wgpu_renderer
                .update_texture(device, queue, *id, image_delta);
        }

        // egui needs its own encoder due to wgpu 22 RenderPass lifetime requirements
        let mut egui_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("egui_encoder"),
        });

        self.wgpu_renderer
            .update_buffers(device, queue, &mut egui_encoder, &tris, &screen_desc);

        // Submit the update buffers encoder first
        queue.submit(std::iter::once(egui_encoder.finish()));

        // Render egui using a raw render pass.
        // wgpu 22's RenderPass has 'static lifetime, so we create+finish the encoder
        // in a way that satisfies the borrow checker.
        {
            let mut render_encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("egui_render"),
                });
            let pass = render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            let mut static_pass = pass.forget_lifetime();
            self.wgpu_renderer
                .render(&mut static_pass, &tris, &screen_desc);
            drop(static_pass);
            queue.submit(std::iter::once(render_encoder.finish()));
        }

        for id in &full_output.textures_delta.free {
            self.wgpu_renderer.free_texture(id);
        }
    }
}
