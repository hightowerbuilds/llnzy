use std::sync::Arc;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Fullscreen, Window, WindowAttributes, WindowId};

use llnzy::config::Config;
use llnzy::diagnostics::write_diagnostic;
use llnzy::error_log::{ErrorLog, ErrorPanel};
use llnzy::input;
use llnzy::keybindings::Action;
use llnzy::layout::{LayoutInputs, ScreenLayout};
use llnzy::renderer::{RenderRequest, Renderer};
use llnzy::search::Search;
use llnzy::selection::Selection;
use llnzy::session::{Rect as PaneRect, Session};
use llnzy::ui::{ActiveView, PendingClose, SavePromptResponse, UiState, BUMPER_WIDTH};
use llnzy::workspace::{TabContent, WorkspaceTab};
use llnzy::UserEvent;


struct ClickState {
    last_time: Instant,
    last_pos: (usize, usize),
    count: u8,
}

impl ClickState {
    fn new() -> Self {
        Self {
            last_time: Instant::now() - std::time::Duration::from_secs(10),
            last_pos: (usize::MAX, usize::MAX),
            count: 0,
        }
    }

    fn click(&mut self, row: usize, col: usize) -> u8 {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_time);
        let same_pos = self.last_pos == (row, col);
        if same_pos && elapsed.as_millis() < 500 && self.count < 3 {
            self.count += 1;
        } else {
            self.count = 1;
        }
        self.last_time = now;
        self.last_pos = (row, col);
        self.count
    }
}

struct App {
    config: Config,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    tabs: Vec<WorkspaceTab>,
    active_tab: usize,
    next_tab_id: u64,
    proxy: winit::event_loop::EventLoopProxy<UserEvent>,
    modifiers: ModifiersState,
    selection: Selection,
    search: Search,
    error_log: ErrorLog,
    error_panel: ErrorPanel,
    clipboard: Option<arboard::Clipboard>,
    cursor_pos: winit::dpi::PhysicalPosition<f64>,
    mouse_pressed: bool,
    click_state: ClickState,
    ui: Option<UiState>,
    screen_layout: Option<ScreenLayout>,
    visual_bell_until: Option<Instant>,
    last_ui_config_change: Instant,
    last_blink_toggle: Instant,
    last_keypress: Instant,
    last_config_check: Instant,
}

impl App {
    fn new(proxy: winit::event_loop::EventLoopProxy<UserEvent>) -> Self {
        let clipboard = arboard::Clipboard::new().ok();
        Self {
            config: Config::load(),
            window: None,
            renderer: None,
            tabs: Vec::new(),
            active_tab: 0,
            next_tab_id: 1,
            proxy,
            modifiers: ModifiersState::empty(),
            selection: Selection::new(),
            search: Search::new(),
            error_log: ErrorLog::new(),
            error_panel: ErrorPanel::new(),
            clipboard,
            cursor_pos: winit::dpi::PhysicalPosition::new(0.0, 0.0),
            mouse_pressed: false,
            click_state: ClickState::new(),
            ui: None,
            screen_layout: None,
            visual_bell_until: None,
            last_ui_config_change: Instant::now() - std::time::Duration::from_secs(60),
            last_blink_toggle: Instant::now(),
            last_keypress: Instant::now(),
            last_config_check: Instant::now(),
        }
    }

    fn alloc_tab_id(&mut self) -> u64 {
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        id
    }

    fn active_tab(&self) -> Option<&WorkspaceTab> {
        self.tabs.get(self.active_tab)
    }

    fn active_tab_mut(&mut self) -> Option<&mut WorkspaceTab> {
        self.tabs.get_mut(self.active_tab)
    }

    fn active_session(&self) -> Option<&Session> {
        match self.active_tab()?.content {
            TabContent::Terminal(ref s) => Some(s),
            _ => None,
        }
    }

    fn active_session_mut(&mut self) -> Option<&mut Session> {
        match self.active_tab_mut()?.content {
            TabContent::Terminal(ref mut s) => Some(s),
            _ => None,
        }
    }

    fn process_all_output(&mut self) {
        let mut any_changed = false;
        for tab in &mut self.tabs {
            if let TabContent::Terminal(ref mut session) = tab.content {
                let (changed, clip, bell) = session.process_output();
                if changed {
                    any_changed = true;
                }
                if bell {
                    self.visual_bell_until =
                        Some(Instant::now() + std::time::Duration::from_millis(150));
                }
                if let Some(text) = clip {
                    if let Some(cb) = &mut self.clipboard {
                        let _ = cb.set_text(text);
                    }
                }
            }
        }

        // Update window title from active session
        if any_changed {
            if let (Some(window), Some(session)) = (&self.window, self.active_session()) {
                let title = if session.title.is_empty() {
                    "llnzy".to_string()
                } else {
                    format!("{} — llnzy", session.title)
                };
                window.set_title(&title);
            }
            self.request_redraw();
        }

        // Auto-close terminal tabs whose shells have exited
        let before = self.tabs.len();
        self.tabs.retain(|tab| {
            if let TabContent::Terminal(ref session) = tab.content {
                session.exited.is_none()
            } else {
                true // non-terminal tabs never auto-close
            }
        });
        let closed = before - self.tabs.len();
        if closed > 0 {
            self.error_log
                .info(format!("{} tab(s) closed (shell exited)", closed));
        }
        if self.active_tab >= self.tabs.len() && !self.tabs.is_empty() {
            self.active_tab = self.tabs.len() - 1;
            self.request_redraw();
        }
    }

    fn recompute_layout(&mut self) {
        if let Some(renderer) = &self.renderer {
            let (cw, ch) = renderer.cell_dimensions();
            let gox = renderer.glyph_offset_x();
            let w = self
                .window
                .as_ref()
                .map(|w| w.inner_size().width as f32)
                .unwrap_or(900.0);
            let h = self
                .window
                .as_ref()
                .map(|w| w.inner_size().height as f32)
                .unwrap_or(600.0);
            let sidebar_w = self.ui.as_ref().map(|u| u.sidebar_width()).unwrap_or(0.0);
            self.screen_layout = Some(ScreenLayout::compute(LayoutInputs {
                window_w: w,
                window_h: h,
                cell_w: cw,
                cell_h: ch,
                padding_x: self.config.padding_x,
                padding_y: self.config.padding_y,
                glyph_offset_x: gox,
                sidebar_w,
            }));
        }
    }

    fn grid_size(&self) -> (u16, u16) {
        if let Some(layout) = &self.screen_layout {
            (layout.grid_cols, layout.grid_rows)
        } else {
            (80, 24)
        }
    }

    fn pixel_to_grid(&self, pos: winit::dpi::PhysicalPosition<f64>) -> (usize, usize) {
        if let Some(layout) = &self.screen_layout {
            layout.pixel_to_grid(pos.x as f32, pos.y as f32)
        } else {
            (0, 0)
        }
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn write_to_active(&mut self, data: &[u8]) {
        if let Some(session) = self.active_session_mut() {
            session.write(data);
        }
    }

    fn paste_text(&mut self, text: &str) {
        let bracketed = self
            .active_session()
            .is_some_and(|s| s.terminal.bracketed_paste());
        if bracketed {
            let mut bytes = Vec::with_capacity(text.len() + 12);
            bytes.extend_from_slice(b"\x1b[200~");
            bytes.extend_from_slice(text.as_bytes());
            bytes.extend_from_slice(b"\x1b[201~");
            self.write_to_active(&bytes);
        } else {
            self.write_to_active(text.as_bytes());
        }
    }

    fn copy_selection(&mut self) {
        if self.selection.is_active() {
            if let Some(session) = self.active_session() {
                let text = self.selection.text(&session.terminal);
                if let Some(cb) = &mut self.clipboard {
                    let _ = cb.set_text(text);
                }
            }
            self.selection.clear();
            self.request_redraw();
        }
    }

    /// Resize all terminal sessions to the current content rect.
    fn resize_terminal_tabs(&mut self) {
        if let Some(layout) = &self.screen_layout {
            let (cols, rows) = (layout.grid_cols, layout.grid_rows);
            for tab in &mut self.tabs {
                if let TabContent::Terminal(ref mut session) = tab.content {
                    session.resize(cols, rows);
                }
            }
        }
    }

    #[allow(dead_code)] // Will be used in Phase 3 (conditional rendering)
    fn content_rect(&self) -> Option<PaneRect> {
        self.screen_layout.as_ref().map(|l| PaneRect {
            x: l.content.x,
            y: l.content.y,
            w: l.content.w,
            h: l.content.h,
        })
    }

    fn mouse_reporting(&self) -> bool {
        self.active_session()
            .is_some_and(|s| s.terminal.mouse_mode())
    }

    fn app_cursor(&self) -> bool {
        self.active_session()
            .is_some_and(|s| s.terminal.app_cursor())
    }

    fn sgr_mouse(&self) -> bool {
        self.active_session()
            .is_some_and(|s| s.terminal.sgr_mouse())
    }

    fn invalidate_and_redraw(&mut self) {
        if let Some(r) = &mut self.renderer {
            r.invalidate_text_cache();
        }
        self.request_redraw();
    }

    fn do_paste(&mut self) {
        if let Some(cb) = &mut self.clipboard {
            if let Ok(text) = cb.get_text() {
                self.paste_text(&text);
            }
        }
    }

    fn do_select_all(&mut self) {
        if let Some(s) = self.active_session() {
            let (cols, rows) = s.terminal.size();
            self.selection.select_all(rows, cols);
        }
        self.request_redraw();
    }

    fn toggle_effects(&mut self) {
        self.config.effects.enabled = !self.config.effects.enabled;
        if let Some(renderer) = &mut self.renderer {
            renderer.update_config(self.config.clone());
        }
        self.request_redraw();
    }

    fn new_tab(&mut self) {
        let (cols, rows) = self.grid_size();
        // Use CWD from active terminal, or fall back to the project root
        let cwd = self
            .active_session()
            .and_then(|s| s.cwd.clone())
            .or_else(|| {
                self.ui.as_ref().and_then(|ui| {
                    let root = &ui.explorer.root;
                    if !ui.explorer.tree.is_empty() {
                        Some(root.to_string_lossy().into_owned())
                    } else {
                        None
                    }
                })
            });
        match Session::new_in_dir(cols, rows, &self.config, self.proxy.clone(), cwd.as_deref()) {
            Ok(session) => {
                let id = self.alloc_tab_id();
                self.tabs.push(WorkspaceTab {
                    content: TabContent::Terminal(Box::new(session)),
                    name: None,
                    id,
                });
                self.active_tab = self.tabs.len() - 1;
                self.selection.clear();
                self.recompute_layout();
                self.request_redraw();
            }
            Err(e) => self.error_log.error(format!("Failed to create tab: {}", e)),
        }
    }

    fn close_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        // Check for unsaved CodeFile buffer
        if let TabContent::CodeFile { buffer_idx, .. } = &self.tabs[self.active_tab].content {
            if let Some(ui) = &self.ui {
                if *buffer_idx < ui.editor_view.editor.buffers.len()
                    && ui.editor_view.editor.buffers[*buffer_idx].is_modified()
                {
                    let name = ui.editor_view.editor.buffers[*buffer_idx].file_name().to_string();
                    if let Some(ui) = &mut self.ui {
                        ui.pending_close = Some(PendingClose::Tab(self.active_tab, name));
                    }
                    self.request_redraw();
                    return;
                }
            }
        }
        self.force_close_tab();
    }

    fn force_close_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        self.tabs.remove(self.active_tab);
        if self.tabs.is_empty() {
            self.active_tab = 0;
            // Return to Home when all tabs are closed
            if let Some(ui) = &mut self.ui {
                ui.active_view = ActiveView::Home;
            }
        } else if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
        self.selection.clear();
        self.recompute_layout();
        self.request_redraw();
    }

    fn switch_tab(&mut self, idx: usize) {
        if idx < self.tabs.len() && idx != self.active_tab {
            self.active_tab = idx;
            self.selection.clear();
            self.invalidate_and_redraw();
        }
    }

    fn toggle_fullscreen(&self) {
        if let Some(window) = &self.window {
            if window.fullscreen().is_some() {
                window.set_fullscreen(None);
            } else {
                window.set_fullscreen(Some(Fullscreen::Borderless(None)));
            }
        }
    }

    /// Save window position and size for persistence.
    fn save_window_state(&self) {
        let Some(window) = &self.window else { return };
        let size = window.inner_size();
        let pos = window.outer_position().ok();

        let Some(config_dir) = dirs::config_dir() else {
            return;
        };
        let state_path = config_dir.join("llnzy").join("window_state.toml");
        let _ = std::fs::create_dir_all(config_dir.join("llnzy"));

        let mut content = format!("width = {}\nheight = {}\n", size.width, size.height);
        if let Some(pos) = pos {
            content.push_str(&format!("x = {}\ny = {}\n", pos.x, pos.y));
        }
        let _ = std::fs::write(state_path, content);
    }

    fn tab_titles(&self) -> Vec<(String, bool)> {
        self.tabs
            .iter()
            .enumerate()
            .map(|(i, tab)| {
                let title = tab.display_name(i);
                (title, i == self.active_tab)
            })
            .collect()
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        // Try to restore saved window size
        let saved_size = load_window_state();
        let inner_size = saved_size
            .map(|(w, h)| winit::dpi::PhysicalSize::new(w, h))
            .unwrap_or(winit::dpi::PhysicalSize::new(900, 600));

        let mut attrs = WindowAttributes::default()
            .with_title("llnzy")
            .with_inner_size(inner_size);

        if let Ok(image) = image::load_from_memory(include_bytes!("../llnzy.jpg")) {
            let image = image.to_rgba8();
            let (width, height) = image.dimensions();
            if let Ok(icon) = winit::window::Icon::from_rgba(image.into_raw(), width, height) {
                attrs = attrs.with_window_icon(Some(icon));
            }
        }

        if self.config.opacity < 1.0 {
            attrs = attrs.with_transparent(true);
        }

        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        // Wispr Flow and input methods can deliver text through AppKit's IME/text-input path.
        // winit disables that by default, so opt in for terminal text entry.
        window.set_ime_allowed(true);
        let renderer = pollster::block_on(Renderer::new(window.clone(), self.config.clone()));

        let (cw, ch) = renderer.cell_dimensions();
        let gox = renderer.glyph_offset_x();
        let size = window.inner_size();
        let layout = ScreenLayout::compute(LayoutInputs {
            window_w: size.width as f32,
            window_h: size.height as f32,
            cell_w: cw,
            cell_h: ch,
            padding_x: self.config.padding_x,
            padding_y: self.config.padding_y,
            glyph_offset_x: gox,
            sidebar_w: BUMPER_WIDTH, // bumper always visible
        });
        self.screen_layout = Some(layout);

        let ui_state = UiState::new(
            &window,
            renderer.gpu_device(),
            renderer.gpu_surface_format(),
        );

        self.window = Some(window);
        self.renderer = Some(renderer);
        self.ui = Some(ui_state);

        // Set up native macOS menu bar
        #[cfg(target_os = "macos")]
        llnzy::menu::setup_menu_bar(self.proxy.clone());
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // Route events to egui first
        if let (Some(window), Some(ui)) = (&self.window, &mut self.ui) {
            let response = ui.handle_event(window, &event);
            // Sketch owns raw canvas input; do not let unconsumed pointer/text events leak
            // into the terminal while that workspace is active.
            if ui.captures_terminal_input() && terminal_input_event(&event) {
                self.request_redraw();
                return;
            }
            // If egui consumed a mouse/keyboard event, don't pass to terminal.
            // The footer and bumper are always visible, so any egui-consumed
            // event must be respected regardless of which view is active.
            if response {
                self.request_redraw();
                match &event {
                    WindowEvent::CloseRequested | WindowEvent::Resized(_) => {}
                    _ => return,
                }
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                // Check for unsaved CodeFile buffers before quitting
                let mut modified = Vec::new();
                if let Some(ui) = &self.ui {
                    for (i, tab) in self.tabs.iter().enumerate() {
                        if let TabContent::CodeFile { buffer_idx, .. } = &tab.content {
                            if *buffer_idx < ui.editor_view.editor.buffers.len()
                                && ui.editor_view.editor.buffers[*buffer_idx].is_modified()
                            {
                                let name = ui.editor_view.editor.buffers[*buffer_idx].file_name().to_string();
                                modified.push((i, name));
                            }
                        }
                    }
                }
                if !modified.is_empty() {
                    if let Some(ui) = &mut self.ui {
                        ui.pending_close = Some(PendingClose::Window(modified));
                    }
                    self.request_redraw();
                } else {
                    self.error_log.info("Close requested");
                    self.save_window_state();
                    event_loop.exit();
                }
            }

            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.set_scale_factor(scale_factor as f32);
                }
            }

            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                    renderer.invalidate_text_cache();
                }
                self.recompute_layout();
                self.resize_terminal_tabs();
                self.selection.clear();
            }

            WindowEvent::RedrawRequested => {
                self.process_all_output();

                // Feed frame time to UI for FPS overlay
                if let (Some(renderer), Some(ui)) = (&self.renderer, &mut self.ui) {
                    ui.record_frame_time(renderer.gpu_delta_time());
                }

                let bell_active = self.visual_bell_until.is_some_and(|t| Instant::now() < t);
                if bell_active {
                    self.request_redraw();
                } else {
                    self.visual_bell_until = None;
                }

                let titles = self.tab_titles();
                let (cw, ch) = self
                    .renderer
                    .as_ref()
                    .map(|r| r.cell_dimensions())
                    .unwrap_or((1.0, 1.0));
                let sel_info = self
                    .active_session()
                    .map(|session| {
                        let cols = session.terminal.size().0;
                        self.selection.rects(
                            cw,
                            ch,
                            cols,
                            self.config.colors.selection,
                            self.config.colors.selection_alpha,
                        )
                    })
                    .unwrap_or_default();
                let search_rects = self.search.highlight_rects(cw, ch);
                let search_bar = if self.search.active {
                    Some((self.search.query.as_str(), self.search.status()))
                } else {
                    None
                };
                let search_bar_ref = search_bar.as_ref().map(|(q, s)| (*q, s.as_str()));

                let err_panel = if self.error_panel.visible {
                    Some((&self.error_panel, &self.error_log))
                } else {
                    None
                };

                // Snapshot sidebar width before egui render so we can detect bumper clicks
                let sidebar_w_before = self.ui.as_ref().map(|u| u.sidebar_width()).unwrap_or(0.0);

                if let Some(renderer) = &mut self.renderer {
                    if let Some(layout) = &self.screen_layout {
                        let active_kind = self.tabs.get(self.active_tab).map(|t| t.content.kind());
                        let effects_on_ui = match active_kind {
                            Some(llnzy::workspace::TabKind::Terminal) => self.config.effects.effects_on_ui,
                            Some(llnzy::workspace::TabKind::Sketch) => true,
                            Some(llnzy::workspace::TabKind::CodeFile) => self.config.effects.effects_on_ui,
                            _ => false,
                        };
                        let effects_mask = self.ui.as_ref().and_then(|u| {
                            u.sketch_canvas_px.and_then(|px| {
                                if matches!(active_kind, Some(llnzy::workspace::TabKind::Sketch)) {
                                    let size = self.window.as_ref()?.inner_size();
                                    let w = size.width as f32;
                                    let h = size.height as f32;
                                    Some([px[0] / w, px[1] / h, px[2] / w, px[3] / h])
                                } else {
                                    None
                                }
                            })
                        });
                        // Supply clipboard content to editor for paste + init LSP
                        if let Some(ui) = self.ui.as_mut() {
                            if let Some(cb) = &mut self.clipboard {
                                if let Ok(text) = cb.get_text() {
                                    ui.editor_view.clipboard_in = Some(text);
                                }
                            }
                            ui.editor_view.init_lsp(self.proxy.clone());
                        }
                        if let Some(ui) = self.ui.as_mut() {
                            ui.set_tab_context(self.tabs.len(), self.active_tab);
                            ui.active_tab_kind = self.tabs.get(self.active_tab).map(|t| t.content.kind());
                            // Populate tab names for egui tab bar
                            ui.tab_names = titles.clone();
                        }

                        // Get the active terminal session (if any) for the renderer
                        let active_tab = self.tabs.get(self.active_tab);
                        let terminal_session = active_tab.and_then(|t| t.content.as_terminal());
                        let tab_id = active_tab.map(|t| t.id).unwrap_or(0);

                        let ui_state = &mut self.ui;
                        let window_ref = &self.window;
                        let config_ref = &self.config;
                        let mut egui_cb =
                            |device: &wgpu::Device,
                             queue: &wgpu::Queue,
                             view: &wgpu::TextureView,
                             desc: egui_wgpu::ScreenDescriptor| {
                                if let (Some(ui), Some(window)) =
                                    (ui_state.as_mut(), window_ref.as_ref())
                                {
                                    ui.render(window, device, queue, view, desc, config_ref);
                                }
                            };
                        renderer.render(RenderRequest {
                            terminal: terminal_session,
                            tab_id,
                            tab_titles: &titles,
                            selection_rects: &sel_info,
                            search_rects: &search_rects,
                            search_bar: search_bar_ref,
                            error_panel: err_panel,
                            visual_bell: bell_active,
                            screen_layout: layout,
                            egui_render: Some(&mut egui_cb),
                            apply_effects_to_ui: effects_on_ui,
                            effects_mask,
                        });
                    }
                }

                // Detect sidebar state change from bumper click and recompute layout
                let sidebar_w_after = self.ui.as_ref().map(|u| u.sidebar_width()).unwrap_or(0.0);
                if (sidebar_w_after - sidebar_w_before).abs() > 0.1 {
                    self.recompute_layout();
                    self.resize_terminal_tabs();
                    self.selection.clear();
                    self.invalidate_and_redraw();
                }

                // Apply config changes and tab renames from UI after render
                let mut need_redraw = false;
                let mut sidebar_changed = false;
                let mut clip_text: Option<String> = None;
                let mut tab_rename: Option<(usize, String)> = None;
                // Extract pending actions before borrowing ui for other state
                let footer_action = self.ui.as_mut().and_then(|u| u.footer_action.take());
                let pending_file = self.ui.as_mut().and_then(|u| u.editor_view.pending_file_tab.take());
                let pending_task = self.ui.as_mut().and_then(|u| u.editor_view.pending_task.take());
                let tab_bar_action = self.ui.as_mut().and_then(|u| u.tab_bar_action.take());
                let save_response = self.ui.as_mut().and_then(|u| u.save_prompt_response.take());

                // Handle save prompt response
                if let Some(resp) = save_response {
                    let pending = self.ui.as_mut().and_then(|u| u.pending_close.take());
                    match (resp, pending) {
                        (SavePromptResponse::Save, Some(PendingClose::Tab(idx, _))) => {
                            // Save the buffer, then close the tab
                            if let Some(ui) = &mut self.ui {
                                if let Some(TabContent::CodeFile { buffer_idx, .. }) = self.tabs.get(idx).map(|t| &t.content) {
                                    if let Some(buf) = ui.editor_view.editor.buffers.get_mut(*buffer_idx) {
                                        let _ = buf.save();
                                    }
                                }
                            }
                            self.active_tab = idx;
                            self.force_close_tab();
                            need_redraw = true;
                        }
                        (SavePromptResponse::DontSave, Some(PendingClose::Tab(idx, _))) => {
                            self.active_tab = idx;
                            self.force_close_tab();
                            need_redraw = true;
                        }
                        (SavePromptResponse::Save, Some(PendingClose::Window(_tabs))) => {
                            // Save all modified buffers, then exit
                            if let Some(ui) = &mut self.ui {
                                for buf in &mut ui.editor_view.editor.buffers {
                                    if buf.is_modified() {
                                        let _ = buf.save();
                                    }
                                }
                            }
                            self.save_window_state();
                            event_loop.exit();
                        }
                        (SavePromptResponse::DontSave, Some(PendingClose::Window(_))) => {
                            self.save_window_state();
                            event_loop.exit();
                        }
                        (SavePromptResponse::Cancel, pending) => {
                            // Put it back as None (already taken)
                            drop(pending);
                            need_redraw = true;
                        }
                        _ => {}
                    }
                }

                // Handle egui tab bar clicks
                if let Some(action) = tab_bar_action {
                    if let Some(idx) = action.close_tab {
                        self.active_tab = idx;
                        self.close_tab();
                        need_redraw = true;
                    } else if let Some(idx) = action.switch_to {
                        self.switch_tab(idx);
                        need_redraw = true;
                    }
                }
                if let Some(ui) = &mut self.ui {
                    if let Some(new_config) = ui.take_config() {
                        self.config = new_config;
                        if let Some(renderer) = &mut self.renderer {
                            renderer.update_config(self.config.clone());
                        }
                        self.last_ui_config_change = Instant::now();
                        need_redraw = true;
                    }
                    clip_text = ui.clipboard_text.take();
                    tab_rename = ui.take_saved_tab_name();
                    // Handle "Open Project" from Home screen
                    if let Some(project_path) = ui.open_project.take() {
                        ui.explorer.set_root(project_path.clone());
                        llnzy::explorer::add_recent_project(&mut ui.recent_projects, project_path);
                        ui.sidebar_open = true;
                        sidebar_changed = true;
                        need_redraw = true;
                    }
                }
                // Handle footer singleton tab actions (after releasing ui borrow)
                if let Some(llnzy::ui::FooterAction::NewTerminalTab) = &footer_action {
                    self.new_tab();
                    if let Some(ui) = &mut self.ui {
                        ui.active_view = ActiveView::Shells;
                    }
                    need_redraw = true;
                }
                if let Some(llnzy::ui::FooterAction::OpenSingletonTab(kind)) = footer_action {
                    use llnzy::workspace::{find_singleton, TabKind};
                    if let Some(idx) = find_singleton(&self.tabs, kind) {
                        self.active_tab = idx;
                    } else {
                        let id = self.alloc_tab_id();
                        let content = match kind {
                            TabKind::Stacker => TabContent::Stacker,
                            TabKind::Sketch => TabContent::Sketch,
                            TabKind::Appearances => TabContent::Appearances,
                            TabKind::Settings => TabContent::Settings,
                            _ => unreachable!(),
                        };
                        self.tabs.push(WorkspaceTab {
                            content,
                            name: None,
                            id,
                        });
                        self.active_tab = self.tabs.len() - 1;
                    }
                    // Dismiss any overlay
                    if let Some(ui) = &mut self.ui {
                        ui.active_view = ActiveView::Shells;
                    }
                    need_redraw = true;
                }
                // Handle file opened from sidebar → create CodeFile tab
                if let Some((path, buffer_idx)) = pending_file {
                    // Check if a tab for this file already exists
                    let existing = self.tabs.iter().position(|t| {
                        matches!(&t.content, TabContent::CodeFile { path: p, .. } if *p == path)
                    });
                    if let Some(idx) = existing {
                        self.active_tab = idx;
                    } else {
                        let id = self.alloc_tab_id();
                        self.tabs.push(WorkspaceTab {
                            content: TabContent::CodeFile { path, buffer_idx },
                            name: None,
                            id,
                        });
                        self.active_tab = self.tabs.len() - 1;
                    }
                    // Dismiss any overlay to show the file
                    if let Some(ui) = &mut self.ui {
                        ui.active_view = ActiveView::Shells;
                    }
                    need_redraw = true;
                }
                // Handle pending task: create a terminal tab that runs the command
                if let Some(task) = pending_task {
                    let (cols, rows) = self.grid_size();
                    let cwd = task.cwd.to_string_lossy().to_string();
                    // Build the command string to send to the shell
                    let cmd_str = if task.args.is_empty() {
                        format!("{}\n", task.command)
                    } else {
                        format!("{} {}\n", task.command, task.args.join(" "))
                    };
                    match Session::new_in_dir(cols, rows, &self.config, self.proxy.clone(), Some(&cwd)) {
                        Ok(mut session) => {
                            // Send the command to the shell
                            session.write(cmd_str.as_bytes());
                            let id = self.alloc_tab_id();
                            self.tabs.push(WorkspaceTab {
                                content: TabContent::Terminal(Box::new(session)),
                                name: Some(task.name),
                                id,
                            });
                            self.active_tab = self.tabs.len() - 1;
                            if let Some(ui) = &mut self.ui {
                                ui.active_view = ActiveView::Shells;
                            }
                            self.recompute_layout();
                            need_redraw = true;
                        }
                        Err(e) => self.error_log.error(format!("Failed to run task: {e}")),
                    }
                }
                // Recompute layout if sidebar changed via Open Project
                if sidebar_changed {
                    self.recompute_layout();
                    self.resize_terminal_tabs();
                }
                if need_redraw {
                    self.request_redraw();
                }
                if let Some(text) = clip_text {
                    if let Some(cb) = &mut self.clipboard {
                        let _ = cb.set_text(text);
                    }
                }
                // Apply tab name changes
                if let Some((tab_idx, new_name)) = tab_rename {
                    if let Some(tab) = self.tabs.get_mut(tab_idx) {
                        if new_name.trim().is_empty() {
                            tab.name = None; // Clear custom name to show default
                        } else {
                            tab.name = Some(new_name);
                        }
                        self.request_redraw();
                    }
                }
            }

            WindowEvent::ModifiersChanged(mods) => {
                self.modifiers = mods.state();
            }

            // --- Mouse wheel ---
            WindowEvent::MouseWheel { delta, .. } => {
                if self.mouse_reporting() {
                    let (row, col) = self.pixel_to_grid(self.cursor_pos);
                    let sgr = self.sgr_mouse();
                    let lines = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y as i32,
                        MouseScrollDelta::PixelDelta(p) => {
                            let (_, ch) = self
                                .renderer
                                .as_ref()
                                .map(|r| r.cell_dimensions())
                                .unwrap_or((1.0, 1.0));
                            (p.y / ch as f64) as i32
                        }
                    };
                    for _ in 0..lines.unsigned_abs() {
                        let button = if lines > 0 { 64 } else { 65 };
                        let bytes =
                            input::encode_mouse(button, col, row, true, sgr, &self.modifiers);
                        self.write_to_active(&bytes);
                    }
                } else {
                    let scroll_mult = self.config.scroll_lines as f32;
                    let lines = match delta {
                        MouseScrollDelta::LineDelta(_, y) => (y * scroll_mult) as i32,
                        MouseScrollDelta::PixelDelta(pos) => {
                            let (_, ch) = self
                                .renderer
                                .as_ref()
                                .map(|r| r.cell_dimensions())
                                .unwrap_or((1.0, 1.0));
                            (pos.y / ch as f64) as i32
                        }
                    };
                    if lines != 0 {
                        if let Some(session) = self.active_session_mut() {
                            session.terminal.scroll(lines);
                        }
                        self.invalidate_and_redraw();
                    }
                }
            }

            // --- Mouse buttons ---
            WindowEvent::MouseInput { state, button, .. } => {
                let (row, col) = self.pixel_to_grid(self.cursor_pos);




                if button == MouseButton::Right && state == ElementState::Pressed {
                    if self.selection.is_active() {
                        self.copy_selection();
                    } else if let Some(cb) = &mut self.clipboard {
                        if let Ok(text) = cb.get_text() {
                            self.paste_text(&text);
                        }
                    }
                    return;
                }

                if button == MouseButton::Left
                    && state == ElementState::Pressed
                    && self.modifiers.super_key()
                {
                    if let Some(session) = self.active_session() {
                        // Try to extract a file:line:col pattern from the line
                        let line_text = {
                            let (cols, _) = session.terminal.size();
                            (0..cols).map(|c| session.terminal.cell_char(row, c)).collect::<String>()
                        };
                        let file_loc = parse_file_location(&line_text, col);

                        if let Some((path, line, col_num)) = file_loc {
                            // Open in editor
                            if let Some(ui) = &mut self.ui {
                                match ui.editor_view.open_file(path) {
                                    Ok(idx) => {
                                        let view = &mut ui.editor_view.editor.views[idx];
                                        view.cursor.pos = llnzy::editor::buffer::Position::new(
                                            line.saturating_sub(1),
                                            col_num.saturating_sub(1),
                                        );
                                        view.cursor.clear_selection();
                                        view.cursor.desired_col = None;
                                        // Dismiss any overlay to show the editor
                                        ui.active_view = ActiveView::Shells;
                                    }
                                    Err(e) => {
                                        self.error_log.error(format!("Cannot open file: {e}"));
                                    }
                                }
                                self.request_redraw();
                            }
                            return;
                        }

                        // Fall back to URL detection
                        let url = session.terminal.cell_hyperlink(row, col).or_else(|| {
                            let text = Selection::word_at(row, col, &session.terminal);
                            if text.starts_with("http://") || text.starts_with("https://") {
                                Some(text)
                            } else {
                                None
                            }
                        });
                        if let Some(url) = url {
                            let _ = std::process::Command::new("open").arg(&url).spawn();
                        }
                    }
                    return;
                }

                if self.mouse_reporting() && button == MouseButton::Left {
                    let sgr = self.sgr_mouse();
                    let pressed = state == ElementState::Pressed;
                    let bytes = input::encode_mouse(0, col, row, pressed, sgr, &self.modifiers);
                    self.write_to_active(&bytes);
                    self.mouse_pressed = pressed;
                    return;
                }

                if button == MouseButton::Left {
                    match state {
                        ElementState::Pressed => {
                            let click_count = self.click_state.click(row, col);
                            match click_count {
                                2 => {
                                    if let Some(terminal) = self.tabs.get(self.active_tab).and_then(|t| t.content.as_terminal()).map(|s| &s.terminal) {
                                        self.selection.select_word(row, col, terminal);
                                    }
                                }
                                3 => {
                                    let cols = self
                                        .active_session()
                                        .map(|s| s.terminal.size().0)
                                        .unwrap_or(80);
                                    self.selection.select_line(row, cols);
                                }
                                _ => self.selection.start(row, col),
                            }
                            self.mouse_pressed = true;
                            self.request_redraw();
                        }
                        ElementState::Released => self.mouse_pressed = false,
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = position;

                if self.mouse_pressed {
                    let (row, col) = self.pixel_to_grid(position);
                    if self.mouse_reporting() {
                        let sgr = self.sgr_mouse();
                        let bytes = input::encode_mouse(32, col, row, true, sgr, &self.modifiers);
                        self.write_to_active(&bytes);
                    } else if self.click_state.count <= 1 {
                        self.selection.update(row, col);
                        self.request_redraw();
                    }
                }
            }

            // --- Keyboard ---
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                if key_event.state != ElementState::Pressed {
                    return;
                }

                // When error panel is visible, arrow keys scroll it
                if self.error_panel.visible {
                    match &key_event.logical_key {
                        Key::Named(NamedKey::Escape) => {
                            self.error_panel.toggle();
                            self.request_redraw();
                            return;
                        }
                        Key::Named(NamedKey::ArrowUp) | Key::Named(NamedKey::PageUp) => {
                            self.error_panel.scroll_up();
                            self.request_redraw();
                            return;
                        }
                        Key::Named(NamedKey::ArrowDown) | Key::Named(NamedKey::PageDown) => {
                            self.error_panel.scroll_down();
                            self.request_redraw();
                            return;
                        }
                        _ => {} // Other keys pass through to terminal
                    }
                }

                // When search bar is active, route keys to search
                if self.search.active {
                    match &key_event.logical_key {
                        Key::Named(NamedKey::Escape) => {
                            self.search.close();
                            self.request_redraw();
                            return;
                        }
                        Key::Named(NamedKey::Enter) => {
                            if self.modifiers.shift_key() {
                                self.search.prev();
                            } else {
                                self.search.next();
                            }
                            // Scroll to focused match
                            if let Some(m) = self.search.focused_match() {
                                let target_row = m.row;
                                if let Some(session) = self.active_session_mut() {
                                    let (_, rows) = session.terminal.size();
                                    if target_row >= rows {
                                        session.terminal.scroll_to_bottom();
                                    }
                                }
                            }
                            self.request_redraw();
                            return;
                        }
                        Key::Named(NamedKey::Backspace) => {
                            if let Some(terminal) = self.tabs.get(self.active_tab).and_then(|t| t.content.as_terminal()).map(|s| &s.terminal) {
                                self.search.pop_char(terminal);
                            }
                            self.request_redraw();
                            return;
                        }
                        _ => {
                            // Ctrl+R toggles regex mode
                            if self.modifiers.control_key() {
                                if let Key::Character(c) = &key_event.logical_key {
                                    if c.as_str() == "r" {
                                        self.search.toggle_regex();
                                        if let Some(terminal) = self.tabs.get(self.active_tab).and_then(|t| t.content.as_terminal()).map(|s| &s.terminal) {
                                            self.search.update_matches(terminal);
                                        }
                                        self.request_redraw();
                                        return;
                                    }
                                }
                            }
                            // Type into search query
                            if let Some(ref text) = key_event.text {
                                for ch in text.chars() {
                                    if !ch.is_control() {
                                        if let Some(terminal) = self.tabs.get(self.active_tab).and_then(|t| t.content.as_terminal()).map(|s| &s.terminal) {
                                            self.search.push_char(ch, terminal);
                                        }
                                    }
                                }
                                self.request_redraw();
                            }
                            return;
                        }
                    }
                }

                // Dispatch through keybinding registry
                if let Some(action) = self
                    .config
                    .keybindings
                    .match_key(&key_event, self.modifiers)
                {
                    match action {
                        Action::Search => {
                            // Search only works on terminal tabs
                            if self.active_session().is_some() {
                                self.search.open();
                                self.request_redraw();
                            }
                        }
                        Action::Copy => self.copy_selection(),
                        Action::Paste => self.do_paste(),
                        Action::SelectAll => self.do_select_all(),
                        Action::NewTab => self.new_tab(),
                        Action::CloseTab => self.close_tab(),
                        Action::NextTab => {
                            let next = (self.active_tab + 1) % self.tabs.len().max(1);
                            self.switch_tab(next);
                        }
                        Action::PrevTab => {
                            let prev = if self.active_tab == 0 {
                                self.tabs.len().saturating_sub(1)
                            } else {
                                self.active_tab - 1
                            };
                            self.switch_tab(prev);
                        }
                        Action::SplitVertical | Action::SplitHorizontal => {
                            // Split panes removed — open a new terminal tab instead
                            self.new_tab();
                        }
                        Action::ToggleFullscreen => self.toggle_fullscreen(),
                        Action::ToggleEffects => self.toggle_effects(),
                        Action::ToggleFps => {
                            if let Some(ui) = &mut self.ui {
                                ui.show_fps = !ui.show_fps;
                            }
                            self.request_redraw();
                        }
                        Action::ToggleErrorPanel => {
                            self.error_panel.toggle();
                            self.request_redraw();
                        }
                        Action::ToggleSidebar => {
                            if let Some(ui) = &mut self.ui {
                                ui.toggle_sidebar();
                            }
                            self.recompute_layout();
                            self.resize_terminal_tabs();
                            self.selection.clear();
                            self.invalidate_and_redraw();
                        }
                        Action::CyclePaneForward | Action::CyclePaneBackward => {
                            // Pane cycling removed — these are no-ops now
                        }
                        Action::ScrollPageUp => {
                            if !self.mouse_reporting() {
                                if let Some(s) = self.active_session_mut() {
                                    s.terminal.scroll_page_up();
                                }
                                self.invalidate_and_redraw();
                            }
                        }
                        Action::ScrollPageDown => {
                            if !self.mouse_reporting() {
                                if let Some(s) = self.active_session_mut() {
                                    s.terminal.scroll_page_down();
                                }
                                self.invalidate_and_redraw();
                            }
                        }
                        Action::ToggleTerminalPanel => {
                            // Terminal panel in explorer removed — no-op
                        }
                        Action::ZoomIn => {
                            self.config.font_size = (self.config.font_size + 1.0).min(40.0);
                            if let Some(renderer) = &mut self.renderer {
                                renderer.update_config(self.config.clone());
                                renderer.invalidate_text_cache();
                            }
                            self.recompute_layout();
                            self.resize_terminal_tabs();
                            self.invalidate_and_redraw();
                        }
                        Action::ZoomOut => {
                            self.config.font_size = (self.config.font_size - 1.0).max(8.0);
                            if let Some(renderer) = &mut self.renderer {
                                renderer.update_config(self.config.clone());
                                renderer.invalidate_text_cache();
                            }
                            self.recompute_layout();
                            self.resize_terminal_tabs();
                            self.invalidate_and_redraw();
                        }
                        Action::ZoomReset => {
                            self.config.font_size = 14.0;
                            if let Some(renderer) = &mut self.renderer {
                                renderer.update_config(self.config.clone());
                                renderer.invalidate_text_cache();
                            }
                            self.recompute_layout();
                            self.resize_terminal_tabs();
                            self.invalidate_and_redraw();
                        }
                        Action::SwitchTab(n) => {
                            self.switch_tab((n - 1) as usize);
                        }
                    }
                    return;
                }

                // Only send raw keys to PTY if active tab is a terminal
                if self.active_session().is_some() {
                    if self.selection.is_active() {
                        self.selection.clear();
                        self.request_redraw();
                    }

                    self.last_keypress = Instant::now();
                    let app_cursor = self.app_cursor();
                    if let Some(bytes) = input::encode_key(&key_event, self.modifiers, app_cursor)
                    {
                        self.write_to_active(&bytes);
                    }
                }
            }

            // IME (input method) events — handles composed text from
            // non-US keyboards, dead keys, and CJK input methods.
            WindowEvent::Ime(ime) => {
                if self.active_session().is_some() {
                    match ime {
                        Ime::Commit(text) => {
                            if self.search.active {
                                if let Some(terminal) = self.tabs.get(self.active_tab).and_then(|t| t.content.as_terminal()).map(|s| &s.terminal) {
                                    for ch in text.chars() {
                                        if !ch.is_control() {
                                            self.search.push_char(ch, terminal);
                                        }
                                    }
                                }
                                self.request_redraw();
                            } else {
                                self.write_to_active(text.as_bytes());
                            }
                        }
                        Ime::Preedit(_, _) => {}
                        Ime::Enabled | Ime::Disabled => {}
                    }
                }
            }

            // Drag-and-drop: insert escaped file path into terminal
            WindowEvent::DroppedFile(path) => {
                if self.active_session().is_some() {
                    let path_str = path.to_string_lossy();
                    let escaped = format!("'{}'", path_str.replace('\'', "'\\''"));
                    self.write_to_active(escaped.as_bytes());
                    self.write_to_active(b" ");
                }
            }

            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::PtyOutput => self.request_redraw(),
            UserEvent::LspMessage => self.request_redraw(),
            UserEvent::FileChanged(_) => self.request_redraw(),
            #[cfg(target_os = "macos")]
            UserEvent::MenuAction(action) => {
                use llnzy::menu::MenuAction;
                match action {
                    MenuAction::NewTab => self.new_tab(),
                    MenuAction::CloseTab => self.close_tab(),
                    MenuAction::Copy => self.copy_selection(),
                    MenuAction::Paste => self.do_paste(),
                    MenuAction::SelectAll => self.do_select_all(),
                    MenuAction::Find => {
                        self.search.open();
                        self.request_redraw();
                    }
                    MenuAction::ToggleFullscreen => self.toggle_fullscreen(),
                    MenuAction::SplitVertical | MenuAction::SplitHorizontal => self.new_tab(),
                    MenuAction::ToggleEffects => self.toggle_effects(),
                    MenuAction::OpenProject => {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_title("Open Project Folder")
                            .pick_folder()
                        {
                            if let Some(ui) = &mut self.ui {
                                ui.explorer.set_root(path.clone());
                                llnzy::explorer::add_recent_project(
                                    &mut ui.recent_projects,
                                    path,
                                );
                                ui.sidebar_open = true;
                                ui.active_view = ActiveView::Shells;
                            }
                            self.recompute_layout();
                            self.resize_terminal_tabs();
                            self.request_redraw();
                        }
                    }
                    MenuAction::CloseProject => {
                        if let Some(ui) = &mut self.ui {
                            ui.explorer.clear();
                            ui.sidebar_open = false;
                            ui.active_view = ActiveView::Home;
                        }
                        // Close all tabs
                        self.tabs.clear();
                        self.active_tab = 0;
                        self.recompute_layout();
                        self.request_redraw();
                    }
                }
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.process_all_output();

        let now = Instant::now();

        // Cursor blink
        let blink_ms = self.config.cursor_blink_ms;
        if blink_ms > 0 {
            let since_key = now.duration_since(self.last_keypress).as_millis() as u64;
            if since_key < blink_ms {
                if let Some(r) = &mut self.renderer {
                    if !r.cursor_visible {
                        r.cursor_visible = true;
                        self.request_redraw();
                    }
                }
                self.last_blink_toggle = now;
            } else {
                let since_blink = now.duration_since(self.last_blink_toggle).as_millis() as u64;
                if since_blink >= blink_ms {
                    if let Some(r) = &mut self.renderer {
                        r.cursor_visible = !r.cursor_visible;
                        self.request_redraw();
                    }
                    self.last_blink_toggle = now;
                }
            }
            let next = self.last_blink_toggle + std::time::Duration::from_millis(blink_ms);
            event_loop.set_control_flow(ControlFlow::WaitUntil(next));
        }

        // Advance theme color transition
        if let Some(ref mut trans) = self.config.transition {
            let dt = self
                .renderer
                .as_ref()
                .map(|r| r.gpu_delta_time())
                .unwrap_or(1.0 / 60.0);
            let done = trans.advance(dt);
            let blended = trans.current();
            // Apply blended colors to renderer without overwriting the target config
            if let Some(renderer) = &mut self.renderer {
                let mut render_config = self.config.clone();
                render_config.colors = blended;
                renderer.update_config(render_config);
            }
            if done {
                self.config.transition = None;
            }
            self.request_redraw();
        }

        // Apply config changes from settings UI
        if let Some(ui) = &mut self.ui {
            if let Some(new_config) = ui.pending_config.take() {
                self.config = new_config;
                if let Some(renderer) = &mut self.renderer {
                    renderer.update_config(self.config.clone());
                }
                self.request_redraw();
            }
        }

        // Config hot-reload from disk (skip when settings UI is open or recently changed)
        let settings_active = self.ui.as_ref().is_some_and(|u| u.settings_open());
        let recently_changed = now.duration_since(self.last_ui_config_change).as_secs() < 10;
        if !settings_active
            && !recently_changed
            && now.duration_since(self.last_config_check).as_secs() >= 2
        {
            self.last_config_check = now;
            if self.config.check_reload() {
                self.error_log.info("Config reloaded from disk");
                if let Some(renderer) = &mut self.renderer {
                    renderer.update_config(self.config.clone());
                }
                self.request_redraw();
            }
        }

        // Continuous animation mode — only when effects actually need it
        let ui_active = self.ui.as_ref().is_some_and(|u| u.settings_open());
        if self.config.effects.any_active() || ui_active {
            event_loop.set_control_flow(ControlFlow::Poll);
            self.request_redraw();
        }
    }
}

/// Parse a file:line:col location from a terminal line near the click column.
/// Supports patterns like:
///   src/main.rs:42:10
///   file.py:123
///   File "test.py", line 42
///   at Object.<anonymous> (file.js:10:5)
fn parse_file_location(line: &str, _click_col: usize) -> Option<(std::path::PathBuf, usize, usize)> {
    let line = line.trim();

    // Pattern: file.ext:line:col or file.ext:line
    let re_colon = regex::Regex::new(r"([a-zA-Z0-9_./-]+\.[a-zA-Z0-9]+):(\d+)(?::(\d+))?").ok()?;
    if let Some(caps) = re_colon.captures(line) {
        let path = std::path::PathBuf::from(&caps[1]);
        let line_num: usize = caps[2].parse().ok()?;
        let col_num: usize = caps.get(3).and_then(|m| m.as_str().parse().ok()).unwrap_or(1);
        // Only match if the file exists (or is a relative path that could exist)
        if path.exists() || (!path.is_absolute() && path.components().count() <= 10) {
            return Some((path, line_num, col_num));
        }
    }

    // Pattern: File "path", line N (Python tracebacks)
    let re_python = regex::Regex::new(r#"File "([^"]+)", line (\d+)"#).ok()?;
    if let Some(caps) = re_python.captures(line) {
        let path = std::path::PathBuf::from(&caps[1]);
        let line_num: usize = caps[2].parse().ok()?;
        return Some((path, line_num, 1));
    }

    None
}

fn terminal_input_event(event: &WindowEvent) -> bool {
    matches!(
        event,
        WindowEvent::KeyboardInput { .. }
            | WindowEvent::Ime(_)
            | WindowEvent::MouseInput { .. }
            | WindowEvent::MouseWheel { .. }
            | WindowEvent::CursorMoved { .. }
            | WindowEvent::DroppedFile(_)
    )
}

fn load_window_state() -> Option<(u32, u32)> {
    let config_dir = dirs::config_dir()?;
    let path = config_dir.join("llnzy").join("window_state.toml");
    let content = std::fs::read_to_string(path).ok()?;

    #[derive(serde::Deserialize)]
    struct WinState {
        width: Option<u32>,
        height: Option<u32>,
    }
    let state: WinState = toml::from_str(&content).ok()?;
    Some((state.width.unwrap_or(900), state.height.unwrap_or(600)))
}

fn main() {
    env_logger::init();

    // Install panic handler that logs to disk so panics remain visible when stderr is lost.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let msg = format!("llnzy panic: {}\n", info);
        let _ = write_diagnostic("crash.log", &msg);
        default_hook(info);
    }));

    // Catch signals that would silently kill the process
    #[cfg(unix)]
    unsafe {
        // SIGPIPE: writing to broken pipe (dead PTY)
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);
    }

    let event_loop = EventLoop::<UserEvent>::with_user_event().build().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);

    let proxy = event_loop.create_proxy();
    let mut app = App::new(proxy);

    // Log startup
    app.error_log.info("llnzy starting up");

    let result = event_loop.run_app(&mut app);
    if let Err(err) = &result {
        let exit_msg = format!("llnzy event loop exited with error: {:?}\n", err);
        let _ = write_diagnostic("exit.log", exit_msg);
    }
    result.unwrap();
}
