use std::sync::Arc;
use std::time::Instant;

mod runtime;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};

use llnzy::app::click_state::ClickState;
use llnzy::app::commands::AppCommand;
use llnzy::app::file_location::parse_file_location;
use llnzy::app::keybinding_commands::app_command_for_keybinding;
use llnzy::app::terminal_events::terminal_input_event;
use llnzy::app::window_state;
use llnzy::config::Config;
use llnzy::diagnostics::write_diagnostic;
use llnzy::error_log::{ErrorLog, ErrorPanel};
use llnzy::input;
use llnzy::keybindings::{primary_modifier, Action};
use llnzy::layout::{LayoutInputs, ScreenLayout};
use llnzy::renderer::{RenderRequest, Renderer, TerminalPane};
use llnzy::search::Search;
use llnzy::stacker::commands::StackerCommandId;
use llnzy::ui::command_palette::CommandId;
use llnzy::ui::{ActiveView, PendingClose, UiFrameOutput, UiState, BUMPER_WIDTH};
use llnzy::workspace::{TabContent, TabKind, WorkspaceTab};
use llnzy::workspace_layout::{
    active_joined_tabs, joined_terminal_content_rects, terminal_effect_rect, JoinedTabs,
};
use llnzy::UserEvent;

#[derive(Clone, Copy)]
struct TerminalPaneHit {
    tab_idx: usize,
    row: usize,
    col: usize,
}

enum StackerHistoryCommand {
    Undo,
    Redo,
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
    search: Search,
    error_log: ErrorLog,
    error_panel: ErrorPanel,
    clipboard: Option<arboard::Clipboard>,
    cursor_pos: winit::dpi::PhysicalPosition<f64>,
    mouse_pressed: bool,
    terminal_selection_drag: bool,
    terminal_pending_mouse_press: Option<(usize, usize)>,
    click_state: ClickState,
    ui: Option<UiState>,
    screen_layout: Option<ScreenLayout>,
    visual_bell_until: Option<Instant>,
    last_ui_config_change: Instant,
    last_blink_toggle: Instant,
    last_keypress: Instant,
    last_config_check: Instant,
    #[cfg(target_os = "macos")]
    stacker_bridge_active: Option<bool>,
    #[cfg(target_os = "macos")]
    stacker_bridge_text: String,
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
            search: Search::new(),
            error_log: ErrorLog::new(),
            error_panel: ErrorPanel::new(),
            clipboard,
            cursor_pos: winit::dpi::PhysicalPosition::new(0.0, 0.0),
            mouse_pressed: false,
            terminal_selection_drag: false,
            terminal_pending_mouse_press: None,
            click_state: ClickState::new(),
            ui: None,
            screen_layout: None,
            visual_bell_until: None,
            last_ui_config_change: Instant::now() - std::time::Duration::from_secs(60),
            last_blink_toggle: Instant::now(),
            last_keypress: Instant::now(),
            last_config_check: Instant::now(),
            #[cfg(target_os = "macos")]
            stacker_bridge_active: None,
            #[cfg(target_os = "macos")]
            stacker_bridge_text: String::new(),
        }
    }

    fn alloc_tab_id(&mut self) -> u64 {
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        id
    }

    fn active_stacker_tab(&self) -> bool {
        self.active_tab()
            .is_some_and(|tab| matches!(tab.content, TabContent::Stacker))
    }

    #[cfg(target_os = "macos")]
    fn sync_macos_text_bridge(&mut self) {
        let Some(window) = &self.window else { return };
        let active = self.active_stacker_tab();
        let text = if active {
            self.ui
                .as_ref()
                .map(|ui| ui.stacker.editor.text())
                .unwrap_or("")
                .to_string()
        } else {
            String::new()
        };
        if !active && self.stacker_bridge_active == Some(false) {
            return;
        }
        llnzy::macos_text_bridge::set_stacker_active(window, active, &text);
        self.stacker_bridge_active = Some(active);
        self.stacker_bridge_text = text;
    }

    fn joined_terminal_tab_at_cursor(&self) -> Option<usize> {
        self.terminal_pane_hit_at_cursor().map(|hit| hit.tab_idx)
    }

    fn terminal_pane_hit_at_cursor(&self) -> Option<TerminalPaneHit> {
        let layout = self.screen_layout.as_ref()?;
        let x = self.cursor_pos.x as f32;
        let y = self.cursor_pos.y as f32;
        let joined = self
            .ui
            .as_ref()
            .and_then(|ui| active_joined_tabs(&self.tabs, self.active_tab, &ui.tab_groups));

        if let Some(joined) = joined {
            let (left_rect, right_rect) = joined_terminal_content_rects(layout, joined.ratio);
            for (idx, rect) in [(joined.primary, left_rect), (joined.secondary, right_rect)] {
                if let Some(hit) = self.terminal_pane_hit(idx, rect, x, y) {
                    return Some(hit);
                }
            }
            return None;
        }

        let rect = llnzy::session::Rect {
            x: layout.content.x,
            y: layout.content.y,
            w: layout.content.w,
            h: layout.content.h,
        };
        self.terminal_pane_hit(self.active_tab, rect, x, y)
    }

    fn terminal_pane_hit(
        &self,
        tab_idx: usize,
        rect: llnzy::session::Rect,
        x: f32,
        y: f32,
    ) -> Option<TerminalPaneHit> {
        if !rect_contains(rect, x, y) {
            return None;
        }
        let session = self.session_for_tab(tab_idx)?;
        let layout = self.screen_layout.as_ref()?;
        let (cols, rows) = session.terminal.size();
        let col = ((x - rect.x) / layout.cell_w).max(0.0) as usize;
        let row = ((y - rect.y) / layout.cell_h).max(0.0) as usize;
        Some(TerminalPaneHit {
            tab_idx,
            row: row.min(rows.saturating_sub(1)),
            col: col.min(cols.saturating_sub(1)),
        })
    }

    fn route_terminal_mouse_wheel(&mut self, delta: &MouseScrollDelta) -> bool {
        if self.cursor_over_non_terminal_chrome() {
            return false;
        }
        let Some(hit) = self.terminal_pane_hit_at_cursor() else {
            return false;
        };

        let Some(session) = self.session_for_tab(hit.tab_idx) else {
            return false;
        };
        if session.terminal.mouse_mode() {
            let sgr = session.terminal.sgr_mouse();
            let lines = self.wheel_lines(delta, 1.0);
            for _ in 0..lines.unsigned_abs() {
                let button = if lines > 0 { 64 } else { 65 };
                let bytes =
                    input::encode_mouse(button, hit.col, hit.row, true, sgr, &self.modifiers);
                self.write_to_terminal_tab(hit.tab_idx, &bytes);
            }
            self.request_redraw();
            return true;
        }

        let lines = self.wheel_lines(delta, self.config.scroll_lines as f32);
        if lines != 0 {
            if let Some(session) = self.session_for_tab_mut(hit.tab_idx) {
                session.terminal.scroll(lines);
            }
            self.invalidate_and_redraw();
        } else {
            self.request_redraw();
        }
        true
    }

    fn wheel_lines(&self, delta: &MouseScrollDelta, line_multiplier: f32) -> i32 {
        match delta {
            MouseScrollDelta::LineDelta(_, y) => (y * line_multiplier) as i32,
            MouseScrollDelta::PixelDelta(pos) => {
                let (_, ch) = self
                    .renderer
                    .as_ref()
                    .map(|r| r.cell_dimensions())
                    .unwrap_or((1.0, 1.0));
                let lines = (pos.y / ch as f64) as i32;
                if lines == 0 && pos.y != 0.0 {
                    pos.y.signum() as i32
                } else {
                    lines
                }
            }
        }
    }

    fn toggle_editor_word_wrap(&mut self) {
        self.config.editor.word_wrap = !self.config.editor.word_wrap;
        if let Some(renderer) = &mut self.renderer {
            renderer.update_config(self.config.clone());
        }
        self.request_redraw();
    }
}

fn rect_contains(rect: llnzy::session::Rect, x: f32, y: f32) -> bool {
    x >= rect.x && x < rect.x + rect.w && y >= rect.y && y < rect.y + rect.h
}

fn local_terminal_selection_requested(
    mouse_reporting: bool,
    shift_key: bool,
    terminal_selection_drag: bool,
) -> bool {
    mouse_reporting && (shift_key || terminal_selection_drag)
}

fn stacker_history_shortcut(key: &Key, modifiers: ModifiersState) -> Option<StackerHistoryCommand> {
    if !primary_modifier(modifiers) || modifiers.alt_key() {
        return None;
    }

    let Key::Character(key) = key else {
        return None;
    };

    match (key.to_lowercase().as_str(), modifiers.shift_key()) {
        ("z", false) => Some(StackerHistoryCommand::Undo),
        ("z", true) | ("y", false) => Some(StackerHistoryCommand::Redo),
        _ => None,
    }
}

fn stacker_editor_shortcut(key: &Key, modifiers: ModifiersState) -> Option<StackerCommandId> {
    if !primary_modifier(modifiers) || modifiers.alt_key() {
        return None;
    }

    let Key::Character(key) = key else {
        return None;
    };

    match (key.to_lowercase().as_str(), modifiers.shift_key()) {
        ("b", _) => Some(StackerCommandId::Bold),
        ("`", _) => Some(StackerCommandId::InlineCode),
        _ => None,
    }
}

fn stacker_keyboard_text_fallback_candidate(
    event: &WindowEvent,
    modifiers: ModifiersState,
) -> bool {
    let WindowEvent::KeyboardInput {
        event: key_event, ..
    } = event
    else {
        return false;
    };
    if key_event.state != ElementState::Pressed
        || modifiers.control_key()
        || modifiers.alt_key()
        || modifiers.super_key()
    {
        return false;
    }
    key_event
        .text
        .as_ref()
        .is_some_and(|text| input::text_should_use_paste_path(text.as_str()))
}

fn terminal_mouse_drag_exceeded(start: (usize, usize), row: usize, col: usize) -> bool {
    start != (row, col)
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        // Try to restore saved window size
        let saved_size = window_state::load_window_size();
        let inner_size = saved_size
            .map(|(w, h)| winit::dpi::PhysicalSize::new(w, h))
            .unwrap_or(winit::dpi::PhysicalSize::new(900, 600));

        let mut attrs = WindowAttributes::default()
            .with_title("LLNZY")
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
        // Input methods can deliver text through AppKit's IME/text-input path.
        // winit disables that by default, so opt in for terminal and Stacker text entry.
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
        self.restore_last_session();
        if self.tabs.is_empty() {
            self.open_singleton_tab(llnzy::workspace::TabKind::Home);
        }

        // Set up native macOS menu bar
        #[cfg(target_os = "macos")]
        llnzy::menu::setup_menu_bar(self.proxy.clone());
        #[cfg(target_os = "macos")]
        {
            llnzy::macos_text_bridge::setup(self.proxy.clone());
            if let Some(window) = &self.window {
                llnzy::macos_text_bridge::install(window);
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let native_hover_target = match &event {
            WindowEvent::HoveredFile(path) => {
                Some((path.clone(), self.native_file_drop_target(path)))
            }
            _ => None,
        };
        let terminal_ime_commit =
            matches!(&event, WindowEvent::Ime(Ime::Commit(_))) && self.active_session().is_some();
        let stacker_ime_commit =
            matches!(&event, WindowEvent::Ime(Ime::Commit(_))) && self.active_stacker_tab();
        let stacker_keyboard_text_commit = self.active_stacker_tab()
            && stacker_keyboard_text_fallback_candidate(&event, self.modifiers);

        if let WindowEvent::KeyboardInput {
            event: key_event, ..
        } = &event
        {
            if self.active_session().is_some() && key_event.state == ElementState::Pressed {
                if let Some(action) = self.config.keybindings.match_key(key_event, self.modifiers) {
                    match action {
                        Action::Copy => {
                            self.copy_selection();
                            return;
                        }
                        Action::Paste => {
                            self.do_paste();
                            return;
                        }
                        Action::SelectAll => {
                            self.do_select_all();
                            return;
                        }
                        _ => {}
                    }
                }
            }

            if self.active_stacker_tab() && key_event.state == ElementState::Pressed {
                if let Some(command) =
                    stacker_editor_shortcut(&key_event.logical_key, self.modifiers)
                {
                    self.apply_stacker_editor_command(command);
                    return;
                }

                if let Some(history_command) =
                    stacker_history_shortcut(&key_event.logical_key, self.modifiers)
                {
                    let handled = match history_command {
                        StackerHistoryCommand::Undo => self.undo_stacker_editor(),
                        StackerHistoryCommand::Redo => self.redo_stacker_editor(),
                    };
                    if handled {
                        return;
                    }
                }

                if let Some(action) = self.config.keybindings.match_key(key_event, self.modifiers) {
                    match action {
                        Action::Copy => {
                            self.copy_stacker_editor_selection();
                            return;
                        }
                        Action::Paste => {
                            self.do_paste();
                            return;
                        }
                        Action::SelectAll => {
                            self.select_all_stacker_editor();
                            return;
                        }
                        _ => {}
                    }
                }
            }

            if self.route_code_editor_keybinding(key_event) {
                return;
            }
        }

        if let WindowEvent::MouseInput {
            state: ElementState::Pressed,
            button: MouseButton::Left,
            ..
        } = &event
        {
            if let Some(tab_idx) = self.joined_terminal_tab_at_cursor() {
                if tab_idx != self.active_tab {
                    self.switch_tab(tab_idx);
                    return;
                }
            }
        }

        if let WindowEvent::MouseWheel { delta, .. } = &event {
            if self.route_terminal_mouse_wheel(delta) {
                return;
            }
        }

        // Route events to egui first
        if let (Some(window), Some(ui)) = (&self.window, &mut self.ui) {
            let stacker_input_before_egui =
                stacker_ime_commit.then(|| ui.stacker.editor.text().to_string());
            let stacker_keyboard_text_before_egui =
                stacker_keyboard_text_commit.then(|| ui.stacker.editor.text().to_string());
            let response = ui.handle_event(window, &event);
            let terminal_should_receive_consumed_ime = terminal_ime_commit
                && !ui.captures_terminal_input()
                && !ui.ctx.wants_keyboard_input();
            let stacker_should_receive_consumed_ime = stacker_input_before_egui
                .as_ref()
                .is_some_and(|input_before| input_before == ui.stacker.editor.text());
            let stacker_prompt_editor_focused = ui.ctx.memory(|memory| {
                memory.has_focus(egui::Id::new(llnzy::ui::STACKER_PROMPT_EDITOR_ID))
            });
            let stacker_should_receive_consumed_key_text = stacker_keyboard_text_before_egui
                .as_ref()
                .is_some_and(|input_before| {
                    stacker_prompt_editor_focused && input_before == ui.stacker.editor.text()
                });
            match &event {
                WindowEvent::HoveredFile(path) => {
                    ui.drag_drop.hover_native_file(path.clone());
                    ui.drag_drop.active_target = native_hover_target.and_then(|(_, target)| target);
                    window.request_redraw();
                }
                WindowEvent::HoveredFileCancelled => {
                    ui.drag_drop.cancel();
                    window.request_redraw();
                }
                _ => {}
            }
            // Sketch owns raw canvas input; do not let unconsumed pointer/text events leak
            // into the terminal while that workspace is active.
            if ui.captures_terminal_input() && terminal_input_event(&event) {
                self.request_redraw();
                return;
            }
            // If egui consumed a mouse/keyboard event, don't pass to terminal.
            // The footer and bumper are always visible, so any egui-consumed
            // event must be respected regardless of which view is active.
            if response
                && !terminal_should_receive_consumed_ime
                && !stacker_should_receive_consumed_ime
                && !stacker_should_receive_consumed_key_text
            {
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
                let modified = self.modified_code_tabs();
                if !modified.is_empty() {
                    if let Some(ui) = &mut self.ui {
                        ui.pending_close = Some(PendingClose::Window(modified));
                        ui.save_prompt_error = None;
                    }
                    self.request_redraw();
                } else {
                    self.error_log.info("Close requested");
                    self.save_window_state();
                    event_loop.exit();
                }
            }

            WindowEvent::Focused(focused) => {
                #[cfg(target_os = "macos")]
                {
                    self.stacker_bridge_active = None;
                    if focused {
                        self.sync_macos_text_bridge();
                    }
                }
                if focused {
                    self.request_redraw();
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
                self.clear_terminal_selection();
                self.terminal_pending_mouse_press = None;
            }

            WindowEvent::RedrawRequested => {
                self.process_all_output();
                self.update_ime_cursor_area();
                #[cfg(target_os = "macos")]
                self.sync_macos_text_bridge();

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

                let tab_info = self.tab_titles();
                let tab_pane_info = self.tab_panes();
                let render_titles: Vec<(String, bool)> = tab_info
                    .iter()
                    .enumerate()
                    .map(|(i, tab)| (tab.title.clone(), i == self.active_tab))
                    .collect();
                let (cw, ch) = self
                    .renderer
                    .as_ref()
                    .map(|r| r.cell_dimensions())
                    .unwrap_or((1.0, 1.0));
                let sel_info = self
                    .active_session()
                    .map(|session| {
                        session.terminal.selection_rects(
                            cw,
                            ch,
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
                            ui.active_tab_kind =
                                self.tabs.get(self.active_tab).map(|t| t.content.kind());
                            #[cfg(target_os = "macos")]
                            llnzy::menu::set_save_enabled(matches!(
                                ui.active_tab_kind,
                                Some(TabKind::CodeFile)
                            ));
                            // Populate tab names for egui tab bar
                            ui.tab_names = tab_info.clone();
                            ui.tab_panes = tab_pane_info.clone();
                        }

                        // Get the active terminal session (if any) for the renderer
                        let active_tab = self.tabs.get(self.active_tab);
                        let tab_id = active_tab.map(|t| t.id).unwrap_or(0);
                        let joined_tabs = self.ui.as_ref().and_then(|ui| {
                            active_joined_tabs(&self.tabs, self.active_tab, &ui.tab_groups)
                        });
                        let terminal_panes = joined_tabs
                            .map(|joined| {
                                joined_terminal_panes(&self.tabs, self.active_tab, layout, joined)
                            })
                            .unwrap_or_default();
                        let terminal_session = if terminal_panes.is_empty() {
                            active_tab.and_then(|t| t.content.as_terminal())
                        } else {
                            None
                        };
                        let terminal_effect_rect = self.ui.as_ref().and_then(|ui| {
                            terminal_effect_rect(
                                &self.tabs,
                                layout,
                                &ui.tab_groups,
                                self.active_tab,
                            )
                        });
                        let terminal_effects_enabled = terminal_effect_rect.is_some();
                        let effects_mask = terminal_effect_rect.and_then(|rect| {
                            self.window
                                .as_ref()
                                .map(|window| rect_to_uv(rect, window.inner_size()))
                        });

                        let ui_state = &mut self.ui;
                        let window_ref = &self.window;
                        let config_ref = &self.config;
                        let mut ui_frame_output = UiFrameOutput::default();
                        let mut egui_cb =
                            |device: &wgpu::Device,
                             queue: &wgpu::Queue,
                             view: &wgpu::TextureView,
                             desc: egui_wgpu::ScreenDescriptor| {
                                if let (Some(ui), Some(window)) =
                                    (ui_state.as_mut(), window_ref.as_ref())
                                {
                                    ui_frame_output =
                                        ui.render(window, device, queue, view, desc, config_ref);
                                }
                            };
                        renderer.render(RenderRequest {
                            terminal: terminal_session,
                            tab_id,
                            terminal_panes: &terminal_panes,
                            tab_titles: &render_titles,
                            selection_rects: &sel_info,
                            search_rects: &search_rects,
                            search_bar: search_bar_ref,
                            error_panel: err_panel,
                            visual_bell: bell_active,
                            screen_layout: layout,
                            egui_render: Some(&mut egui_cb),
                            effects_enabled: terminal_effects_enabled,
                            apply_effects_to_ui: false,
                            effects_mask,
                        });
                        self.handle_ui_frame_output(ui_frame_output, event_loop);
                    }
                }

                // Detect sidebar state change from bumper click and recompute layout
                let sidebar_w_after = self.ui.as_ref().map(|u| u.sidebar_width()).unwrap_or(0.0);
                if (sidebar_w_after - sidebar_w_before).abs() > 0.1 {
                    self.recompute_layout();
                    self.resize_terminal_tabs();
                    self.clear_terminal_selection();
                    self.invalidate_and_redraw();
                }
            }

            WindowEvent::ModifiersChanged(mods) => {
                self.modifiers = mods.state();
            }

            // --- Mouse wheel ---
            WindowEvent::MouseWheel { delta, .. } => {
                let _ = self.route_terminal_mouse_wheel(&delta);
            }

            // --- Mouse buttons ---
            WindowEvent::MouseInput { state, button, .. } => {
                if self.cursor_over_non_terminal_chrome() {
                    self.request_redraw();
                    return;
                }
                let (row, col) = self.pixel_to_grid(self.cursor_pos);

                if button == MouseButton::Right && state == ElementState::Pressed {
                    if self.terminal_selection_active() {
                        self.copy_selection();
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
                            (0..cols)
                                .map(|c| session.terminal.cell_char(row, c))
                                .collect::<String>()
                        };
                        let file_loc = parse_file_location(&line_text, col);

                        if let Some((path, line, col_num)) = file_loc {
                            // Open in editor
                            if let Some(ui) = &mut self.ui {
                                match ui.editor_view.open_file(path) {
                                    Ok(buffer_id) => {
                                        let Some(idx) =
                                            ui.editor_view.editor.index_for_id(buffer_id)
                                        else {
                                            return;
                                        };
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

                        // Fall back to URL detection using regex-based detect_urls
                        let url = session
                            .terminal
                            .cell_hyperlink(row, col)
                            .or_else(|| {
                                let line_text = session.terminal.row_text(row);
                                llnzy::terminal::detect_urls(&line_text)
                                    .into_iter()
                                    .find(|(start, end, _)| col >= *start && col < *end)
                                    .map(|(_, _, url)| url)
                            })
                            .or_else(|| {
                                let text = session.terminal.word_at(row, col);
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

                let local_terminal_selection = local_terminal_selection_requested(
                    self.mouse_reporting(),
                    self.modifiers.shift_key(),
                    self.terminal_selection_drag,
                );
                if self.mouse_reporting()
                    && button == MouseButton::Left
                    && !local_terminal_selection
                {
                    match state {
                        ElementState::Pressed => {
                            self.clear_terminal_selection();
                            self.mouse_pressed = true;
                            self.terminal_selection_drag = false;
                            self.terminal_pending_mouse_press = Some((row, col));
                            self.request_redraw();
                        }
                        ElementState::Released => {
                            if let Some((press_row, press_col)) =
                                self.terminal_pending_mouse_press.take()
                            {
                                let sgr = self.sgr_mouse();
                                let press = input::encode_mouse(
                                    0,
                                    press_col,
                                    press_row,
                                    true,
                                    sgr,
                                    &self.modifiers,
                                );
                                let release =
                                    input::encode_mouse(0, col, row, false, sgr, &self.modifiers);
                                self.write_to_active(&press);
                                self.write_to_active(&release);
                            } else if self.terminal_selection_drag {
                                if let Some(session) = self.active_session_mut() {
                                    session.terminal.update_selection(row, col);
                                }
                                self.request_redraw();
                            }
                            self.mouse_pressed = false;
                            self.terminal_selection_drag = false;
                        }
                    }
                    return;
                }

                if button == MouseButton::Left {
                    match state {
                        ElementState::Pressed => {
                            self.terminal_selection_drag = local_terminal_selection;
                            let click_count = self.click_state.click(row, col);
                            match click_count {
                                2 => {
                                    if let Some(session) = self.active_session_mut() {
                                        session.terminal.select_word(row, col);
                                    }
                                }
                                3 => {
                                    if let Some(session) = self.active_session_mut() {
                                        session.terminal.select_line(row);
                                    }
                                }
                                _ => {
                                    if let Some(session) = self.active_session_mut() {
                                        session.terminal.start_selection(row, col);
                                    }
                                }
                            }
                            self.mouse_pressed = true;
                            self.request_redraw();
                        }
                        ElementState::Released => {
                            if let Some(session) = self.active_session_mut() {
                                session.terminal.update_selection(row, col);
                            }
                            self.mouse_pressed = false;
                            self.terminal_selection_drag = false;
                            self.terminal_pending_mouse_press = None;
                        }
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = position;

                if self.mouse_pressed {
                    let (row, col) = self.pixel_to_grid(position);
                    if self.mouse_reporting() && !self.modifiers.shift_key() {
                        if self.terminal_selection_drag {
                            if let Some(session) = self.active_session_mut() {
                                session.terminal.update_selection(row, col);
                            }
                            self.request_redraw();
                        } else if let Some(start) = self.terminal_pending_mouse_press {
                            if terminal_mouse_drag_exceeded(start, row, col) {
                                self.terminal_pending_mouse_press = None;
                                self.terminal_selection_drag = true;
                                if let Some(session) = self.active_session_mut() {
                                    session.terminal.start_selection(start.0, start.1);
                                    session.terminal.update_selection(row, col);
                                }
                                self.request_redraw();
                            }
                        }
                    } else if self.click_state.count() <= 1 {
                        if let Some(session) = self.active_session_mut() {
                            session.terminal.update_selection(row, col);
                        }
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
                            if let Some(terminal) = self
                                .tabs
                                .get(self.active_tab)
                                .and_then(|t| t.content.as_terminal())
                                .map(|s| &s.terminal)
                            {
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
                                        if let Some(terminal) = self
                                            .tabs
                                            .get(self.active_tab)
                                            .and_then(|t| t.content.as_terminal())
                                            .map(|s| &s.terminal)
                                        {
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
                                        if let Some(terminal) = self
                                            .tabs
                                            .get(self.active_tab)
                                            .and_then(|t| t.content.as_terminal())
                                            .map(|s| &s.terminal)
                                        {
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
                    if let Some(command) =
                        app_command_for_keybinding(&action, self.active_tab, self.tabs.len())
                    {
                        let mut sidebar_changed = false;
                        if self.handle_app_command(command, &mut sidebar_changed) && sidebar_changed
                        {
                            self.recompute_layout();
                            self.resize_terminal_tabs();
                        }
                        return;
                    }

                    match action {
                        Action::Search => {
                            if self.route_code_editor_command(CommandId::Find) {
                                return;
                            }
                            // Search only works on terminal tabs
                            if self.active_session().is_some() {
                                self.search.toggle();
                                self.request_redraw();
                            }
                        }
                        Action::Copy => {
                            if self.route_code_editor_command(CommandId::Copy) {
                                return;
                            }
                            if !self.copy_stacker_editor_selection() {
                                self.copy_selection();
                            }
                        }
                        Action::Paste => {
                            if self.route_code_editor_command(CommandId::Paste) {
                                return;
                            }
                            self.do_paste();
                        }
                        Action::SelectAll => {
                            if self.route_code_editor_command(CommandId::SelectAll) {
                                return;
                            }
                            if !self.select_all_stacker_editor() {
                                self.do_select_all();
                            }
                        }
                        Action::ToggleErrorPanel => {
                            self.error_panel.toggle();
                            self.request_redraw();
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
                        Action::NewTab
                        | Action::CloseTab
                        | Action::NextTab
                        | Action::PrevTab
                        | Action::SplitVertical
                        | Action::SplitHorizontal
                        | Action::ToggleFullscreen
                        | Action::ToggleEffects
                        | Action::ToggleFps
                        | Action::ToggleSidebar
                        | Action::SwitchTab(_) => {}
                    }
                    return;
                }

                if self.active_stacker_tab()
                    && !self.modifiers.control_key()
                    && !self.modifiers.alt_key()
                    && !self.modifiers.super_key()
                {
                    match key_event.logical_key {
                        Key::Named(NamedKey::Backspace) => {
                            self.delete_stacker_editor_backward();
                            return;
                        }
                        Key::Named(NamedKey::Delete) => {
                            self.delete_stacker_editor_forward();
                            return;
                        }
                        _ => {}
                    }
                }

                if let Some(ref text) = key_event.text {
                    let s = text.as_str();
                    if !s.is_empty()
                        && !self.modifiers.control_key()
                        && !self.modifiers.alt_key()
                        && !self.modifiers.super_key()
                        && self.append_text_to_stacker_editor(s)
                    {
                        llnzy::external_input_trace::trace("stacker.keyboard_text", || {
                            format!("chars={}", s.chars().count())
                        });
                        return;
                    }
                }

                // Only send raw keys to PTY if active tab is a terminal
                if self.active_session().is_some() {
                    if input::is_modifier_only_key(&key_event) {
                        return;
                    }

                    if self.terminal_selection_active() {
                        self.clear_terminal_selection();
                        self.request_redraw();
                    }

                    self.last_keypress = Instant::now();
                    if let Some(ref text) = key_event.text {
                        let s = text.as_str();
                        if !s.is_empty()
                            && !self.modifiers.control_key()
                            && !self.modifiers.alt_key()
                            && !self.modifiers.super_key()
                            && input::text_should_use_paste_path(s)
                        {
                            self.write_text_to_active(s);
                            return;
                        }
                    }
                    let app_cursor = self.app_cursor();
                    if let Some(bytes) = input::encode_key(&key_event, self.modifiers, app_cursor) {
                        self.write_to_active(&bytes);
                    }
                }
            }

            // IME (input method) events — handles composed text from
            // non-US keyboards, dead keys, and CJK input methods.
            WindowEvent::Ime(ime) => {
                if let Ime::Commit(text) = &ime {
                    if self.append_text_to_stacker_editor(text) {
                        llnzy::external_input_trace::trace("stacker.ime_commit", || {
                            format!("chars={}", text.chars().count())
                        });
                        return;
                    }
                }

                if self.active_session().is_some() {
                    match ime {
                        Ime::Commit(text) => {
                            self.last_keypress = Instant::now();
                            if self.terminal_selection_active() {
                                self.clear_terminal_selection();
                            }
                            if self.search.active {
                                if let Some(terminal) = self
                                    .tabs
                                    .get(self.active_tab)
                                    .and_then(|t| t.content.as_terminal())
                                    .map(|s| &s.terminal)
                                {
                                    for ch in text.chars() {
                                        if !ch.is_control() {
                                            self.search.push_char(ch, terminal);
                                        }
                                    }
                                }
                                self.request_redraw();
                            } else {
                                self.write_text_to_active(&text);
                                self.request_redraw();
                            }
                        }
                        Ime::Preedit(_, _) => {}
                        Ime::Enabled | Ime::Disabled => {}
                    }
                }
            }

            // Drag-and-drop: insert escaped file path into terminal
            WindowEvent::DroppedFile(path) => {
                let target = self.native_file_drop_target(&path);
                if let (Some(ui), Some(target)) = (&mut self.ui, target) {
                    if let Some(command) =
                        ui.drag_drop.command_for_external_files(vec![path], target)
                    {
                        let mut sidebar_changed = false;
                        if self
                            .handle_app_command(AppCommand::DragDrop(command), &mut sidebar_changed)
                        {
                            if sidebar_changed {
                                self.recompute_layout();
                                self.resize_terminal_tabs();
                            }
                            self.request_redraw();
                        }
                    }
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
            UserEvent::StackerNativeEdit(edit) => {
                self.apply_stacker_native_edit(edit);
            }
            #[cfg(target_os = "macos")]
            UserEvent::MenuAction(action) => {
                use llnzy::menu::MenuAction;
                match action {
                    MenuAction::NewTab => {
                        let mut sidebar_changed = false;
                        self.handle_app_command(AppCommand::NewTerminalTab, &mut sidebar_changed);
                    }
                    MenuAction::Save => {
                        self.route_code_editor_command(CommandId::Save);
                    }
                    MenuAction::CloseTab => {
                        let mut sidebar_changed = false;
                        self.handle_app_command(
                            AppCommand::CloseTab(self.active_tab),
                            &mut sidebar_changed,
                        );
                    }
                    MenuAction::TabJoin => {
                        self.join_active_tab_with_next();
                    }
                    MenuAction::TabSeparate => {
                        let mut sidebar_changed = false;
                        self.handle_app_command(AppCommand::SeparateTabs, &mut sidebar_changed);
                    }
                    MenuAction::TabSplit => {
                        self.split_active_tab();
                    }
                    MenuAction::TabRename => {
                        self.start_renaming_active_tab();
                    }
                    MenuAction::Undo => {
                        if !self.undo_stacker_editor() {
                            self.route_code_editor_command(CommandId::Undo);
                        }
                    }
                    MenuAction::Redo => {
                        if !self.redo_stacker_editor() {
                            self.route_code_editor_command(CommandId::Redo);
                        }
                    }
                    MenuAction::Copy => {
                        if self.route_code_editor_command(CommandId::Copy) {
                            return;
                        }
                        if !self.copy_stacker_editor_selection() {
                            self.copy_selection();
                        }
                    }
                    MenuAction::Paste => {
                        if self.route_code_editor_command(CommandId::Paste) {
                            return;
                        }
                        self.do_paste();
                    }
                    MenuAction::SelectAll => {
                        if self.route_code_editor_command(CommandId::SelectAll) {
                            return;
                        }
                        if !self.select_all_stacker_editor() {
                            self.do_select_all();
                        }
                    }
                    MenuAction::Find => {
                        if self.route_code_editor_command(CommandId::Find) {
                            return;
                        }
                        self.search.toggle();
                        self.request_redraw();
                    }
                    MenuAction::ToggleFullscreen => {
                        let mut sidebar_changed = false;
                        self.handle_app_command(AppCommand::ToggleFullscreen, &mut sidebar_changed);
                    }
                    MenuAction::SplitVertical | MenuAction::SplitHorizontal => {
                        let mut sidebar_changed = false;
                        self.handle_app_command(AppCommand::NewTerminalTab, &mut sidebar_changed);
                    }
                    MenuAction::ToggleWordWrap => {
                        self.toggle_editor_word_wrap();
                    }
                    MenuAction::ToggleEffects => {
                        let mut sidebar_changed = false;
                        self.handle_app_command(AppCommand::ToggleEffects, &mut sidebar_changed);
                    }
                    MenuAction::OpenProject => {
                        let mut sidebar_changed = false;
                        if self
                            .handle_app_command(AppCommand::PickOpenProject, &mut sidebar_changed)
                        {
                            if sidebar_changed {
                                self.recompute_layout();
                                self.resize_terminal_tabs();
                            }
                            self.request_redraw();
                        }
                    }
                    MenuAction::CloseProject => {
                        if let Some(ui) = &mut self.ui {
                            ui.explorer.clear();
                            ui.sidebar.open = false;
                            ui.active_view = ActiveView::Shells;
                        }
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
        let terminal_active = self.screen_layout.as_ref().is_some_and(|layout| {
            self.ui.as_ref().is_some_and(|ui| {
                terminal_effect_rect(&self.tabs, layout, &ui.tab_groups, self.active_tab).is_some()
            })
        });
        let ui_active = self.ui.as_ref().is_some_and(|u| u.settings_open());
        if (terminal_active && self.config.effects.any_active()) || ui_active {
            event_loop.set_control_flow(ControlFlow::Poll);
            self.request_redraw();
        }
    }
}

fn joined_terminal_panes<'a>(
    tabs: &'a [llnzy::workspace::WorkspaceTab],
    active_tab: usize,
    layout: &ScreenLayout,
    joined: JoinedTabs,
) -> Vec<TerminalPane<'a>> {
    let (left_rect, right_rect) = joined_terminal_content_rects(layout, joined.ratio);
    [(joined.primary, left_rect), (joined.secondary, right_rect)]
        .into_iter()
        .filter_map(|(idx, rect)| {
            tabs.get(idx)
                .and_then(|tab| tab.content.as_terminal().map(|terminal| (tab, terminal)))
                .map(|(tab, terminal)| TerminalPane {
                    terminal,
                    tab_id: tab.id,
                    rect,
                    active: idx == active_tab,
                })
        })
        .collect()
}

fn rect_to_uv(rect: llnzy::session::Rect, size: winit::dpi::PhysicalSize<u32>) -> [f32; 4] {
    let w = size.width.max(1) as f32;
    let h = size.height.max(1) as f32;
    [
        rect.x / w,
        rect.y / h,
        (rect.x + rect.w) / w,
        (rect.y + rect.h) / h,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_mouse_reporting_uses_local_selection_when_shift_is_held() {
        assert!(local_terminal_selection_requested(true, true, false));
    }

    #[test]
    fn terminal_mouse_reporting_keeps_existing_local_selection_drag() {
        assert!(local_terminal_selection_requested(true, false, true));
    }

    #[test]
    fn terminal_mouse_reporting_routes_normal_mouse_to_cli() {
        assert!(!local_terminal_selection_requested(true, false, false));
        assert!(!local_terminal_selection_requested(false, true, false));
    }

    #[test]
    fn terminal_mouse_drag_starts_after_leaving_press_cell() {
        assert!(!terminal_mouse_drag_exceeded((4, 8), 4, 8));
        assert!(terminal_mouse_drag_exceeded((4, 8), 4, 9));
        assert!(terminal_mouse_drag_exceeded((4, 8), 5, 8));
    }
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
