use gpui::prelude::*;
use gpui::{
    actions, div, px, rgb, size, App, Application, Bounds, Context, Entity, Focusable, KeyBinding,
    Render, Window, WindowBounds, WindowOptions,
};

use crate::gpui_editor::{bind_editor_keys, EditorPrototype};
use crate::gpui_stacker::{bind_stacker_keys, StackerPrototype};

actions!(workspace_gpui, [Quit]);

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
            stacker: cx.new(StackerPrototype::new),
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
            .bg(rgb(0x0b0d12))
            .text_color(rgb(0xe6edf3))
            .font_family("Inter")
            .child(workspace_header())
            .child(
                div()
                    .flex_1()
                    .flex()
                    .overflow_hidden()
                    .child(
                        div()
                            .w(px(430.0))
                            .h_full()
                            .border_r_1()
                            .border_color(rgb(0x30363d))
                            .overflow_hidden()
                            .child(self.stacker.clone()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .overflow_hidden()
                            .child(self.editor.clone()),
                    ),
            )
    }
}

fn workspace_header() -> impl IntoElement {
    div()
        .h(px(44.0))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px_4()
        .border_b_1()
        .border_color(rgb(0x30363d))
        .bg(rgb(0x111722))
        .child(
            div()
                .font_weight(gpui::FontWeight::BOLD)
                .text_size(px(15.0))
                .child("LLNZY GPUI Workspace"),
        )
        .child(
            div()
                .text_size(px(12.0))
                .text_color(rgb(0x8b949e))
                .child("Stacker + editor, terminal intentionally absent"),
        )
}
