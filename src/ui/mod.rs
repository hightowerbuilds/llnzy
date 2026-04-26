mod explorer_view;
mod settings_tabs;
mod sketch_view;
mod stacker_view;
pub mod types;

use std::time::Instant;
use winit::window::Window;

use crate::config::Config;
use crate::explorer::ExplorerState;
use crate::sketch::{save_default_document, SketchState};
use crate::stacker::{
    apply_prompt_edit, load_stacker_prompts, save_stacker_prompts, StackerPrompt,
};

pub use types::{ActiveView, CopyGhost, SettingsTab, SIDEBAR_WIDTH};
use types::{GHOST_DURATION_SECS, GHOST_FLOAT_PX};

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
    // Debug overlay
    pub show_fps: bool,
    frame_times: std::collections::VecDeque<f32>,
    // Stacker state
    pub stacker_prompts: Vec<StackerPrompt>,
    pub stacker_input: String,
    pub stacker_category_input: String,
    pub stacker_search: String,
    pub stacker_filter_category: String, // empty = show all
    pub stacker_editing: Option<usize>,  // index of prompt being edited
    pub stacker_edit_text: String,
    pub stacker_dirty: bool, // needs save to disk
    pub copy_ghosts: Vec<CopyGhost>,
    // Sketch state
    pub sketch: SketchState,
    // Explorer state
    pub explorer: ExplorerState,
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
            active_view: ActiveView::Shells,
            settings_tab: SettingsTab::Themes,
            footer_height: 36.0,
            pending_config: None,
            clipboard_text: None,
            sidebar_open: false,
            show_fps: false,
            frame_times: std::collections::VecDeque::with_capacity(120),
            stacker_prompts,
            stacker_input: String::new(),
            stacker_category_input: String::new(),
            stacker_search: String::new(),
            stacker_filter_category: String::new(),
            stacker_editing: None,
            stacker_edit_text: String::new(),
            stacker_dirty: false,
            copy_ghosts: Vec::new(),
            sketch: SketchState::load_default(),
            explorer: ExplorerState::new(),
            editing_tab: None,
            editing_tab_text: String::new(),
            saved_tab_name: None,
            last_tab_click: None,
            sketch_canvas_px: None,
            tab_count: 0,
            active_tab_index: 0,
        }
    }

    /// Pass a winit event to egui. Returns true if egui consumed it.
    pub fn handle_event(&mut self, window: &Window, event: &winit::event::WindowEvent) -> bool {
        let response = self.winit_state.on_window_event(window, event);
        response.consumed
    }

    /// Whether the terminal is covered by a full-screen view.
    pub fn settings_open(&self) -> bool {
        matches!(
            self.active_view,
            ActiveView::Appearances
                | ActiveView::Settings
                | ActiveView::Stacker
                | ActiveView::Sketch
                | ActiveView::Explorer
        )
    }

    pub fn captures_terminal_input(&self) -> bool {
        matches!(self.active_view, ActiveView::Sketch)
    }

    pub fn toggle_sidebar(&mut self) {
        self.sidebar_open = !self.sidebar_open;
    }

    pub fn sidebar_width(&self) -> f32 {
        if self.sidebar_open {
            SIDEBAR_WIDTH
        } else {
            0.0
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
        let mut settings_tab = self.settings_tab;
        let mut nav_target: Option<ActiveView> = None;
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
        let mut saved_edit_idx: Option<usize> = None;
        let mut copy_ghosts = std::mem::take(&mut self.copy_ghosts);
        let mut sketch = std::mem::take(&mut self.sketch);
        let mut sketch_canvas_px: Option<[f32; 4]> = None;
        let mut explorer = std::mem::take(&mut self.explorer);
        let show_fps = self.show_fps;
        let fps_info = if show_fps && !self.frame_times.is_empty() {
            let avg_dt: f32 = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
            let fps = if avg_dt > 0.0 { 1.0 / avg_dt } else { 0.0 };
            Some((fps, avg_dt * 1000.0))
        } else {
            None
        };

        let sidebar_open = self.sidebar_open;
        let mut editing_tab = self.editing_tab;
        let mut editing_tab_text = std::mem::take(&mut self.editing_tab_text);
        let tab_count = self.tab_count;
        let mut last_tab_click = self.last_tab_click.take();
        let mut saved_tab_name_out: Option<(usize, String)> = None;

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

            // ── Footer bar (blank) ──
            egui::TopBottomPanel::bottom("footer")
                .exact_height(footer_height)
                .frame(egui::Frame::none().fill(chrome_bg))
                .show(ctx, |_ui| {});

            // ── Tab bar interaction (for renaming) ──
            let tab_bar_height = 32.0;
            const DOUBLE_CLICK_TIME_MS: u128 = 300;

            // Create interactive areas over each tab for double-click detection
            if tab_count > 0 && matches!(current_view, ActiveView::Shells) {
                let viewport_rect = ctx.screen_rect();
                let tab_w = (viewport_rect.width() / tab_count as f32).min(200.0);
                let sidebar_offset = if sidebar_open { SIDEBAR_WIDTH } else { 0.0 };

                // Check for clicks on tabs
                let mut tab_clicked: Option<usize> = None;
                ctx.input(|input| {
                    if input.pointer.button_pressed(egui::PointerButton::Primary) {
                        if let Some(pos) = input.pointer.latest_pos() {
                            if pos.y >= viewport_rect.top()
                                && pos.y < viewport_rect.top() + tab_bar_height
                            {
                                let rel_x = pos.x - viewport_rect.left() - sidebar_offset;
                                if rel_x >= 0.0 && rel_x < viewport_rect.width() - sidebar_offset {
                                    let tab_idx = (rel_x / tab_w).floor() as usize;
                                    if tab_idx < tab_count {
                                        tab_clicked = Some(tab_idx);
                                    }
                                }
                            }
                        }
                    }
                });

                // Detect double-click based on last click time
                if let Some(tab_idx) = tab_clicked {
                    if let Some((last_idx, last_time)) = last_tab_click {
                        if last_idx == tab_idx
                            && last_time.elapsed().as_millis() < DOUBLE_CLICK_TIME_MS
                        {
                            // Double-click detected!
                            editing_tab = Some(tab_idx);
                            editing_tab_text.clear();
                            last_tab_click = None;
                        } else {
                            // Single click on different tab or too late
                            last_tab_click = Some((tab_idx, Instant::now()));
                        }
                    } else {
                        // First click
                        last_tab_click = Some((tab_idx, Instant::now()));
                    }
                }

                // If editing a tab, show a text input area overlay
                if let Some(edit_idx) = editing_tab {
                    let tab_x = sidebar_offset + edit_idx as f32 * tab_w;

                    egui::Area::new(egui::Id::new(("tab_edit", edit_idx)))
                        .fixed_pos(egui::pos2(tab_x + 4.0, viewport_rect.top() + 4.0))
                        .show(ctx, |ui| {
                            ui.set_max_width(tab_w - 8.0);
                            let mut text = editing_tab_text.clone();
                            let response = ui.text_edit_singleline(&mut text);
                            editing_tab_text = text;

                            // Request focus on this input
                            response.request_focus();

                            // Check for Enter or Escape
                            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                            let escape_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));

                            if enter_pressed {
                                // Save the edited name
                                saved_tab_name_out = Some((edit_idx, editing_tab_text.clone()));
                                editing_tab = None;
                                editing_tab_text.clear();
                                last_tab_click = None;
                            } else if escape_pressed {
                                // Cancel editing
                                editing_tab = None;
                                editing_tab_text.clear();
                                last_tab_click = None;
                            }
                        });
                }
            }

            // Navigation sidebar (toggle with Cmd+B) ──
            if sidebar_open {
                egui::SidePanel::left("nav_sidebar")
                    .exact_width(SIDEBAR_WIDTH)
                    .frame(
                        egui::Frame::none()
                            .fill(chrome_bg)
                            .inner_margin(egui::Margin::same(12.0)),
                    )
                    .show(ctx, |ui| {
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("llnzy")
                                .size(20.0)
                                .color(text_color)
                                .strong(),
                        );
                        ui.add_space(16.0);

                        let views = [
                            ("Shells", ActiveView::Shells),
                            ("Explorer", ActiveView::Explorer),
                            ("Stacker", ActiveView::Stacker),
                            ("Sketch", ActiveView::Sketch),
                            ("Appearances", ActiveView::Appearances),
                            ("Settings", ActiveView::Settings),
                        ];

                        for (name, view) in views {
                            let text = egui::RichText::new(name).size(16.0);
                            let btn = if current_view == view {
                                ui.add(
                                    egui::Button::new(text.color(egui::Color32::WHITE))
                                        .fill(active_btn)
                                        .min_size(egui::Vec2::new(ui.available_width(), 32.0)),
                                )
                            } else {
                                ui.add(
                                    egui::Button::new(text.color(text_color))
                                        .fill(egui::Color32::TRANSPARENT)
                                        .min_size(egui::Vec2::new(ui.available_width(), 32.0)),
                                )
                            };
                            if btn.clicked() && current_view != view {
                                nav_target = Some(view);
                            }
                            ui.add_space(4.0);
                        }

                        // ── Prompt queue ──
                        if !stacker_prompts.is_empty() {
                            ui.add_space(8.0);
                            ui.separator();
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new("Queue").size(13.0).color(
                                egui::Color32::from_rgb(
                                    (fg[0] as f32 * 0.6) as u8,
                                    (fg[1] as f32 * 0.6) as u8,
                                    (fg[2] as f32 * 0.6) as u8,
                                ),
                            ));
                            ui.add_space(4.0);

                            egui::ScrollArea::vertical()
                                .id_salt("sidebar_queue")
                                .show(ui, |ui| {
                                    for prompt in &stacker_prompts {
                                        let btn = ui.add(
                                            egui::Button::new(
                                                egui::RichText::new(&prompt.label)
                                                    .size(13.0)
                                                    .color(text_color),
                                            )
                                            .fill(egui::Color32::TRANSPARENT)
                                            .min_size(egui::Vec2::new(ui.available_width(), 24.0))
                                            .wrap_mode(egui::TextWrapMode::Truncate),
                                        );
                                        if btn.clicked() {
                                            clipboard_copy = Some(prompt.text.clone());
                                            let r = btn.rect;
                                            copy_ghosts.push(CopyGhost {
                                                text: prompt.label.clone(),
                                                x: r.left(),
                                                y: r.center().y,
                                                created: Instant::now(),
                                            });
                                        }
                                        btn.on_hover_text("Click to copy to clipboard");
                                    }
                                });
                        }
                    });
            }

            // ── Appearances view ──
            if current_view == ActiveView::Appearances {
                egui::SidePanel::left("appearances_sidebar")
                    .exact_width(170.0)
                    .frame(
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(24, 24, 32))
                            .inner_margin(egui::Margin::same(12.0)),
                    )
                    .show(ctx, |ui| {
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("Appearances")
                                .size(22.0)
                                .color(egui::Color32::WHITE),
                        );
                        ui.add_space(16.0);

                        let tabs = [
                            ("Themes", SettingsTab::Themes),
                            ("Background", SettingsTab::Background),
                            ("Text", SettingsTab::Text),
                        ];
                        for (name, tab) in tabs {
                            if ui
                                .selectable_label(settings_tab == tab, label(name))
                                .clicked()
                            {
                                settings_tab = tab;
                            }
                            ui.add_space(4.0);
                        }
                    });

                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(20, 20, 26))
                            .inner_margin(egui::Margin::same(20.0)),
                    )
                    .show(ctx, |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| match settings_tab {
                            SettingsTab::Themes => settings_tabs::render_themes_tab(ui, &mut config_clone),
                            SettingsTab::Background => settings_tabs::render_background_tab(ui, &mut config_clone),
                            SettingsTab::Text => settings_tabs::render_text_tab(ui, &mut config_clone),
                        });
                    });
            }

            // ── Stacker view ──
            if current_view == ActiveView::Stacker {
                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(20, 20, 26))
                            .inner_margin(egui::Margin::same(20.0)),
                    )
                    .show(ctx, |ui| {
                        stacker_view::render_stacker_view(
                            ui,
                            &mut stacker_prompts,
                            &mut stacker_input,
                            &mut stacker_category_input,
                            &mut stacker_search,
                            &mut stacker_filter_category,
                            &mut stacker_editing,
                            &mut stacker_edit_text,
                            &mut stacker_dirty,
                            &mut saved_edit_idx,
                            &mut clipboard_copy,
                        );
                    });
            }

            // ── Explorer view ──
            if current_view == ActiveView::Explorer {
                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(bg[0], bg[1], bg[2]))
                            .inner_margin(egui::Margin::same(20.0)),
                    )
                    .show(ctx, |ui| {
                        explorer_view::render_explorer_view(ui, &mut explorer);
                    });
            }

            // ── Sketch view ──
            if current_view == ActiveView::Sketch {
                let sketch_bg = egui::Color32::from_rgb(bg[0], bg[1], bg[2]);
                let sketch_appearance = sketch_view::SketchAppearance {
                    canvas_bg: sketch_bg,
                    text_color,
                    active_btn,
                };
                let mut canvas_rect_out = None;
                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::none()
                            .fill(sketch_bg)
                            .inner_margin(egui::Margin::same(14.0)),
                    )
                    .show(ctx, |ui| {
                        canvas_rect_out = Some(sketch_view::render_sketch_view(ctx, ui, &mut sketch, &sketch_appearance));
                    });
                if let Some(rect) = canvas_rect_out {
                    let ppp = ctx.pixels_per_point();
                    sketch_canvas_px = Some([
                        rect.left() * ppp,
                        rect.top() * ppp,
                        rect.right() * ppp,
                        rect.bottom() * ppp,
                    ]);
                }
            }

            // ── Settings view (blank for now) ──
            if current_view == ActiveView::Settings {
                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(20, 20, 26))
                            .inner_margin(egui::Margin::same(20.0)),
                    )
                    .show(ctx, |ui| {
                        ui.label(
                            egui::RichText::new("Settings")
                                .size(22.0)
                                .color(egui::Color32::WHITE),
                        );
                    });
            }

            // ── Copy ghost animations ──
            let now = Instant::now();
            copy_ghosts
                .retain(|g| now.duration_since(g.created).as_secs_f32() < GHOST_DURATION_SECS);
            for (i, ghost) in copy_ghosts.iter().enumerate() {
                let t = now.duration_since(ghost.created).as_secs_f32() / GHOST_DURATION_SECS;
                let alpha = ((1.0 - t) * 200.0) as u8;
                let y_offset = t * GHOST_FLOAT_PX;
                egui::Area::new(egui::Id::new("copy_ghost").with(i))
                    .fixed_pos(egui::Pos2::new(ghost.x, ghost.y - y_offset))
                    .interactable(false)
                    .order(egui::Order::Tooltip)
                    .show(ctx, |ui| {
                        ui.label(
                            egui::RichText::new(&ghost.text)
                                .size(12.0)
                                .color(egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha)),
                        );
                    });
            }
            if !copy_ghosts.is_empty() {
                ctx.request_repaint();
            }

            // FPS overlay
            if let Some((fps, ms)) = fps_info {
                egui::Area::new(egui::Id::new("fps_overlay"))
                    .fixed_pos(egui::Pos2::new(8.0, 8.0))
                    .show(ctx, |ui| {
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgba_premultiplied(0, 0, 0, 180))
                            .rounding(egui::Rounding::same(4.0))
                            .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(format!("{:.0} FPS  {:.1}ms", fps, ms))
                                        .size(12.0)
                                        .color(egui::Color32::from_rgb(150, 255, 150))
                                        .monospace(),
                                );
                            });
                    });
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
        self.copy_ghosts = copy_ghosts;
        if sketch.is_dirty() && save_default_document(&sketch.document).is_ok() {
            sketch.mark_saved();
        }
        self.sketch = sketch;
        self.explorer = explorer;
        self.sketch_canvas_px = sketch_canvas_px;

        // Restore tab editing state
        self.editing_tab = editing_tab;
        self.editing_tab_text = editing_tab_text;
        self.saved_tab_name = saved_tab_name_out;
        self.last_tab_click = last_tab_click;

        if let Some(view) = nav_target {
            self.active_view = view;
        }
        if let Some(text) = clipboard_copy {
            self.clipboard_text = Some(text);
        }

        // Push config changes when on appearances view
        if current_view == ActiveView::Appearances {
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

const S: f32 = 16.0;

fn label(text: &str) -> egui::RichText {
    egui::RichText::new(text).size(S)
}

