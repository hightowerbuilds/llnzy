use std::sync::Arc;
use std::time::{Duration, Instant};

mod main_app;
mod runtime;
#[path = "main/stacker_input.rs"]
mod stacker_input;
#[path = "main/terminal_mouse.rs"]
mod terminal_mouse;

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
use llnzy::app::zoom_shortcuts::app_zoom_shortcut_command;
use llnzy::config::Config;
use llnzy::diagnostics::write_diagnostic;
use llnzy::error_log::{ErrorLog, ErrorPanel};
use llnzy::external_command::ExternalAction;
use llnzy::keybindings::{primary_modifier, Action};
use llnzy::layout::{logical_to_physical_width, LayoutInputs, ScreenLayout};
use llnzy::performance::PowerSource;
use llnzy::renderer::{RenderRequest, Renderer, TerminalPane};
use llnzy::search::Search;
use llnzy::stacker::commands::{stacker_command_registry, StackerCommandId};
#[cfg(target_os = "macos")]
use llnzy::stacker::input::StackerSelection;
#[cfg(target_os = "macos")]
use llnzy::stacker_input_client::StackerInputClient;
use llnzy::ui::command_palette::CommandId;
#[cfg(target_os = "macos")]
use llnzy::ui::{stacker_cursor, STACKER_PROMPT_EDITOR_ID};
use llnzy::ui::{ActiveView, UiFrameOutput, UiState, BUMPER_WIDTH};
use llnzy::workspace::{TabContent, TabKind, WorkspaceTab};
use llnzy::workspace_layout::{
    active_joined_tabs, joined_pane_rects, joined_terminal_content_rects, terminal_effect_rect,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HistoryCommand {
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
    rects: Arc<[SelectionRect]>,
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
    #[cfg(target_os = "macos")]
    stacker_input_client: Option<StackerInputClient>,
    #[cfg(target_os = "macos")]
    stacker_pending_focus: bool,
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
            #[cfg(target_os = "macos")]
            stacker_input_client: None,
            #[cfg(target_os = "macos")]
            stacker_pending_focus: false,
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
        }
    }

    fn alloc_tab_id(&mut self) -> u64 {
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        id
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
                self.dispatch_active_external_action(ExternalAction::Undo);
            }
            menu::COMMAND_REDO => {
                self.dispatch_active_external_action(ExternalAction::Redo);
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

fn local_terminal_selection_requested(
    mouse_reporting: bool,
    shift_key: bool,
    terminal_selection_drag: bool,
) -> bool {
    mouse_reporting && (shift_key || terminal_selection_drag)
}

fn document_history_shortcut(key: &Key, modifiers: ModifiersState) -> Option<HistoryCommand> {
    if !primary_modifier(modifiers) || modifiers.alt_key() {
        return None;
    }

    let Key::Character(key) = key else {
        return None;
    };

    match (key.to_lowercase().as_str(), modifiers.shift_key()) {
        ("z", false) => Some(HistoryCommand::Undo),
        ("z", true) | ("y", false) => Some(HistoryCommand::Redo),
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

fn main() {
    env_logger::init();

    // Headless CLI dispatch — must run before any GUI/winit/wgpu setup so
    // agents calling `llnzy prompt add ...` never spin up a window.
    {
        let argv: Vec<String> = std::env::args().collect();
        if argv.get(1).map(String::as_str) == Some("prompt") {
            std::process::exit(llnzy::stacker::cli::run_from_env());
        }
    }

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

#[cfg(test)]
#[path = "main/tests.rs"]
mod tests;
