use gpui::prelude::*;
use gpui::{
    canvas, div, point, px, rgb, size, App, Application, Bounds, Context, PathBuilder, Render,
    Window, WindowBounds, WindowOptions,
};

struct FoundationSpike {
    tabs: Vec<&'static str>,
    files: Vec<String>,
}

impl FoundationSpike {
    fn new() -> Self {
        let tabs = vec!["Shell 1", "Stacker", "Sketch", "Settings"];
        let files = (0..80)
            .map(|ix| format!("src/example/module_{ix:02}.rs"))
            .collect();

        Self { tabs, files }
    }
}

impl Render for FoundationSpike {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0x101014))
            .text_color(rgb(0xe7e7ee))
            .font_family("Inter")
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .child(top_bar(&self.tabs))
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .child(sidebar(&self.files))
                            .child(main_pane()),
                    )
                    .child(footer()),
            )
    }
}

fn top_bar(tabs: &[&'static str]) -> impl IntoElement {
    tabs.iter().fold(
        div()
            .h(px(42.0))
            .w_full()
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .border_b_1()
            .border_color(rgb(0x272732))
            .bg(rgb(0x181820))
            .child(
                div()
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(rgb(0xf4f4f8))
                    .mr_2()
                    .child("LLNZY GPUI Spike"),
            ),
        |bar, tab| {
            bar.child(
                div()
                    .h(px(28.0))
                    .px_3()
                    .flex()
                    .items_center()
                    .rounded_md()
                    .bg(rgb(0x232331))
                    .text_size(px(13.0))
                    .child(*tab),
            )
        },
    )
}

fn sidebar(files: &[String]) -> impl IntoElement {
    let list = files
        .iter()
        .fold(div().flex().flex_col().gap_1().p_2(), |list, file| {
            list.child(
                div()
                    .h(px(24.0))
                    .flex()
                    .items_center()
                    .px_2()
                    .rounded_sm()
                    .text_size(px(12.0))
                    .text_color(rgb(0xbfc1cc))
                    .child(file.clone()),
            )
        });

    div()
        .w(px(240.0))
        .h_full()
        .flex()
        .flex_col()
        .border_r_1()
        .border_color(rgb(0x272732))
        .bg(rgb(0x15151c))
        .child(
            div()
                .h(px(36.0))
                .flex()
                .items_center()
                .px_3()
                .text_size(px(12.0))
                .text_color(rgb(0x8f94a3))
                .child("Explorer"),
        )
        .child(div().flex_1().overflow_hidden().child(list))
}

fn main_pane() -> impl IntoElement {
    div()
        .flex_1()
        .h_full()
        .flex()
        .flex_col()
        .bg(rgb(0x0f0f13))
        .child(
            div()
                .h(px(36.0))
                .flex()
                .items_center()
                .px_4()
                .text_size(px(12.0))
                .text_color(rgb(0x8f94a3))
                .child("Custom render host placeholder"),
        )
        .child(
            div().flex_1().p_4().child(
                div()
                    .size_full()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2f3340))
                    .bg(rgb(0x08090d))
                    .child(
                        canvas(
                            move |_, _, _| {},
                            move |bounds, _, window, _| {
                                let mut builder = PathBuilder::stroke(px(2.0));
                                let top = bounds.origin.y + px(30.0);
                                let left = bounds.origin.x + px(30.0);
                                let width = bounds.size.width - px(60.0);

                                builder.move_to(point(left, top));
                                for step in 0..48 {
                                    let x = left + width * (step as f32 / 47.0);
                                    let y =
                                        top + px(80.0) + px(((step as f32) * 0.55).sin() * 44.0);
                                    builder.line_to(point(x, y));
                                }

                                if let Ok(path) = builder.build() {
                                    window.paint_path(path, rgb(0x7dd3fc));
                                }
                            },
                        )
                        .size_full(),
                    ),
            ),
        )
}

fn footer() -> impl IntoElement {
    div()
        .h(px(34.0))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px_3()
        .border_t_1()
        .border_color(rgb(0x272732))
        .bg(rgb(0x181820))
        .text_size(px(12.0))
        .text_color(rgb(0x9ea3b3))
        .child("Phase 0")
        .child("Pinned gpui = 0.2.2")
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1100.0), px(720.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| FoundationSpike::new()),
        )
        .unwrap();
        cx.activate(true);
    });
}
