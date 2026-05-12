use gpui::prelude::*;
use gpui::{
    actions, div, px, rgb, size, App, Application, Bounds, Context, Entity, Focusable, KeyBinding,
    Render, Window, WindowBounds, WindowOptions,
};

use crate::gpui_editor::{bind_editor_keys, EditorPrototype};
use crate::gpui_stacker::{bind_stacker_keys, StackerPrototype};

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
const SIDEBAR_WIDTH: f32 = 180.0;
const BUMPER_WIDTH: f32 = 20.0;

pub fn run_workspace_prototype() {
    Application::new().run(|cx: &mut App| {
        bind_stacker_keys(cx);
        bind_editor_keys(cx);
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
}

impl WorkspacePrototype {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            stacker: cx.new(StackerPrototype::embedded),
            editor: cx.new(EditorPrototype::new),
        }
    }
}

impl Render for WorkspacePrototype {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(CHROME_BG))
            .text_color(rgb(SIDEBAR_TEXT))
            .font_family("Atkinson Hyperlegible")
            .child(workspace_tab_bar())
            .child(
                div()
                    .flex_1()
                    .flex()
                    .overflow_hidden()
                    .child(workspace_sidebar())
                    .child(sidebar_bumper())
                    .child(workspace_content(self.stacker.clone(), self.editor.clone())),
            )
            .child(workspace_footer())
    }
}

fn workspace_tab_bar() -> impl IntoElement {
    div()
        .h(px(TAB_BAR_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .py_1()
        .border_b_1()
        .border_color(rgb(BORDER))
        .bg(rgb(CHROME_BG))
        .child(workspace_tab("Stacker", false))
        .child(workspace_tab("Code Workbench", true))
        .child(workspace_tab("Explorer", false))
        .child(
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

fn workspace_tab(title: &'static str, active: bool) -> impl IntoElement {
    div()
        .w(px(if active { 184.0 } else { 120.0 }))
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
        .child(title)
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
                .child("x"),
        )
}

fn workspace_sidebar() -> impl IntoElement {
    div()
        .w(px(SIDEBAR_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .p_2()
        .border_r_1()
        .border_color(rgb(BORDER))
        .bg(rgb(CHROME_BG))
        .child(
            div()
                .h(px(28.0))
                .flex()
                .items_center()
                .justify_between()
                .text_size(px(14.0))
                .text_color(rgb(ACTIVE_TEXT))
                .child("LLNZY")
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(rgb(MUTED_TEXT))
                        .child("x"),
                ),
        )
        .child(sidebar_button("Open Project", true))
        .child(sidebar_button("Open Recent", false))
        .child(sidebar_section_label("PROJECT"))
        .child(sidebar_tree_row("src", true, 0))
        .child(sidebar_tree_row("gpui_workspace.rs", false, 1))
        .child(sidebar_tree_row("gpui_editor.rs", false, 1))
        .child(sidebar_tree_row("gpui_stacker.rs", false, 1))
        .child(sidebar_tree_row("daily-growth", true, 0))
        .child(sidebar_tree_row("gpui2-modular-integration.md", false, 1))
}

fn sidebar_button(label: &'static str, primary: bool) -> impl IntoElement {
    div()
        .w_full()
        .h(px(28.0))
        .mt_1()
        .flex()
        .items_center()
        .px_2()
        .rounded_sm()
        .bg(rgb(if primary { ACCENT } else { 0x303440 }))
        .text_color(rgb(0xe1e6ee))
        .text_size(px(14.0))
        .child(label)
}

fn sidebar_section_label(label: &'static str) -> impl IntoElement {
    div()
        .mt_3()
        .mb_1()
        .text_size(px(11.0))
        .text_color(rgb(0x787d8c))
        .child(label)
}

fn sidebar_tree_row(label: &'static str, folder: bool, depth: usize) -> impl IntoElement {
    div()
        .w_full()
        .h(px(24.0))
        .flex()
        .items_center()
        .pl(px(10.0 + depth as f32 * 16.0))
        .pr_2()
        .rounded_sm()
        .text_size(px(14.0))
        .text_color(rgb(if folder { FOLDER_BLUE } else { SIDEBAR_TEXT }))
        .child(if folder { "v " } else { "  " })
        .child(label)
}

fn sidebar_bumper() -> impl IntoElement {
    div()
        .w(px(BUMPER_WIDTH))
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(rgb(BUMPER_BG))
        .border_r_1()
        .border_color(rgb(BORDER))
        .text_color(rgb(0x787d8c))
        .text_size(px(14.0))
        .child("<")
}

fn workspace_content(
    stacker: Entity<StackerPrototype>,
    editor: Entity<EditorPrototype>,
) -> impl IntoElement {
    div()
        .flex_1()
        .h_full()
        .flex()
        .overflow_hidden()
        .bg(rgb(EDITOR_BG))
        .child(
            div()
                .w(px(320.0))
                .h_full()
                .border_r_1()
                .border_color(rgb(BORDER))
                .bg(rgb(PANEL_BG))
                .overflow_hidden()
                .child(stacker),
        )
        .child(
            div().flex_1().h_full().p_4().bg(rgb(EDITOR_BG)).child(
                div()
                    .size_full()
                    .border_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(EDITOR_BG))
                    .overflow_hidden()
                    .child(editor),
            ),
        )
}

fn workspace_footer() -> impl IntoElement {
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
        .child(footer_button("Home", false))
        .child(footer_button("Stacker", true))
        .child(footer_button("Editor", true))
        .child(footer_button("Sketch", false))
        .child(footer_button("Git", false))
        .child(footer_button("Settings", false))
        .child(div().flex_1())
        .child(
            div()
                .text_size(px(12.0))
                .text_color(rgb(MUTED_TEXT))
                .child("GPUI visual parity shell"),
        )
}

fn footer_button(label: &'static str, active: bool) -> impl IntoElement {
    div()
        .h(px(36.0))
        .flex()
        .items_center()
        .px_3()
        .rounded_sm()
        .bg(rgb(if active { ACCENT } else { CHROME_BG }))
        .text_color(rgb(if active { ACTIVE_TEXT } else { SIDEBAR_TEXT }))
        .text_size(px(14.0))
        .child(label)
}
