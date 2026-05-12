use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use gpui::prelude::*;
use gpui::{
    actions, div, px, rgb, size, App, Application, Bounds, Context, DragMoveEvent, Entity,
    Focusable, KeyBinding, MouseButton, MouseDownEvent, Pixels, Render, Window, WindowBounds,
    WindowOptions,
};

use crate::gpui_editor::{bind_editor_keys, EditorPrototype};
use crate::gpui_stacker::{bind_stacker_keys, StackerPrototype};
use crate::gpui_terminal::{bind_terminal_keys, TerminalSurface};
use crate::{
    config::{Config, CursorStyle},
    theme::builtin_themes,
};

actions!(workspace_gpui, [Quit]);

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
const FOOTER_HEIGHT: f32 = 48.0;
const SIDEBAR_DEFAULT_WIDTH: f32 = 220.0;
const SIDEBAR_MIN_WIDTH: f32 = 160.0;
const SIDEBAR_MAX_WIDTH: f32 = 380.0;
const BUMPER_WIDTH: f32 = 20.0;
const EXPLORER_ENTRY_LIMIT: usize = 260;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorkspaceSurface {
    Home,
    Stacker,
    Editor,
    Terminal,
    Appearances,
    Settings,
}

impl WorkspaceSurface {
    fn title(self) -> &'static str {
        match self {
            WorkspaceSurface::Home => "Home",
            WorkspaceSurface::Stacker => "Stacker",
            WorkspaceSurface::Editor => "Code Workbench",
            WorkspaceSurface::Terminal => "Terminal",
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

pub fn run_workspace_prototype() {
    Application::new().run(|cx: &mut App| {
        bind_stacker_keys(cx);
        bind_editor_keys(cx);
        bind_terminal_keys(cx);
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
                window.focus(&view.editor.focus_handle(cx));
            })
            .unwrap();
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.activate(true);
    });
}

struct WorkspacePrototype {
    stacker: Entity<StackerPrototype>,
    editor: Entity<EditorPrototype>,
    terminal: Entity<TerminalSurface>,
    tabs: Vec<WorkspaceTab>,
    active_tab_id: WorkspaceTabId,
    next_tab_id: u64,
    workspace_root: PathBuf,
    expanded_dirs: BTreeSet<PathBuf>,
    selected_path: Option<PathBuf>,
    sidebar_visible: bool,
    sidebar_width: f32,
    last_sidebar_width: f32,
    appearance_config: Config,
    appearance_page: AppearancePage,
}

impl WorkspacePrototype {
    fn new(cx: &mut Context<Self>) -> Self {
        let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let tabs = vec![
            WorkspaceTab::new(WorkspaceTabId(1), WorkspaceSurface::Stacker),
            WorkspaceTab::new(WorkspaceTabId(2), WorkspaceSurface::Editor),
            WorkspaceTab::new(WorkspaceTabId(3), WorkspaceSurface::Terminal),
        ];
        let expanded_dirs = initial_expanded_dirs(&workspace_root);
        Self {
            stacker: cx.new(StackerPrototype::embedded),
            editor: cx.new(EditorPrototype::new),
            terminal: cx.new(TerminalSurface::new),
            tabs,
            active_tab_id: WorkspaceTabId(2),
            next_tab_id: 4,
            workspace_root,
            expanded_dirs,
            selected_path: None,
            sidebar_visible: true,
            sidebar_width: SIDEBAR_DEFAULT_WIDTH,
            last_sidebar_width: SIDEBAR_DEFAULT_WIDTH,
            appearance_config: Config::default(),
            appearance_page: AppearancePage::Terminal,
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
            self.focus_surface(tab.surface, window, cx);
            cx.notify();
            return;
        }

        let tab_id = WorkspaceTabId(self.next_tab_id);
        self.next_tab_id += 1;
        self.tabs.push(WorkspaceTab::new(tab_id, surface));
        self.active_tab_id = tab_id;
        self.focus_surface(surface, window, cx);
        cx.notify();
    }

    fn close_tab(&mut self, tab_id: WorkspaceTabId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.tabs.iter().position(|tab| tab.id == tab_id) else {
            return;
        };

        let was_active = self.active_tab_id == tab_id;
        self.tabs.remove(index);
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
        let width = (x / px(1.0) - BUMPER_WIDTH / 2.0).clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH);
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

    fn apply_appearance_config(&mut self, cx: &mut Context<Self>) {
        let config = self.appearance_config.clone();
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
        self.apply_appearance_config(cx);
    }

    fn adjust_editor_font_size(&mut self, delta: f32, cx: &mut Context<Self>) {
        let current = self
            .appearance_config
            .editor
            .font_size
            .unwrap_or((self.appearance_config.font_size - 2.0).max(10.0));
        self.appearance_config.editor.font_size = Some((current + delta).clamp(8.0, 28.0));
        cx.notify();
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
}

impl Render for WorkspacePrototype {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active_surface = self.active_surface();
        let active_tab_id = self.active_tab_id;
        let tabs = self.tabs.clone();
        let appearance_config = self.appearance_config.clone();
        let appearance_page = self.appearance_page;
        let explorer_entries = collect_explorer_entries(&self.workspace_root, &self.expanded_dirs);
        let selected_path = self.selected_path.clone();
        let workspace_root = self.workspace_root.clone();
        let sidebar_width = self.sidebar_width;
        let sidebar_visible = self.sidebar_visible;

        let mut main = div().flex_1().flex().overflow_hidden();
        if sidebar_visible {
            main = main.child(workspace_sidebar(
                workspace_root,
                explorer_entries,
                selected_path,
                sidebar_width,
                cx,
            ));
        }
        main = main
            .child(sidebar_bumper(sidebar_visible, cx))
            .child(workspace_content(
                self.stacker.clone(),
                self.editor.clone(),
                self.terminal.clone(),
                active_surface,
                appearance_config,
                appearance_page,
                cx,
            ));

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(CHROME_BG))
            .text_color(rgb(SIDEBAR_TEXT))
            .font_family("Atkinson Hyperlegible")
            .child(workspace_tab_bar(tabs, active_tab_id, cx))
            .child(main)
            .child(workspace_footer(active_surface, cx))
    }
}

fn workspace_tab_bar(
    tabs: Vec<WorkspaceTab>,
    active_tab_id: WorkspaceTabId,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
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

    for tab in tabs {
        bar = bar.child(workspace_tab(tab.clone(), active_tab_id, cx));
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

fn workspace_tab(
    tab: WorkspaceTab,
    active_tab_id: WorkspaceTabId,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let active = tab.id == active_tab_id;
    let tab_id = tab.id;
    div()
        .w(px(workspace_tab_width(tab.surface, active)))
        .h(px(32.0))
        .flex()
        .items_center()
        .justify_between()
        .px_3()
        .rounded_sm()
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
        .child(tab.surface.title())
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

fn workspace_tab_width(surface: WorkspaceSurface, active: bool) -> f32 {
    match (surface, active) {
        (WorkspaceSurface::Editor, true) => 184.0,
        (WorkspaceSurface::Editor, false) => 170.0,
        (WorkspaceSurface::Appearances, true) => 156.0,
        (WorkspaceSurface::Appearances, false) => 142.0,
        (WorkspaceSurface::Settings, _) => 128.0,
        (WorkspaceSurface::Home, _) => 104.0,
        (_, true) => 140.0,
        (_, false) => 120.0,
    }
}

fn workspace_sidebar(
    workspace_root: PathBuf,
    entries: Vec<ExplorerEntry>,
    selected_path: Option<PathBuf>,
    sidebar_width: f32,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let root_label = workspace_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Project")
        .to_string();
    let mut sidebar = div()
        .w(px(sidebar_width))
        .h_full()
        .flex()
        .flex_col()
        .border_r_1()
        .border_color(rgb(BORDER))
        .bg(rgb(CHROME_BG))
        .child(
            div()
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
                .child(
                    div()
                        .w(px(22.0))
                        .h(px(22.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_sm()
                        .text_size(px(12.0))
                        .text_color(rgb(MUTED_TEXT))
                        .cursor_pointer()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _: &MouseDownEvent, _window, cx| {
                                cx.stop_propagation();
                                this.toggle_sidebar(cx);
                            }),
                        )
                        .child("x"),
                ),
        );

    let mut tree = div().flex_1().flex().flex_col().overflow_hidden().py_1();
    if entries.is_empty() {
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
            .child(format!("{}px", sidebar_width.round())),
    );
    sidebar
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
    let icon = if is_dir && entry.expanded {
        "v"
    } else if is_dir {
        ">"
    } else {
        " "
    };

    div()
        .w_full()
        .h(px(26.0))
        .flex()
        .items_center()
        .pl(px(8.0 + depth as f32 * 14.0))
        .pr_2()
        .rounded_sm()
        .bg(rgb(if selected { 0x303440 } else { CHROME_BG }))
        .text_size(px(13.0))
        .text_color(rgb(if is_dir { FOLDER_BLUE } else { SIDEBAR_TEXT }))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                if is_dir {
                    this.toggle_explorer_dir(path.clone(), cx);
                } else {
                    this.open_sidebar_file(path.clone(), window, cx);
                }
            }),
        )
        .child(
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

fn sidebar_bumper(sidebar_visible: bool, cx: &mut Context<WorkspacePrototype>) -> impl IntoElement {
    div()
        .id("workspace-sidebar-bumper")
        .w(px(BUMPER_WIDTH))
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(rgb(BUMPER_BG))
        .border_r_1()
        .border_color(rgb(BORDER))
        .text_color(rgb(if sidebar_visible {
            0x787d8c
        } else {
            QUEUE_GREEN
        }))
        .text_size(px(14.0))
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
        .child(
            div()
                .w(px(16.0))
                .h(px(32.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded_sm()
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

fn workspace_content(
    stacker: Entity<StackerPrototype>,
    editor: Entity<EditorPrototype>,
    terminal: Entity<TerminalSurface>,
    active_surface: WorkspaceSurface,
    appearance_config: Config,
    appearance_page: AppearancePage,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let content = div()
        .flex_1()
        .h_full()
        .flex()
        .overflow_hidden()
        .bg(rgb(EDITOR_BG));

    match active_surface {
        WorkspaceSurface::Stacker => content.child(
            div()
                .flex_1()
                .h_full()
                .bg(rgb(PANEL_BG))
                .overflow_hidden()
                .child(stacker),
        ),
        WorkspaceSurface::Editor => content
            .child(
                div()
                    .w(px(320.0))
                    .h_full()
                    .border_r_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(PANEL_BG))
                    .overflow_hidden()
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _: &MouseDownEvent, window, cx| {
                            this.focus_surface(WorkspaceSurface::Stacker, window, cx);
                        }),
                    )
                    .child(stacker),
            )
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .p_4()
                    .bg(rgb(EDITOR_BG))
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _: &MouseDownEvent, window, cx| {
                            this.focus_surface(WorkspaceSurface::Editor, window, cx);
                        }),
                    )
                    .child(
                        div()
                            .size_full()
                            .border_1()
                            .border_color(rgb(BORDER))
                            .bg(rgb(EDITOR_BG))
                            .overflow_hidden()
                            .child(editor),
                    ),
            ),
        WorkspaceSurface::Terminal => content.child(terminal),
        WorkspaceSurface::Appearances => {
            content.child(appearances_surface(appearance_config, appearance_page, cx))
        }
        WorkspaceSurface::Home => content.child(home_placeholder()),
        WorkspaceSurface::Settings => content.child(settings_placeholder()),
    }
}

fn appearances_surface(
    config: Config,
    page: AppearancePage,
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
                .child(appearance_controls_column(config, page, cx)),
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
        AppearancePage::Terminal => terminal_appearance_controls(content, config, cx),
        AppearancePage::Editor => editor_appearance_controls(content, config, cx),
        AppearancePage::Sketch => sketch_appearance_controls(content),
    }
}

fn terminal_appearance_controls(
    content: gpui::Div,
    config: Config,
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
                    "Matrix".to_string(),
                    config.effects.background == "matrix",
                    cx,
                    |this, cx| this.set_background_mode("matrix", cx),
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
                .child("Sketch canvas appearance remains on the preserved app path until the GPUI sketch surface lands."),
        )
        .child(
            div()
                .mt_2()
                .text_size(px(12.0))
                .text_color(rgb(MUTED_TEXT))
                .child("The tab is present now so the footer and workspace model are ready for those controls."),
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

fn home_placeholder() -> impl IntoElement {
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
                .child("Home workspace launcher"),
        )
        .child(
            div()
                .mt_2()
                .text_size(px(12.0))
                .text_color(rgb(MUTED_TEXT))
                .child("Recent projects and workspace restore move here after the tab shell is stable."),
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
            "Editor",
            WorkspaceSurface::Editor,
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
        .child(
            div()
                .text_size(px(12.0))
                .text_color(rgb(MUTED_TEXT))
                .child("GPUI visual parity shell"),
        )
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
