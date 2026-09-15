use gpui::prelude::*;
use gpui::{div, px, rgb, Context};

use super::{UiTheme, WorkspacePrototype, WorkspaceSurface, FOOTER_HEIGHT};

pub(super) fn workspace_footer(
    active_surface: Option<WorkspaceSurface>,
    palette: UiTheme,
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
            "Courses",
            WorkspaceSurface::Academy,
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
    palette: UiTheme,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let active = active_surface == Some(surface);
    crate::ui::button_with_state(
        gpui::SharedString::from(format!("footer-{label}")),
        label,
        palette,
        crate::ui::ButtonVariant::Ghost,
        crate::ui_theme::ControlSize::Regular,
        crate::ui::ControlState {
            selected: active,
            disabled: false,
        },
        cx.listener(move |this, _, window, cx| this.open_footer_surface(surface, window, cx)),
    )
}
