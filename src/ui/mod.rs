pub mod command_palette;
mod editor_view;
mod explorer_view;
mod footer;
mod git_state;
mod git_view;
mod home_view;
mod overlays;
mod settings_state;
mod settings_tabs;
mod sidebar;
mod sidebar_state;
mod sketch_state;
mod sketch_view;
mod stacker_state;
mod stacker_view;
pub mod tab_bar;
mod tab_bar_joined;
mod tab_content;
pub mod types;

use winit::window::Window;

use crate::app::commands::AppCommand;
use crate::app::drag_drop::DragDropState;
use crate::config::Config;
use crate::explorer::ExplorerState;
use crate::stacker::apply_prompt_edit;
use crate::workspace_layout::{tab_bar_entries, JoinedTabs};

pub use footer::FooterAction;
pub use types::{
    ActiveView, CopyGhost, PendingClose, SavePromptResponse, UiFrameOutput, UiTabInfo,
    UiTabPaneInfo, BUMPER_WIDTH, SIDEBAR_WIDTH,
};

pub const STACKER_PROMPT_EDITOR_ID: &str = "stacker_prompt_editor";

/// State for the egui-driven UI overlay.
pub struct UiState {
    pub ctx: egui::Context,
    pub winit_state: egui_winit::State,
    pub wgpu_renderer: egui_wgpu::Renderer,
    pub active_view: ActiveView,
    pub settings: settings_state::SettingsUiState,
    /// How much vertical space the footer occupies (for terminal content layout)
    pub footer_height: f32,
    pub sidebar: sidebar_state::SidebarUiState,
    // Debug overlay
    pub show_fps: bool,
    frame_times: std::collections::VecDeque<f32>,
    pub perf_stats: crate::editor::perf::PerfStats,
    // Stacker state
    pub stacker: stacker_state::StackerUiState,
    // Sketch state
    pub sketch: sketch_state::SketchUiState,
    // Explorer state
    pub explorer: ExplorerState,
    // Git dashboard state
    pub git: git_state::GitUiState,
    // Editor view state (cursor, scroll, etc.)
    pub editor_view: explorer_view::EditorViewState,
    // Tab renaming
    pub editing_tab: Option<usize>,
    pub editing_tab_text: String,
    pub saved_tab_name: Option<(usize, String)>, // (tab_index, new_name) to apply after render
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
    /// Active tab kind (set by main loop before render, used by footer highlighting)
    pub active_tab_kind: Option<crate::workspace::TabKind>,
    /// Terminal voice input mode. When enabled, multi-character macOS text commits
    /// are routed straight to the active PTY before egui handles them.
    pub wispr_flow_mode: bool,
    /// Workspace tabs reference for egui tab bar rendering (set by main loop before render)
    pub tab_names: Vec<UiTabInfo>,
    /// Lightweight tab content metadata for joined-tab rendering.
    pub tab_panes: Vec<UiTabPaneInfo>,
    /// Joined-tab pair rendered side by side.
    pub joined_tabs: Option<JoinedTabs>,
    /// App-wide drag and drop state. Phase 1 uses this for native file drops;
    /// later phases will let each surface register typed payloads and targets.
    pub drag_drop: DragDropState,
    /// Pending close confirmation for unsaved buffers.
    pub pending_close: Option<PendingClose>,
    /// Save failure shown in the unsaved-changes prompt.
    pub save_prompt_error: Option<String>,
}

impl UiState {
    pub fn new(
        window: &Window,
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let ctx = egui::Context::default();
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "Atkinson Hyperlegible".to_string(),
            egui::FontData::from_static(include_bytes!(
                "../../assets/fonts/AtkinsonHyperlegible-Regular.ttf"
            )),
        );
        fonts.families.insert(
            egui::FontFamily::Name("Atkinson Hyperlegible".into()),
            vec!["Atkinson Hyperlegible".to_string()],
        );
        ctx.set_fonts(fonts);

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

        UiState {
            ctx,
            winit_state,
            wgpu_renderer,
            active_view: ActiveView::Shells,
            settings: settings_state::SettingsUiState::default(),
            footer_height: crate::layout::FOOTER_HEIGHT,
            sidebar: sidebar_state::SidebarUiState::default(),
            show_fps: false,
            frame_times: std::collections::VecDeque::with_capacity(120),
            perf_stats: crate::editor::perf::PerfStats::default(),
            stacker: stacker_state::StackerUiState::load(),
            sketch: sketch_state::SketchUiState::default(),
            explorer: ExplorerState::new(),
            git: git_state::GitUiState::default(),
            editor_view: explorer_view::EditorViewState::default(),
            editing_tab: None,
            editing_tab_text: String::new(),
            saved_tab_name: None,
            tab_count: 0,
            active_tab_index: 0,
            terminal_panel_open: false,
            terminal_panel_ratio: 0.35,
            palette: command_palette::PaletteState::default(),
            palette_command: None,
            recent_projects: crate::explorer::load_recent_projects(),
            active_tab_kind: None,
            wispr_flow_mode: false,
            tab_names: Vec::new(),
            tab_panes: Vec::new(),
            joined_tabs: None,
            drag_drop: DragDropState::default(),
            pending_close: None,
            save_prompt_error: None,
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
        if matches!(
            self.active_view,
            ActiveView::Appearances | ActiveView::Settings
        ) {
            return true;
        }
        // Non-terminal tabs cover the terminal content area
        !matches!(
            self.active_tab_kind,
            Some(crate::workspace::TabKind::Terminal)
        )
    }

    pub fn toggle_terminal_panel(&mut self) {
        self.terminal_panel_open = !self.terminal_panel_open;
    }

    pub fn captures_terminal_input(&self) -> bool {
        matches!(
            self.active_tab_kind,
            Some(crate::workspace::TabKind::Sketch)
        )
    }

    pub fn toggle_sidebar(&mut self) {
        self.sidebar.toggle();
    }

    /// Total width consumed by sidebar UI (bumper is always visible).
    pub fn sidebar_width(&self) -> f32 {
        self.sidebar.total_width()
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
    ) -> UiFrameOutput {
        let raw_input = self.winit_state.take_egui_input(window);

        // Extract state to avoid borrowing self inside the closure
        let footer_height = self.footer_height;
        let mut nav_target: Option<ActiveView> = None;
        let mut commands_out: Vec<AppCommand> = Vec::new();
        let active_tab_kind = self.active_tab_kind;
        let mut wispr_flow_mode = self.wispr_flow_mode;
        let mut config_clone = config.clone();
        let mut settings = std::mem::take(&mut self.settings);

        // Stacker state — extract for closure
        let mut stacker = std::mem::take(&mut self.stacker);
        let mut saved_edit_idx: Option<usize> = None;
        let mut sketch = std::mem::take(&mut self.sketch);
        sketch.canvas_px = None;
        let mut explorer = std::mem::take(&mut self.explorer);
        let mut git = std::mem::take(&mut self.git);
        let mut editor_view = std::mem::take(&mut self.editor_view);
        let _terminal_panel_open = self.terminal_panel_open;
        let _terminal_panel_ratio = self.terminal_panel_ratio;
        let mut palette = std::mem::take(&mut self.palette);
        let mut palette_command: Option<command_palette::CommandId> = None;
        let recent_projects = self.recent_projects.clone();
        let show_fps = self.show_fps;
        let fps_info = if show_fps && !self.frame_times.is_empty() {
            let avg_dt: f32 = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
            let fps = if avg_dt > 0.0 { 1.0 / avg_dt } else { 0.0 };
            Some((fps, avg_dt * 1000.0))
        } else {
            None
        };
        let perf_summary = if show_fps {
            Some(self.perf_stats.summary())
        } else {
            None
        };

        let pending_close = self.pending_close.take();
        let save_prompt_error = self.save_prompt_error.clone();
        let mut save_prompt_response: Option<SavePromptResponse> = None;

        let mut sidebar_state = std::mem::take(&mut self.sidebar);
        let editing_tab = self.editing_tab;
        let editing_tab_text = std::mem::take(&mut self.editing_tab_text);
        let _tab_count = self.tab_count;
        let active_tab_index = self.active_tab_index;
        let tab_names = std::mem::take(&mut self.tab_names);
        let mut tab_bar_state = tab_bar::TabBarEditState {
            editing_tab,
            editing_tab_text: editing_tab_text.clone(),
        };
        let mut tab_bar_action = tab_bar::TabBarAction::default();
        let mut joined_tabs = self.joined_tabs.map(JoinedTabs::clamped);
        let tab_bar_entries = tab_bar_entries(tab_names.len(), joined_tabs);
        let tab_panes = std::mem::take(&mut self.tab_panes);
        let drag_drop = self.drag_drop.clone();

        let full_output = self.ctx.run(raw_input, |ctx| {
            // ── Theme-derived colors ──
            let bg = config_clone.colors.background;
            let fg = config_clone.colors.foreground;
            let cursor_c = config_clone.colors.cursor;
            let chrome_bg = egui::Color32::from_rgb(36, 36, 36);
            let active_btn = egui::Color32::from_rgb(
                (cursor_c[0] as f32 * 0.4) as u8,
                (cursor_c[1] as f32 * 0.4) as u8,
                (cursor_c[2] as f32 * 0.4) as u8,
            );
            let text_color = egui::Color32::from_rgb(fg[0], fg[1], fg[2]);
            let footer_queue_prompts =
                if matches!(active_tab_kind, Some(crate::workspace::TabKind::Terminal)) {
                    footer_queue_prompts(&stacker)
                } else {
                    Vec::new()
                };

            // ── Footer ──
            if let Some(action) = footer::render_footer(
                ctx,
                footer_height,
                active_singleton_tab(active_tab_kind),
                active_tab_kind,
                chrome_bg,
                active_btn,
                text_color,
                &footer_queue_prompts,
                &mut wispr_flow_mode,
            ) {
                apply_footer_action(action, &mut nav_target, &mut commands_out);
            }

            // ── Tab bar (egui) ──
            tab_bar_action = tab_bar::render_workspace_tab_bar(
                ctx,
                tab_bar::TabBarRenderInput {
                    tabs: &tab_names,
                    entries: &tab_bar_entries,
                    active_tab_index,
                    joined_tabs,
                    bar_bg: egui::Color32::from_rgb(36, 36, 36),
                },
                &mut tab_bar_state,
            );

            // ── Sidebar ──
            let sidebar_result = sidebar::render_sidebar(
                ctx,
                sidebar_state.open,
                sidebar_state.recent_open,
                chrome_bg,
                bg,
                text_color,
                &mut explorer,
                &mut editor_view,
                &recent_projects,
                &config_clone,
            );
            sidebar_state.open = sidebar_result.open;
            sidebar_state.recent_open = sidebar_result.recent_open;
            sidebar_state.actual_width = sidebar_result.panel_width;
            if sidebar_result.close_folder {
                explorer.clear();
                sidebar_state.recent_open = false;
            }
            if let Some(project_path) = sidebar_result.open_project {
                commands_out.push(AppCommand::OpenProject(project_path));
            }

            // ── Tab content views ──
            tab_content::render_tab_content(
                ctx,
                active_tab_kind,
                active_tab_index,
                &mut joined_tabs,
                &tab_panes,
                &mut config_clone,
                tab_content::TabContentAppearance {
                    bg,
                    text_color,
                    active_btn,
                },
                tab_content::TabContentState {
                    settings: &mut settings,
                    stacker: &mut stacker,
                    sketch: &mut sketch,
                    git: &mut git,
                    explorer: &mut explorer,
                    editor_view: &mut editor_view,
                    recent_projects: &recent_projects,
                    saved_edit_idx: &mut saved_edit_idx,
                    commands: &mut commands_out,
                },
            );

            // ── Overlays ──
            overlays::render_drag_drop_overlay(ctx, &drag_drop);
            overlays::render_copy_ghosts(ctx, &mut stacker.copy_ghosts);
            palette_command = overlays::render_command_palette(ctx, &mut palette);
            if let Some((fps, ms)) = fps_info {
                overlays::render_fps_overlay(ctx, fps, ms, perf_summary.as_deref());
            }
            // Save prompt dialog (rendered on top of everything)
            if let Some(ref pc) = pending_close {
                save_prompt_response =
                    overlays::render_save_prompt(ctx, pc, save_prompt_error.as_deref());
            }
        });

        // Apply inline edit after egui releases its temporary borrows.
        if let Some(idx) = saved_edit_idx {
            if apply_prompt_edit(&mut stacker.prompts, idx, &stacker.edit_text) {
                stacker.dirty = true;
            }
            stacker.edit_text.clear();
        }

        // Persist to disk when dirty
        stacker.persist_if_dirty();

        // Apply state changes
        self.settings = settings;
        self.stacker = stacker;
        sketch.persist_if_dirty();
        self.sketch = sketch;
        self.sidebar = sidebar_state;
        self.explorer = explorer;
        self.git = git;
        self.tab_panes = tab_panes;
        self.joined_tabs = joined_tabs;
        self.wispr_flow_mode = wispr_flow_mode;
        if let Some((path, buffer_idx)) = editor_view.pending_file_tab.take() {
            commands_out.push(AppCommand::OpenCodeFile { path, buffer_idx });
        }
        if let Some(task) = editor_view.pending_task.take() {
            commands_out.push(AppCommand::RunTask(task));
        }
        self.editor_view = editor_view;
        self.palette = palette;
        self.palette_command = palette_command;

        tab_bar_action.append_commands(&mut commands_out);

        // Restore tab editing state
        self.editing_tab = tab_bar_state.editing_tab;
        self.editing_tab_text = tab_bar_state.editing_tab_text;
        self.saved_tab_name = None;

        self.pending_close = pending_close;

        if let Some(view) = nav_target {
            self.active_view = view;
        }
        // Editor clipboard: copy/cut puts text here for the main loop
        if let Some(text) = self.editor_view.clipboard_out.take() {
            commands_out.push(AppCommand::CopyToClipboard(text));
        }

        // Push config changes when a settings surface is active
        if matches!(
            active_tab_kind,
            Some(crate::workspace::TabKind::Appearances | crate::workspace::TabKind::Settings)
        ) {
            commands_out.push(AppCommand::ApplyConfig(config_clone));
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

        UiFrameOutput {
            commands: commands_out,
            save_prompt_response,
        }
    }
}

fn active_singleton_tab(
    active_tab_kind: Option<crate::workspace::TabKind>,
) -> Option<crate::workspace::TabKind> {
    active_tab_kind.filter(|kind| {
        matches!(
            kind,
            crate::workspace::TabKind::Stacker
                | crate::workspace::TabKind::Sketch
                | crate::workspace::TabKind::Git
        )
    })
}

fn apply_footer_action(
    action: footer::FooterAction,
    nav_target: &mut Option<ActiveView>,
    commands: &mut Vec<AppCommand>,
) {
    match action {
        footer::FooterAction::ShowOverlay(view) => {
            *nav_target = Some(view);
        }
        footer::FooterAction::OpenSingletonTab(kind) => {
            commands.push(AppCommand::OpenSingletonTab(kind));
        }
        footer::FooterAction::NewTerminalTab => {
            commands.push(AppCommand::NewTerminalTab);
        }
        footer::FooterAction::CopyQueuedPrompt(text) => {
            commands.push(AppCommand::CopyToClipboard(text));
        }
    }
}

fn footer_queue_prompts(stacker: &stacker_state::StackerUiState) -> Vec<footer::FooterQueuePrompt> {
    stacker
        .queued_prompts
        .iter()
        .take(5)
        .enumerate()
        .map(|(slot, prompt)| footer::FooterQueuePrompt {
            letter: (b'A' + slot as u8) as char,
            preview: crate::stacker::queue::footer_preview(&prompt.text),
            clipboard_text: crate::stacker::queue::clipboard_markdown(prompt),
        })
        .collect()
}
