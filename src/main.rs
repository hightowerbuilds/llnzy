use std::sync::Arc;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Fullscreen, Window, WindowAttributes, WindowId};

use llnzy::config::Config;
use llnzy::error_log::{ErrorLog, ErrorPanel};
use llnzy::input;
use llnzy::renderer::Renderer;
use llnzy::search::Search;
use llnzy::selection::Selection;
use llnzy::session::{split_pane, PaneNode, Rect as PaneRect, Session, SplitDir};
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

/// A tab contains a pane tree (which may be a single pane or splits).
struct Tab {
    root: PaneNode,
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
    click_state: ClickState,
    visual_bell_until: Option<Instant>,
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
            click_state: ClickState::new(),
            visual_bell_until: None,
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
            let (changed, clips) = tab.root.process_all();
            if changed {
                any_changed = true;
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

    fn grid_size(&self) -> (u16, u16) {
        if let Some(renderer) = &self.renderer {
            let (cw, ch) = renderer.cell_dimensions();
            let rect = renderer.content_rect(self.tabs.len());
            let cols = (rect.w / cw).max(1.0) as u16;
            let rows = (rect.h / ch).max(1.0) as u16;
            (cols, rows)
        } else {
            (80, 24)
        }
    }

    fn pixel_to_grid(&self, pos: winit::dpi::PhysicalPosition<f64>) -> (usize, usize) {
        let (cw, ch) = self
            .renderer
            .as_ref()
            .map(|r| r.cell_dimensions())
            .unwrap_or((1.0, 1.0));
        let content_rect = self
            .renderer
            .as_ref()
            .map(|r| r.content_rect(self.tabs.len()))
            .unwrap_or(PaneRect {
                x: 0.0,
                y: 0.0,
                w: 800.0,
                h: 600.0,
            });
        let (cols, rows) = self
            .active_session()
            .map(|s| s.terminal.size())
            .unwrap_or((80, 24));

        let col =
            (((pos.x as f32 - content_rect.x) / cw).max(0.0) as usize).min(cols.saturating_sub(1));
        let row =
            (((pos.y as f32 - content_rect.y) / ch).max(0.0) as usize).min(rows.saturating_sub(1));
        (row, col)
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
            self.write_to_active(b"\x1b[200~");
            self.write_to_active(text.as_bytes());
            self.write_to_active(b"\x1b[201~");
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

    fn new_tab(&mut self) {
        let (cols, rows) = self.grid_size();
        match Session::new(cols, rows, &self.config, self.proxy.clone()) {
            Ok(session) => {
                self.tabs.push(Tab {
                    root: PaneNode::Leaf(Box::new(session)),
                });
                self.active_tab = self.tabs.len() - 1;
                self.selection.clear();
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
        self.request_redraw();
    }

    fn switch_tab(&mut self, idx: usize) {
        if idx < self.tabs.len() && idx != self.active_tab {
            self.active_tab = idx;
            self.selection.clear();
            if let Some(renderer) = &mut self.renderer {
                renderer.invalidate_text_cache();
            }
            self.request_redraw();
        }
    }

    fn split_active_pane(&mut self, dir: SplitDir) {
        let (cols, rows) = self.grid_size();
        let tab_count = self.tabs.len();

        // Compute layout info before borrowing tabs mutably
        let layout_info = self.renderer.as_ref().map(|r| {
            let (cw, ch) = r.cell_dimensions();
            let rect = r.content_rect(tab_count);
            (cw, ch, rect)
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

        if let Some(renderer) = &mut self.renderer {
            renderer.invalidate_text_cache();
        }
        self.request_redraw();
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
                let title = tab.root.active().title.clone();
                let short = if title.len() > 20 {
                    format!("{}…", &title[..19])
                } else {
                    title
                };
                (short, i == self.active_tab)
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
        if self.config.opacity < 1.0 {
            attrs = attrs.with_transparent(true);
        }

        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        let renderer = pollster::block_on(Renderer::new(window.clone(), self.config.clone()));

        let (cw, ch) = renderer.cell_dimensions();
        let rect = renderer.content_rect(1); // 1 tab initially
        let cols = (rect.w / cw).max(1.0) as u16;
        let rows = (rect.h / ch).max(1.0) as u16;

        match Session::new(cols, rows, &self.config, self.proxy.clone()) {
            Ok(session) => {
                self.tabs.push(Tab {
                    root: PaneNode::Leaf(Box::new(session)),
                });
            }
            Err(e) => {
                log::error!("Failed to spawn shell: {}", e);
                event_loop.exit();
                return;
            }
        }

        self.window = Some(window);
        self.renderer = Some(renderer);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.save_window_state();
                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                    renderer.invalidate_text_cache();

                    let (cw, ch) = renderer.cell_dimensions();
                    let rect = renderer.content_rect(self.tabs.len());

                    for tab in &mut self.tabs {
                        tab.root.resize_all(rect, cw, ch);
                    }
                }
                self.selection.clear();
            }

            WindowEvent::RedrawRequested => {
                self.process_all_output();

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
                    renderer.render(
                        &tab.root,
                        &titles,
                        &sel_info,
                        &search_rects,
                        search_bar_ref,
                        err_panel,
                        bell_active,
                    );
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
                        if let Some(renderer) = &mut self.renderer {
                            renderer.invalidate_text_cache();
                        }
                        self.request_redraw();
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

                // Cmd shortcuts
                if self.modifiers.super_key() {
                    match &key_event.logical_key {
                        // Cmd+F: open search
                        Key::Character(c) if c.as_str() == "f" => {
                            self.search.open();
                            self.request_redraw();
                            return;
                        }
                        Key::Character(c) if c.as_str() == "c" => {
                            self.copy_selection();
                            return;
                        }
                        Key::Character(c) if c.as_str() == "v" => {
                            if let Some(cb) = &mut self.clipboard {
                                if let Ok(text) = cb.get_text() {
                                    self.paste_text(&text);
                                }
                            }
                            return;
                        }
                        Key::Character(c) if c.as_str() == "a" => {
                            if let Some(s) = self.active_session() {
                                let (cols, rows) = s.terminal.size();
                                self.selection.select_all(rows, cols);
                            }
                            self.request_redraw();
                            return;
                        }
                        // Tab management
                        Key::Character(c) if c.as_str() == "t" => {
                            self.new_tab();
                            return;
                        }
                        Key::Character(c) if c.as_str() == "w" => {
                            self.close_tab();
                            return;
                        }
                        // Cmd+Shift+] / Cmd+Shift+[ : next/prev tab
                        Key::Character(c) if c.as_str() == "}" || c.as_str() == "]" => {
                            let next = (self.active_tab + 1) % self.tabs.len().max(1);
                            self.switch_tab(next);
                            return;
                        }
                        Key::Character(c) if c.as_str() == "{" || c.as_str() == "[" => {
                            let prev = if self.active_tab == 0 {
                                self.tabs.len().saturating_sub(1)
                            } else {
                                self.active_tab - 1
                            };
                            self.switch_tab(prev);
                            return;
                        }
                        // Cmd+D / Cmd+Shift+D: split pane
                        Key::Character(c) if c.as_str() == "d" => {
                            if self.modifiers.shift_key() {
                                self.split_active_pane(SplitDir::Horizontal);
                            } else {
                                self.split_active_pane(SplitDir::Vertical);
                            }
                            return;
                        }
                        // Cmd+Enter: toggle fullscreen
                        Key::Named(NamedKey::Enter) => {
                            self.toggle_fullscreen();
                            return;
                        }
                        // Cmd+Shift+E: toggle error/diagnostics panel
                        Key::Character(c) if c.as_str() == "e" || c.as_str() == "E" => {
                            if self.modifiers.shift_key() {
                                self.error_panel.toggle();
                                self.request_redraw();
                                return;
                            }
                        }
                        // Cmd+1-9: switch tabs
                        Key::Character(c) => {
                            if let Some(d) = c.chars().next().and_then(|ch| ch.to_digit(10)) {
                                if (1..=9).contains(&d) {
                                    self.switch_tab((d - 1) as usize);
                                    return;
                                }
                            }
                        }
                        // Cmd+Arrow: cycle focus between panes
                        Key::Named(NamedKey::ArrowRight | NamedKey::ArrowDown) => {
                            if let Some(tab) = self.active_tab_mut() {
                                tab.root.cycle_focus();
                            }
                            self.selection.clear();
                            if let Some(r) = &mut self.renderer {
                                r.invalidate_text_cache();
                            }
                            self.request_redraw();
                            return;
                        }
                        Key::Named(NamedKey::ArrowLeft | NamedKey::ArrowUp) => {
                            if let Some(tab) = self.active_tab_mut() {
                                tab.root.cycle_focus();
                            }
                            self.selection.clear();
                            if let Some(r) = &mut self.renderer {
                                r.invalidate_text_cache();
                            }
                            self.request_redraw();
                            return;
                        }
                        _ => {}
                    }
                }

                // Shift+PageUp/Down
                if self.modifiers.shift_key() && !self.mouse_reporting() {
                    match &key_event.logical_key {
                        Key::Named(NamedKey::PageUp) => {
                            if let Some(s) = self.active_session_mut() {
                                s.terminal.scroll_page_up();
                            }
                            if let Some(r) = &mut self.renderer {
                                r.invalidate_text_cache();
                            }
                            self.request_redraw();
                            return;
                        }
                        Key::Named(NamedKey::PageDown) => {
                            if let Some(s) = self.active_session_mut() {
                                s.terminal.scroll_page_down();
                            }
                            if let Some(r) = &mut self.renderer {
                                r.invalidate_text_cache();
                            }
                            self.request_redraw();
                            return;
                        }
                        _ => {}
                    }
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
            UserEvent::PtyOutput => self.process_all_output(),
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

        // Config hot-reload
        if now.duration_since(self.last_config_check).as_secs() >= 2 {
            self.last_config_check = now;
            if self.config.check_reload() {
                self.error_log.info("Config reloaded from disk");
                if let Some(renderer) = &mut self.renderer {
                    renderer.update_config(self.config.clone());
                }
                self.request_redraw();
            }
        }
    }
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

    // Install panic handler that logs to stderr (and could log to ErrorLog if we had a global ref)
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        eprintln!("llnzy panic: {}", info);
        default_hook(info);
    }));

    let event_loop = EventLoop::<UserEvent>::with_user_event().build().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);

    let proxy = event_loop.create_proxy();
    let mut app = App::new(proxy);

    // Log startup
    app.error_log.info("llnzy starting up");

    event_loop.run_app(&mut app).unwrap();
}
