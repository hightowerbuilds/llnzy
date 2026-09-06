use gpui::prelude::*;
use gpui::{div, px, rgb, Context, MouseButton, MouseDownEvent};

use super::{WorkspacePalette, WorkspacePrototype, WorkspaceSurface, FOOTER_HEIGHT};

pub(super) fn workspace_footer(
    active_surface: Option<WorkspaceSurface>,
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
            "Settings",
            WorkspaceSurface::Settings,
            active_surface,
            palette,
            cx,
        ))
        .child(div().flex_1())
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
