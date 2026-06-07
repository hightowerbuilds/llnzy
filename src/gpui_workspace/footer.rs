use gpui::prelude::*;
use gpui::{div, px, rgb, ClipboardItem, Context, MouseButton, MouseDownEvent};

use super::{WorkspacePalette, WorkspacePrototype, WorkspaceSurface, FOOTER_HEIGHT};

pub(super) fn workspace_footer(
    active_surface: Option<WorkspaceSurface>,
    queued_prompts: Vec<crate::stacker::queue::QueuedPrompt>,
    palette: WorkspacePalette,
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
        .border_color(rgb(palette.border))
        .bg(rgb(palette.chrome_bg))
        .child(footer_button(
            "Home",
            WorkspaceSurface::Home,
            active_surface,
            palette,
            cx,
        ))
        .child(footer_button(
            "Terminal",
            WorkspaceSurface::Terminal,
            active_surface,
            palette,
            cx,
        ))
        .child(footer_button(
            "Stacker",
            WorkspaceSurface::Stacker,
            active_surface,
            palette,
            cx,
        ))
        .child(footer_button(
            "Sketch",
            WorkspaceSurface::Sketch,
            active_surface,
            palette,
            cx,
        ))
        .child(footer_button(
            "Settings",
            WorkspaceSurface::Settings,
            active_surface,
            palette,
            cx,
        ))
        .child(div().flex_1())
        .child(footer_queue_tray(active_surface, queued_prompts, cx))
}

fn footer_queue_tray(
    active_surface: Option<WorkspaceSurface>,
    queued_prompts: Vec<crate::stacker::queue::QueuedPrompt>,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    let mut tray = div()
        .h(px(36.0))
        .flex()
        .items_center()
        .justify_end()
        .gap_1();
    if active_surface != Some(WorkspaceSurface::Terminal) || queued_prompts.is_empty() {
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
        .text_color(rgb(0x6aff90))
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

fn footer_button(
    label: &'static str,
    surface: WorkspaceSurface,
    active_surface: Option<WorkspaceSurface>,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let active = active_surface == Some(surface);
    div()
        .h(px(36.0))
        .flex()
        .items_center()
        .px_3()
        .rounded_sm()
        .bg(rgb(if active {
            palette.accent
        } else {
            palette.chrome_bg
        }))
        .text_color(rgb(if active {
            palette.active_text
        } else {
            palette.sidebar_text
        }))
        .text_size(px(14.0))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                this.open_footer_surface(surface, window, cx);
            }),
        )
        .child(label)
}
