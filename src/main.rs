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
use llnzy::session::{split_pane, PaneNode, Rect as PaneRect, Session, SplitDir};
use llnzy::ui::{ActiveView, UiState};
use llnzy::UserEvent;
use winit::window::CursorIcon;

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

/// A tab contains a pane tree (which may be a single pane or splits).
struct Tab {
    root: PaneNode,
    name: Option<String>,
}

/// Tracks an active divider drag for pane resizing.
struct DividerDrag {
    path: Vec<bool>,
    dir: SplitDir,
    parent_rect: PaneRect,
}

struct App {
    config: Config,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    tabs: Vec<Tab>,
    active_tab: usize,
    proxy: winit::event_loop::EventLoopProxy<UserEvent>,
    modifiers: ModifiersState,
    selection: Selection,
    search: Search,
    error_log: ErrorLog,
    error_panel: ErrorPanel,
    clipboard: Option<arboard::Clipboard>,
    cursor_pos: winit::dpi::PhysicalPosition<f64>,
    mouse_pressed: bool,
    divider_drag: Option<DividerDrag>,
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
            proxy,
            modifiers: ModifiersState::empty(),
            selection: Selection::new(),
            search: Search::new(),
            error_log: ErrorLog::new(),
            error_panel: ErrorPanel::new(),
            clipboard,
            cursor_pos: winit::dpi::PhysicalPosition::new(0.0, 0.0),
            mouse_pressed: false,
            divider_drag: None,
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

    fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active_tab)
    }

    fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active_tab)
    }

    fn active_session(&self) -> Option<&Session> {
        self.active_tab().map(|t| t.root.active())
    }

    fn active_session_mut(&mut self) -> Option<&mut Session> {
        self.active_tab_mut().map(|t| t.root.active_mut())
    }

    fn process_all_output(&mut self) {
        let mut any_changed = false;
        for tab in &mut self.tabs {
            let (changed, clips, bell) = tab.root.process_all();
            if changed {
                any_changed = true;
            }
            if bell {
                self.visual_bell_until =
                    Some(Instant::now() + std::time::Duration::from_millis(150));
            }
            // Handle OSC 52 clipboard stores
            for text in clips {
                if let Some(cb) = &mut self.clipboard {
                    let _ = cb.set_text(text);
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

        // Auto-close tabs whose shells have exited
        let before = self.tabs.len();
        self.tabs.retain(|tab| tab.root.active().exited.is_none());
        let closed = before - self.tabs.len();
        if closed > 0 {
            self.error_log
                .info(format!("{} tab(s) closed (shell exited)", closed));
        }
        if self.tabs.is_empty() {
            self.error_log.info("All shells exited, spawning new tab");
            self.new_tab();
        } else if self.active_tab >= self.tabs.len() {
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
        // Spawn new tab in the current session's working directory
        let cwd = self.active_session().and_then(|s| s.cwd.clone());
        match Session::new_in_dir(cols, rows, &self.config, self.proxy.clone(), cwd.as_deref()) {
            Ok(session) => {
                self.tabs.push(Tab {
                    root: PaneNode::Leaf(Box::new(session)),
                    name: None,
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
        if self.tabs.len() <= 1 {
            return;
        }
        self.tabs.remove(self.active_tab);
        if self.active_tab >= self.tabs.len() {
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

    fn split_active_pane(&mut self, dir: SplitDir) {
        let (cols, rows) = self.grid_size();
        // Compute layout info before borrowing tabs mutably
        let layout_info = self.screen_layout.as_ref().map(|l| {
            let rect = PaneRect {
                x: l.content.x,
                y: l.content.y,
                w: l.content.w,
                h: l.content.h,
            };
            (l.cell_w, l.cell_h, rect)
        });

        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            let dummy_session = match Session::new(1, 1, &self.config, self.proxy.clone()) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("Failed to split: {}", e);
                    return;
                }
            };
            let root = std::mem::replace(&mut tab.root, PaneNode::Leaf(Box::new(dummy_session)));
            match split_pane(root, dir, &self.config, cols, rows, self.proxy.clone()) {
                Ok(new_root) => {
                    tab.root = new_root;
                    if let Some((cw, ch, rect)) = layout_info {
                        tab.root.resize_all(rect, cw, ch);
                    }
                }
                Err(e) => self.error_log.error(format!("Failed to split pane: {}", e)),
            }
        }

        self.invalidate_and_redraw();
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
                let title = tab
                    .name
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| format!("Shell {}", i + 1));
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
            sidebar_w: 0.0, // sidebar closed at startup
        });
        let cols = layout.grid_cols;
        let rows = layout.grid_rows;
        self.screen_layout = Some(layout);

        match Session::new(cols, rows, &self.config, self.proxy.clone()) {
            Ok(session) => {
                self.tabs.push(Tab {
                    root: PaneNode::Leaf(Box::new(session)),
                    name: None,
                });
            }
            Err(e) => {
                log::error!("Failed to spawn shell: {}", e);
                event_loop.exit();
                return;
            }
        }

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
            // If egui consumed a mouse/keyboard event, don't pass to terminal
            if response && (ui.settings_open() || ui.sidebar_open) {
                self.request_redraw();
                match &event {
                    WindowEvent::CloseRequested | WindowEvent::Resized(_) => {}
                    _ => return,
                }
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                self.error_log.info("Close requested");
                self.save_window_state();
                event_loop.exit();
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
                if let Some(layout) = &self.screen_layout {
                    let rect = PaneRect {
                        x: layout.content.x,
                        y: layout.content.y,
                        w: layout.content.w,
                        h: layout.content.h,
                    };
                    let (cw, ch) = (layout.cell_w, layout.cell_h);
                    for tab in &mut self.tabs {
                        tab.root.resize_all(rect, cw, ch);
                    }
                }
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
                    .tabs
                    .get(self.active_tab)
                    .map(|tab| {
                        let cols = tab.root.active().terminal.size().0;
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

                if let (Some(renderer), Some(tab)) =
                    (&mut self.renderer, self.tabs.get(self.active_tab))
                {
                    if let Some(layout) = &self.screen_layout {
                        // Shells honors the theme-level UI shader flag so app chrome can stay
                        // clean. Sketch opts into the real shader path so the canvas gets CRT,
                        // bloom, and background effects. A CRT mask restricts effects to just the
                        // canvas rect so sidebar/footer stay clean.
                        let effects_on_ui = self.ui.as_ref().is_some_and(|u| match u.active_view {
                            ActiveView::Shells => self.config.effects.effects_on_ui,
                            ActiveView::Sketch => true,
                            ActiveView::Stacker
                            | ActiveView::Appearances
                            | ActiveView::Settings => false,
                        });
                        // Build CRT effects mask: restrict effects to the canvas rect on Sketch view
                        let effects_mask = self.ui.as_ref().and_then(|u| {
                            u.sketch_canvas_px.and_then(|px| {
                                if matches!(u.active_view, ActiveView::Sketch) {
                                    let size = self.window.as_ref()?.inner_size();
                                    let w = size.width as f32;
                                    let h = size.height as f32;
                                    Some([px[0] / w, px[1] / h, px[2] / w, px[3] / h])
                                } else {
                                    None
                                }
                            })
                        });
                        // Update tab context for tab bar interaction
                        if let Some(ui) = self.ui.as_mut() {
                            ui.set_tab_context(self.tabs.len(), self.active_tab);
                        }
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
                            pane_root: &tab.root,
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

                // Apply config changes and tab renames from UI after render
                let mut need_redraw = false;
                let mut clip_text: Option<String> = None;
                let mut tab_rename: Option<(usize, String)> = None;
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

                // Tab bar click detection
                if button == MouseButton::Left && state == ElementState::Pressed {
                    if let Some(layout) = &self.screen_layout {
                        let px = self.cursor_pos.x as f32;
                        let py = self.cursor_pos.y as f32;
                        let tab_bar = &layout.tab_bar;
                        if py >= tab_bar.y && py < tab_bar.y + tab_bar.h && tab_bar.h > 0.0 {
                            let tab_count = self.tabs.len();
                            if tab_count > 0 {
                                let tab_w = (tab_bar.w / tab_count as f32).min(200.0);
                                let clicked_idx = ((px - tab_bar.x) / tab_w) as usize;
                                if clicked_idx < tab_count {
                                    self.switch_tab(clicked_idx);
                                    return;
                                }
                            }
                        }
                    }
                }

                // Divider drag detection
                if button == MouseButton::Left && state == ElementState::Pressed {
                    let px = self.cursor_pos.x as f32;
                    let py = self.cursor_pos.y as f32;
                    let hit = self.content_rect().and_then(|cr| {
                        let tab = self.tabs.get(self.active_tab)?;
                        let mut path = Vec::new();
                        tab.root.find_divider_at(cr, px, py, 5.0, &mut path)
                    });
                    if let Some((p, dir, parent)) = hit {
                        self.divider_drag = Some(DividerDrag {
                            path: p,
                            dir,
                            parent_rect: parent,
                        });
                        return;
                    }
                }
                if button == MouseButton::Left
                    && state == ElementState::Released
                    && self.divider_drag.take().is_some()
                {
                    if let Some(window) = &self.window {
                        window.set_cursor(CursorIcon::Default);
                    }
                    return;
                }

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
                                    if let Some(tab) = self.tabs.get(self.active_tab) {
                                        let terminal = &tab.root.active().terminal;
                                        self.selection.select_word(row, col, terminal);
                                    }
                                }
                                3 => {
                                    let cols = self
                                        .tabs
                                        .get(self.active_tab)
                                        .map(|t| t.root.active().terminal.size().0)
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
                let px = position.x as f32;
                let py = position.y as f32;

                // Handle active divider drag
                if let Some(drag) = &self.divider_drag {
                    let new_ratio = match drag.dir {
                        SplitDir::Vertical => (px - drag.parent_rect.x) / drag.parent_rect.w,
                        SplitDir::Horizontal => (py - drag.parent_rect.y) / drag.parent_rect.h,
                    };
                    let path = drag.path.clone();
                    let content_rect = self.content_rect();
                    let (cw, ch) = self
                        .renderer
                        .as_ref()
                        .map(|r| r.cell_dimensions())
                        .unwrap_or((1.0, 1.0));
                    if let Some(tab) = self.active_tab_mut() {
                        tab.root.set_ratio_at(&path, new_ratio);
                        if let Some(cr) = content_rect {
                            tab.root.resize_all(cr, cw, ch);
                        }
                    }
                    if let Some(r) = &mut self.renderer {
                        r.invalidate_text_cache();
                    }
                    self.request_redraw();
                    return;
                }

                // Hover cursor icon for dividers
                if !self.mouse_pressed {
                    let near_divider = self.content_rect().and_then(|cr| {
                        self.tabs.get(self.active_tab).and_then(|tab| {
                            let mut path = Vec::new();
                            tab.root.find_divider_at(cr, px, py, 5.0, &mut path)
                        })
                    });
                    if let (Some(window), Some((_, dir, _))) = (&self.window, near_divider) {
                        let icon = match dir {
                            SplitDir::Vertical => CursorIcon::ColResize,
                            SplitDir::Horizontal => CursorIcon::RowResize,
                        };
                        window.set_cursor(icon);
                    } else if let Some(window) = &self.window {
                        window.set_cursor(CursorIcon::Default);
                    }
                }

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
                            if let Some(tab) = self.tabs.get(self.active_tab) {
                                let terminal = &tab.root.active().terminal;
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
                                        if let Some(tab) = self.tabs.get(self.active_tab) {
                                            self.search.update_matches(&tab.root.active().terminal);
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
                                        if let Some(tab) = self.tabs.get(self.active_tab) {
                                            self.search.push_char(ch, &tab.root.active().terminal);
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
                            self.search.open();
                            self.request_redraw();
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
                        Action::SplitVertical => self.split_active_pane(SplitDir::Vertical),
                        Action::SplitHorizontal => self.split_active_pane(SplitDir::Horizontal),
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
                            if let Some(layout) = &self.screen_layout {
                                let rect = PaneRect {
                                    x: layout.content.x,
                                    y: layout.content.y,
                                    w: layout.content.w,
                                    h: layout.content.h,
                                };
                                let (cw, ch) = (layout.cell_w, layout.cell_h);
                                for tab in &mut self.tabs {
                                    tab.root.resize_all(rect, cw, ch);
                                }
                            }
                            self.selection.clear();
                            self.invalidate_and_redraw();
                        }
                        Action::CyclePaneForward | Action::CyclePaneBackward => {
                            if let Some(tab) = self.active_tab_mut() {
                                tab.root.cycle_focus();
                            }
                            self.selection.clear();
                            self.invalidate_and_redraw();
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
                        Action::SwitchTab(n) => {
                            self.switch_tab((n - 1) as usize);
                        }
                    }
                    return;
                }

                if self.selection.is_active() {
                    self.selection.clear();
                    self.request_redraw();
                }

                self.last_keypress = Instant::now();
                let app_cursor = self.app_cursor();
                if let Some(bytes) = input::encode_key(&key_event, self.modifiers, app_cursor) {
                    self.write_to_active(&bytes);
                }
            }

            // IME (input method) events — handles composed text from
            // non-US keyboards, dead keys, and CJK input methods.
            WindowEvent::Ime(ime) => {
                match ime {
                    Ime::Commit(text) => {
                        if self.search.active {
                            for ch in text.chars() {
                                if !ch.is_control() {
                                    if let Some(tab) = self.tabs.get(self.active_tab) {
                                        self.search.push_char(ch, &tab.root.active().terminal);
                                    }
                                }
                            }
                            self.request_redraw();
                        } else {
                            self.write_to_active(text.as_bytes());
                        }
                    }
                    Ime::Preedit(_, _) => {
                        // TODO: show preedit text overlay for CJK input methods
                    }
                    Ime::Enabled | Ime::Disabled => {}
                }
            }

            // Drag-and-drop: insert escaped file path
            WindowEvent::DroppedFile(path) => {
                let path_str = path.to_string_lossy();
                // Shell-escape the path (wrap in single quotes, escape existing quotes)
                let escaped = format!("'{}'", path_str.replace('\'', "'\\''"));
                self.write_to_active(escaped.as_bytes());
                self.write_to_active(b" ");
            }

            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::PtyOutput => self.request_redraw(),
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
                    MenuAction::SplitVertical => self.split_active_pane(SplitDir::Vertical),
                    MenuAction::SplitHorizontal => self.split_active_pane(SplitDir::Horizontal),
                    MenuAction::ToggleEffects => self.toggle_effects(),
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
