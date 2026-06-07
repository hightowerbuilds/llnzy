use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    time::{Duration, Instant},
};

use gpui::prelude::*;
use gpui::{
    actions, div, px, rgb, size, App, Application, Bounds, Context, Entity, FocusHandle, Focusable,
    KeyBinding, KeyDownEvent, Menu, MenuItem, MouseButton, MouseDownEvent, Render, Window,
    WindowBounds, WindowOptions,
};

mod appearance_actions;
mod appearances;
mod command_palette;
mod footer;
mod home;
mod menu_actions;
mod panes;
mod project;
mod recovery;
mod sidebar;
mod tabs;

use crate::config::Config;
use crate::gpui_editor::{bind_editor_keys, EditorPrototype};
use crate::gpui_sketch::{bind_sketch_keys, SketchSurface};
use crate::gpui_stacker::{bind_stacker_keys, StackerPrototype};
use crate::gpui_tabs::{GpuiTabChoice, GpuiTabManager};
use crate::gpui_terminal::{bind_terminal_keys, TerminalSurface};

use self::footer::workspace_footer;
use self::panes::{workspace_content, WorkspaceSurfaceContext};
use self::recovery::{
    plan_restore, recovery_file, save_snapshot, WorkspaceRecoveryAxis,
    WorkspaceRecoveryJoinedGroup, WorkspaceRecoveryPlan, WorkspaceRecoverySnapshot,
    WorkspaceRecoverySurface, WorkspaceRecoveryTab, WorkspaceRecoveryTabNameOverride,
    WORKSPACE_RECOVERY_VERSION,
};
use self::sidebar::{
    collect_explorer_entries, sidebar_bumper, workspace_sidebar, workspace_sidebar_context_menu,
    ExplorerEntry, ExplorerState, SidebarContextMenuState, SidebarNewEntryState,
    SidebarRenameState, WorkspaceSidebarContext,
};
use self::tabs::{
    place_workspace_tabs_together, reorder_workspace_tab_block, workspace_tab_bar,
    workspace_tab_context_menu, workspace_tab_label, workspace_tab_layouts,
    workspace_tab_menu_anchor, TabRenameState, WorkspaceTab, WorkspaceTabId,
    WorkspaceTabMenuAnchor,
};

actions!(
    workspace_gpui,
    [
        Quit,
        MenuNewWindow,
        MenuNewTab,
        MenuCloseTab,
        MenuNextTab,
        MenuPreviousTab,
        MenuActivateTab1,
        MenuActivateTab2,
        MenuActivateTab3,
        MenuActivateTab4,
        MenuActivateTab5,
        MenuActivateTab6,
        MenuActivateTab7,
        MenuActivateTab8,
        MenuActivateTab9,
        MenuShowCommandPalette,
        MenuShowFileFinder,
        MenuJoinTabs,
        MenuSeparateTabs,
        MenuSwapTabs,
        MenuPartitionVertical,
        MenuPartitionHorizontal,
        MenuSave,
        MenuOpenProject,
        MenuCloseProject,
        MenuUndo,
        MenuRedo,
        MenuCopy,
        MenuPaste,
        MenuSelectAll,
        MenuFind,
        MenuEditorCheckDisk,
        MenuEditorReopenClosed,
        MenuEditorCloseOthers,
        MenuEditorCloseSaved,
        MenuMarkdownSource,
        MenuMarkdownPreview,
        MenuMarkdownSplit,
        MenuMarkdownCycle,
        MenuLspHover,
        MenuLspCompletion,
        MenuLspDefinition,
        MenuLspReferences,
        MenuLspSignatureHelp,
        MenuLspRename,
        MenuLspCodeActions,
        MenuLspFormat,
        MenuLspSymbols,
        MenuToggleSidebar,
        MenuShowHome,
        MenuShowTerminal,
        MenuShowStacker,
        MenuShowEditor,
        MenuShowSketch,
        MenuShowAppearances,
        MenuZoomIn,
        MenuZoomOut,
        MenuZoomReset,
    ]
);

const CHROME_BG: u32 = 0x242424;
const BUMPER_BG: u32 = 0x242424;
const PANEL_BG: u32 = 0x1b1b22;
const EDITOR_BG: u32 = 0x191920;
const BORDER: u32 = 0x30323a;
const ACTIVE_TAB_BG: u32 = 0x161616;
const INACTIVE_TAB_BG: u32 = 0x0e0e0e;
const ACTIVE_TEXT: u32 = 0xffffff;
const MUTED_TEXT: u32 = 0xa0a5b4;
const SIDEBAR_TEXT: u32 = 0xabb2bf;
const FOLDER_BLUE: u32 = 0x64b4ff;
const ACCENT: u32 = 0x214966;
const QUEUE_GREEN: u32 = 0x6aff90;

const RECOVERY_PERSIST_INTERVAL: Duration = Duration::from_secs(5);

const FOOTER_HEIGHT: f32 = 48.0;
const JOINED_TAB_DIVIDER_WIDTH: f32 = 8.0;
const SIDEBAR_DEFAULT_WIDTH: f32 = 220.0;
const SIDEBAR_MIN_WIDTH: f32 = 160.0;
const SIDEBAR_MAX_WIDTH: f32 = 380.0;
const BUMPER_WIDTH: f32 = 20.0;
const BUMPER_RESIZE_WIDTH: f32 = 6.0;
const EXPLORER_ENTRY_LIMIT: usize = 260;
const GPUI_TERMINAL_BACKGROUND_MAX_EDGE: u32 = 2048;
const EMPTY_WORKSPACE_TAB_ID: WorkspaceTabId = WorkspaceTabId(0);
const SIDEBAR_ROW_BG: u32 = CHROME_BG;
const SIDEBAR_ROW_HOVER_BG: u32 = 0x2b2e36;
const SIDEBAR_ROW_SELECTED_BG: u32 = 0x303440;
const SIDEBAR_ROW_SELECTED_HOVER_BG: u32 = 0x3a4050;
const SIDEBAR_DROP_VALID_BG: u32 = 0x1f3a2b;
const SIDEBAR_DROP_INVALID_BG: u32 = 0x3d2428;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct WorkspacePalette {
    pub(super) is_light: bool,
    pub(super) chrome_bg: u32,
    pub(super) bumper_bg: u32,
    pub(super) panel_bg: u32,
    pub(super) editor_bg: u32,
    pub(super) border: u32,
    pub(super) active_tab_bg: u32,
    pub(super) inactive_tab_bg: u32,
    pub(super) active_text: u32,
    pub(super) muted_text: u32,
    pub(super) sidebar_text: u32,
    pub(super) accent: u32,
    pub(super) queue_green: u32,
    pub(super) sidebar_row_selected_bg: u32,
    pub(super) sidebar_row_hover_bg: u32,
}

impl WorkspacePalette {
    pub(super) fn from_config(config: &Config) -> Self {
        if config
            .colors
            .background
            .iter()
            .map(|channel| *channel as u16)
            .sum::<u16>()
            > 600
        {
            Self::light()
        } else {
            Self::dark()
        }
    }

    fn dark() -> Self {
        Self {
            is_light: false,
            chrome_bg: CHROME_BG,
            bumper_bg: BUMPER_BG,
            panel_bg: PANEL_BG,
            editor_bg: EDITOR_BG,
            border: BORDER,
            active_tab_bg: ACTIVE_TAB_BG,
            inactive_tab_bg: INACTIVE_TAB_BG,
            active_text: ACTIVE_TEXT,
            muted_text: MUTED_TEXT,
            sidebar_text: SIDEBAR_TEXT,
            accent: ACCENT,
            queue_green: QUEUE_GREEN,
            sidebar_row_selected_bg: SIDEBAR_ROW_SELECTED_BG,
            sidebar_row_hover_bg: SIDEBAR_ROW_HOVER_BG,
        }
    }

    fn light() -> Self {
        Self {
            is_light: true,
            chrome_bg: 0xf0e3d1,
            bumper_bg: 0xe8d8c1,
            panel_bg: 0xfffbf2,
            editor_bg: 0xfaf2e2,
            border: 0xd8c6ad,
            active_tab_bg: 0xfffbf2,
            inactive_tab_bg: 0xe9d8bf,
            active_text: 0x3e372f,
            muted_text: 0x7d7064,
            sidebar_text: 0x5f554b,
            accent: 0xb7d8d4,
            queue_green: 0x5f9f79,
            sidebar_row_selected_bg: 0xe2d0ed,
            sidebar_row_hover_bg: 0xeadcc8,
        }
    }
}

#[cfg(test)]
mod workspace_palette_tests {
    use super::{WorkspacePalette, CHROME_BG};
    use crate::config::Config;

    #[test]
    fn palette_switches_to_light_for_light_config_background() {
        let mut config = Config::default();
        config.colors.background = [250, 242, 226];

        let palette = WorkspacePalette::from_config(&config);

        assert_eq!(palette.editor_bg, 0xfaf2e2);
        assert_ne!(palette.chrome_bg, CHROME_BG);
    }

    #[test]
    fn palette_keeps_dark_defaults_for_dark_config_background() {
        let config = Config::default();

        let palette = WorkspacePalette::from_config(&config);

        assert_eq!(palette.chrome_bg, CHROME_BG);
    }
}

/// Build the initial appearance config for a new workspace session, honoring
/// the user's last persisted background image selection. Any image-related
/// preferences are applied on top of the config-file defaults so a user who
/// imports a background once sees it again on next launch.
fn appearance_config_from_preferences(
    preferences: &crate::preferences::WorkspacePreferences,
) -> Config {
    let mut config = Config::load();
    if let Some(image_ref) = preferences.terminal_background_image.as_deref() {
        if !image_ref.is_empty() {
            config.effects.enabled = true;
            config.effects.background = "image".to_string();
            config.effects.background_image = Some(image_ref.to_string());
        }
    }
    if let Some(fit) =
        crate::config::BackgroundImageFit::parse(preferences.terminal_background_image_fit.as_str())
    {
        config.effects.background_image_fit = fit;
    }
    // Layer the rest of the persisted appearance state on top of the
    // config-file defaults. Each field stays None / empty unless the
    // user has explicitly chosen an override.
    if let Some([c1, c2, c3]) = preferences.terminal_palette {
        config.effects.background_color = Some(c1);
        config.effects.background_color2 = Some(c2);
        config.effects.background_color3 = Some(c3);
    }
    if let Some(intensity) = preferences.terminal_background_intensity {
        config.effects.background_intensity = intensity.clamp(0.05, 1.0);
    }
    if let Some(family) = preferences.terminal_font_family.as_deref() {
        if !family.is_empty() {
            config.font_family = Some(family.to_string());
        }
    }
    if let Some(layout) =
        crate::config::TerminalLayoutMode::parse(preferences.terminal_layout.as_str())
    {
        config.terminal_layout = layout;
    }
    if let Some(theme_name) = preferences.editor_syntax_theme.as_deref() {
        if let Some(theme) = crate::config::editor_syntax_preset(theme_name) {
            config.syntax_colors = theme.colors_map();
        }
    }
    if let Some(word_wrap) = preferences.editor_word_wrap {
        apply_editor_word_wrap_preference(&mut config, word_wrap);
    }
    config
}

fn apply_editor_word_wrap_preference(config: &mut Config, word_wrap: bool) {
    config.editor.word_wrap = word_wrap;
    for language in config.editor.languages.values_mut() {
        language.word_wrap = Some(word_wrap);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorkspaceSurface {
    Home,
    Stacker,
    Editor,
    Terminal,
    Explorer,
    Sketch,
    Appearances,
    Settings,
}

impl WorkspaceSurface {
    fn title(self) -> &'static str {
        match self {
            WorkspaceSurface::Home => "Home",
            WorkspaceSurface::Stacker => "Stacker",
            WorkspaceSurface::Editor => "Editor",
            WorkspaceSurface::Terminal => "Terminal",
            WorkspaceSurface::Explorer => "Explorer",
            WorkspaceSurface::Sketch => "Sketch Pad",
            WorkspaceSurface::Appearances => "Appearances",
            WorkspaceSurface::Settings => "Settings",
        }
    }
}

impl From<WorkspaceRecoverySurface> for WorkspaceSurface {
    fn from(surface: WorkspaceRecoverySurface) -> Self {
        match surface {
            WorkspaceRecoverySurface::Home => Self::Home,
            WorkspaceRecoverySurface::Stacker => Self::Stacker,
            WorkspaceRecoverySurface::Editor => Self::Editor,
            WorkspaceRecoverySurface::Terminal => Self::Terminal,
            WorkspaceRecoverySurface::Explorer => Self::Explorer,
            WorkspaceRecoverySurface::Sketch => Self::Sketch,
            WorkspaceRecoverySurface::Appearances => Self::Appearances,
            WorkspaceRecoverySurface::Settings => Self::Settings,
        }
    }
}

impl From<WorkspaceSurface> for WorkspaceRecoverySurface {
    fn from(surface: WorkspaceSurface) -> Self {
        match surface {
            WorkspaceSurface::Home => Self::Home,
            WorkspaceSurface::Stacker => Self::Stacker,
            WorkspaceSurface::Editor => Self::Editor,
            WorkspaceSurface::Terminal => Self::Terminal,
            WorkspaceSurface::Explorer => Self::Explorer,
            WorkspaceSurface::Sketch => Self::Sketch,
            WorkspaceSurface::Appearances => Self::Appearances,
            WorkspaceSurface::Settings => Self::Settings,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AppearancePage {
    Terminal,
    Editor,
    Stacker,
    Sketch,
    App,
    Advanced,
}

impl AppearancePage {
    const ALL: [Self; 6] = [
        Self::Terminal,
        Self::Editor,
        Self::Stacker,
        Self::Sketch,
        Self::App,
        Self::Advanced,
    ];

    fn title(self) -> &'static str {
        match self {
            AppearancePage::Terminal => "Terminal",
            AppearancePage::Editor => "Editor",
            AppearancePage::Stacker => "Stacker",
            AppearancePage::Sketch => "Sketch",
            AppearancePage::App => "App",
            AppearancePage::Advanced => "Advanced",
        }
    }
}

#[derive(Clone, Copy)]
struct JoinedWorkspacePane {
    id: WorkspaceTabId,
    surface: WorkspaceSurface,
}

#[derive(Clone)]
struct JoinedWorkspacePanes {
    panes: Vec<JoinedWorkspacePane>,
    shares: Vec<f32>,
    ratio: f32,
    axis: crate::tab_groups::PartitionAxis,
}

pub fn run_workspace_prototype() {
    Application::new().run(|cx: &mut App| {
        bind_stacker_keys(cx);
        bind_editor_keys(cx);
        bind_terminal_keys(cx);
        bind_sketch_keys(cx);
        install_workspace_menu_bar(cx);
        cx.bind_keys([
            KeyBinding::new("cmd-q", Quit, None),
            KeyBinding::new("cmd-t", MenuNewTab, None),
            KeyBinding::new("cmd-w", MenuCloseTab, None),
            KeyBinding::new("cmd-]", MenuNextTab, None),
            KeyBinding::new("cmd-[", MenuPreviousTab, None),
            KeyBinding::new("cmd-b", MenuToggleSidebar, None),
            // Cmd+1..Cmd+9: activate the Nth tab by position. If the index
            // is out of range (fewer tabs than N) the action is a no-op.
            KeyBinding::new("cmd-1", MenuActivateTab1, None),
            KeyBinding::new("cmd-2", MenuActivateTab2, None),
            KeyBinding::new("cmd-3", MenuActivateTab3, None),
            KeyBinding::new("cmd-4", MenuActivateTab4, None),
            KeyBinding::new("cmd-5", MenuActivateTab5, None),
            KeyBinding::new("cmd-6", MenuActivateTab6, None),
            KeyBinding::new("cmd-7", MenuActivateTab7, None),
            KeyBinding::new("cmd-8", MenuActivateTab8, None),
            KeyBinding::new("cmd-9", MenuActivateTab9, None),
            // Cmd+Shift+P opens the command palette. While the palette is
            // visible, normal key events are routed into its query buffer
            // via `on_workspace_key_down` before the action system sees them.
            KeyBinding::new("cmd-shift-p", MenuShowCommandPalette, None),
            KeyBinding::new("cmd-p", MenuShowFileFinder, None),
        ]);

        let bounds = Bounds::centered(None, size(px(1320.0), px(820.0)), cx);
        let window = match cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(WorkspacePrototype::new),
        ) {
            Ok(window) => window,
            Err(error) => {
                log::error!("failed to open workspace window: {error:?}");
                cx.quit();
                return;
            }
        };
        if let Err(error) = window.update(cx, |view, window, cx| {
            view.focus_active_surface(window, cx);
        }) {
            log::error!("failed to focus workspace window: {error:?}");
            cx.quit();
            return;
        }
        // Capture the window handle (Copy) so the Quit action can reach
        // the workspace entity and persist pending Stacker drafts before
        // the app tears down. Save errors are logged but never block the
        // quit — the user pressed Cmd+Q and wants to leave.
        let window_for_quit = window;
        cx.on_action(move |_: &Quit, cx| {
            if let Err(error) = window_for_quit.update(cx, |view, _window, cx| {
                view.save_drafts_before_quit(cx);
            }) {
                log::error!("failed to save drafts before quit: {error:?}");
            }
            cx.quit();
        });
        cx.on_action(|_: &MenuNewWindow, cx| {
            let bounds = Bounds::centered(None, size(px(1320.0), px(820.0)), cx);
            let window = match cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_, cx| cx.new(WorkspacePrototype::new_secondary),
            ) {
                Ok(window) => window,
                Err(error) => {
                    log::error!("failed to open new workspace window: {error:?}");
                    return;
                }
            };
            if let Err(error) = window.update(cx, |view, window, cx| {
                view.focus_active_surface(window, cx);
            }) {
                log::error!("failed to focus new workspace window: {error:?}");
            }
        });
        cx.activate(true);
    });
}

fn install_workspace_menu_bar(cx: &mut App) {
    cx.set_menus(vec![
        Menu {
            name: "LLNZY".into(),
            items: vec![MenuItem::action("Quit LLNZY", Quit)],
        },
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("New Window", MenuNewWindow),
                MenuItem::separator(),
                MenuItem::action("New Tab", MenuNewTab),
                MenuItem::action("Close Tab", MenuCloseTab),
                MenuItem::separator(),
                MenuItem::action("Open Project...", MenuOpenProject),
                MenuItem::action("Close Project", MenuCloseProject),
                MenuItem::separator(),
                MenuItem::action("Save", MenuSave),
            ],
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::action("Undo", MenuUndo),
                MenuItem::action("Redo", MenuRedo),
                MenuItem::separator(),
                MenuItem::action("Copy", MenuCopy),
                MenuItem::action("Paste", MenuPaste),
                MenuItem::action("Select All", MenuSelectAll),
                MenuItem::separator(),
                MenuItem::action("Find", MenuFind),
            ],
        },
        Menu {
            name: "Editor".into(),
            items: vec![
                MenuItem::action("Check Disk for Changes", MenuEditorCheckDisk),
                MenuItem::action("Reopen Closed File", MenuEditorReopenClosed),
                MenuItem::separator(),
                MenuItem::action("Close Other Files", MenuEditorCloseOthers),
                MenuItem::action("Close Saved Files", MenuEditorCloseSaved),
                MenuItem::separator(),
                MenuItem::action("Markdown Source", MenuMarkdownSource),
                MenuItem::action("Markdown Preview", MenuMarkdownPreview),
                MenuItem::action("Markdown Split", MenuMarkdownSplit),
                MenuItem::action("Cycle Markdown Mode", MenuMarkdownCycle),
                MenuItem::separator(),
                MenuItem::action("Hover", MenuLspHover),
                MenuItem::action("Completion", MenuLspCompletion),
                MenuItem::action("Go to Definition", MenuLspDefinition),
                MenuItem::action("Find References", MenuLspReferences),
                MenuItem::action("Signature Help", MenuLspSignatureHelp),
                MenuItem::action("Rename Symbol", MenuLspRename),
                MenuItem::action("Code Actions", MenuLspCodeActions),
                MenuItem::action("Format Document", MenuLspFormat),
                MenuItem::action("Document Symbols", MenuLspSymbols),
            ],
        },
        Menu {
            name: "Tab".into(),
            items: vec![
                MenuItem::action("New", MenuNewTab),
                MenuItem::action("Close", MenuCloseTab),
                MenuItem::separator(),
                MenuItem::action("Next Tab", MenuNextTab),
                MenuItem::action("Previous Tab", MenuPreviousTab),
                MenuItem::separator(),
                MenuItem::action("Join Tabs", MenuJoinTabs),
                MenuItem::action("Swap Tabs", MenuSwapTabs),
                MenuItem::action("Separate Tabs", MenuSeparateTabs),
                MenuItem::submenu(Menu {
                    name: "Partition".into(),
                    items: vec![
                        MenuItem::action("Partition Vertical", MenuPartitionVertical),
                        MenuItem::action("Partition Horizontal", MenuPartitionHorizontal),
                    ],
                }),
                MenuItem::separator(),
                MenuItem::action("Home", MenuShowHome),
                MenuItem::action("Terminal", MenuShowTerminal),
                MenuItem::action("Stacker", MenuShowStacker),
                MenuItem::action("Sketch Pad", MenuShowSketch),
                MenuItem::action("Settings", MenuShowAppearances),
            ],
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Toggle Sidebar", MenuToggleSidebar),
                MenuItem::separator(),
                MenuItem::action("Increase Font Size", MenuZoomIn),
                MenuItem::action("Decrease Font Size", MenuZoomOut),
                MenuItem::action("Reset Font Size", MenuZoomReset),
            ],
        },
    ]);
}

fn restore_joined_group_shares(tab_manager: &mut GpuiTabManager, primary: u64, shares: &[f32]) {
    if shares.len() < 2 || shares.iter().any(|share| !share.is_finite()) {
        return;
    }
    let mut boundary = 0.0;
    for (divider_index, share) in shares
        .iter()
        .take(shares.len().saturating_sub(1))
        .enumerate()
    {
        boundary += *share;
        tab_manager.set_split_for_tab(primary, divider_index, boundary);
    }
}

struct WorkspacePrototype {
    stacker: Entity<StackerPrototype>,
    editor: Entity<EditorPrototype>,
    file_editors: BTreeMap<u64, Entity<EditorPrototype>>,
    terminals: BTreeMap<u64, Entity<TerminalSurface>>,
    sketch: Entity<SketchSurface>,
    focus_handle: FocusHandle,
    tabs: Vec<WorkspaceTab>,
    tab_manager: GpuiTabManager,
    tab_name_overrides: BTreeMap<u64, String>,
    tab_rename: Option<TabRenameState>,
    tab_overflow_open: bool,
    sidebar_context_menu: Option<SidebarContextMenuState>,
    sidebar_rename: Option<SidebarRenameState>,
    sidebar_new_entry: Option<SidebarNewEntryState>,
    active_tab_id: WorkspaceTabId,
    next_tab_id: u64,
    workspace_root: Option<PathBuf>,
    sidebar_explorer: ExplorerState,
    explorers: BTreeMap<u64, ExplorerState>,
    recent_projects: Vec<PathBuf>,
    recent_projects_open: bool,
    sidebar_visible: bool,
    sidebar_width: f32,
    last_sidebar_width: f32,
    appearance_config: Config,
    appearance_page: AppearancePage,
    terminal_background_import_error: Option<String>,
    palette: command_palette::CommandPaletteState,
    preferences: crate::preferences::WorkspacePreferences,
    recovery_enabled: bool,
    last_recovery_snapshot: Option<WorkspaceRecoverySnapshot>,
    last_recovery_persist_attempt: Option<Instant>,
    cached_explorer_signature: Option<(Option<PathBuf>, BTreeSet<PathBuf>)>,
    cached_explorer_entries: Vec<ExplorerEntry>,
    error_log_expanded: bool,
    error_log_filter: ErrorLogFilter,
    pending_clear_error_log: bool,
}

/// Which severity levels the Settings → Error Log panel should display.
/// Session-only state; not persisted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ErrorLogFilter {
    All,
    WarnAndError,
    ErrorOnly,
}

impl ErrorLogFilter {
    fn includes(self, level: crate::error_log::LogLevel) -> bool {
        use crate::error_log::LogLevel;
        matches!(
            (self, level),
            (ErrorLogFilter::All, _)
                | (
                    ErrorLogFilter::WarnAndError,
                    LogLevel::Warn | LogLevel::Error
                )
                | (ErrorLogFilter::ErrorOnly, LogLevel::Error)
        )
    }
}

impl WorkspacePrototype {
    fn new(cx: &mut Context<Self>) -> Self {
        Self::with_recovery(cx, true)
    }

    fn new_secondary(cx: &mut Context<Self>) -> Self {
        Self::with_recovery(cx, false)
    }

    fn with_recovery(cx: &mut Context<Self>, recovery_enabled: bool) -> Self {
        let workspace_root = std::env::current_dir().ok();
        let tabs = vec![WorkspaceTab::new(WorkspaceTabId(1), WorkspaceSurface::Home)];
        let sidebar_explorer = ExplorerState::for_root(workspace_root.as_deref());
        let sketch = cx.new(SketchSurface::new);
        sketch.update(cx, |sketch, _cx| {
            sketch.set_workspace_root(workspace_root.clone());
        });

        let terminals = BTreeMap::new();

        let preferences = crate::preferences::WorkspacePreferences::load();
        let appearance_config = appearance_config_from_preferences(&preferences);
        let initial_light_mode = WorkspacePalette::from_config(&appearance_config).is_light;
        let stacker = cx.new(StackerPrototype::embedded);
        stacker.update(cx, |stacker, cx| {
            stacker.set_light_mode(initial_light_mode, cx);
        });
        sketch.update(cx, |sketch, cx| {
            sketch.set_light_mode(initial_light_mode, cx);
        });
        let editor = cx.new(EditorPrototype::new);
        editor.update(cx, |editor, cx| {
            editor.set_appearance_config(appearance_config.clone(), cx);
        });

        let mut workspace = Self {
            stacker,
            editor,
            file_editors: BTreeMap::new(),
            terminals,
            sketch,
            focus_handle: cx.focus_handle(),
            tabs,
            tab_manager: GpuiTabManager::default(),
            tab_name_overrides: BTreeMap::new(),
            tab_rename: None,
            tab_overflow_open: false,
            sidebar_context_menu: None,
            sidebar_rename: None,
            sidebar_new_entry: None,
            active_tab_id: WorkspaceTabId(1),
            next_tab_id: 2,
            workspace_root,
            sidebar_explorer,
            explorers: BTreeMap::new(),
            recent_projects: crate::explorer::load_recent_projects(),
            recent_projects_open: false,
            sidebar_visible: true,
            sidebar_width: SIDEBAR_DEFAULT_WIDTH,
            last_sidebar_width: SIDEBAR_DEFAULT_WIDTH,
            appearance_config,
            appearance_page: AppearancePage::Terminal,
            terminal_background_import_error: None,
            palette: command_palette::CommandPaletteState::default(),
            preferences,
            recovery_enabled,
            last_recovery_snapshot: None,
            last_recovery_persist_attempt: None,
            cached_explorer_signature: None,
            cached_explorer_entries: Vec::new(),
            error_log_expanded: false,
            error_log_filter: ErrorLogFilter::All,
            pending_clear_error_log: false,
        };

        if recovery_enabled {
            workspace.restore_after_unclean_shutdown(cx);
            workspace.persist_workspace_recovery(false);
        }
        workspace
    }

    fn tab_join_limit(&self) -> usize {
        self.preferences.joined_tab_limit()
    }

    fn editor_word_wrap_enabled(&self) -> bool {
        self.appearance_config.editor.word_wrap
    }

    pub(super) fn toggle_editor_word_wrap(&mut self, cx: &mut Context<Self>) {
        let enabled = !self.editor_word_wrap_enabled();
        self.preferences.editor_word_wrap = Some(enabled);
        self.preferences.save();
        apply_editor_word_wrap_preference(&mut self.appearance_config, enabled);
        self.apply_appearance_config(cx);
    }

    pub(super) fn set_joined_tab_limit(&mut self, limit: u8, cx: &mut Context<Self>) {
        let limit = limit.clamp(2, 4);
        if self.preferences.joined_tab_limit == limit {
            return;
        }
        self.preferences.joined_tab_limit = limit;
        self.preferences.save();
        self.tab_manager.enforce_join_limit(limit as usize);
        cx.notify();
    }

    pub(super) fn toggle_error_log_expanded(&mut self, cx: &mut Context<Self>) {
        self.error_log_expanded = !self.error_log_expanded;
        cx.notify();
    }

    /// Best-effort save of any in-flight Stacker draft. Invoked from the
    /// app-level Quit handler before `cx.quit()` so closing the app
    /// doesn't drop pending work or leave the recovery snapshot marked dirty.
    pub(crate) fn save_drafts_before_quit(&mut self, cx: &mut Context<Self>) {
        self.stacker
            .update(cx, |stacker, cx| stacker.save_active_prompt(cx));
        self.persist_workspace_recovery(true);
        self.recovery_enabled = false;
    }

    fn restore_after_unclean_shutdown(&mut self, cx: &mut Context<Self>) {
        let Some(path) = recovery_file() else {
            return;
        };
        let snapshot = match recovery::load_snapshot(&path) {
            Ok(Some(snapshot)) => snapshot,
            Ok(None) => return,
            Err(err) => {
                log::warn!("{err}");
                if let Err(remove_err) = recovery::remove_snapshot(&path) {
                    log::warn!("{remove_err}");
                }
                return;
            }
        };
        let Some(plan) = plan_restore(snapshot) else {
            return;
        };
        self.apply_recovery_plan(plan, cx);
    }

    fn apply_recovery_plan(&mut self, plan: WorkspaceRecoveryPlan, cx: &mut Context<Self>) {
        self.workspace_root = plan.workspace_root.clone();
        self.sketch.update(cx, |sketch, _cx| {
            sketch.set_workspace_root(plan.workspace_root.clone())
        });
        let restore_status = plan.status_message();
        self.sidebar_explorer = ExplorerState::for_root(plan.workspace_root.as_deref());
        self.sidebar_explorer.expanded_dirs = plan.sidebar_expanded_dirs;
        self.sidebar_explorer.selected_path = plan.sidebar_selected_path;
        self.sidebar_explorer.status = restore_status;
        self.sidebar_visible = plan.sidebar_visible;
        self.sidebar_width = plan
            .sidebar_width
            .clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH);
        self.last_sidebar_width = plan
            .last_sidebar_width
            .clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH);
        self.recent_projects_open = false;
        self.close_context_menus(cx);

        self.tabs.clear();
        self.file_editors.clear();
        self.terminals.clear();
        self.explorers.clear();
        self.tab_manager = GpuiTabManager::default();

        let mut skipped_open_files = Vec::new();
        for restored in plan.tabs {
            let tab_id = WorkspaceTabId(restored.id);
            let surface = WorkspaceSurface::from(restored.surface);
            if let Some(path) = restored.file_path {
                let Some(editor) = self.new_file_editor(path.clone(), cx) else {
                    skipped_open_files.push(path);
                    continue;
                };
                self.file_editors.insert(tab_id.0, editor);
                self.tabs.push(WorkspaceTab::file(tab_id, path));
                continue;
            }

            self.tabs.push(WorkspaceTab::new(tab_id, surface));
            match surface {
                WorkspaceSurface::Terminal => {
                    let terminal = self.new_terminal_surface(cx);
                    self.terminals.insert(tab_id.0, terminal);
                }
                WorkspaceSurface::Explorer => {
                    self.explorers.insert(
                        tab_id.0,
                        ExplorerState::for_root(self.workspace_root.as_deref()),
                    );
                }
                _ => {}
            }
        }

        if self.tabs.is_empty() {
            self.tabs
                .push(WorkspaceTab::new(WorkspaceTabId(1), WorkspaceSurface::Home));
        }

        self.next_tab_id = plan
            .next_tab_id
            .max(self.tabs.iter().map(|tab| tab.id.0).max().unwrap_or(1) + 1);
        let valid_tabs = self.tab_ids();
        self.tab_name_overrides = plan
            .tab_name_overrides
            .into_iter()
            .filter(|(tab_id, _)| valid_tabs.contains(tab_id))
            .collect();
        self.restore_joined_groups(&plan.joined_groups);

        self.active_tab_id = if valid_tabs.contains(&plan.active_tab_id) {
            WorkspaceTabId(plan.active_tab_id)
        } else {
            self.tabs
                .iter()
                .find(|tab| tab.surface == WorkspaceSurface::Home)
                .or_else(|| self.tabs.first())
                .map(|tab| tab.id)
                .unwrap_or(WorkspaceTabId(1))
        };
        self.tab_manager.set_active_tab(self.active_tab_id.0);
        if let Some(path) = self
            .tab_by_id(self.active_tab_id)
            .and_then(|tab| tab.file_path.clone())
        {
            self.sidebar_explorer.selected_path = Some(path);
        }

        if !skipped_open_files.is_empty() {
            let extra = if skipped_open_files.len() == 1 {
                format!("Skipped file {}", skipped_open_files[0].display())
            } else {
                format!(
                    "Skipped {} files that failed to open",
                    skipped_open_files.len()
                )
            };
            self.sidebar_explorer.status = Some(match self.sidebar_explorer.status.take() {
                Some(existing) => format!("{existing}; {extra}"),
                None => extra,
            });
        }
    }

    fn restore_joined_groups(&mut self, groups: &[WorkspaceRecoveryJoinedGroup]) {
        let valid_tabs = self.tab_ids();
        for group in groups {
            let members = group
                .members
                .iter()
                .copied()
                .filter(|member| valid_tabs.contains(member))
                .take(self.tab_join_limit())
                .collect::<Vec<_>>();
            let Some((&primary, rest)) = members.split_first() else {
                continue;
            };
            if rest.is_empty() {
                continue;
            }
            let axis = crate::tab_groups::PartitionAxis::from(group.axis);
            for member in rest {
                self.tab_manager
                    .join_tabs_with_axis(primary, *member, self.tab_join_limit(), axis);
            }
            restore_joined_group_shares(&mut self.tab_manager, primary, &group.shares);
        }
        self.tab_manager.retain_tabs(&valid_tabs);
    }

    fn explorer_entries(&mut self) -> Vec<ExplorerEntry> {
        let signature = (
            self.workspace_root.clone(),
            self.sidebar_explorer.expanded_dirs.clone(),
        );
        if self.cached_explorer_signature.as_ref() == Some(&signature) {
            return self.cached_explorer_entries.clone();
        }
        let entries = signature
            .0
            .as_ref()
            .map(|root| collect_explorer_entries(root, &signature.1))
            .unwrap_or_default();
        self.cached_explorer_signature = Some(signature);
        self.cached_explorer_entries = entries.clone();
        entries
    }

    fn persist_workspace_recovery(&mut self, clean_shutdown: bool) {
        if !self.recovery_enabled {
            return;
        }
        if !clean_shutdown
            && self
                .last_recovery_persist_attempt
                .is_some_and(|t| t.elapsed() < RECOVERY_PERSIST_INTERVAL)
        {
            return;
        }
        self.last_recovery_persist_attempt = Some(Instant::now());
        let Some(path) = recovery_file() else {
            return;
        };
        let snapshot = self.workspace_recovery_snapshot(clean_shutdown);
        if self.last_recovery_snapshot.as_ref() == Some(&snapshot) {
            return;
        }
        if let Err(err) = save_snapshot(&path, &snapshot) {
            log::warn!("{err}");
            return;
        }
        self.last_recovery_snapshot = Some(snapshot);
    }

    fn workspace_recovery_snapshot(&self, clean_shutdown: bool) -> WorkspaceRecoverySnapshot {
        WorkspaceRecoverySnapshot {
            version: WORKSPACE_RECOVERY_VERSION,
            clean_shutdown,
            workspace_root: self.workspace_root.clone(),
            active_tab_id: self.active_tab_id.0,
            next_tab_id: self.next_tab_id,
            tabs: self
                .tabs
                .iter()
                .map(|tab| WorkspaceRecoveryTab {
                    id: tab.id.0,
                    surface: WorkspaceRecoverySurface::from(tab.surface),
                    file_path: tab.file_path.clone(),
                })
                .collect(),
            tab_name_overrides: self
                .tab_name_overrides
                .iter()
                .map(|(id, name)| WorkspaceRecoveryTabNameOverride {
                    id: *id,
                    name: name.clone(),
                })
                .collect(),
            joined_groups: self.workspace_recovery_joined_groups(),
            sidebar_visible: self.sidebar_visible,
            sidebar_width: self.sidebar_width,
            last_sidebar_width: self.last_sidebar_width,
            sidebar_selected_path: self.sidebar_explorer.selected_path.clone(),
            sidebar_expanded_dirs: self
                .sidebar_explorer
                .expanded_dirs
                .iter()
                .cloned()
                .collect(),
        }
    }

    fn workspace_recovery_joined_groups(&self) -> Vec<WorkspaceRecoveryJoinedGroup> {
        let valid_tabs = self.tab_ids();
        let mut seen = Vec::<Vec<u64>>::new();
        let mut groups = Vec::new();
        for tab_id in &valid_tabs {
            let Some(joined) = self.tab_manager.joined_group_for(*tab_id, &valid_tabs) else {
                continue;
            };
            let mut key = joined.members.clone();
            key.sort_unstable();
            if seen.contains(&key) {
                continue;
            }
            seen.push(key);
            groups.push(WorkspaceRecoveryJoinedGroup {
                members: joined.members,
                shares: joined.shares,
                axis: WorkspaceRecoveryAxis::from(joined.axis),
            });
        }
        groups
    }

    /// Copy the full diagnostics report (system context + every captured
    /// runtime entry) to the system clipboard. Lets users hand the log over
    /// to whoever's debugging the app or — once extensions ship — to the
    /// author of a misbehaving extension.
    pub(super) fn copy_error_log(&mut self, cx: &mut Context<Self>) {
        let log = crate::error_log::global();
        let report = crate::diagnostics::render_diagnostics_report(Some(log));
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(report));
        cx.notify();
    }

    pub(super) fn set_error_log_filter(&mut self, filter: ErrorLogFilter, cx: &mut Context<Self>) {
        if self.error_log_filter != filter {
            self.error_log_filter = filter;
            cx.notify();
        }
    }

    /// Open the clear-confirmation modal. The actual wipe runs only on
    /// `confirm_clear_error_log` to keep destructive actions explicit —
    /// the persisted log includes every prior session, so a misclick
    /// would burn debug context.
    pub(super) fn request_clear_error_log(&mut self, cx: &mut Context<Self>) {
        self.pending_clear_error_log = true;
        cx.notify();
    }

    pub(super) fn cancel_clear_error_log(&mut self, cx: &mut Context<Self>) {
        if self.pending_clear_error_log {
            self.pending_clear_error_log = false;
            cx.notify();
        }
    }

    pub(super) fn confirm_clear_error_log(&mut self, cx: &mut Context<Self>) {
        self.pending_clear_error_log = false;
        crate::error_log::global().clear();
        cx.notify();
    }

    /// Open the file logged for an error-log entry in the editor and jump
    /// to the recorded line. Falls back silently when the path does not
    /// resolve under the active workspace root — entries from external
    /// crates often log paths the user has no checkout of.
    pub(super) fn open_error_log_source(
        &mut self,
        file: String,
        line: u32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let candidate = PathBuf::from(&file);
        let resolved = if candidate.is_absolute() && candidate.exists() {
            Some(candidate)
        } else if let Some(root) = self.workspace_root.as_ref() {
            let joined = root.join(&candidate);
            joined.exists().then_some(joined)
        } else {
            None
        };
        let Some(path) = resolved else {
            return;
        };

        self.open_sidebar_file(path, window, cx);

        let editor = self.active_editor_entity();
        editor.update(cx, |editor, cx| {
            editor.navigate_to_position_from_workspace(line, 0, cx);
        });
    }

    fn active_surface_if_present(&self) -> Option<WorkspaceSurface> {
        self.tabs
            .iter()
            .find(|tab| tab.id == self.active_tab_id)
            .or_else(|| self.tabs.first())
            .map(|tab| tab.surface)
    }

    fn active_surface(&self) -> WorkspaceSurface {
        self.active_surface_if_present()
            .unwrap_or(WorkspaceSurface::Home)
    }

    fn select_empty_workspace(&mut self) {
        self.active_tab_id = EMPTY_WORKSPACE_TAB_ID;
        self.tab_overflow_open = false;
        self.sidebar_explorer.selected_path = None;
        self.tab_manager.retain_tabs(&[]);
    }

    fn tab_ids(&self) -> Vec<u64> {
        self.tabs.iter().map(|tab| tab.id.0).collect()
    }

    fn tab_label(&self, tab: &WorkspaceTab) -> String {
        self.tab_name_overrides
            .get(&tab.id.0)
            .cloned()
            .unwrap_or_else(|| workspace_tab_label(tab))
    }

    fn tab_choices(&self) -> Vec<GpuiTabChoice> {
        self.tabs
            .iter()
            .map(|tab| GpuiTabChoice {
                id: tab.id.0,
                title: self.tab_label(tab),
                joined: self.tab_manager.is_joined(tab.id.0),
            })
            .collect()
    }

    fn tab_by_id(&self, tab_id: WorkspaceTabId) -> Option<&WorkspaceTab> {
        self.tabs.iter().find(|tab| tab.id == tab_id)
    }

    fn active_editor_entity(&self) -> Entity<EditorPrototype> {
        self.file_editors
            .get(&self.active_tab_id.0)
            .cloned()
            .unwrap_or_else(|| self.editor.clone())
    }

    fn editor_entities(&self) -> Vec<Entity<EditorPrototype>> {
        let mut editors = Vec::with_capacity(self.file_editors.len() + 1);
        editors.push(self.editor.clone());
        editors.extend(self.file_editors.values().cloned());
        editors
    }

    fn active_explorer_state_mut(&mut self) -> &mut ExplorerState {
        if self.active_surface() == WorkspaceSurface::Explorer {
            let active_tab_id = self.active_tab_id.0;
            if let Some(explorer) = self.explorers.get_mut(&active_tab_id) {
                return explorer;
            }
        }
        &mut self.sidebar_explorer
    }

    fn joined_panes_for_active(&self) -> Option<JoinedWorkspacePanes> {
        let joined = self
            .tab_manager
            .joined_group_for(self.active_tab_id.0, &self.tab_ids())?;
        let panes = joined
            .members
            .into_iter()
            .filter_map(|member| {
                let id = WorkspaceTabId(member);
                Some(JoinedWorkspacePane {
                    id,
                    surface: self.tab_by_id(id)?.surface,
                })
            })
            .collect::<Vec<_>>();
        if panes.len() < 2 {
            return None;
        }
        Some(JoinedWorkspacePanes {
            panes,
            shares: joined.shares,
            ratio: joined.ratio,
            axis: joined.axis,
        })
    }

    fn close_tab_context_menu(&mut self, cx: &mut Context<Self>) {
        if self.tab_manager.context_menu().is_some() {
            self.tab_manager.close_context_menu();
            self.tab_rename = None;
            cx.notify();
        }
    }

    fn close_sidebar_context_menu(&mut self, cx: &mut Context<Self>) {
        if self.sidebar_context_menu.is_some()
            || self.sidebar_rename.is_some()
            || self.sidebar_new_entry.is_some()
        {
            self.sidebar_context_menu = None;
            self.sidebar_rename = None;
            self.sidebar_new_entry = None;
            cx.notify();
        }
    }

    fn close_context_menus(&mut self, cx: &mut Context<Self>) {
        let had_tab_menu = self.tab_manager.context_menu().is_some();
        let had_tab_overflow_menu = self.tab_overflow_open;
        let had_sidebar_menu = self.sidebar_context_menu.is_some()
            || self.sidebar_rename.is_some()
            || self.sidebar_new_entry.is_some();
        self.tab_manager.close_context_menu();
        self.tab_overflow_open = false;
        self.tab_rename = None;
        self.sidebar_context_menu = None;
        self.sidebar_rename = None;
        self.sidebar_new_entry = None;
        if had_tab_menu || had_tab_overflow_menu || had_sidebar_menu {
            cx.notify();
        }
    }

    fn toggle_tab_overflow_menu(&mut self, cx: &mut Context<Self>) {
        self.tab_manager.close_context_menu();
        self.tab_rename = None;
        self.sidebar_context_menu = None;
        self.sidebar_rename = None;
        self.sidebar_new_entry = None;
        self.tab_overflow_open = !self.tab_overflow_open;
        cx.notify();
    }

    fn open_tab_context_menu(
        &mut self,
        tab_id: WorkspaceTabId,
        anchor: WorkspaceTabMenuAnchor,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle);
        self.tab_overflow_open = false;
        self.tab_rename = None;
        self.sidebar_context_menu = None;
        self.sidebar_rename = None;
        self.sidebar_new_entry = None;
        self.tab_manager
            .open_context_menu(tab_id.0, anchor.x, anchor.y, anchor.width);
        cx.notify();
    }

    fn show_tab_join_targets(&mut self, cx: &mut Context<Self>) {
        self.tab_rename = None;
        self.tab_manager.show_join_targets();
        cx.notify();
    }

    fn start_tab_rename(
        &mut self,
        tab_id: WorkspaceTabId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.tab_by_id(tab_id) else {
            self.close_tab_context_menu(cx);
            return;
        };
        let text = self.tab_label(tab);
        window.focus(&self.focus_handle);
        self.tab_rename = Some(TabRenameState {
            tab_id,
            text,
            replace_on_input: true,
        });
        self.tab_manager.show_rename();
        cx.notify();
    }

    fn commit_tab_rename(&mut self, cx: &mut Context<Self>) {
        let Some(rename) = self.tab_rename.take() else {
            return;
        };
        let name = rename.text.trim().to_string();
        if name.is_empty() {
            self.tab_name_overrides.remove(&rename.tab_id.0);
        } else {
            self.tab_name_overrides.insert(rename.tab_id.0, name);
        }
        self.tab_manager.close_context_menu();
        cx.notify();
    }

    fn cancel_tab_rename(&mut self, cx: &mut Context<Self>) {
        self.tab_rename = None;
        self.tab_manager.close_context_menu();
        cx.notify();
    }

    fn on_workspace_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.palette.open {
            self.on_palette_key_down(event, window, cx);
            return;
        }

        if self.sidebar_new_entry.is_some() {
            self.on_sidebar_new_entry_key_down(event, cx);
            return;
        }

        if self.sidebar_rename.is_some() {
            self.on_sidebar_rename_key_down(event, cx);
            return;
        }

        if self.tab_rename.is_none() {
            return;
        }

        cx.stop_propagation();
        match event.keystroke.key.as_str() {
            "enter" => {
                self.commit_tab_rename(cx);
            }
            "escape" => {
                self.cancel_tab_rename(cx);
            }
            "backspace" => {
                if let Some(rename) = &mut self.tab_rename {
                    if rename.replace_on_input {
                        rename.text.clear();
                        rename.replace_on_input = false;
                    } else {
                        rename.text.pop();
                    }
                }
                cx.notify();
            }
            _ => {
                let modifiers = event.keystroke.modifiers;
                if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                    return;
                }
                let Some(text) = event.keystroke.key_char.as_deref() else {
                    return;
                };
                if text.chars().any(char::is_control) {
                    return;
                }
                if let Some(rename) = &mut self.tab_rename {
                    if rename.text.chars().count() < 64 {
                        if rename.replace_on_input {
                            rename.text.clear();
                            rename.replace_on_input = false;
                        }
                        rename.text.push_str(text);
                        cx.notify();
                    }
                }
            }
        }
    }

    fn open_command_palette(&mut self, cx: &mut Context<Self>) {
        self.close_context_menus(cx);
        self.palette.reset();
        self.palette.open = true;
        cx.notify();
    }

    pub(super) fn open_file_finder(&mut self, cx: &mut Context<Self>) {
        self.close_context_menus(cx);
        self.palette.reset();
        self.palette.open = true;
        self.palette.mode = command_palette::PaletteMode::Files;
        if let Some(root) = self.workspace_root.clone() {
            self.palette.files = command_palette::collect_project_files(&root);
            self.palette.project_root = Some(root);
        }
        cx.notify();
    }

    fn close_command_palette(&mut self, cx: &mut Context<Self>) {
        if self.palette.open {
            self.palette.reset();
            cx.notify();
        }
    }

    fn visible_palette_count(&self) -> usize {
        match self.palette.mode {
            command_palette::PaletteMode::Commands => {
                let entries = command_palette::palette_entries();
                command_palette::filter_entries(&entries, &self.palette.query).len()
            }
            command_palette::PaletteMode::Files => {
                let Some(root) = self.palette.project_root.as_ref() else {
                    return 0;
                };
                command_palette::filter_files(&self.palette.files, root, &self.palette.query).len()
            }
        }
    }

    pub(super) fn invoke_palette_at(
        &mut self,
        display_idx: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.palette.mode {
            command_palette::PaletteMode::Commands => {
                let entries = command_palette::palette_entries();
                let visible = command_palette::filter_entries(&entries, &self.palette.query);
                let Some(&entry_idx) = visible.get(display_idx) else {
                    return;
                };
                let action = (entries[entry_idx].build_action)();
                self.palette.reset();
                cx.notify();
                // Defer dispatch so the palette overlay is gone before the
                // action runs (otherwise an action that toggles UI would
                // race with the open palette).
                window.dispatch_action(action, cx);
            }
            command_palette::PaletteMode::Files => {
                let Some(root) = self.palette.project_root.clone() else {
                    return;
                };
                let visible =
                    command_palette::filter_files(&self.palette.files, &root, &self.palette.query);
                let Some(&file_idx) = visible.get(display_idx) else {
                    return;
                };
                let path = self.palette.files[file_idx].clone();
                self.palette.reset();
                cx.notify();
                self.open_sidebar_file(path, window, cx);
            }
        }
    }

    fn on_palette_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        // Cmd+Shift+P while open: toggle closed.
        if modifiers.platform && modifiers.shift && key == "p" {
            self.close_command_palette(cx);
            return;
        }
        // Cmd+P while open: toggle closed (matches the file-finder binding).
        if modifiers.platform && !modifiers.shift && key == "p" {
            self.close_command_palette(cx);
            return;
        }
        match key {
            "escape" => {
                self.close_command_palette(cx);
            }
            "enter" => {
                let display_idx = self.palette.selected;
                self.invoke_palette_at(display_idx, window, cx);
            }
            "up" => {
                if self.palette.selected > 0 {
                    self.palette.selected -= 1;
                    cx.notify();
                }
            }
            "down" => {
                let visible_count = self.visible_palette_count();
                if visible_count > 0 && self.palette.selected + 1 < visible_count {
                    self.palette.selected += 1;
                    cx.notify();
                }
            }
            "backspace" => {
                self.palette.query.pop();
                self.palette.selected = 0;
                cx.notify();
            }
            _ => {
                if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                    return;
                }
                let Some(text) = event.keystroke.key_char.as_deref() else {
                    return;
                };
                if text.chars().any(char::is_control) {
                    return;
                }
                self.palette.query.push_str(text);
                self.palette.selected = 0;
                cx.notify();
            }
        }
    }

    fn join_tabs_by_id(
        &mut self,
        primary_id: WorkspaceTabId,
        secondary_id: WorkspaceTabId,
        cx: &mut Context<Self>,
    ) {
        if self.tab_by_id(primary_id).is_none() || self.tab_by_id(secondary_id).is_none() {
            return;
        }
        if self
            .tab_manager
            .join_tabs(primary_id.0, secondary_id.0, self.tab_join_limit())
        {
            let group_ids = self.joined_tab_ids_for(primary_id);
            self.place_joined_tabs_together(&group_ids);
            self.active_tab_id = primary_id;
            cx.notify();
        }
    }

    fn joined_tab_ids_for(&self, tab_id: WorkspaceTabId) -> Vec<WorkspaceTabId> {
        self.tab_manager
            .joined_group_for(tab_id.0, &self.tab_ids())
            .map(|joined| {
                joined
                    .members
                    .into_iter()
                    .map(WorkspaceTabId)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| vec![tab_id])
    }

    fn place_joined_tabs_together(&mut self, group_ids: &[WorkspaceTabId]) {
        place_workspace_tabs_together(&mut self.tabs, group_ids);
    }

    fn separate_tab_by_id(&mut self, tab_id: WorkspaceTabId, cx: &mut Context<Self>) {
        if self.tab_manager.separate_tab(tab_id.0) {
            self.active_tab_id = tab_id;
            cx.notify();
        }
    }

    fn swap_tabs_by_id(&mut self, tab_id: WorkspaceTabId, cx: &mut Context<Self>) {
        if self.tab_manager.swap_tabs_for_tab(tab_id.0) {
            let group_ids = self.joined_tab_ids_for(tab_id);
            self.place_joined_tabs_together(&group_ids);
            cx.notify();
        }
    }

    fn resize_joined_panes_by_tab(
        &mut self,
        tab_id: WorkspaceTabId,
        divider_index: usize,
        ratio: f32,
        cx: &mut Context<Self>,
    ) {
        if self
            .tab_manager
            .set_split_for_tab(tab_id.0, divider_index, ratio)
        {
            cx.notify();
        }
    }

    fn active_tab_menu_anchor(&self) -> WorkspaceTabMenuAnchor {
        let tabs = self
            .tabs
            .iter()
            .cloned()
            .map(|tab| {
                let label = self.tab_label(&tab);
                (tab, label)
            })
            .collect::<Vec<_>>();
        let layouts = workspace_tab_layouts(&tabs, self.active_tab_id);
        workspace_tab_menu_anchor(
            self.active_tab_id,
            &self.tabs,
            self.active_tab_id,
            &self.tab_manager,
            &layouts,
        )
    }

    fn open_active_tab_context_menu(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let tab_id = self.active_tab_id;
        if self.tab_by_id(tab_id).is_none() {
            return false;
        }
        let anchor = self.active_tab_menu_anchor();
        self.open_tab_context_menu(tab_id, anchor, window, cx);
        true
    }

    fn reorder_tab(
        &mut self,
        dragged_id: WorkspaceTabId,
        target_id: WorkspaceTabId,
        cx: &mut Context<Self>,
    ) {
        if dragged_id == target_id {
            return;
        }

        let valid_tabs = self.tab_ids();
        let block_ids = self
            .tab_manager
            .joined_group_for(dragged_id.0, &valid_tabs)
            .map(|joined| {
                let mut ids = joined
                    .members
                    .into_iter()
                    .map(WorkspaceTabId)
                    .collect::<Vec<_>>();
                ids.sort_by_key(|id| {
                    self.tabs
                        .iter()
                        .position(|tab| tab.id == *id)
                        .unwrap_or(usize::MAX)
                });
                ids
            })
            .unwrap_or_else(|| vec![dragged_id]);

        if reorder_workspace_tab_block(&mut self.tabs, &block_ids, target_id) {
            self.close_tab_context_menu(cx);
            cx.notify();
        }
    }

    fn focus_surface(
        &self,
        surface: WorkspaceSurface,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match surface {
            WorkspaceSurface::Stacker => window.focus(&self.stacker.focus_handle(cx)),
            WorkspaceSurface::Editor
            | WorkspaceSurface::Home
            | WorkspaceSurface::Appearances
            | WorkspaceSurface::Settings => {
                if surface == WorkspaceSurface::Editor {
                    window.focus(&self.active_editor_entity().focus_handle(cx));
                } else {
                    window.focus(&self.editor.focus_handle(cx));
                }
            }
            WorkspaceSurface::Terminal => {
                if let Some(terminal) = self.terminals.get(&self.active_tab_id.0) {
                    window.focus(&terminal.focus_handle(cx));
                } else {
                    window.focus(&self.focus_handle);
                }
            }
            WorkspaceSurface::Explorer => window.focus(&self.focus_handle),
            WorkspaceSurface::Sketch => window.focus(&self.sketch.focus_handle(cx)),
        }
    }

    fn focus_active_surface(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(surface) = self.active_surface_if_present() {
            self.focus_surface(surface, window, cx);
        } else {
            window.focus(&self.focus_handle);
        }
    }

    fn activate_tab(
        &mut self,
        tab_id: WorkspaceTabId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(tab) = self.tabs.iter().find(|tab| tab.id == tab_id).cloned() {
            self.active_tab_id = tab.id;
            self.tab_manager.set_active_tab(tab.id.0);
            self.tab_overflow_open = false;
            if let Some(path) = tab.file_path.clone() {
                self.sidebar_explorer.selected_path = Some(path.clone());
                self.active_editor_entity().update(cx, |editor, cx| {
                    editor.activate_path_from_workspace(path, cx)
                });
            }
            self.focus_surface(tab.surface, window, cx);
            cx.notify();
        }
    }

    fn open_or_activate_surface(
        &mut self,
        surface: WorkspaceSurface,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.active_surface_if_present() == Some(surface) {
            self.focus_surface(surface, window, cx);
            cx.notify();
            return;
        }

        if let Some(tab) = self.tabs.iter().find(|tab| tab.surface == surface) {
            self.active_tab_id = tab.id;
            self.tab_manager.set_active_tab(tab.id.0);
            self.focus_surface(tab.surface, window, cx);
            cx.notify();
            return;
        }

        let tab_id = WorkspaceTabId(self.next_tab_id);
        self.next_tab_id += 1;
        self.tabs.push(WorkspaceTab::new(tab_id, surface));
        if surface == WorkspaceSurface::Terminal {
            let terminal = self.new_terminal_surface(cx);
            self.terminals.insert(tab_id.0, terminal);
        }
        self.active_tab_id = tab_id;
        self.tab_manager.set_active_tab(tab_id.0);
        self.focus_surface(surface, window, cx);
        cx.notify();
    }

    fn new_terminal_surface(&self, cx: &mut Context<Self>) -> Entity<TerminalSurface> {
        let cwd = self.workspace_root.clone();
        let terminal = cx.new(|cx| TerminalSurface::new_with_cwd(cwd, cx));
        let config = std::sync::Arc::new(self.appearance_config.clone());
        terminal.update(cx, |terminal, cx| terminal.set_config(config, cx));
        terminal
    }

    fn open_new_terminal_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let tab_id = WorkspaceTabId(self.next_tab_id);
        self.next_tab_id += 1;
        self.tabs
            .push(WorkspaceTab::new(tab_id, WorkspaceSurface::Terminal));
        let terminal = self.new_terminal_surface(cx);
        self.terminals.insert(tab_id.0, terminal);
        self.active_tab_id = tab_id;
        self.tab_manager.set_active_tab(tab_id.0);
        self.focus_surface(WorkspaceSurface::Terminal, window, cx);
        cx.notify();
    }

    /// Split the active shelf in two and fill the new region with a fresh
    /// terminal. The active tab becomes the `primary` (kept focused) and the
    /// new terminal becomes the `secondary`. The `axis` determines the
    /// orientation of the divider: vertical = side-by-side, horizontal =
    /// stacked top/bottom. No-ops when the active tab is already inside a
    /// partition (separate first).
    pub(super) fn partition_active_with_new_terminal(
        &mut self,
        axis: crate::tab_groups::PartitionAxis,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let primary_id = self.active_tab_id;
        if self.tab_by_id(primary_id).is_none() {
            self.open_new_terminal_tab(window, cx);
            return;
        }
        if self.tab_manager.joined_member_count(primary_id.0) >= self.tab_join_limit() {
            return;
        }
        let secondary_id = WorkspaceTabId(self.next_tab_id);
        self.next_tab_id += 1;
        self.tabs
            .push(WorkspaceTab::new(secondary_id, WorkspaceSurface::Terminal));
        let terminal = self.new_terminal_surface(cx);
        self.terminals.insert(secondary_id.0, terminal);

        let joined = self.tab_manager.join_tabs_with_axis(
            primary_id.0,
            secondary_id.0,
            self.tab_join_limit(),
            axis,
        );
        if !joined {
            // Roll back the freshly allocated terminal if the join failed.
            self.tabs.retain(|tab| tab.id != secondary_id);
            self.terminals.remove(&secondary_id.0);
            return;
        }

        let group_ids = self.joined_tab_ids_for(primary_id);
        self.place_joined_tabs_together(&group_ids);
        self.active_tab_id = primary_id;
        self.tab_manager.set_active_tab(primary_id.0);
        if let Some(active_surface) = self.tab_by_id(primary_id).map(|tab| tab.surface) {
            self.focus_surface(active_surface, window, cx);
        }
        cx.notify();
    }

    pub(super) fn pop_out_sidebar_explorer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let tab_id = self
            .tabs
            .iter()
            .find(|tab| tab.surface == WorkspaceSurface::Explorer)
            .map(|tab| tab.id)
            .unwrap_or_else(|| {
                let tab_id = WorkspaceTabId(self.next_tab_id);
                self.next_tab_id += 1;
                self.tabs
                    .push(WorkspaceTab::new(tab_id, WorkspaceSurface::Explorer));
                tab_id
            });

        self.explorers
            .insert(tab_id.0, self.sidebar_explorer.clone());
        if self.sidebar_visible {
            self.last_sidebar_width = self.sidebar_width;
        }
        self.sidebar_visible = false;
        self.sidebar_context_menu = None;
        self.sidebar_rename = None;
        self.sidebar_new_entry = None;
        self.active_tab_id = tab_id;
        self.tab_manager.set_active_tab(tab_id.0);
        self.tab_overflow_open = false;
        self.focus_surface(WorkspaceSurface::Explorer, window, cx);
        cx.notify();
    }

    fn open_footer_surface(
        &mut self,
        surface: WorkspaceSurface,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if surface == WorkspaceSurface::Terminal {
            self.open_new_terminal_tab(window, cx);
        } else {
            self.open_or_activate_surface(surface, window, cx);
        }
    }

    fn close_tab(&mut self, tab_id: WorkspaceTabId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.tabs.iter().position(|tab| tab.id == tab_id) else {
            return;
        };

        let was_active = self.active_tab_id == tab_id;
        if let Some(path) = self.tabs[index].file_path.clone() {
            let closed = self.file_editors.get(&tab_id.0).is_none_or(|editor| {
                editor.update(cx, |editor, cx| editor.close_path_from_workspace(&path, cx))
            });
            if !closed {
                return;
            }
            self.file_editors.remove(&tab_id.0);
            if self
                .sidebar_explorer
                .selected_path
                .as_ref()
                .is_some_and(|selected| crate::path_utils::same_path(selected, &path))
            {
                self.sidebar_explorer.selected_path = None;
            }
        }
        // Persist the Stacker draft when the user closes its tab. The
        // Stacker entity itself stays alive across tab open/close cycles
        // (it's a workspace-level singleton), but the user's mental model
        // is "closing the tab = closing the work" — so we save here so
        // nothing is lost between sessions.
        if self.tabs[index].surface == WorkspaceSurface::Stacker {
            self.stacker
                .update(cx, |stacker, cx| stacker.save_active_prompt(cx));
        }
        if self.tabs[index].surface == WorkspaceSurface::Terminal {
            self.terminals.remove(&tab_id.0);
        }
        if self.tabs[index].surface == WorkspaceSurface::Explorer {
            let removed = self.explorers.remove(&tab_id.0);
            if self.explorers.is_empty() {
                if let Some(state) = removed {
                    self.sidebar_explorer = state;
                }
                if self.sidebar_visible {
                    self.last_sidebar_width = self.sidebar_width;
                }
                self.sidebar_visible = false;
                self.sidebar_width = self
                    .last_sidebar_width
                    .clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH);
            }
        }
        self.tabs.remove(index);
        self.tab_name_overrides.remove(&tab_id.0);
        if self
            .tab_rename
            .as_ref()
            .is_some_and(|rename| rename.tab_id == tab_id)
        {
            self.tab_rename = None;
        }
        let valid_tabs = self.tab_ids();
        self.tab_manager.retain_tabs(&valid_tabs);
        if self.tabs.is_empty() {
            self.select_empty_workspace();
            window.focus(&self.focus_handle);
            cx.notify();
            return;
        }

        if was_active || !valid_tabs.contains(&self.active_tab_id.0) {
            let next_index = index.min(self.tabs.len().saturating_sub(1));
            self.active_tab_id = self.tabs[next_index].id;
            let tab = self.tabs[next_index].clone();
            if let Some(path) = tab.file_path.clone() {
                self.sidebar_explorer.selected_path = Some(path.clone());
                self.active_editor_entity().update(cx, |editor, cx| {
                    editor.activate_path_from_workspace(path, cx)
                });
            }
            let surface = tab.surface;
            self.tab_manager.set_active_tab(self.active_tab_id.0);
            self.focus_surface(surface, window, cx);
        }
        cx.notify();
    }
}

impl Focusable for WorkspacePrototype {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for WorkspacePrototype {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.persist_workspace_recovery(false);

        let active_surface = self.active_surface_if_present();
        let active_tab_id = self.active_tab_id;
        let tabs = self.tabs.clone();
        let joined_panes = active_surface
            .is_some()
            .then(|| self.joined_panes_for_active())
            .flatten();
        let tab_context_menu = self.tab_manager.context_menu();
        let tab_overflow_open = self.tab_overflow_open;
        let tab_name_overrides = self.tab_name_overrides.clone();
        let tab_rename = self.tab_rename.clone();
        let sidebar_context_menu = self.sidebar_context_menu.clone();
        let sidebar_rename = self.sidebar_rename.clone();
        let sidebar_new_entry = self.sidebar_new_entry.clone();
        let queued_prompts = self.stacker.read(cx).queued_prompts().to_vec();
        let appearance_config = self.appearance_config.clone();
        let workspace_palette = WorkspacePalette::from_config(&appearance_config);
        let appearance_page = self.appearance_page;
        let terminal_background_import_error = self.terminal_background_import_error.clone();
        let explorer_entries = self.explorer_entries();
        let selected_path = self.sidebar_explorer.selected_path.clone();
        let workspace_root = self.workspace_root.clone();
        let recent_projects = self.recent_projects.clone();
        let recent_projects_open = self.recent_projects_open;
        let sidebar_width = self.sidebar_width;
        let sidebar_visible = self.sidebar_visible;
        let explorer_status = self.sidebar_explorer.status.clone();

        let mut main = div().flex_1().flex().overflow_hidden();
        if sidebar_visible {
            main = main.child(workspace_sidebar(
                WorkspaceSidebarContext {
                    workspace_root: workspace_root.clone(),
                    entries: explorer_entries,
                    selected_path: selected_path.clone(),
                    recent_projects: recent_projects.clone(),
                    recent_projects_open,
                    sidebar_width,
                    explorer_status,
                    palette: workspace_palette,
                },
                cx,
            ));
        }
        main = main
            .child(sidebar_bumper(sidebar_visible, workspace_palette, cx))
            .child(workspace_content(
                WorkspaceSurfaceContext {
                    stacker: self.stacker.clone(),
                    editor: self.editor.clone(),
                    file_editors: self.file_editors.clone(),
                    terminals: self.terminals.clone(),
                    sketch: self.sketch.clone(),
                    workspace_root,
                    recent_projects,
                    explorers: self.explorers.clone(),
                    appearance_config,
                    appearance_page,
                    terminal_background_import_error,
                    editor_word_wrap: self.editor_word_wrap_enabled(),
                    joined_tab_limit: self.tab_join_limit(),
                    error_log_expanded: self.error_log_expanded,
                    error_log_filter: self.error_log_filter,
                    pending_clear_error_log: self.pending_clear_error_log,
                },
                active_surface,
                active_tab_id,
                joined_panes,
                cx,
            ));

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(workspace_palette.chrome_bg))
            .text_color(rgb(workspace_palette.sidebar_text))
            .font_family("Atkinson Hyperlegible")
            .key_context("Workspace")
            .track_focus(&self.focus_handle(cx))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _window, cx| {
                    this.close_context_menus(cx);
                }),
            )
            .on_key_down(cx.listener(Self::on_workspace_key_down))
            .on_action(cx.listener(Self::menu_new_tab))
            .on_action(cx.listener(Self::menu_close_tab))
            .on_action(cx.listener(Self::menu_next_tab))
            .on_action(cx.listener(Self::menu_previous_tab))
            .on_action(cx.listener(Self::menu_activate_tab_1))
            .on_action(cx.listener(Self::menu_activate_tab_2))
            .on_action(cx.listener(Self::menu_activate_tab_3))
            .on_action(cx.listener(Self::menu_activate_tab_4))
            .on_action(cx.listener(Self::menu_activate_tab_5))
            .on_action(cx.listener(Self::menu_activate_tab_6))
            .on_action(cx.listener(Self::menu_activate_tab_7))
            .on_action(cx.listener(Self::menu_activate_tab_8))
            .on_action(cx.listener(Self::menu_activate_tab_9))
            .on_action(cx.listener(Self::menu_show_command_palette))
            .on_action(cx.listener(Self::menu_show_file_finder))
            .on_action(cx.listener(Self::menu_join_tabs))
            .on_action(cx.listener(Self::menu_separate_tabs))
            .on_action(cx.listener(Self::menu_partition_vertical))
            .on_action(cx.listener(Self::menu_partition_horizontal))
            .on_action(cx.listener(Self::menu_swap_tabs))
            .on_action(cx.listener(Self::menu_open_project))
            .on_action(cx.listener(Self::menu_close_project))
            .on_action(cx.listener(Self::menu_save))
            .on_action(cx.listener(Self::menu_undo))
            .on_action(cx.listener(Self::menu_redo))
            .on_action(cx.listener(Self::menu_copy))
            .on_action(cx.listener(Self::menu_paste))
            .on_action(cx.listener(Self::menu_select_all))
            .on_action(cx.listener(Self::menu_find))
            .on_action(cx.listener(Self::menu_editor_check_disk))
            .on_action(cx.listener(Self::menu_editor_reopen_closed))
            .on_action(cx.listener(Self::menu_editor_close_others))
            .on_action(cx.listener(Self::menu_editor_close_saved))
            .on_action(cx.listener(Self::menu_markdown_source))
            .on_action(cx.listener(Self::menu_markdown_preview))
            .on_action(cx.listener(Self::menu_markdown_split))
            .on_action(cx.listener(Self::menu_markdown_cycle))
            .on_action(cx.listener(Self::menu_lsp_hover))
            .on_action(cx.listener(Self::menu_lsp_completion))
            .on_action(cx.listener(Self::menu_lsp_definition))
            .on_action(cx.listener(Self::menu_lsp_references))
            .on_action(cx.listener(Self::menu_lsp_signature_help))
            .on_action(cx.listener(Self::menu_lsp_rename))
            .on_action(cx.listener(Self::menu_lsp_code_actions))
            .on_action(cx.listener(Self::menu_lsp_format))
            .on_action(cx.listener(Self::menu_lsp_symbols))
            .on_action(cx.listener(Self::menu_toggle_sidebar))
            .on_action(cx.listener(Self::menu_show_home))
            .on_action(cx.listener(Self::menu_show_terminal))
            .on_action(cx.listener(Self::menu_show_stacker))
            .on_action(cx.listener(Self::menu_show_editor))
            .on_action(cx.listener(Self::menu_show_sketch))
            .on_action(cx.listener(Self::menu_show_appearances))
            .on_action(cx.listener(Self::menu_zoom_in))
            .on_action(cx.listener(Self::menu_zoom_out))
            .on_action(cx.listener(Self::menu_zoom_reset))
            .child(workspace_tab_bar(
                tabs,
                active_tab_id,
                tab_name_overrides,
                &self.tab_manager,
                tab_overflow_open,
                workspace_palette,
                cx,
            ))
            .child(main)
            .child(workspace_footer(
                active_surface,
                queued_prompts,
                workspace_palette,
                cx,
            ))
            .when_some(tab_context_menu, |root, menu| {
                root.child(workspace_tab_context_menu(
                    menu,
                    self.tab_choices(),
                    &self.tab_manager,
                    tab_rename,
                    self.tab_join_limit(),
                    cx,
                ))
            })
            .when(tab_overflow_open, |root| {
                root.child(self::tabs::workspace_tab_overflow_menu(
                    self.tabs.clone(),
                    active_tab_id,
                    &self.tab_manager,
                    cx,
                ))
            })
            .when_some(sidebar_context_menu, |root, menu| {
                root.child(workspace_sidebar_context_menu(
                    menu,
                    self.workspace_root.clone(),
                    sidebar_rename,
                    sidebar_new_entry,
                    cx,
                ))
            })
            .when(self.palette.open, |root| {
                let entries = command_palette::palette_entries();
                let visible = match self.palette.mode {
                    command_palette::PaletteMode::Commands => {
                        command_palette::filter_entries(&entries, &self.palette.query)
                    }
                    command_palette::PaletteMode::Files => self
                        .palette
                        .project_root
                        .as_ref()
                        .map(|root| {
                            command_palette::filter_files(
                                &self.palette.files,
                                root,
                                &self.palette.query,
                            )
                        })
                        .unwrap_or_default(),
                };
                root.child(command_palette::render_command_palette(
                    &self.palette,
                    &entries,
                    &visible,
                    cx,
                ))
            })
    }
}
