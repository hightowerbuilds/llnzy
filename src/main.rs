use std::sync::Arc;
use std::time::{Duration, Instant};

mod main_app;
mod runtime;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};
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
use llnzy::keybindings::{primary_modifier, Action};
use llnzy::layout::{logical_to_physical_width, LayoutInputs, ScreenLayout};
use llnzy::performance::PowerSource;
use llnzy::renderer::{RenderRequest, Renderer, TerminalPane};
use llnzy::search::Search;
use llnzy::stacker::commands::{stacker_command_registry, StackerCommandId};
use llnzy::stacker::input::StackerSelection;
use llnzy::stacker_webview::{StackerWebView, StackerWebViewMessage};
use llnzy::ui::command_palette::CommandId;
use llnzy::ui::{stacker_cursor, STACKER_PROMPT_EDITOR_ID};
use llnzy::ui::{ActiveView, UiFrameOutput, UiState, BUMPER_WIDTH};
use llnzy::workspace::{TabContent, TabKind, WorkspaceTab};
use llnzy::workspace_layout::{
    active_joined_tabs, joined_content_rects, joined_terminal_content_rects, terminal_effect_rect,
    JoinedTabs,
};
use llnzy::UserEvent;

type SelectionRect = (f32, f32, f32, f32, [f32; 4]);

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

#[derive(Clone)]
struct SelectionRectCache {
    tab_id: u64,
    revision: u64,
    cell_w_bits: u32,
    cell_h_bits: u32,
    color: [u8; 3],
    alpha_bits: u32,
    rects: Vec<SelectionRect>,
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
    clipboard: llnzy::platform::clipboard::PlatformClipboard,
    cursor_pos: winit::dpi::PhysicalPosition<f64>,
    mouse_pressed: bool,
    terminal_selection_drag: bool,
    terminal_pending_mouse_press: Option<(usize, usize)>,
    selection_rect_cache: Option<SelectionRectCache>,
    click_state: ClickState,
    stacker_webview: Option<StackerWebView>,
    stacker_webview_pending_focus: bool,
    ui: Option<UiState>,
    screen_layout: Option<ScreenLayout>,
    visual_bell_until: Option<Instant>,
    last_ui_config_change: Instant,
    last_blink_toggle: Instant,
    last_keypress: Instant,
    last_config_check: Instant,
    last_power_check: Instant,
    current_power_source: PowerSource,
    last_editor_recovery_save: Instant,
    #[cfg(target_os = "macos")]
    stacker_bridge_active: Option<bool>,
    #[cfg(target_os = "macos")]
    stacker_bridge_text: String,
}

impl App {
    fn new(proxy: winit::event_loop::EventLoopProxy<UserEvent>) -> Self {
        let clipboard = llnzy::platform::clipboard::PlatformClipboard::current();
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
            selection_rect_cache: None,
            click_state: ClickState::new(),
            stacker_webview: None,
            stacker_webview_pending_focus: false,
            ui: None,
            screen_layout: None,
            visual_bell_until: None,
            last_ui_config_change: Instant::now() - std::time::Duration::from_secs(60),
            last_blink_toggle: Instant::now(),
            last_keypress: Instant::now(),
            last_config_check: Instant::now(),
            last_power_check: Instant::now() - Duration::from_secs(60),
            current_power_source: PowerSource::Unknown,
            last_editor_recovery_save: Instant::now(),
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

    fn stacker_visible_in_active_context(&self) -> bool {
        self.stacker_tab_in_active_context().is_some()
    }

    fn stacker_tab_in_active_context(&self) -> Option<usize> {
        if self.active_stacker_tab() {
            return Some(self.active_tab);
        }
        let Some(ui) = &self.ui else {
            return None;
        };
        let Some(joined) = active_joined_tabs(&self.tabs, self.active_tab, &ui.tab_groups) else {
            return None;
        };

        [joined.primary, joined.secondary].into_iter().find(|&idx| {
            self.tabs
                .get(idx)
                .is_some_and(|tab| matches!(tab.content, TabContent::Stacker))
        })
    }

    fn create_stacker_webview(&mut self) {
        if self.stacker_webview.is_some() {
            return;
        }
        let Some(window) = &self.window else { return };
        match StackerWebView::new(window.as_ref(), self.proxy.clone()) {
            Ok(webview) => {
                self.stacker_webview = Some(webview);
                self.error_log
                    .info("Stacker WebView editor initialized for native text input");
            }
            Err(err) => {
                self.error_log.error(err);
            }
        }
    }

    fn sync_stacker_webview(&mut self) {
        let active = self.stacker_visible_in_active_context();
        let focus_when_shown = self.active_stacker_tab();
        let Some(webview) = &mut self.stacker_webview else {
            return;
        };
        if !active {
            webview.set_visible(false);
            self.stacker_webview_pending_focus = false;
            return;
        }

        let Some(ui) = &self.ui else {
            webview.set_visible(false);
            return;
        };
        let modal_open = ui.stacker.pending_draft_switch.is_some()
            || ui.stacker.pending_prompt_delete.is_some()
            || ui.pending_close.is_some();
        let Some(rect) = ui.stacker.web_editor_rect else {
            webview.set_visible(false);
            return;
        };
        if modal_open {
            webview.set_visible(false);
            return;
        }

        webview.set_bounds(rect);
        webview.set_font_size(ui.stacker.editor_font_size);
        webview.set_document(ui.stacker.editor.text(), ui.stacker.editor.selection());
        let became_visible = webview.set_visible(true);
        let should_focus =
            focus_when_shown && (self.stacker_webview_pending_focus || became_visible);
        if should_focus {
            webview.focus();
            self.stacker_webview_pending_focus = false;
        }
    }

    fn apply_stacker_webview_message(&mut self, raw: String) -> bool {
        let Ok(message) = serde_json::from_str::<StackerWebViewMessage>(&raw) else {
            self.error_log
                .error(format!("Invalid Stacker WebView message: {raw}"));
            return false;
        };
        let Some(stacker_tab_idx) = self.stacker_tab_in_active_context() else {
            return false;
        };
        let should_activate_stacker = matches!(
            message.message_type.as_str(),
            "pointerDown" | "focus" | "textChanged" | "selectionChanged"
        ) && stacker_tab_idx != self.active_tab;
        if should_activate_stacker {
            self.switch_tab(stacker_tab_idx);
            self.stacker_webview_pending_focus = true;
        }

        let Some(ui) = &mut self.ui else {
            return false;
        };
        let selection = StackerSelection {
            start: llnzy::stacker_webview::utf16_index_to_char_index(
                &message.text,
                message.selection_start,
            ),
            end: llnzy::stacker_webview::utf16_index_to_char_index(
                &message.text,
                message.selection_end,
            ),
        };

        match message.message_type.as_str() {
            "textChanged" => {
                let changed = ui
                    .stacker
                    .editor
                    .replace_all_with_history(message.text.clone(), selection);
                store_stacker_webview_selection(ui, selection);
                ui.stacker
                    .draft
                    .record_current_text(ui.stacker.editor.text().to_string());
                if let Some(webview) = &mut self.stacker_webview {
                    webview.note_webview_document(ui.stacker.editor.text(), selection);
                }
                if changed {
                    llnzy::external_input_trace::trace("stacker.webview_text_changed", || {
                        format!("chars={}", ui.stacker.editor.text().chars().count())
                    });
                }
                self.request_redraw();
                true
            }
            "selectionChanged" | "focus" => {
                store_stacker_webview_selection(ui, selection);
                if let Some(webview) = &mut self.stacker_webview {
                    webview.note_webview_document(ui.stacker.editor.text(), selection);
                }
                true
            }
            "pointerDown" => {
                self.stacker_webview_pending_focus = true;
                self.request_redraw();
                true
            }
            _ => false,
        }
    }

    #[cfg(target_os = "macos")]
    fn sync_macos_text_bridge(&mut self) {
        let Some(window) = &self.window else { return };
        if self.stacker_webview.is_some() {
            if self.stacker_bridge_active != Some(false) {
                llnzy::macos_text_bridge::set_stacker_active(window, false, "");
                self.stacker_bridge_active = Some(false);
                self.stacker_bridge_text.clear();
            }
            return;
        }
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

    fn joined_tab_at_cursor(&self) -> Option<usize> {
        let layout = self.screen_layout.as_ref()?;
        let x = self.cursor_pos.x as f32;
        let y = self.cursor_pos.y as f32;
        let joined = self
            .ui
            .as_ref()
            .and_then(|ui| active_joined_tabs(&self.tabs, self.active_tab, &ui.tab_groups))?;
        let (left_rect, right_rect) = joined_content_rects(layout, joined.ratio);

        if rect_contains(left_rect, x, y) {
            Some(joined.primary)
        } else if rect_contains(right_rect, x, y) {
            Some(joined.secondary)
        } else {
            None
        }
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

    fn active_selection_rects(&mut self, cell_w: f32, cell_h: f32) -> Vec<SelectionRect> {
        let Some(tab) = self.tabs.get(self.active_tab) else {
            self.selection_rect_cache = None;
            return Vec::new();
        };
        let Some(session) = tab.content.as_terminal() else {
            self.selection_rect_cache = None;
            return Vec::new();
        };

        let tab_id = tab.id;
        let revision = session.terminal.selection_revision();
        let cell_w_bits = cell_w.to_bits();
        let cell_h_bits = cell_h.to_bits();
        let color = self.config.colors.selection;
        let alpha = self.config.colors.selection_alpha;
        let alpha_bits = alpha.to_bits();

        if let Some(cache) = &self.selection_rect_cache {
            if cache.tab_id == tab_id
                && cache.revision == revision
                && cache.cell_w_bits == cell_w_bits
                && cache.cell_h_bits == cell_h_bits
                && cache.color == color
                && cache.alpha_bits == alpha_bits
            {
                return cache.rects.clone();
            }
        }

        let rects = session
            .terminal
            .selection_rects(cell_w, cell_h, color, alpha);
        self.selection_rect_cache = Some(SelectionRectCache {
            tab_id,
            revision,
            cell_w_bits,
            cell_h_bits,
            color,
            alpha_bits,
            rects: rects.clone(),
        });
        rects
    }

    fn update_active_terminal_selection(&mut self, row: usize, col: usize) -> bool {
        self.active_session_mut()
            .is_some_and(|session| session.terminal.update_selection(row, col))
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
                let intent = llnzy::platform::input::mouse_report_intent(
                    button,
                    hit.col,
                    hit.row,
                    true,
                    sgr,
                    &self.modifiers,
                );
                if let llnzy::platform::input::PlatformInputIntent::MouseReport(bytes) = intent {
                    self.write_to_terminal_tab(hit.tab_idx, &bytes);
                }
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

    #[cfg(target_os = "macos")]
    fn handle_platform_menu_command(&mut self, command_id: &str) {
        use llnzy::platform::menu;

        match command_id {
            menu::COMMAND_NEW_WINDOW => {
                let mut sidebar_changed = false;
                self.handle_app_command(AppCommand::NewWindow, &mut sidebar_changed);
            }
            menu::COMMAND_NEW_TAB => {
                let mut sidebar_changed = false;
                self.handle_app_command(AppCommand::NewTerminalTab, &mut sidebar_changed);
            }
            menu::COMMAND_SAVE => {
                self.route_code_editor_command(CommandId::Save);
            }
            menu::COMMAND_CLOSE_TAB => {
                let mut sidebar_changed = false;
                self.handle_app_command(
                    AppCommand::CloseTab(self.active_tab),
                    &mut sidebar_changed,
                );
            }
            menu::COMMAND_TAB_JOIN => {
                self.join_active_tab_with_next();
            }
            menu::COMMAND_TAB_SEPARATE => {
                let mut sidebar_changed = false;
                self.handle_app_command(AppCommand::SeparateTabs, &mut sidebar_changed);
            }
            menu::COMMAND_TAB_SPLIT => {
                self.split_active_tab();
            }
            menu::COMMAND_TAB_RENAME => {
                self.start_renaming_active_tab();
            }
            menu::COMMAND_UNDO => {
                if !self.undo_stacker_editor() {
                    self.route_code_editor_command(CommandId::Undo);
                }
            }
            menu::COMMAND_REDO => {
                if !self.redo_stacker_editor() {
                    self.route_code_editor_command(CommandId::Redo);
                }
            }
            menu::COMMAND_COPY => {
                if self.route_code_editor_command(CommandId::Copy) {
                    return;
                }
                if !self.copy_stacker_editor_selection() {
                    self.copy_selection();
                }
            }
            menu::COMMAND_PASTE => {
                if self.route_code_editor_command(CommandId::Paste) {
                    return;
                }
                self.do_paste();
            }
            menu::COMMAND_SELECT_ALL => {
                if self.route_code_editor_command(CommandId::SelectAll) {
                    return;
                }
                if !self.select_all_stacker_editor() {
                    self.do_select_all();
                }
            }
            menu::COMMAND_FIND => {
                if self.route_code_editor_command(CommandId::Find) {
                    return;
                }
                self.search.toggle();
                self.request_redraw();
            }
            menu::COMMAND_TOGGLE_FULLSCREEN => {
                let mut sidebar_changed = false;
                self.handle_app_command(AppCommand::ToggleFullscreen, &mut sidebar_changed);
            }
            menu::COMMAND_SPLIT_VERTICAL | menu::COMMAND_SPLIT_HORIZONTAL => {
                let mut sidebar_changed = false;
                self.handle_app_command(AppCommand::NewTerminalTab, &mut sidebar_changed);
            }
            menu::COMMAND_TOGGLE_WORD_WRAP => {
                self.toggle_editor_word_wrap();
            }
            menu::COMMAND_TOGGLE_EFFECTS => {
                let mut sidebar_changed = false;
                self.handle_app_command(AppCommand::ToggleEffects, &mut sidebar_changed);
            }
            menu::COMMAND_ZOOM_IN => {
                let mut sidebar_changed = false;
                self.handle_app_command(AppCommand::ZoomIn, &mut sidebar_changed);
            }
            menu::COMMAND_ZOOM_OUT => {
                let mut sidebar_changed = false;
                self.handle_app_command(AppCommand::ZoomOut, &mut sidebar_changed);
            }
            menu::COMMAND_ZOOM_RESET => {
                let mut sidebar_changed = false;
                self.handle_app_command(AppCommand::ZoomReset, &mut sidebar_changed);
            }
            menu::COMMAND_OPEN_PROJECT => {
                let mut sidebar_changed = false;
                if self.handle_app_command(AppCommand::PickOpenProject, &mut sidebar_changed) {
                    if sidebar_changed {
                        self.recompute_layout();
                        self.resize_terminal_tabs();
                    }
                    self.request_redraw();
                }
            }
            menu::COMMAND_CLOSE_PROJECT => {
                if let Some(ui) = &mut self.ui {
                    ui.explorer.clear();
                    ui.sidebar.open = false;
                    ui.active_view = ActiveView::Shells;
                }
                self.recompute_layout();
                self.request_redraw();
            }
            _ => {
                log::warn!("Unknown platform menu command: {command_id}");
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

fn app_zoom_shortcut_command(
    key_event: &winit::event::KeyEvent,
    modifiers: ModifiersState,
) -> Option<AppCommand> {
    app_zoom_shortcut_command_for_key(&key_event.logical_key, key_event.physical_key, modifiers)
}

fn app_zoom_shortcut_command_for_key(
    logical_key: &Key,
    physical_key: PhysicalKey,
    modifiers: ModifiersState,
) -> Option<AppCommand> {
    if !primary_modifier(modifiers) || modifiers.alt_key() {
        return None;
    }

    match physical_key {
        PhysicalKey::Code(KeyCode::Equal) | PhysicalKey::Code(KeyCode::NumpadAdd) => {
            Some(AppCommand::ZoomIn)
        }
        PhysicalKey::Code(KeyCode::Minus) | PhysicalKey::Code(KeyCode::NumpadSubtract) => {
            Some(AppCommand::ZoomOut)
        }
        PhysicalKey::Code(KeyCode::Digit0) | PhysicalKey::Code(KeyCode::Numpad0) => {
            Some(AppCommand::ZoomReset)
        }
        _ => match logical_key {
            Key::Character(ch) if ch.as_str() == "+" || ch.as_str() == "=" => {
                Some(AppCommand::ZoomIn)
            }
            Key::Character(ch) if ch.as_str() == "-" => Some(AppCommand::ZoomOut),
            Key::Character(ch) if ch.as_str() == "0" => Some(AppCommand::ZoomReset),
            _ => None,
        },
    }
}

fn store_stacker_webview_selection(ui: &mut UiState, selection: StackerSelection) {
    let editor_id = egui::Id::new(STACKER_PROMPT_EDITOR_ID);
    let ctx = ui.ctx.clone();
    stacker_cursor::store_document_selection(&ctx, editor_id, &mut ui.stacker.editor, selection);
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

    let key = key.to_lowercase();
    stacker_command_registry()
        .iter()
        .find(|command| {
            !matches!(
                command.id,
                StackerCommandId::Clear | StackerCommandId::Undo | StackerCommandId::Redo
            ) && stacker_command_shortcut_matches(command.keybinding, &key, modifiers.shift_key())
        })
        .map(|command| command.id)
}

fn stacker_command_shortcut_matches(keybinding: &str, key: &str, shift_pressed: bool) -> bool {
    let Some(rest) = keybinding.strip_prefix("Cmd+") else {
        return false;
    };
    let (requires_shift, expected) = rest
        .strip_prefix("Shift+")
        .map(|expected| (true, expected))
        .unwrap_or((false, rest));

    requires_shift == shift_pressed && expected.eq_ignore_ascii_case(key)
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
    key_event.text.as_ref().is_some_and(|text| {
        llnzy::platform::input::paste_like_text_input(text.as_str(), modifiers).is_some()
    })
}

fn terminal_mouse_drag_exceeded(start: (usize, usize), row: usize, col: usize) -> bool {
    start != (row, col)
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

    fn primary_mods() -> ModifiersState {
        if cfg!(target_os = "macos") {
            ModifiersState::SUPER
        } else {
            ModifiersState::CONTROL
        }
    }

    fn ch(s: &str) -> Key {
        Key::Character(s.into())
    }

    #[test]
    fn stacker_format_shortcuts_come_from_command_registry() {
        assert_eq!(
            stacker_editor_shortcut(&ch("b"), primary_mods()),
            Some(StackerCommandId::Bold)
        );
        assert_eq!(
            stacker_editor_shortcut(&ch("`"), primary_mods()),
            Some(StackerCommandId::InlineCode)
        );
    }

    #[test]
    fn stacker_format_shortcuts_ignore_plain_text_keys() {
        assert_eq!(
            stacker_editor_shortcut(&ch("b"), ModifiersState::empty()),
            None
        );
    }

    #[test]
    fn app_zoom_shortcuts_use_physical_plus_minus_keys() {
        assert!(matches!(
            app_zoom_shortcut_command_for_key(
                &ch("="),
                PhysicalKey::Code(KeyCode::Equal),
                primary_mods()
            ),
            Some(AppCommand::ZoomIn)
        ));
        assert!(matches!(
            app_zoom_shortcut_command_for_key(
                &ch("-"),
                PhysicalKey::Code(KeyCode::Minus),
                primary_mods()
            ),
            Some(AppCommand::ZoomOut)
        ));
    }

    #[test]
    fn app_zoom_shortcuts_block_plain_plus_minus() {
        assert!(app_zoom_shortcut_command_for_key(
            &ch("="),
            PhysicalKey::Code(KeyCode::Equal),
            ModifiersState::empty()
        )
        .is_none());
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
