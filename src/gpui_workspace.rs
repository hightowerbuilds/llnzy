use std::{
    collections::hash_map::DefaultHasher,
    collections::BTreeMap,
    collections::BTreeSet,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

use gpui::prelude::*;
use gpui::{
    actions, div, px, relative, rgb, size, App, Application, Bounds, ClickEvent, ClipboardItem,
    Context, DragMoveEvent, Entity, FocusHandle, Focusable, KeyBinding, KeyDownEvent, Menu,
    MenuItem, MouseButton, MouseDownEvent, Pixels, Render, Window, WindowBounds, WindowOptions,
};

use crate::gpui_editor::{bind_editor_keys, EditorPrototype};
use crate::gpui_sketch::{bind_sketch_keys, SketchSurface};
use crate::gpui_stacker::{bind_stacker_keys, StackerPrototype};
use crate::gpui_tabs::{GpuiTabChoice, GpuiTabContextMenu, GpuiTabContextMenuView, GpuiTabManager};
use crate::gpui_terminal::{bind_terminal_keys, TerminalSurface};
use crate::{
    config::{BackgroundImageFit, Config, CursorStyle},
    path_utils::{path_contains, same_path},
    sidebar_move::{plan_sidebar_move, MoveOrigin, SidebarMovePlanItem, SidebarMoveRequest},
    theme::builtin_themes,
};

actions!(
    workspace_gpui,
    [
        Quit,
        MenuNewTab,
        MenuCloseTab,
        MenuNextTab,
        MenuPreviousTab,
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

const TAB_BAR_HEIGHT: f32 = 44.0;
const TAB_BAR_PADDING_X: f32 = 8.0;
const TAB_BAR_PADDING_Y: f32 = 4.0;
const TAB_BAR_GAP: f32 = 4.0;
const TAB_HEIGHT: f32 = 32.0;
const TAB_MENU_ITEM_HEIGHT: f32 = 32.0;
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
    Sketch,
}

impl AppearancePage {
    fn title(self) -> &'static str {
        match self {
            AppearancePage::Terminal => "Terminal",
            AppearancePage::Editor => "Editor",
            AppearancePage::Sketch => "Sketch",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WorkspaceTabId(u64);

#[derive(Clone, Debug)]
struct WorkspaceTab {
    id: WorkspaceTabId,
    surface: WorkspaceSurface,
}

impl WorkspaceTab {
    fn new(id: WorkspaceTabId, surface: WorkspaceSurface) -> Self {
        Self { id, surface }
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

#[derive(Clone, Copy)]
struct WorkspaceTabLayout {
    id: WorkspaceTabId,
    x: f32,
    width: f32,
}

#[derive(Clone, Copy)]
struct WorkspaceTabMenuAnchor {
    x: f32,
    y: f32,
    width: f32,
}

#[derive(Clone, Debug)]
struct TabRenameState {
    tab_id: WorkspaceTabId,
    text: String,
    replace_on_input: bool,
}

#[derive(Clone, Debug)]
struct ExplorerEntry {
    path: PathBuf,
    name: String,
    is_dir: bool,
    depth: usize,
    expanded: bool,
}

#[derive(Clone, Copy)]
struct SidebarResizeDrag;

impl Render for SidebarResizeDrag {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(2.0))
            .h(px(28.0))
            .rounded_sm()
            .bg(rgb(QUEUE_GREEN))
    }
}

#[derive(Clone, Debug)]
struct ExplorerDragPayload {
    paths: Vec<PathBuf>,
    label: String,
    is_dir: bool,
}

impl ExplorerDragPayload {
    fn new(path: PathBuf, label: String, is_dir: bool) -> Self {
        Self {
            paths: vec![path],
            label,
            is_dir,
        }
    }
}

struct ExplorerDragPreview {
    label: String,
    is_dir: bool,
}

impl ExplorerDragPreview {
    fn new(payload: &ExplorerDragPayload) -> Self {
        Self {
            label: payload.label.clone(),
            is_dir: payload.is_dir,
        }
    }
}

impl Render for ExplorerDragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(220.0))
            .h(px(30.0))
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .rounded_sm()
            .border_1()
            .border_color(rgb(0x4c5262))
            .bg(rgb(0x20232b))
            .text_size(px(12.0))
            .text_color(rgb(ACTIVE_TEXT))
            .shadow_md()
            .child(
                div()
                    .w(px(34.0))
                    .text_size(px(10.0))
                    .text_color(rgb(if self.is_dir { FOLDER_BLUE } else { MUTED_TEXT }))
                    .child(if self.is_dir { "DIR" } else { "FILE" }),
            )
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .child(self.label.clone()),
            )
    }
}

pub fn run_workspace_prototype() {
    Application::new().run(|cx: &mut App| {
        bind_stacker_keys(cx);
        bind_editor_keys(cx);
        bind_terminal_keys(cx);
        bind_sketch_keys(cx);
        install_workspace_menu_bar(cx);
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);

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
    terminal: Entity<TerminalSurface>,
    sketch: Entity<SketchSurface>,
    focus_handle: FocusHandle,
    tabs: Vec<WorkspaceTab>,
    tab_manager: GpuiTabManager,
    tab_name_overrides: BTreeMap<u64, String>,
    tab_rename: Option<TabRenameState>,
    active_tab_id: WorkspaceTabId,
    next_tab_id: u64,
    workspace_root: Option<PathBuf>,
    expanded_dirs: BTreeSet<PathBuf>,
    selected_path: Option<PathBuf>,
    recent_projects: Vec<PathBuf>,
    recent_projects_open: bool,
    sidebar_visible: bool,
    sidebar_width: f32,
    last_sidebar_width: f32,
    explorer_status: Option<String>,
    appearance_config: Config,
    appearance_page: AppearancePage,
    terminal_background_import_error: Option<String>,
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
        let expanded_dirs = workspace_root
            .as_ref()
            .map(|root| initial_expanded_dirs(root))
            .unwrap_or_default();
        Self {
            stacker: cx.new(StackerPrototype::embedded),
            editor: cx.new(EditorPrototype::new),
            terminal: cx.new(TerminalSurface::new),
            sketch: cx.new(SketchSurface::new),
            focus_handle: cx.focus_handle(),
            tabs,
            tab_manager: GpuiTabManager::default(),
            tab_name_overrides: BTreeMap::new(),
            tab_rename: None,
            active_tab_id: WorkspaceTabId(1),
            next_tab_id: 5,
            workspace_root,
            expanded_dirs,
            selected_path: None,
            recent_projects: crate::explorer::load_recent_projects(),
            recent_projects_open: false,
            sidebar_visible: true,
            sidebar_width: SIDEBAR_DEFAULT_WIDTH,
            last_sidebar_width: SIDEBAR_DEFAULT_WIDTH,
            explorer_status: None,
            appearance_config: Config::default(),
            appearance_page: AppearancePage::Terminal,
            terminal_background_import_error: None,
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
            .unwrap_or_else(|| {
                workspace_tab_label(tab.surface, self.selected_path.as_deref(), None)
            })
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

    fn open_tab_context_menu(
        &mut self,
        tab_id: WorkspaceTabId,
        anchor: WorkspaceTabMenuAnchor,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle);
        self.tab_rename = None;
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
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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

    fn active_tab_menu_anchor(&self) -> WorkspaceTabMenuAnchor {
        let layouts = workspace_tab_layouts(&self.tabs, self.active_tab_id);
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
                window.focus(&self.editor.focus_handle(cx));
            }
            WorkspaceSurface::Terminal => window.focus(&self.terminal.focus_handle(cx)),
            WorkspaceSurface::Sketch => window.focus(&self.sketch.focus_handle(cx)),
        }
    }

    fn activate_tab(
        &mut self,
        tab_id: WorkspaceTabId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(tab) = self.tabs.iter().find(|tab| tab.id == tab_id) {
            self.active_tab_id = tab.id;
            self.tab_manager.set_active_tab(tab.id.0);
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
        self.active_tab_id = tab_id;
        self.tab_manager.set_active_tab(tab_id.0);
        self.focus_surface(surface, window, cx);
        cx.notify();
    }

    fn close_tab(&mut self, tab_id: WorkspaceTabId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.tabs.iter().position(|tab| tab.id == tab_id) else {
            return;
        };

        let was_active = self.active_tab_id == tab_id;
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
            let surface = self.tabs[next_index].surface;
            self.tab_manager.set_active_tab(self.active_tab_id.0);
            self.focus_surface(surface, window, cx);
        }
        cx.notify();
    }

    fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        if self.sidebar_visible {
            self.last_sidebar_width = self.sidebar_width;
            self.sidebar_visible = false;
        } else {
            self.sidebar_visible = true;
            self.sidebar_width = self
                .last_sidebar_width
                .clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH);
        }
        cx.notify();
    }

    fn resize_sidebar_from_x(&mut self, x: Pixels, cx: &mut Context<Self>) {
        let width =
            (x / px(1.0) - BUMPER_RESIZE_WIDTH / 2.0).clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH);
        self.sidebar_visible = true;
        self.sidebar_width = width;
        self.last_sidebar_width = width;
        cx.notify();
    }

    fn toggle_explorer_dir(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if !self.expanded_dirs.remove(&path) {
            self.expanded_dirs.insert(path);
        }
        cx.notify();
    }

    fn open_sidebar_file(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.selected_path = Some(path.clone());
        let editor_path = path.clone();
        self.editor.update(cx, |editor, cx| {
            editor.open_path(editor_path, cx);
        });
        self.open_or_activate_surface(WorkspaceSurface::Editor, window, cx);
    }

    fn move_explorer_items_to_folder(
        &mut self,
        sources: Vec<PathBuf>,
        destination_folder: PathBuf,
        cx: &mut Context<Self>,
    ) {
        let request = SidebarMoveRequest::new(sources, destination_folder, MoveOrigin::DragDrop);
        let plan = match plan_sidebar_move(&request) {
            Ok(plan) => plan,
            Err(message) => {
                self.explorer_status = Some(message);
                cx.notify();
                return;
            }
        };

        let moved_sources = plan
            .items
            .iter()
            .map(|item| (item.source.clone(), item.is_dir))
            .collect::<Vec<_>>();
        if let Some(message) = self
            .editor
            .read(cx)
            .modified_open_path_for_move(&moved_sources)
        {
            self.explorer_status = Some(message);
            cx.notify();
            return;
        }

        for item in &plan.items {
            if let Err(error) = fs::rename(&item.source, &item.destination) {
                self.explorer_status = Some(format!("Move failed: {error}"));
                cx.notify();
                return;
            }
        }

        self.expanded_dirs.insert(plan.destination_folder.clone());
        self.remap_after_explorer_move(&plan.items, cx);
        let moved_count = plan.len();
        self.explorer_status = Some(if moved_count == 1 {
            "Moved item".to_string()
        } else {
            format!("Moved {moved_count} items")
        });
        cx.notify();
    }

    fn remap_after_explorer_move(&mut self, moved: &[SidebarMovePlanItem], cx: &mut Context<Self>) {
        let expanded_dirs = self.expanded_dirs.iter().cloned().collect::<Vec<_>>();
        for expanded_dir in expanded_dirs {
            if let Some(remapped) = remap_path_after_explorer_move(&expanded_dir, moved) {
                self.expanded_dirs.remove(&expanded_dir);
                self.expanded_dirs.insert(remapped);
            }
        }

        if let Some(selected_path) = self.selected_path.clone() {
            if let Some(remapped) = remap_path_after_explorer_move(&selected_path, moved) {
                self.selected_path = Some(remapped);
            }
        }

        let editor_moves = moved
            .iter()
            .map(|item| (item.source.clone(), item.destination.clone(), item.is_dir))
            .collect::<Vec<_>>();
        self.editor
            .update(cx, |editor, cx| editor.remap_moved_paths(&editor_moves, cx));
    }

    fn pick_open_project(&mut self, cx: &mut Context<Self>) {
        cx.spawn(
            |workspace: gpui::WeakEntity<WorkspacePrototype>, cx: &mut gpui::AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    let Some(folder) = rfd::AsyncFileDialog::new()
                        .set_title("Open Project Folder")
                        .pick_folder()
                        .await
                    else {
                        return;
                    };
                    let path = folder.path().to_path_buf();
                    let _ = workspace.update(&mut cx, |workspace, cx| {
                        workspace.open_project(path, cx);
                    });
                }
            },
        )
        .detach();
    }

    fn open_project(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if !path.is_dir() {
            return;
        }
        crate::explorer::add_recent_project(&mut self.recent_projects, path.clone());
        self.workspace_root = Some(path.clone());
        self.expanded_dirs = initial_expanded_dirs(&path);
        self.selected_path = None;
        self.close_editor_file_tab();
        self.sidebar_visible = true;
        self.recent_projects_open = false;
        self.explorer_status = None;
        cx.notify();
    }

    fn close_project(&mut self, cx: &mut Context<Self>) {
        self.workspace_root = None;
        self.expanded_dirs.clear();
        self.selected_path = None;
        self.close_editor_file_tab();
        self.sidebar_visible = true;
        self.recent_projects_open = false;
        self.explorer_status = None;
        cx.notify();
    }

    fn close_editor_file_tab(&mut self) {
        let active_was_editor = self.active_surface() == WorkspaceSurface::Editor;
        self.tabs
            .retain(|tab| tab.surface != WorkspaceSurface::Editor);
        if self.tabs.is_empty() {
            self.tabs.push(WorkspaceTab::new(
                WorkspaceTabId(self.next_tab_id),
                WorkspaceSurface::Home,
            ));
            self.next_tab_id += 1;
        }
        let active_tab_still_exists = self.tabs.iter().any(|tab| tab.id == self.active_tab_id);
        if active_was_editor || !active_tab_still_exists {
            self.active_tab_id = self
                .tabs
                .iter()
                .find(|tab| tab.surface == WorkspaceSurface::Home)
                .or_else(|| self.tabs.first())
                .map(|tab| tab.id)
                .unwrap_or(self.active_tab_id);
            self.tab_manager.set_active_tab(self.active_tab_id.0);
        }
        let valid_tabs = self.tab_ids();
        self.tab_manager.retain_tabs(&valid_tabs);
        self.tab_name_overrides
            .retain(|tab_id, _| valid_tabs.contains(tab_id));
        if self
            .tab_rename
            .as_ref()
            .is_some_and(|rename| !valid_tabs.contains(&rename.tab_id.0))
        {
            self.tab_rename = None;
        }
    }

    fn toggle_recent_projects(&mut self, cx: &mut Context<Self>) {
        self.recent_projects_open = !self.recent_projects_open;
        cx.notify();
    }

    fn apply_appearance_config(&mut self, cx: &mut Context<Self>) {
        let config = self.appearance_config.clone();
        self.editor.update(cx, |editor, cx| {
            editor.set_appearance_config(config.clone(), cx)
        });
        self.terminal
            .update(cx, |terminal, cx| terminal.set_config(config, cx));
        cx.notify();
    }

    fn set_appearance_page(&mut self, page: AppearancePage, cx: &mut Context<Self>) {
        self.appearance_page = page;
        cx.notify();
    }

    fn apply_builtin_theme(&mut self, theme_name: &str, cx: &mut Context<Self>) {
        if let Some(theme) = builtin_themes()
            .into_iter()
            .find(|theme| theme.name == theme_name)
        {
            theme.apply_to(&mut self.appearance_config);
            self.apply_appearance_config(cx);
        }
    }

    fn adjust_font_size(&mut self, delta: f32, cx: &mut Context<Self>) {
        self.appearance_config.font_size =
            (self.appearance_config.font_size + delta).clamp(8.0, 40.0);
        self.apply_appearance_config(cx);
    }

    fn adjust_line_height(&mut self, delta: f32, cx: &mut Context<Self>) {
        self.appearance_config.line_height =
            (self.appearance_config.line_height + delta).clamp(0.9, 2.2);
        self.apply_appearance_config(cx);
    }

    fn set_cursor_style(&mut self, style: CursorStyle, cx: &mut Context<Self>) {
        self.appearance_config.cursor_style = style;
        self.apply_appearance_config(cx);
    }

    fn set_background_mode(&mut self, mode: &'static str, cx: &mut Context<Self>) {
        self.appearance_config.effects.background = mode.to_string();
        self.terminal_background_import_error = None;
        self.apply_appearance_config(cx);
    }

    fn import_terminal_background(&mut self, cx: &mut Context<Self>) {
        cx.spawn(
            |workspace: gpui::WeakEntity<WorkspacePrototype>, cx: &mut gpui::AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    let Some(file) = rfd::AsyncFileDialog::new()
                        .add_filter("Images", &["png", "jpg", "jpeg", "bmp", "webp", "gif"])
                        .pick_file()
                        .await
                    else {
                        return;
                    };

                    let import_result = crate::theme_store::import_background(file.path())
                        .and_then(|saved_path| gpui_terminal_background_reference(&saved_path));

                    let _ = workspace.update(&mut cx, |workspace, cx| match import_result {
                        Ok(reference) => {
                            workspace.terminal_background_import_error = None;
                            workspace.appearance_config.effects.enabled = true;
                            workspace.appearance_config.effects.background = "image".to_string();
                            workspace.appearance_config.effects.background_image = Some(reference);
                            workspace.apply_appearance_config(cx);
                        }
                        Err(error) => {
                            workspace.terminal_background_import_error = Some(error);
                            cx.notify();
                        }
                    });
                }
            },
        )
        .detach();
    }

    fn clear_terminal_background_image(&mut self, cx: &mut Context<Self>) {
        self.terminal_background_import_error = None;
        self.appearance_config.effects.background_image = None;
        if self.appearance_config.effects.background == "image" {
            self.appearance_config.effects.background = "none".to_string();
        }
        self.apply_appearance_config(cx);
    }

    fn set_background_image_fit(&mut self, fit: BackgroundImageFit, cx: &mut Context<Self>) {
        self.appearance_config.effects.background_image_fit = fit;
        self.apply_appearance_config(cx);
    }

    fn toggle_bloom(&mut self, cx: &mut Context<Self>) {
        self.appearance_config.effects.bloom_enabled =
            !self.appearance_config.effects.bloom_enabled;
        self.apply_appearance_config(cx);
    }

    fn toggle_crt(&mut self, cx: &mut Context<Self>) {
        self.appearance_config.effects.crt_enabled = !self.appearance_config.effects.crt_enabled;
        self.apply_appearance_config(cx);
    }

    fn toggle_particles(&mut self, cx: &mut Context<Self>) {
        self.appearance_config.effects.particles_enabled =
            !self.appearance_config.effects.particles_enabled;
        self.apply_appearance_config(cx);
    }

    fn toggle_cursor_glow(&mut self, cx: &mut Context<Self>) {
        self.appearance_config.effects.cursor_glow = !self.appearance_config.effects.cursor_glow;
        self.apply_appearance_config(cx);
    }

    fn toggle_cursor_trail(&mut self, cx: &mut Context<Self>) {
        self.appearance_config.effects.cursor_trail = !self.appearance_config.effects.cursor_trail;
        self.apply_appearance_config(cx);
    }

    fn toggle_text_animation(&mut self, cx: &mut Context<Self>) {
        self.appearance_config.effects.text_animation =
            !self.appearance_config.effects.text_animation;
        self.apply_appearance_config(cx);
    }

    fn toggle_effects_enabled(&mut self, cx: &mut Context<Self>) {
        self.appearance_config.effects.enabled = !self.appearance_config.effects.enabled;
        self.apply_appearance_config(cx);
    }

    fn adjust_editor_font_size(&mut self, delta: f32, cx: &mut Context<Self>) {
        let current = self
            .appearance_config
            .editor
            .font_size
            .unwrap_or((self.appearance_config.font_size - 2.0).max(10.0));
        self.appearance_config.editor.font_size = Some((current + delta).clamp(8.0, 28.0));
        self.apply_appearance_config(cx);
    }

    fn adjust_sidebar_font_size(&mut self, delta: f32, cx: &mut Context<Self>) {
        self.appearance_config.editor.sidebar_font_size =
            (self.appearance_config.editor.sidebar_font_size + delta).clamp(8.0, 24.0);
        cx.notify();
    }

    fn adjust_selection_alpha(&mut self, delta: f32, cx: &mut Context<Self>) {
        self.appearance_config.colors.selection_alpha =
            (self.appearance_config.colors.selection_alpha + delta).clamp(0.05, 1.0);
        self.apply_appearance_config(cx);
    }

    fn toggle_time_of_day(&mut self, cx: &mut Context<Self>) {
        self.appearance_config.time_of_day_enabled = !self.appearance_config.time_of_day_enabled;
        cx.notify();
    }

    fn activate_relative_tab(
        &mut self,
        offset: isize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.tabs.is_empty() {
            return;
        }
        let current = self
            .tabs
            .iter()
            .position(|tab| tab.id == self.active_tab_id)
            .unwrap_or(0);
        let len = self.tabs.len() as isize;
        let next = (current as isize + offset).rem_euclid(len) as usize;
        let tab_id = self.tabs[next].id;
        self.activate_tab(tab_id, window, cx);
    }

    fn menu_new_tab(&mut self, _: &MenuNewTab, window: &mut Window, cx: &mut Context<Self>) {
        self.open_or_activate_surface(WorkspaceSurface::Terminal, window, cx);
    }

    fn menu_close_tab(&mut self, _: &MenuCloseTab, window: &mut Window, cx: &mut Context<Self>) {
        self.close_tab(self.active_tab_id, window, cx);
    }

    fn menu_next_tab(&mut self, _: &MenuNextTab, window: &mut Window, cx: &mut Context<Self>) {
        self.activate_relative_tab(1, window, cx);
    }

    fn menu_previous_tab(
        &mut self,
        _: &MenuPreviousTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.activate_relative_tab(-1, window, cx);
    }

    fn menu_join_tabs(&mut self, _: &MenuJoinTabs, window: &mut Window, cx: &mut Context<Self>) {
        if self.open_active_tab_context_menu(window, cx) {
            self.show_tab_join_targets(cx);
        }
    }

    fn menu_separate_tabs(
        &mut self,
        _: &MenuSeparateTabs,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.separate_tab_by_id(self.active_tab_id, cx);
    }

    fn menu_swap_tabs(&mut self, _: &MenuSwapTabs, _window: &mut Window, cx: &mut Context<Self>) {
        self.swap_tabs_by_id(self.active_tab_id, cx);
    }

    fn menu_open_project(&mut self, _: &MenuOpenProject, _: &mut Window, cx: &mut Context<Self>) {
        self.pick_open_project(cx);
    }

    fn menu_close_project(&mut self, _: &MenuCloseProject, _: &mut Window, cx: &mut Context<Self>) {
        self.close_project(cx);
    }

    fn menu_save(&mut self, _: &MenuSave, _: &mut Window, cx: &mut Context<Self>) {
        match self.active_surface() {
            WorkspaceSurface::Editor => {
                self.editor
                    .update(cx, |editor, cx| editor.save_active_buffer(cx));
            }
            WorkspaceSurface::Sketch => {
                self.sketch
                    .update(cx, |sketch, cx| sketch.save_from_workspace(cx));
            }
            _ => {}
        }
    }

    fn menu_undo(&mut self, _: &MenuUndo, _: &mut Window, cx: &mut Context<Self>) {
        match self.active_surface() {
            WorkspaceSurface::Editor => {
                self.editor.update(cx, |editor, cx| editor.undo_edit(cx));
            }
            WorkspaceSurface::Sketch => {
                self.sketch
                    .update(cx, |sketch, cx| sketch.undo_from_workspace(cx));
            }
            _ => {}
        }
    }

    fn menu_redo(&mut self, _: &MenuRedo, _: &mut Window, cx: &mut Context<Self>) {
        match self.active_surface() {
            WorkspaceSurface::Editor => {
                self.editor.update(cx, |editor, cx| editor.redo_edit(cx));
            }
            WorkspaceSurface::Sketch => {
                self.sketch
                    .update(cx, |sketch, cx| sketch.redo_from_workspace(cx));
            }
            _ => {}
        }
    }

    fn menu_copy(&mut self, _: &MenuCopy, _: &mut Window, cx: &mut Context<Self>) {
        if self.active_surface() == WorkspaceSurface::Editor {
            self.editor
                .update(cx, |editor, cx| editor.copy_selection_to_clipboard(cx));
        }
    }

    fn menu_paste(&mut self, _: &MenuPaste, _: &mut Window, cx: &mut Context<Self>) {
        if self.active_surface() == WorkspaceSurface::Editor {
            self.editor
                .update(cx, |editor, cx| editor.paste_from_clipboard(cx));
        }
    }

    fn menu_select_all(&mut self, _: &MenuSelectAll, _: &mut Window, cx: &mut Context<Self>) {
        if self.active_surface() == WorkspaceSurface::Editor {
            self.editor
                .update(cx, |editor, cx| editor.select_all_text(cx));
        }
    }

    fn menu_find(&mut self, _: &MenuFind, window: &mut Window, cx: &mut Context<Self>) {
        self.open_or_activate_surface(WorkspaceSurface::Editor, window, cx);
        self.editor
            .update(cx, |editor, cx| editor.open_find_from_workspace(window, cx));
    }

    fn menu_toggle_sidebar(
        &mut self,
        _: &MenuToggleSidebar,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_sidebar(cx);
    }

    fn menu_show_surface(
        &mut self,
        surface: WorkspaceSurface,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_or_activate_surface(surface, window, cx);
    }

    fn menu_show_home(&mut self, _: &MenuShowHome, window: &mut Window, cx: &mut Context<Self>) {
        self.menu_show_surface(WorkspaceSurface::Home, window, cx);
    }

    fn menu_show_terminal(
        &mut self,
        _: &MenuShowTerminal,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.menu_show_surface(WorkspaceSurface::Terminal, window, cx);
    }

    fn menu_show_stacker(
        &mut self,
        _: &MenuShowStacker,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.menu_show_surface(WorkspaceSurface::Stacker, window, cx);
    }

    fn menu_show_editor(
        &mut self,
        _: &MenuShowEditor,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.menu_show_surface(WorkspaceSurface::Editor, window, cx);
    }

    fn menu_show_sketch(
        &mut self,
        _: &MenuShowSketch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.menu_show_surface(WorkspaceSurface::Sketch, window, cx);
    }

    fn menu_show_appearances(
        &mut self,
        _: &MenuShowAppearances,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.menu_show_surface(WorkspaceSurface::Appearances, window, cx);
    }

    fn menu_zoom_in(&mut self, _: &MenuZoomIn, _: &mut Window, cx: &mut Context<Self>) {
        self.adjust_font_size(1.0, cx);
    }

    fn menu_zoom_out(&mut self, _: &MenuZoomOut, _: &mut Window, cx: &mut Context<Self>) {
        self.adjust_font_size(-1.0, cx);
    }

    fn menu_zoom_reset(&mut self, _: &MenuZoomReset, _: &mut Window, cx: &mut Context<Self>) {
        self.appearance_config.font_size = Config::default().font_size;
        self.apply_appearance_config(cx);
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
        let tab_name_overrides = self.tab_name_overrides.clone();
        let tab_rename = self.tab_rename.clone();
        let queued_prompts = load_workspace_queue();
        let appearance_config = self.appearance_config.clone();
        let appearance_page = self.appearance_page;
        let terminal_background_import_error = self.terminal_background_import_error.clone();
        let explorer_entries = self
            .workspace_root
            .as_ref()
            .map(|root| collect_explorer_entries(root, &self.expanded_dirs))
            .unwrap_or_default();
        let selected_path = self.selected_path.clone();
        let editor_active_path = self.editor.read(cx).active_path();
        let workspace_root = self.workspace_root.clone();
        let recent_projects = self.recent_projects.clone();
        let recent_projects_open = self.recent_projects_open;
        let sidebar_width = self.sidebar_width;
        let sidebar_visible = self.sidebar_visible;
        let explorer_status = self.explorer_status.clone();

        let mut main = div().flex_1().flex().overflow_hidden();
        if sidebar_visible {
            main = main.child(workspace_sidebar(
                workspace_root.clone(),
                explorer_entries,
                selected_path.clone(),
                recent_projects.clone(),
                recent_projects_open,
                sidebar_width,
                explorer_status.clone(),
                cx,
            ));
        }
        main = main
            .child(sidebar_bumper(sidebar_visible, cx))
            .child(workspace_content(
                self.stacker.clone(),
                self.editor.clone(),
                self.terminal.clone(),
                self.sketch.clone(),
                active_surface,
                joined_panes,
                workspace_root,
                recent_projects,
                appearance_config,
                appearance_page,
                terminal_background_import_error,
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
                    this.close_tab_context_menu(cx);
                }),
            )
            .on_key_down(cx.listener(Self::on_workspace_key_down))
            .on_action(cx.listener(Self::menu_new_tab))
            .on_action(cx.listener(Self::menu_close_tab))
            .on_action(cx.listener(Self::menu_next_tab))
            .on_action(cx.listener(Self::menu_previous_tab))
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
                selected_path.clone(),
                editor_active_path,
                tab_name_overrides,
                &self.tab_manager,
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
    }
}

fn workspace_tab_bar(
    tabs: Vec<WorkspaceTab>,
    active_tab_id: WorkspaceTabId,
    selected_path: Option<PathBuf>,
    editor_active_path: Option<PathBuf>,
    tab_name_overrides: BTreeMap<u64, String>,
    tab_manager: &GpuiTabManager,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let layouts = workspace_tab_layouts(&tabs, active_tab_id);
    let mut bar = div()
        .h(px(TAB_BAR_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .py_1()
        .border_b_1()
        .border_color(rgb(BORDER))
        .bg(rgb(CHROME_BG));

    for tab in tabs.iter().cloned() {
        let label = tab_name_overrides
            .get(&tab.id.0)
            .cloned()
            .unwrap_or_else(|| {
                workspace_tab_label(
                    tab.surface,
                    selected_path.as_deref(),
                    editor_active_path.as_deref(),
                )
            });
        let menu_anchor =
            workspace_tab_menu_anchor(tab.id, &tabs, active_tab_id, tab_manager, &layouts);
        bar = bar.child(workspace_tab(
            tab.clone(),
            active_tab_id,
            label,
            tab_manager.is_joined(tab.id.0),
            menu_anchor,
            cx,
        ));
    }

    bar.child(
        div()
            .ml_2()
            .rounded_sm()
            .border_1()
            .border_color(rgb(0x325c44))
            .bg(rgb(0x102c20))
            .px_2()
            .py_1()
            .text_size(px(11.0))
            .text_color(rgb(QUEUE_GREEN))
            .child("GPUI"),
    )
}

fn workspace_tab_layouts(
    tabs: &[WorkspaceTab],
    active_tab_id: WorkspaceTabId,
) -> Vec<WorkspaceTabLayout> {
    let mut x = TAB_BAR_PADDING_X;
    tabs.iter()
        .map(|tab| {
            let width = workspace_tab_width(tab.surface, tab.id == active_tab_id);
            let layout = WorkspaceTabLayout {
                id: tab.id,
                x,
                width,
            };
            x += width + TAB_BAR_GAP;
            layout
        })
        .collect()
}

fn workspace_tab_menu_anchor(
    tab_id: WorkspaceTabId,
    tabs: &[WorkspaceTab],
    active_tab_id: WorkspaceTabId,
    tab_manager: &GpuiTabManager,
    layouts: &[WorkspaceTabLayout],
) -> WorkspaceTabMenuAnchor {
    let valid_tabs = tabs.iter().map(|tab| tab.id.0).collect::<Vec<_>>();
    let current = layouts
        .iter()
        .find(|layout| layout.id == tab_id)
        .copied()
        .unwrap_or(WorkspaceTabLayout {
            id: tab_id,
            x: TAB_BAR_PADDING_X,
            width: workspace_tab_width(WorkspaceSurface::Home, tab_id == active_tab_id),
        });

    if let Some(joined) = tab_manager.joined_pair_for(tab_id.0, &valid_tabs) {
        let primary = layouts
            .iter()
            .find(|layout| layout.id.0 == joined.primary)
            .copied();
        let secondary = layouts
            .iter()
            .find(|layout| layout.id.0 == joined.secondary)
            .copied();
        if let (Some(primary), Some(secondary)) = (primary, secondary) {
            let left = primary.x.min(secondary.x);
            let right = (primary.x + primary.width).max(secondary.x + secondary.width);
            return WorkspaceTabMenuAnchor {
                x: left,
                y: TAB_BAR_PADDING_Y + TAB_HEIGHT,
                width: right - left,
            };
        }
    }

    WorkspaceTabMenuAnchor {
        x: current.x,
        y: TAB_BAR_PADDING_Y + TAB_HEIGHT,
        width: current.width,
    }
}

fn workspace_tab(
    tab: WorkspaceTab,
    active_tab_id: WorkspaceTabId,
    label: String,
    joined: bool,
    menu_anchor: WorkspaceTabMenuAnchor,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let active = tab.id == active_tab_id;
    let tab_id = tab.id;
    div()
        .w(px(workspace_tab_width(tab.surface, active)))
        .h(px(TAB_HEIGHT))
        .flex()
        .items_center()
        .justify_between()
        .px_3()
        .rounded_sm()
        .border_1()
        .border_color(rgb(if joined { 0x325c44 } else { BORDER }))
        .bg(rgb(if active {
            ACTIVE_TAB_BG
        } else {
            INACTIVE_TAB_BG
        }))
        .text_color(rgb(if active { ACTIVE_TEXT } else { MUTED_TEXT }))
        .text_size(px(14.0))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                this.activate_tab(tab_id, window, cx);
            }),
        )
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                this.open_tab_context_menu(tab_id, menu_anchor, window, cx);
            }),
        )
        .child(label)
        .child(
            div()
                .w(px(18.0))
                .h(px(18.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded_sm()
                .text_size(px(13.0))
                .text_color(rgb(if active { 0xc8c8d2 } else { 0x646973 }))
                .cursor_pointer()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                        cx.stop_propagation();
                        this.close_tab(tab_id, window, cx);
                    }),
                )
                .child("x"),
        )
}

fn workspace_tab_label(
    surface: WorkspaceSurface,
    selected_path: Option<&Path>,
    editor_active_path: Option<&Path>,
) -> String {
    if surface == WorkspaceSurface::Editor {
        return editor_active_path
            .or(selected_path)
            .map(project_display_name)
            .unwrap_or_else(|| surface.title().to_string());
    }
    surface.title().to_string()
}

fn workspace_tab_width(surface: WorkspaceSurface, active: bool) -> f32 {
    match (surface, active) {
        (WorkspaceSurface::Editor, true) => 184.0,
        (WorkspaceSurface::Editor, false) => 170.0,
        (WorkspaceSurface::Sketch, true) => 150.0,
        (WorkspaceSurface::Sketch, false) => 132.0,
        (WorkspaceSurface::Appearances, true) => 156.0,
        (WorkspaceSurface::Appearances, false) => 142.0,
        (WorkspaceSurface::Settings, _) => 128.0,
        (WorkspaceSurface::Home, _) => 104.0,
        (_, true) => 140.0,
        (_, false) => 120.0,
    }
}

fn workspace_tab_context_menu(
    menu: GpuiTabContextMenu,
    tabs: Vec<GpuiTabChoice>,
    tab_manager: &GpuiTabManager,
    tab_rename: Option<TabRenameState>,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let tab_id = WorkspaceTabId(menu.tab_id);
    let tab_title = tabs
        .iter()
        .find(|tab| tab.id == menu.tab_id)
        .map(|tab| tab.title.clone())
        .unwrap_or_else(|| "Tab".to_string());
    let joined = tab_manager.is_joined(menu.tab_id);
    let menu_width = menu.width;

    let mut menu_panel = div()
        .w(px(menu_width))
        .flex()
        .flex_col()
        .gap_1()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x454a56))
        .bg(rgb(0x202229))
        .p_1()
        .text_size(px(14.0))
        .shadow_lg()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_this, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
            }),
        );

    match menu.view {
        GpuiTabContextMenuView::Rename => {
            let rename_text = tab_rename
                .as_ref()
                .filter(|rename| rename.tab_id == tab_id)
                .map(|rename| rename.text.clone())
                .unwrap_or(tab_title);
            let replace_on_input = tab_rename
                .as_ref()
                .filter(|rename| rename.tab_id == tab_id)
                .is_some_and(|rename| rename.replace_on_input);
            let field_text = if rename_text.is_empty() {
                "Type name".to_string()
            } else {
                rename_text
            };
            let field_color = if field_text == "Type name" {
                MUTED_TEXT
            } else {
                ACTIVE_TEXT
            };
            menu_panel = menu_panel
                .child(
                    div()
                        .w_full()
                        .h(px(TAB_MENU_ITEM_HEIGHT))
                        .flex()
                        .items_center()
                        .rounded_sm()
                        .border_1()
                        .border_color(rgb(0x4f5666))
                        .bg(rgb(if replace_on_input { 0x253044 } else { 0x15171d }))
                        .px_2()
                        .text_size(px(14.0))
                        .text_color(rgb(field_color))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .child(field_text),
                )
                .child(tab_menu_button(
                    "Save".to_string(),
                    false,
                    cx,
                    move |this, _window, cx| {
                        this.commit_tab_rename(cx);
                    },
                ))
                .child(tab_menu_button(
                    "Cancel".to_string(),
                    false,
                    cx,
                    move |this, _window, cx| {
                        this.cancel_tab_rename(cx);
                    },
                ));
        }
        GpuiTabContextMenuView::Main | GpuiTabContextMenuView::JoinTargets => {
            let join_targets_active = menu.view == GpuiTabContextMenuView::JoinTargets;
            menu_panel = menu_panel.child(tab_menu_button(
                "Edit name".to_string(),
                false,
                cx,
                move |this, window, cx| {
                    this.start_tab_rename(tab_id, window, cx);
                },
            ));
            if joined {
                menu_panel = menu_panel
                    .child(tab_menu_button(
                        "Swap Tabs".to_string(),
                        false,
                        cx,
                        move |this, _window, cx| {
                            this.swap_tabs_by_id(tab_id, cx);
                        },
                    ))
                    .child(tab_menu_button(
                        "Separate Tabs".to_string(),
                        false,
                        cx,
                        move |this, _window, cx| {
                            this.separate_tab_by_id(tab_id, cx);
                        },
                    ));
            } else {
                menu_panel = menu_panel.child(tab_menu_button(
                    "Join Tab".to_string(),
                    join_targets_active,
                    cx,
                    move |this, _window, cx| {
                        this.show_tab_join_targets(cx);
                    },
                ));
            }
        }
    }

    let mut menu_root = div()
        .absolute()
        .left(px(menu.x))
        .top(px(menu.y))
        .flex()
        .items_start()
        .gap_1()
        .child(menu_panel);

    if !joined && menu.view == GpuiTabContextMenuView::JoinTargets {
        menu_root = menu_root.child(tab_join_side_menu(
            menu_width.max(180.0),
            tab_id,
            tab_manager.join_choices(&tabs, menu.tab_id),
            cx,
        ));
    }

    menu_root
}

fn tab_menu_button(
    label: String,
    active: bool,
    cx: &mut Context<WorkspacePrototype>,
    on_click: impl Fn(&mut WorkspacePrototype, &mut Window, &mut Context<WorkspacePrototype>) + 'static,
) -> impl IntoElement {
    div()
        .w_full()
        .h(px(TAB_MENU_ITEM_HEIGHT))
        .flex()
        .items_center()
        .rounded_sm()
        .bg(rgb(if active { 0x303644 } else { 0x202229 }))
        .px_2()
        .text_size(px(14.0))
        .text_color(rgb(SIDEBAR_TEXT))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0x303644)))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                on_click(this, window, cx);
            }),
        )
        .child(label)
}

fn tab_join_side_menu(
    width: f32,
    source_id: WorkspaceTabId,
    targets: Vec<GpuiTabChoice>,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let mut side_menu = div()
        .w(px(width))
        .flex()
        .flex_col()
        .gap_1()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x454a56))
        .bg(rgb(0x202229))
        .p_1()
        .text_size(px(14.0))
        .shadow_lg()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_this, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
            }),
        );

    if targets.is_empty() {
        return side_menu
            .child(
                div()
                    .w_full()
                    .h(px(TAB_MENU_ITEM_HEIGHT))
                    .flex()
                    .items_center()
                    .rounded_sm()
                    .px_2()
                    .text_size(px(14.0))
                    .text_color(rgb(MUTED_TEXT))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .child("No available tabs"),
            )
            .into_any_element();
    }

    for target in targets {
        let target_id = WorkspaceTabId(target.id);
        side_menu = side_menu.child(tab_menu_button(
            target.title,
            false,
            cx,
            move |this, _window, cx| {
                this.join_tabs_by_id(source_id, target_id, cx);
            },
        ));
    }

    side_menu.into_any_element()
}

fn workspace_sidebar(
    workspace_root: Option<PathBuf>,
    entries: Vec<ExplorerEntry>,
    selected_path: Option<PathBuf>,
    recent_projects: Vec<PathBuf>,
    recent_projects_open: bool,
    sidebar_width: f32,
    explorer_status: Option<String>,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let root_label = workspace_root
        .as_ref()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("No Project")
        .to_string();
    let has_project = workspace_root.is_some();

    let mut header = div()
        .h(px(36.0))
        .px_2()
        .flex()
        .items_center()
        .justify_between()
        .border_b_1()
        .border_color(rgb(0x343743))
        .text_size(px(13.0))
        .text_color(rgb(ACTIVE_TEXT))
        .child(
            div().flex().items_center().gap_2().child("FILES").child(
                div()
                    .rounded_sm()
                    .bg(rgb(0x303440))
                    .px_1()
                    .text_size(px(10.0))
                    .text_color(rgb(MUTED_TEXT))
                    .child(root_label),
            ),
        )
        .child(sidebar_close_project_button(has_project, cx));

    if let Some(root) = workspace_root.clone() {
        let drop_target = root.clone();
        header = header
            .drag_over::<ExplorerDragPayload>(move |style, payload, _window, _cx| {
                style.bg(rgb(explorer_drop_background(payload, &drop_target)))
            })
            .on_drop(
                cx.listener(move |this, payload: &ExplorerDragPayload, _window, cx| {
                    this.move_explorer_items_to_folder(payload.paths.clone(), root.clone(), cx);
                }),
            );
    }

    let mut sidebar = div()
        .w(px(sidebar_width))
        .h_full()
        .flex()
        .flex_col()
        .border_r_1()
        .border_color(rgb(BORDER))
        .bg(rgb(CHROME_BG))
        .child(header)
        .child(sidebar_project_controls(
            recent_projects,
            recent_projects_open,
            cx,
        ));

    let mut tree = div()
        .id("workspace-sidebar-tree")
        .flex_1()
        .flex()
        .flex_col()
        .overflow_y_scroll()
        .scrollbar_width(px(8.0))
        .py_1();
    if !has_project {
        tree = tree.child(
            div()
                .p_3()
                .text_size(px(12.0))
                .text_color(rgb(MUTED_TEXT))
                .child("Open a project to show its files."),
        );
    } else if entries.is_empty() {
        tree = tree.child(
            div()
                .p_3()
                .text_size(px(12.0))
                .text_color(rgb(MUTED_TEXT))
                .child("No readable project files."),
        );
    } else {
        for entry in entries {
            let selected = selected_path.as_ref() == Some(&entry.path);
            tree = tree.child(sidebar_tree_row(entry, selected, cx));
        }
    }

    sidebar = sidebar.child(tree).child(
        div()
            .h(px(28.0))
            .px_2()
            .flex()
            .items_center()
            .border_t_1()
            .border_color(rgb(0x343743))
            .text_size(px(11.0))
            .text_color(rgb(MUTED_TEXT))
            .child(explorer_status.unwrap_or_else(|| format!("{}px", sidebar_width.round()))),
    );
    sidebar
}

fn sidebar_close_project_button(
    has_project: bool,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let button = div()
        .w(px(22.0))
        .h(px(22.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .text_size(px(12.0))
        .text_color(rgb(if has_project { MUTED_TEXT } else { 0x545965 }))
        .child("x");

    if has_project {
        button.cursor_pointer().on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
                this.close_project(cx);
            }),
        )
    } else {
        button
    }
}

fn sidebar_project_controls(
    recent_projects: Vec<PathBuf>,
    recent_projects_open: bool,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let mut controls = div()
        .flex()
        .flex_col()
        .gap_1()
        .p_2()
        .border_b_1()
        .border_color(rgb(0x343743))
        .child(project_button("Open Project", true, cx, |this, cx| {
            this.pick_open_project(cx);
        }))
        .child(project_button("Open Recent", false, cx, |this, cx| {
            this.toggle_recent_projects(cx);
        }));

    if recent_projects_open {
        let recent = recent_projects.into_iter().take(5).collect::<Vec<_>>();
        if recent.is_empty() {
            controls = controls.child(
                div()
                    .px_2()
                    .py_1()
                    .text_size(px(12.0))
                    .text_color(rgb(MUTED_TEXT))
                    .child("No recent projects"),
            );
        } else {
            for project in recent {
                controls = controls.child(recent_project_row(project, cx));
            }
        }
    }

    controls
}

fn project_button(
    label: &'static str,
    primary: bool,
    cx: &mut Context<WorkspacePrototype>,
    on_click: impl Fn(&mut WorkspacePrototype, &mut Context<WorkspacePrototype>) + 'static,
) -> impl IntoElement {
    div()
        .w_full()
        .h(px(30.0))
        .flex()
        .items_center()
        .px_2()
        .rounded_sm()
        .bg(rgb(if primary { ACCENT } else { 0x303440 }))
        .text_color(rgb(0xe1e6ee))
        .text_size(px(13.0))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                on_click(this, cx);
            }),
        )
        .child(label)
}

fn recent_project_row(project: PathBuf, cx: &mut Context<WorkspacePrototype>) -> impl IntoElement {
    let label = project_display_name(&project);
    let path = project;
    div()
        .w_full()
        .h(px(26.0))
        .flex()
        .items_center()
        .px_2()
        .rounded_sm()
        .text_size(px(12.0))
        .text_color(rgb(SIDEBAR_TEXT))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                this.open_project(path.clone(), cx);
            }),
        )
        .child(label)
}

fn project_display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Project")
        .to_string()
}

fn sidebar_tree_row(
    entry: ExplorerEntry,
    selected: bool,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let depth = entry.depth;
    let name = entry.name.clone();
    let path = entry.path.clone();
    let is_dir = entry.is_dir;
    let row_id = explorer_row_id(&path);
    let drag_payload = ExplorerDragPayload::new(path.clone(), name.clone(), is_dir);
    let icon = if is_dir && entry.expanded {
        "v"
    } else if is_dir {
        ">"
    } else {
        " "
    };

    let mut row = div()
        .id(("workspace-sidebar-row", row_id))
        .w_full()
        .h(px(26.0))
        .flex()
        .items_center()
        .pl(px(8.0 + depth as f32 * 14.0))
        .pr_2()
        .rounded_sm()
        .bg(rgb(if selected {
            SIDEBAR_ROW_SELECTED_BG
        } else {
            SIDEBAR_ROW_BG
        }))
        .hover(move |style| {
            style.bg(rgb(if selected {
                SIDEBAR_ROW_SELECTED_HOVER_BG
            } else {
                SIDEBAR_ROW_HOVER_BG
            }))
        })
        .text_size(px(13.0))
        .text_color(rgb(if is_dir { FOLDER_BLUE } else { SIDEBAR_TEXT }))
        .cursor_move()
        .on_drag(
            drag_payload,
            |payload: &ExplorerDragPayload, _offset, _window, cx| {
                cx.new(|_| ExplorerDragPreview::new(payload))
            },
        )
        .on_click(cx.listener({
            let path = path.clone();
            move |this, _: &ClickEvent, window, cx| {
                if is_dir {
                    this.toggle_explorer_dir(path.clone(), cx);
                } else {
                    this.open_sidebar_file(path.clone(), window, cx);
                }
            }
        }));

    if is_dir {
        let drop_target = path.clone();
        row = row
            .drag_over::<ExplorerDragPayload>(move |style, payload, _window, _cx| {
                style.bg(rgb(explorer_drop_background(payload, &drop_target)))
            })
            .on_drop(
                cx.listener(move |this, payload: &ExplorerDragPayload, _window, cx| {
                    this.move_explorer_items_to_folder(payload.paths.clone(), path.clone(), cx);
                }),
            );
    }

    row.child(
        div()
            .w(px(16.0))
            .text_size(px(11.0))
            .text_color(rgb(if is_dir { FOLDER_BLUE } else { 0x646973 }))
            .child(icon),
    )
    .child(
        div()
            .flex_1()
            .overflow_hidden()
            .whitespace_nowrap()
            .child(name),
    )
}

fn explorer_drop_background(payload: &ExplorerDragPayload, destination_folder: &Path) -> u32 {
    if explorer_drop_is_valid(payload, destination_folder) {
        SIDEBAR_DROP_VALID_BG
    } else {
        SIDEBAR_DROP_INVALID_BG
    }
}

fn explorer_drop_is_valid(payload: &ExplorerDragPayload, destination_folder: &Path) -> bool {
    let request = SidebarMoveRequest::new(
        payload.paths.clone(),
        destination_folder.to_path_buf(),
        MoveOrigin::DragDrop,
    );
    plan_sidebar_move(&request).is_ok()
}

fn explorer_row_id(path: &Path) -> u64 {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

fn sidebar_bumper(sidebar_visible: bool, cx: &mut Context<WorkspacePrototype>) -> impl IntoElement {
    div()
        .id("workspace-sidebar-bumper")
        .w(px(BUMPER_WIDTH))
        .h_full()
        .flex()
        .justify_between()
        .gap_0()
        .bg(rgb(BUMPER_BG))
        .border_r_1()
        .border_color(rgb(BORDER))
        .child(
            div()
                .id("workspace-sidebar-resize-handle")
                .w(px(BUMPER_RESIZE_WIDTH))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .cursor_col_resize()
                .on_drag(
                    SidebarResizeDrag,
                    |_drag, _offset, _window, cx: &mut App| cx.new(|_| SidebarResizeDrag),
                )
                .on_drag_move::<SidebarResizeDrag>(cx.listener(
                    |this, event: &DragMoveEvent<SidebarResizeDrag>, _window, cx| {
                        this.resize_sidebar_from_x(event.event.position.x, cx);
                    },
                ))
                .child(div().w(px(2.0)).h(px(40.0)).rounded_sm().bg(rgb(0x343743))),
        )
        .child(
            div()
                .flex_1()
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(rgb(if sidebar_visible {
                    0x787d8c
                } else {
                    QUEUE_GREEN
                }))
                .text_size(px(14.0))
                .cursor_pointer()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _: &MouseDownEvent, _window, cx| {
                        cx.stop_propagation();
                        this.toggle_sidebar(cx);
                    }),
                )
                .child(if sidebar_visible { "<" } else { ">" }),
        )
}

fn initial_expanded_dirs(root: &Path) -> BTreeSet<PathBuf> {
    let mut expanded = BTreeSet::new();
    expanded.insert(root.to_path_buf());
    for child in ["src", "daily-growth", "docs"] {
        let path = root.join(child);
        if path.is_dir() {
            expanded.insert(path);
        }
    }
    expanded
}

fn collect_explorer_entries(root: &Path, expanded_dirs: &BTreeSet<PathBuf>) -> Vec<ExplorerEntry> {
    let mut entries = Vec::new();
    let mut remaining = EXPLORER_ENTRY_LIMIT;
    collect_explorer_children(root, 0, expanded_dirs, &mut entries, &mut remaining);
    entries
}

fn collect_explorer_children(
    dir: &Path,
    depth: usize,
    expanded_dirs: &BTreeSet<PathBuf>,
    entries: &mut Vec<ExplorerEntry>,
    remaining: &mut usize,
) {
    if *remaining == 0 {
        return;
    }

    let Ok(read_dir) = fs::read_dir(dir) else {
        return;
    };

    let mut children = read_dir
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if should_skip_explorer_entry(&name) {
                return None;
            }
            let is_dir = entry.file_type().map(|file_type| file_type.is_dir()).ok()?;
            Some((name, path, is_dir))
        })
        .collect::<Vec<_>>();

    children.sort_by(
        |(left_name, _, left_is_dir), (right_name, _, right_is_dir)| {
            right_is_dir
                .cmp(left_is_dir)
                .then_with(|| left_name.to_lowercase().cmp(&right_name.to_lowercase()))
        },
    );

    for (name, path, is_dir) in children {
        if *remaining == 0 {
            break;
        }
        let expanded = is_dir && expanded_dirs.contains(&path);
        entries.push(ExplorerEntry {
            path: path.clone(),
            name,
            is_dir,
            depth,
            expanded,
        });
        *remaining -= 1;

        if expanded {
            collect_explorer_children(&path, depth + 1, expanded_dirs, entries, remaining);
        }
    }
}

fn should_skip_explorer_entry(name: &str) -> bool {
    matches!(
        name,
        ".git" | ".DS_Store" | "target" | "node_modules" | ".next" | "dist" | "build"
    )
}

fn remap_path_after_explorer_move(path: &Path, moved: &[SidebarMovePlanItem]) -> Option<PathBuf> {
    for item in moved {
        if item.is_dir {
            if same_path(path, &item.source) {
                return Some(item.destination.clone());
            }
            if path_contains(&item.source, path) {
                let relative = path.strip_prefix(&item.source).ok()?;
                return Some(item.destination.join(relative));
            }
        } else if same_path(path, &item.source) {
            return Some(item.destination.clone());
        }
    }
    None
}

fn workspace_content(
    stacker: Entity<StackerPrototype>,
    editor: Entity<EditorPrototype>,
    terminal: Entity<TerminalSurface>,
    sketch: Entity<SketchSurface>,
    active_surface: WorkspaceSurface,
    joined_panes: Option<JoinedWorkspacePanes>,
    workspace_root: Option<PathBuf>,
    recent_projects: Vec<PathBuf>,
    appearance_config: Config,
    appearance_page: AppearancePage,
    terminal_background_import_error: Option<String>,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let content = div()
        .flex_1()
        .h_full()
        .flex()
        .overflow_hidden()
        .bg(rgb(EDITOR_BG));

    if let Some(joined) = joined_panes {
        let ratio = joined.ratio.clamp(0.18, 0.82);
        return content.child(
            div()
                .flex_1()
                .h_full()
                .flex()
                .overflow_hidden()
                .child(
                    workspace_surface_pane(
                        stacker.clone(),
                        editor.clone(),
                        terminal.clone(),
                        sketch.clone(),
                        joined.primary_surface,
                        Some(joined.primary_id),
                        workspace_root.clone(),
                        recent_projects.clone(),
                        appearance_config.clone(),
                        appearance_page,
                        terminal_background_import_error.clone(),
                        cx,
                    )
                    .w(relative(ratio)),
                )
                .child(
                    div()
                        .w(px(JOINED_TAB_DIVIDER_WIDTH))
                        .h_full()
                        .bg(rgb(0x101012))
                        .border_l_1()
                        .border_r_1()
                        .border_color(rgb(BORDER)),
                )
                .child(
                    workspace_surface_pane(
                        stacker,
                        editor,
                        terminal,
                        sketch,
                        joined.secondary_surface,
                        Some(joined.secondary_id),
                        workspace_root,
                        recent_projects,
                        appearance_config,
                        appearance_page,
                        terminal_background_import_error,
                        cx,
                    )
                    .w(relative(1.0 - ratio)),
                ),
        );
    }

    content.child(
        workspace_surface_pane(
            stacker,
            editor,
            terminal,
            sketch,
            active_surface,
            None,
            workspace_root,
            recent_projects,
            appearance_config,
            appearance_page,
            terminal_background_import_error,
            cx,
        )
        .flex_1(),
    )
}

fn workspace_surface_pane(
    stacker: Entity<StackerPrototype>,
    editor: Entity<EditorPrototype>,
    terminal: Entity<TerminalSurface>,
    sketch: Entity<SketchSurface>,
    surface: WorkspaceSurface,
    tab_id: Option<WorkspaceTabId>,
    workspace_root: Option<PathBuf>,
    recent_projects: Vec<PathBuf>,
    appearance_config: Config,
    appearance_page: AppearancePage,
    terminal_background_import_error: Option<String>,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    let pane = div().h_full().overflow_hidden().bg(rgb(EDITOR_BG));

    let pane = if let Some(tab_id) = tab_id {
        pane.on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                this.activate_tab(tab_id, window, cx);
            }),
        )
    } else {
        pane
    };

    match surface {
        WorkspaceSurface::Stacker => pane.child(
            div()
                .size_full()
                .bg(rgb(PANEL_BG))
                .overflow_hidden()
                .child(stacker),
        ),
        WorkspaceSurface::Editor => pane
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, window, cx| {
                    this.focus_surface(WorkspaceSurface::Editor, window, cx);
                }),
            )
            .child(
                div().size_full().p_4().bg(rgb(EDITOR_BG)).child(
                    div()
                        .size_full()
                        .border_1()
                        .border_color(rgb(BORDER))
                        .bg(rgb(EDITOR_BG))
                        .overflow_hidden()
                        .child(editor),
                ),
            ),
        WorkspaceSurface::Terminal => {
            pane.child(div().size_full().overflow_hidden().child(terminal))
        }
        WorkspaceSurface::Sketch => pane.child(sketch),
        WorkspaceSurface::Appearances => pane.child(appearances_surface(
            appearance_config,
            appearance_page,
            terminal_background_import_error,
            cx,
        )),
        WorkspaceSurface::Home => pane.child(home_surface(workspace_root, recent_projects, cx)),
        WorkspaceSurface::Settings => pane.child(settings_placeholder()),
    }
}

fn appearances_surface(
    config: Config,
    page: AppearancePage,
    terminal_background_import_error: Option<String>,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    div()
        .flex_1()
        .h_full()
        .flex()
        .flex_col()
        .bg(rgb(EDITOR_BG))
        .child(
            div()
                .h(px(52.0))
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .px_4()
                .border_b_1()
                .border_color(rgb(BORDER))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_size(px(18.0))
                                .text_color(rgb(ACTIVE_TEXT))
                                .child("Appearances"),
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(rgb(MUTED_TEXT))
                                .child("Theme, terminal, editor, and canvas presentation"),
                        ),
                )
                .child(appearance_page_nav(page, cx)),
        )
        .child(
            div()
                .flex_1()
                .flex()
                .gap_3()
                .p_4()
                .overflow_hidden()
                .child(appearance_theme_column(&config, cx))
                .child(appearance_controls_column(
                    config,
                    page,
                    terminal_background_import_error,
                    cx,
                )),
        )
}

fn appearance_page_nav(
    page: AppearancePage,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_1()
        .child(appearance_page_button(AppearancePage::Terminal, page, cx))
        .child(appearance_page_button(AppearancePage::Editor, page, cx))
        .child(appearance_page_button(AppearancePage::Sketch, page, cx))
}

fn appearance_page_button(
    target: AppearancePage,
    page: AppearancePage,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let active = target == page;
    appearance_button(target.title().to_string(), active, cx, move |this, cx| {
        this.set_appearance_page(target, cx);
    })
}

fn appearance_theme_column(
    config: &Config,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let mut themes = div()
        .w(px(320.0))
        .h_full()
        .flex()
        .flex_col()
        .gap_2()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(PANEL_BG))
        .p_3()
        .overflow_hidden()
        .child(
            div()
                .text_size(px(13.0))
                .text_color(rgb(MUTED_TEXT))
                .child("THEMES"),
        )
        .child(color_strip([
            config.colors.background,
            config.colors.foreground,
            config.colors.cursor,
            config.colors.selection,
            config.colors.ansi[1],
            config.colors.ansi[2],
            config.colors.ansi[4],
            config.colors.ansi[5],
        ]));

    for theme in builtin_themes().into_iter().take(6) {
        let theme_name = theme.name.clone();
        themes = themes.child(
            div()
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .rounded_sm()
                .bg(rgb(0x252935))
                .p_2()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_size(px(13.0))
                                .text_color(rgb(ACTIVE_TEXT))
                                .child(theme.name.clone()),
                        )
                        .child(color_strip([
                            theme.colors.background,
                            theme.colors.foreground,
                            theme.colors.cursor,
                            theme.colors.ansi[1],
                        ])),
                )
                .child(appearance_button(
                    "Apply".to_string(),
                    false,
                    cx,
                    move |this, cx| {
                        this.apply_builtin_theme(&theme_name, cx);
                    },
                )),
        );
    }

    themes
}

fn appearance_controls_column(
    config: Config,
    page: AppearancePage,
    terminal_background_import_error: Option<String>,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let content = div()
        .flex_1()
        .h_full()
        .flex()
        .flex_col()
        .gap_3()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(0x15151c))
        .p_4()
        .overflow_hidden()
        .child(
            div()
                .text_size(px(18.0))
                .text_color(rgb(ACTIVE_TEXT))
                .child(page.title()),
        );

    match page {
        AppearancePage::Terminal => {
            terminal_appearance_controls(content, config, terminal_background_import_error, cx)
        }
        AppearancePage::Editor => editor_appearance_controls(content, config, cx),
        AppearancePage::Sketch => sketch_appearance_controls(content),
    }
}

fn terminal_appearance_controls(
    content: gpui::Div,
    config: Config,
    terminal_background_import_error: Option<String>,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    content
        .child(metric_row(
            "App Font Size",
            format!("{:.0}px", config.font_size),
            cx,
            |this, cx| this.adjust_font_size(-1.0, cx),
            |this, cx| this.adjust_font_size(1.0, cx),
        ))
        .child(metric_row(
            "Terminal Line Height",
            format!("{:.2}x", config.line_height),
            cx,
            |this, cx| this.adjust_line_height(-0.05, cx),
            |this, cx| this.adjust_line_height(0.05, cx),
        ))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(control_label("Effects"))
                .child(effect_toggle_button(
                    "Terminal",
                    config.effects.enabled,
                    cx,
                    |this, cx| this.toggle_effects_enabled(cx),
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(control_label("Cursor Style"))
                .child(appearance_button(
                    "Block".to_string(),
                    config.cursor_style == CursorStyle::Block,
                    cx,
                    |this, cx| this.set_cursor_style(CursorStyle::Block, cx),
                ))
                .child(appearance_button(
                    "Beam".to_string(),
                    config.cursor_style == CursorStyle::Beam,
                    cx,
                    |this, cx| this.set_cursor_style(CursorStyle::Beam, cx),
                ))
                .child(appearance_button(
                    "Underline".to_string(),
                    config.cursor_style == CursorStyle::Underline,
                    cx,
                    |this, cx| this.set_cursor_style(CursorStyle::Underline, cx),
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(control_label("Background"))
                .child(appearance_button(
                    "None".to_string(),
                    config.effects.background == "none",
                    cx,
                    |this, cx| this.set_background_mode("none", cx),
                ))
                .child(appearance_button(
                    "Smoke".to_string(),
                    config.effects.background == "smoke",
                    cx,
                    |this, cx| this.set_background_mode("smoke", cx),
                ))
                .child(appearance_button(
                    "Aurora".to_string(),
                    config.effects.background == "aurora",
                    cx,
                    |this, cx| this.set_background_mode("aurora", cx),
                ))
                .child(appearance_button(
                    "Image".to_string(),
                    config.effects.background == "image",
                    cx,
                    |this, cx| this.import_terminal_background(cx),
                )),
        )
        .child(terminal_background_image_controls(
            &config,
            terminal_background_import_error,
            cx,
        ))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(control_label("Post Effects"))
                .child(effect_toggle_button(
                    "Bloom",
                    config.effects.bloom_enabled,
                    cx,
                    |this, cx| this.toggle_bloom(cx),
                ))
                .child(effect_toggle_button(
                    "CRT",
                    config.effects.crt_enabled,
                    cx,
                    |this, cx| this.toggle_crt(cx),
                ))
                .child(effect_toggle_button(
                    "Particles",
                    config.effects.particles_enabled,
                    cx,
                    |this, cx| this.toggle_particles(cx),
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(control_label("Text Effects"))
                .child(effect_toggle_button(
                    "Glow",
                    config.effects.cursor_glow,
                    cx,
                    |this, cx| this.toggle_cursor_glow(cx),
                ))
                .child(effect_toggle_button(
                    "Trail",
                    config.effects.cursor_trail,
                    cx,
                    |this, cx| this.toggle_cursor_trail(cx),
                ))
                .child(effect_toggle_button(
                    "Text Anim",
                    config.effects.text_animation,
                    cx,
                    |this, cx| this.toggle_text_animation(cx),
                )),
        )
        .child(metric_row(
            "Selection Alpha",
            format!("{:.0}%", config.colors.selection_alpha * 100.0),
            cx,
            |this, cx| this.adjust_selection_alpha(-0.05, cx),
            |this, cx| this.adjust_selection_alpha(0.05, cx),
        ))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(control_label("Time-of-Day Warmth"))
                .child(appearance_button(
                    if config.time_of_day_enabled {
                        "On".to_string()
                    } else {
                        "Off".to_string()
                    },
                    config.time_of_day_enabled,
                    cx,
                    |this, cx| this.toggle_time_of_day(cx),
                )),
        )
}

fn terminal_background_image_controls(
    config: &Config,
    terminal_background_import_error: Option<String>,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    let current_image = config
        .effects
        .background_image
        .as_deref()
        .map(background_image_display_name)
        .unwrap_or_else(|| "No image selected".to_string());

    let mut controls = div().flex().flex_col().gap_2().child(
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(control_label("Image Background"))
            .child(appearance_button(
                "Import Image".to_string(),
                config.effects.background == "image" && config.effects.background_image.is_some(),
                cx,
                |this, cx| this.import_terminal_background(cx),
            ))
            .child(appearance_button(
                "Clear".to_string(),
                false,
                cx,
                |this, cx| {
                    this.clear_terminal_background_image(cx);
                },
            ))
            .child(
                div()
                    .max_w(px(220.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_size(px(12.0))
                    .text_color(rgb(MUTED_TEXT))
                    .child(current_image),
            ),
    );

    if config.effects.background_image.is_some() || config.effects.background == "image" {
        let mut fit_row = div()
            .flex()
            .items_center()
            .gap_2()
            .child(control_label("Image Fit"));
        for fit in BackgroundImageFit::ALL {
            fit_row = fit_row.child(appearance_button(
                fit.label().to_string(),
                config.effects.background_image_fit == fit,
                cx,
                move |this, cx| this.set_background_image_fit(fit, cx),
            ));
        }
        controls = controls.child(fit_row);
    }

    if let Some(error) = terminal_background_import_error {
        controls = controls.child(
            div()
                .pl(px(150.0))
                .text_size(px(12.0))
                .text_color(rgb(0xff8a7a))
                .child(error),
        );
    }

    controls
}

fn editor_appearance_controls(
    content: gpui::Div,
    config: Config,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    let editor_font = config
        .editor
        .font_size
        .unwrap_or((config.font_size - 2.0).max(10.0));
    content
        .child(metric_row(
            "Editor Font Size",
            format!("{editor_font:.0}px"),
            cx,
            |this, cx| this.adjust_editor_font_size(-1.0, cx),
            |this, cx| this.adjust_editor_font_size(1.0, cx),
        ))
        .child(metric_row(
            "Sidebar Font Size",
            format!("{:.0}px", config.editor.sidebar_font_size),
            cx,
            |this, cx| this.adjust_sidebar_font_size(-1.0, cx),
            |this, cx| this.adjust_sidebar_font_size(1.0, cx),
        ))
        .child(
            div()
                .mt_2()
                .text_size(px(12.0))
                .text_color(rgb(MUTED_TEXT))
                .child("Syntax color editing and dirty-file editor tabs come with the next editor pass."),
        )
}

fn sketch_appearance_controls(content: gpui::Div) -> gpui::Div {
    content
        .child(
            div()
                .text_size(px(13.0))
                .text_color(rgb(SIDEBAR_TEXT))
                .child("Sketch canvas rendering is back on the GPUI path with persisted grid, canvas, and selection settings."),
        )
        .child(
            div()
                .mt_2()
                .text_size(px(12.0))
                .text_color(rgb(MUTED_TEXT))
                .child("The next pass should expose full canvas color, grid spacing, image, symbol, and text editing controls here."),
        )
}

fn metric_row(
    label: &'static str,
    value: String,
    cx: &mut Context<WorkspacePrototype>,
    decrement: impl Fn(&mut WorkspacePrototype, &mut Context<WorkspacePrototype>) + 'static,
    increment: impl Fn(&mut WorkspacePrototype, &mut Context<WorkspacePrototype>) + 'static,
) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .items_center()
        .gap_2()
        .child(control_label(label))
        .child(appearance_button("-".to_string(), false, cx, decrement))
        .child(
            div()
                .w(px(72.0))
                .h(px(30.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded_sm()
                .bg(rgb(0x242632))
                .text_size(px(13.0))
                .text_color(rgb(ACTIVE_TEXT))
                .child(value),
        )
        .child(appearance_button("+".to_string(), false, cx, increment))
}

fn control_label(label: &'static str) -> impl IntoElement {
    div()
        .w(px(150.0))
        .text_size(px(12.0))
        .text_color(rgb(MUTED_TEXT))
        .child(label)
}

fn effect_toggle_button(
    label: &'static str,
    active: bool,
    cx: &mut Context<WorkspacePrototype>,
    on_click: impl Fn(&mut WorkspacePrototype, &mut Context<WorkspacePrototype>) + 'static,
) -> impl IntoElement {
    appearance_button(
        if active {
            format!("{label} On")
        } else {
            format!("{label} Off")
        },
        active,
        cx,
        on_click,
    )
}

fn appearance_button(
    label: String,
    active: bool,
    cx: &mut Context<WorkspacePrototype>,
    on_click: impl Fn(&mut WorkspacePrototype, &mut Context<WorkspacePrototype>) + 'static,
) -> impl IntoElement {
    div()
        .h(px(30.0))
        .flex()
        .items_center()
        .justify_center()
        .px_2()
        .rounded_sm()
        .border_1()
        .border_color(rgb(if active { 0x47785f } else { BORDER }))
        .bg(rgb(if active { 0x183725 } else { 0x242632 }))
        .text_size(px(12.0))
        .text_color(rgb(if active { QUEUE_GREEN } else { SIDEBAR_TEXT }))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                on_click(this, cx);
            }),
        )
        .child(label)
}

fn color_strip<const N: usize>(colors: [[u8; 3]; N]) -> impl IntoElement {
    let mut strip = div().flex().items_center().gap_1();
    for color in colors {
        strip = strip.child(
            div()
                .w(px(16.0))
                .h(px(16.0))
                .rounded_sm()
                .border_1()
                .border_color(rgb(0x000000))
                .bg(rgb(color_u32(color))),
        );
    }
    strip
}

fn color_u32(color: [u8; 3]) -> u32 {
    ((color[0] as u32) << 16) | ((color[1] as u32) << 8) | color[2] as u32
}

fn background_library_reference(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| path.display().to_string())
}

fn gpui_terminal_background_reference(path: &Path) -> Result<String, String> {
    ensure_gpui_safe_background_image(path).map(|path| background_library_reference(&path))
}

fn ensure_gpui_safe_background_image(path: &Path) -> Result<PathBuf, String> {
    let (width, height) = image::image_dimensions(path)
        .map_err(|error| format!("Could not read image size: {error}"))?;
    if width.max(height) <= GPUI_TERMINAL_BACKGROUND_MAX_EDGE {
        return Ok(path.to_path_buf());
    }

    let parent = path
        .parent()
        .ok_or("Background image has no parent directory")?;
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("background");
    let target = parent.join(format!(
        "{stem}-gpui-{}px.png",
        GPUI_TERMINAL_BACKGROUND_MAX_EDGE
    ));
    if target.is_file() && image::image_dimensions(&target).is_ok() {
        return Ok(target);
    }

    let image = image::open(path).map_err(|error| format!("Could not load image: {error}"))?;
    let resized = image.resize(
        GPUI_TERMINAL_BACKGROUND_MAX_EDGE,
        GPUI_TERMINAL_BACKGROUND_MAX_EDGE,
        image::imageops::FilterType::Lanczos3,
    );
    resized
        .save(&target)
        .map_err(|error| format!("Could not create GPUI-safe image: {error}"))?;
    Ok(target)
}

fn background_image_display_name(reference: &str) -> String {
    Path::new(reference)
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| reference.to_string())
}

fn home_surface(
    workspace_root: Option<PathBuf>,
    recent_projects: Vec<PathBuf>,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let mut recent_list = div().flex().flex_col().gap_1().w(px(360.0));
    let recent = recent_projects.into_iter().take(5).collect::<Vec<_>>();
    if recent.is_empty() {
        recent_list = recent_list.child(
            div()
                .py_2()
                .text_size(px(13.0))
                .text_color(rgb(MUTED_TEXT))
                .child("No recent projects"),
        );
    } else {
        for project in recent {
            recent_list = recent_list.child(home_recent_project_row(project, cx));
        }
    }

    let mut content = div()
        .flex_1()
        .h_full()
        .flex()
        .flex_col()
        .items_center()
        .bg(rgb(EDITOR_BG))
        .pt(px(80.0))
        .child(
            div()
                .text_size(px(26.0))
                .text_color(rgb(ACTIVE_TEXT))
                .child("Home"),
        )
        .child(
            div()
                .mt_2()
                .mb_5()
                .text_size(px(13.0))
                .text_color(rgb(MUTED_TEXT))
                .child("Open a project or jump back into a recent workspace."),
        )
        .child(home_open_project_button(cx));

    if let Some(root) = workspace_root {
        content = content.child(
            div()
                .mt_5()
                .w(px(360.0))
                .rounded_sm()
                .border_1()
                .border_color(rgb(BORDER))
                .bg(rgb(PANEL_BG))
                .p_3()
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(rgb(MUTED_TEXT))
                        .child("OPEN PROJECT"),
                )
                .child(
                    div()
                        .mt_1()
                        .text_size(px(15.0))
                        .text_color(rgb(ACTIVE_TEXT))
                        .child(project_display_name(&root)),
                )
                .child(
                    div()
                        .mt_1()
                        .text_size(px(11.0))
                        .text_color(rgb(MUTED_TEXT))
                        .child(root.display().to_string()),
                ),
        );
    }

    content = content
        .child(
            div()
                .mt_6()
                .mb_2()
                .text_size(px(12.0))
                .text_color(rgb(MUTED_TEXT))
                .child("RECENT PROJECTS"),
        )
        .child(recent_list);

    content
}

fn home_open_project_button(cx: &mut Context<WorkspacePrototype>) -> impl IntoElement {
    div()
        .w(px(240.0))
        .h(px(42.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .bg(rgb(ACCENT))
        .text_size(px(15.0))
        .text_color(rgb(ACTIVE_TEXT))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _: &MouseDownEvent, _window, cx| {
                this.pick_open_project(cx);
            }),
        )
        .child("Open Project")
}

fn home_recent_project_row(
    project: PathBuf,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let title = project_display_name(&project);
    let detail = project.display().to_string();
    let path = project;
    div()
        .w_full()
        .rounded_sm()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(PANEL_BG))
        .p_2()
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                this.open_project(path.clone(), cx);
            }),
        )
        .child(
            div()
                .text_size(px(14.0))
                .text_color(rgb(SIDEBAR_TEXT))
                .child(title),
        )
        .child(
            div()
                .mt_1()
                .text_size(px(11.0))
                .text_color(rgb(MUTED_TEXT))
                .child(detail),
        )
}

fn settings_placeholder() -> impl IntoElement {
    div()
        .flex_1()
        .h_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .bg(rgb(EDITOR_BG))
        .child(
            div()
                .text_size(px(15.0))
                .text_color(rgb(ACTIVE_TEXT))
                .child("Settings"),
        )
        .child(
            div()
                .mt_2()
                .text_size(px(12.0))
                .text_color(rgb(MUTED_TEXT))
                .child("The existing settings surface is still preserved on the current app path."),
        )
}

fn workspace_footer(
    active_surface: WorkspaceSurface,
    queued_prompts: Vec<crate::stacker::queue::QueuedPrompt>,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    div()
        .h(px(FOOTER_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .gap_1()
        .px_3()
        .py_1()
        .border_t_1()
        .border_color(rgb(BORDER))
        .bg(rgb(CHROME_BG))
        .child(footer_button(
            "Home",
            WorkspaceSurface::Home,
            active_surface,
            cx,
        ))
        .child(footer_button(
            "Terminal",
            WorkspaceSurface::Terminal,
            active_surface,
            cx,
        ))
        .child(footer_button(
            "Stacker",
            WorkspaceSurface::Stacker,
            active_surface,
            cx,
        ))
        .child(footer_button(
            "Sketch",
            WorkspaceSurface::Sketch,
            active_surface,
            cx,
        ))
        .child(footer_button(
            "Appearances",
            WorkspaceSurface::Appearances,
            active_surface,
            cx,
        ))
        .child(footer_button(
            "Settings",
            WorkspaceSurface::Settings,
            active_surface,
            cx,
        ))
        .child(div().flex_1())
        .child(footer_queue_tray(active_surface, queued_prompts, cx))
}

fn footer_queue_tray(
    active_surface: WorkspaceSurface,
    queued_prompts: Vec<crate::stacker::queue::QueuedPrompt>,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    let mut tray = div()
        .h(px(36.0))
        .flex()
        .items_center()
        .justify_end()
        .gap_1();
    if active_surface != WorkspaceSurface::Terminal || queued_prompts.is_empty() {
        return tray;
    }

    for prompt in queued_prompts {
        tray = tray.child(footer_queue_chip(prompt, cx));
    }
    tray
}

fn footer_queue_chip(
    prompt: crate::stacker::queue::QueuedPrompt,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let preview = crate::stacker::queue::footer_preview(&prompt.text);
    let clipboard_text = crate::stacker::queue::clipboard_markdown(&prompt);
    div()
        .h(px(32.0))
        .max_w(px(178.0))
        .min_w(px(72.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x3fd663))
        .bg(rgb(0x14261b))
        .px_2()
        .text_size(px(12.0))
        .text_color(rgb(QUEUE_GREEN))
        .overflow_hidden()
        .whitespace_nowrap()
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |_this, _: &MouseDownEvent, _window, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(clipboard_text.clone()));
            }),
        )
        .child(preview)
}

fn load_workspace_queue() -> Vec<crate::stacker::queue::QueuedPrompt> {
    let mut queued_prompts = crate::stacker::load_stacker_queue();
    crate::stacker::queue::sanitize_prompt_queue(&mut queued_prompts);
    queued_prompts
}

fn footer_button(
    label: &'static str,
    surface: WorkspaceSurface,
    active_surface: WorkspaceSurface,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let active = surface == active_surface;
    div()
        .h(px(36.0))
        .flex()
        .items_center()
        .px_3()
        .rounded_sm()
        .bg(rgb(if active { ACCENT } else { CHROME_BG }))
        .text_color(rgb(if active { ACTIVE_TEXT } else { SIDEBAR_TEXT }))
        .text_size(px(14.0))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                this.open_or_activate_surface(surface, window, cx);
            }),
        )
        .child(label)
}
