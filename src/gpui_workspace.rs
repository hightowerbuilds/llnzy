use std::{collections::BTreeMap, path::PathBuf};

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
mod sidebar;
mod tabs;

use crate::config::Config;
use crate::gpui_editor::{bind_editor_keys, EditorPrototype};
use crate::gpui_sketch::{bind_sketch_keys, SketchSurface};
use crate::gpui_stacker::{bind_stacker_keys, StackerPrototype};
use crate::gpui_tabs::{GpuiTabChoice, GpuiTabManager};
use crate::gpui_terminal::{bind_terminal_keys, TerminalSurface};

use self::footer::{load_workspace_queue, workspace_footer};
use self::panes::{workspace_content, WorkspaceSurfaceContext};
use self::sidebar::{
    collect_explorer_entries, sidebar_bumper, workspace_sidebar, workspace_sidebar_context_menu,
    ExplorerState, SidebarContextMenuState, SidebarRenameState, WorkspaceSidebarContext,
};
use self::tabs::{
    reorder_workspace_tab_block, workspace_tab_bar, workspace_tab_context_menu,
    workspace_tab_label, workspace_tab_layouts, workspace_tab_menu_anchor, TabRenameState,
    WorkspaceTab, WorkspaceTabId, WorkspaceTabMenuAnchor,
};

actions!(
    workspace_gpui,
    [
        Quit,
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
        MenuJoinTabs,
        MenuSeparateTabs,
        MenuSwapTabs,
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

const FOOTER_HEIGHT: f32 = 48.0;
const JOINED_TAB_DIVIDER_WIDTH: f32 = 8.0;
const SIDEBAR_DEFAULT_WIDTH: f32 = 220.0;
const SIDEBAR_MIN_WIDTH: f32 = 160.0;
const SIDEBAR_MAX_WIDTH: f32 = 380.0;
const BUMPER_WIDTH: f32 = 20.0;
const BUMPER_RESIZE_WIDTH: f32 = 6.0;
const EXPLORER_ENTRY_LIMIT: usize = 260;
const GPUI_TERMINAL_BACKGROUND_MAX_EDGE: u32 = 2048;
const SIDEBAR_ROW_BG: u32 = CHROME_BG;
const SIDEBAR_ROW_HOVER_BG: u32 = 0x2b2e36;
const SIDEBAR_ROW_SELECTED_BG: u32 = 0x303440;
const SIDEBAR_ROW_SELECTED_HOVER_BG: u32 = 0x3a4050;
const SIDEBAR_DROP_VALID_BG: u32 = 0x1f3a2b;
const SIDEBAR_DROP_INVALID_BG: u32 = 0x3d2428;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AppearancePage {
    Terminal,
    Editor,
    Markdown,
    Sketch,
}

impl AppearancePage {
    fn title(self) -> &'static str {
        match self {
            AppearancePage::Terminal => "Terminal",
            AppearancePage::Editor => "Editor",
            AppearancePage::Markdown => "Markdown",
            AppearancePage::Sketch => "Sketch",
        }
    }
}

#[derive(Clone, Copy)]
struct JoinedWorkspacePanes {
    primary_id: WorkspaceTabId,
    primary_surface: WorkspaceSurface,
    secondary_id: WorkspaceTabId,
    secondary_surface: WorkspaceSurface,
    ratio: f32,
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
        ]);

        let bounds = Bounds::centered(None, size(px(1320.0), px(820.0)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_, cx| cx.new(WorkspacePrototype::new),
            )
            .unwrap();
        window
            .update(cx, |view, window, cx| {
                view.focus_surface(view.active_surface(), window, cx);
            })
            .unwrap();
        cx.on_action(|_: &Quit, cx| cx.quit());
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
                MenuItem::separator(),
                MenuItem::action("Home", MenuShowHome),
                MenuItem::action("Terminal", MenuShowTerminal),
                MenuItem::action("Stacker", MenuShowStacker),
                MenuItem::action("Sketch Pad", MenuShowSketch),
                MenuItem::action("Appearances", MenuShowAppearances),
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
}

impl WorkspacePrototype {
    fn new(cx: &mut Context<Self>) -> Self {
        let workspace_root = std::env::current_dir().ok();
        let tabs = vec![
            WorkspaceTab::new(WorkspaceTabId(1), WorkspaceSurface::Home),
            WorkspaceTab::new(WorkspaceTabId(2), WorkspaceSurface::Stacker),
            WorkspaceTab::new(WorkspaceTabId(3), WorkspaceSurface::Terminal),
            WorkspaceTab::new(WorkspaceTabId(4), WorkspaceSurface::Sketch),
        ];
        let sidebar_explorer = ExplorerState::for_root(workspace_root.as_deref());
        let sketch = cx.new(SketchSurface::new);
        sketch.update(cx, |sketch, _cx| {
            sketch.set_workspace_root(workspace_root.clone());
        });

        let mut terminals = BTreeMap::new();
        terminals.insert(3, cx.new(TerminalSurface::new));

        Self {
            stacker: cx.new(StackerPrototype::embedded),
            editor: cx.new(EditorPrototype::new),
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
            active_tab_id: WorkspaceTabId(1),
            next_tab_id: 5,
            workspace_root,
            sidebar_explorer,
            explorers: BTreeMap::new(),
            recent_projects: crate::explorer::load_recent_projects(),
            recent_projects_open: false,
            sidebar_visible: true,
            sidebar_width: SIDEBAR_DEFAULT_WIDTH,
            last_sidebar_width: SIDEBAR_DEFAULT_WIDTH,
            appearance_config: Config::default(),
            appearance_page: AppearancePage::Terminal,
            terminal_background_import_error: None,
            palette: command_palette::CommandPaletteState::default(),
        }
    }

    fn active_surface(&self) -> WorkspaceSurface {
        self.tabs
            .iter()
            .find(|tab| tab.id == self.active_tab_id)
            .or_else(|| self.tabs.first())
            .map(|tab| tab.surface)
            .unwrap_or(WorkspaceSurface::Home)
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

    fn active_explorer_state_mut(&mut self) -> &mut ExplorerState {
        if self.active_surface() == WorkspaceSurface::Explorer
            && self.explorers.contains_key(&self.active_tab_id.0)
        {
            return self.explorers.get_mut(&self.active_tab_id.0).unwrap();
        }
        &mut self.sidebar_explorer
    }

    fn joined_panes_for_active(&self) -> Option<JoinedWorkspacePanes> {
        let joined = self
            .tab_manager
            .joined_pair_for(self.active_tab_id.0, &self.tab_ids())?;
        let primary_id = WorkspaceTabId(joined.primary);
        let secondary_id = WorkspaceTabId(joined.secondary);
        let primary_surface = self.tab_by_id(primary_id)?.surface;
        let secondary_surface = self.tab_by_id(secondary_id)?.surface;
        Some(JoinedWorkspacePanes {
            primary_id,
            primary_surface,
            secondary_id,
            secondary_surface,
            ratio: joined.ratio,
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
        if self.sidebar_context_menu.is_some() || self.sidebar_rename.is_some() {
            self.sidebar_context_menu = None;
            self.sidebar_rename = None;
            cx.notify();
        }
    }

    fn close_context_menus(&mut self, cx: &mut Context<Self>) {
        let had_tab_menu = self.tab_manager.context_menu().is_some();
        let had_tab_overflow_menu = self.tab_overflow_open;
        let had_sidebar_menu = self.sidebar_context_menu.is_some() || self.sidebar_rename.is_some();
        self.tab_manager.close_context_menu();
        self.tab_overflow_open = false;
        self.tab_rename = None;
        self.sidebar_context_menu = None;
        self.sidebar_rename = None;
        if had_tab_menu || had_tab_overflow_menu || had_sidebar_menu {
            cx.notify();
        }
    }

    fn toggle_tab_overflow_menu(&mut self, cx: &mut Context<Self>) {
        self.tab_manager.close_context_menu();
        self.tab_rename = None;
        self.sidebar_context_menu = None;
        self.sidebar_rename = None;
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

    fn close_command_palette(&mut self, cx: &mut Context<Self>) {
        if self.palette.open {
            self.palette.reset();
            cx.notify();
        }
    }

    pub(super) fn invoke_palette_at(
        &mut self,
        display_idx: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let entries = command_palette::palette_entries();
        let visible = command_palette::filter_entries(&entries, &self.palette.query);
        let Some(&entry_idx) = visible.get(display_idx) else {
            return;
        };
        let action = (entries[entry_idx].build_action)();
        self.palette.reset();
        cx.notify();
        // Defer dispatch so the palette overlay is gone before the action
        // runs (otherwise an action that toggles UI would race with the
        // open palette).
        window.dispatch_action(action, cx);
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
                let entries = command_palette::palette_entries();
                let visible_count =
                    command_palette::filter_entries(&entries, &self.palette.query).len();
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
        if self.tab_manager.join_pair(primary_id.0, secondary_id.0) {
            self.place_joined_tabs_together(primary_id, secondary_id);
            self.active_tab_id = primary_id;
            cx.notify();
        }
    }

    fn place_joined_tabs_together(
        &mut self,
        primary_id: WorkspaceTabId,
        secondary_id: WorkspaceTabId,
    ) {
        let Some(primary_index) = self.tabs.iter().position(|tab| tab.id == primary_id) else {
            return;
        };
        let Some(secondary_index) = self.tabs.iter().position(|tab| tab.id == secondary_id) else {
            return;
        };
        if secondary_index == primary_index + 1 {
            return;
        }

        let secondary = self.tabs.remove(secondary_index);
        let insert_index = if secondary_index < primary_index {
            primary_index
        } else {
            primary_index + 1
        };
        self.tabs.insert(insert_index, secondary);
    }

    fn separate_tab_by_id(&mut self, tab_id: WorkspaceTabId, cx: &mut Context<Self>) {
        if self.tab_manager.separate_tab(tab_id.0) {
            self.active_tab_id = tab_id;
            cx.notify();
        }
    }

    fn swap_tabs_by_id(&mut self, tab_id: WorkspaceTabId, cx: &mut Context<Self>) {
        if self.tab_manager.swap_tabs_for_tab(tab_id.0) {
            cx.notify();
        }
    }

    fn resize_joined_panes_by_tab(
        &mut self,
        tab_id: WorkspaceTabId,
        ratio: f32,
        cx: &mut Context<Self>,
    ) {
        if self.tab_manager.set_ratio_for_tab(tab_id.0, ratio) {
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
            .joined_pair_for(dragged_id.0, &valid_tabs)
            .map(|joined| {
                let mut ids = [
                    WorkspaceTabId(joined.primary),
                    WorkspaceTabId(joined.secondary),
                ];
                ids.sort_by_key(|id| {
                    self.tabs
                        .iter()
                        .position(|tab| tab.id == *id)
                        .unwrap_or(usize::MAX)
                });
                ids.to_vec()
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
        if self.active_surface() == surface {
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
        let terminal = cx.new(TerminalSurface::new);
        let config = self.appearance_config.clone();
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

    fn open_new_explorer_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.explorers.is_empty() {
            let parked_id = WorkspaceTabId(self.next_tab_id);
            self.next_tab_id += 1;
            self.tabs
                .push(WorkspaceTab::new(parked_id, WorkspaceSurface::Explorer));
            self.explorers
                .insert(parked_id.0, self.sidebar_explorer.clone());
            if self.sidebar_visible {
                self.last_sidebar_width = self.sidebar_width;
            }
            self.sidebar_visible = false;
        }

        let fresh_id = WorkspaceTabId(self.next_tab_id);
        self.next_tab_id += 1;
        self.tabs
            .push(WorkspaceTab::new(fresh_id, WorkspaceSurface::Explorer));
        self.explorers.insert(
            fresh_id.0,
            ExplorerState::for_root(self.workspace_root.as_deref()),
        );
        self.active_tab_id = fresh_id;
        self.tab_manager.set_active_tab(fresh_id.0);
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
        } else if surface == WorkspaceSurface::Explorer {
            self.open_new_explorer_tab(window, cx);
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
            let closed = self.file_editors.get(&tab_id.0).map_or(true, |editor| {
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
        if self.tabs[index].surface == WorkspaceSurface::Terminal {
            self.terminals.remove(&tab_id.0);
        }
        if self.tabs[index].surface == WorkspaceSurface::Explorer {
            let removed = self.explorers.remove(&tab_id.0);
            if self.explorers.is_empty() {
                if let Some(state) = removed {
                    self.sidebar_explorer = state;
                }
                self.sidebar_visible = true;
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
            self.tabs.push(WorkspaceTab::new(
                WorkspaceTabId(self.next_tab_id),
                WorkspaceSurface::Home,
            ));
            self.next_tab_id += 1;
        }

        if was_active {
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
        let active_surface = self.active_surface();
        let active_tab_id = self.active_tab_id;
        let tabs = self.tabs.clone();
        let joined_panes = self.joined_panes_for_active();
        let tab_context_menu = self.tab_manager.context_menu();
        let tab_overflow_open = self.tab_overflow_open;
        let tab_name_overrides = self.tab_name_overrides.clone();
        let tab_rename = self.tab_rename.clone();
        let sidebar_context_menu = self.sidebar_context_menu.clone();
        let sidebar_rename = self.sidebar_rename.clone();
        let queued_prompts = load_workspace_queue();
        let appearance_config = self.appearance_config.clone();
        let appearance_page = self.appearance_page;
        let terminal_background_import_error = self.terminal_background_import_error.clone();
        let explorer_entries = self
            .workspace_root
            .as_ref()
            .map(|root| collect_explorer_entries(root, &self.sidebar_explorer.expanded_dirs))
            .unwrap_or_default();
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
                },
                cx,
            ));
        }
        main = main
            .child(sidebar_bumper(sidebar_visible, cx))
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
            .bg(rgb(CHROME_BG))
            .text_color(rgb(SIDEBAR_TEXT))
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
            .on_action(cx.listener(Self::menu_join_tabs))
            .on_action(cx.listener(Self::menu_separate_tabs))
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
                cx,
            ))
            .child(main)
            .child(workspace_footer(active_surface, queued_prompts, cx))
            .when_some(tab_context_menu, |root, menu| {
                root.child(workspace_tab_context_menu(
                    menu,
                    self.tab_choices(),
                    &self.tab_manager,
                    tab_rename,
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
                    cx,
                ))
            })
            .when(self.palette.open, |root| {
                let entries = command_palette::palette_entries();
                let visible = command_palette::filter_entries(&entries, &self.palette.query);
                root.child(command_palette::render_command_palette(
                    &self.palette,
                    &entries,
                    &visible,
                    cx,
                ))
            })
    }
}
