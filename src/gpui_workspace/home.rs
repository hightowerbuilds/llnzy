use std::path::PathBuf;

use gpui::prelude::*;
use gpui::{div, px, rgb, Context, MouseButton, MouseDownEvent};

use crate::config::Config;

use super::{sidebar::project_display_name, WorkspacePalette, WorkspacePrototype};

pub(super) fn home_surface(
    workspace_root: Option<PathBuf>,
    recent_projects: Vec<PathBuf>,
    config: &Config,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let palette = WorkspacePalette::from_config(config);
    let mut recent_list = div().flex().flex_col().gap_1().w(px(360.0));
    let recent = recent_projects.into_iter().take(5).collect::<Vec<_>>();
    if recent.is_empty() {
        recent_list = recent_list.child(
            div()
                .py_2()
                .text_size(px(13.0))
                .text_color(rgb(palette.muted_text))
                .child("No recent projects"),
        );
    } else {
        for project in recent {
            recent_list = recent_list.child(home_recent_project_row(project, palette, cx));
        }
    }

    let mut content = div()
        .flex_1()
        .h_full()
        .flex()
        .flex_col()
        .items_center()
        .bg(rgb(palette.editor_bg))
        .pt(px(80.0))
        .child(
            div()
                .text_size(px(26.0))
                .text_color(rgb(palette.active_text))
                .child("Home"),
        )
        .child(
            div()
                .mt_2()
                .mb_5()
                .text_size(px(13.0))
                .text_color(rgb(palette.muted_text))
                .child("Open a project or jump back into a recent workspace."),
        )
        .child(home_open_project_button(palette, cx));

    if let Some(root) = workspace_root {
        content = content.child(
            div()
                .mt_5()
                .w(px(360.0))
                .rounded_sm()
                .border_1()
                .border_color(rgb(palette.border))
                .bg(rgb(palette.panel_bg))
                .p_3()
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(rgb(palette.muted_text))
                        .child("OPEN PROJECT"),
                )
                .child(
                    div()
                        .mt_1()
                        .text_size(px(15.0))
                        .text_color(rgb(palette.active_text))
                        .child(project_display_name(&root)),
                )
                .child(
                    div()
                        .mt_1()
                        .text_size(px(11.0))
                        .text_color(rgb(palette.muted_text))
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
                .text_color(rgb(palette.muted_text))
                .child("RECENT PROJECTS"),
        )
        .child(recent_list);

    content
}

fn home_open_project_button(
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    div()
        .w(px(240.0))
        .h(px(42.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .bg(rgb(palette.accent))
        .text_size(px(15.0))
        .text_color(rgb(palette.active_text))
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
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let title = project_display_name(&project);
    let detail = project.display().to_string();
    let path = project;
    div()
        .w_full()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.panel_bg))
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
                .text_color(rgb(palette.sidebar_text))
                .child(title),
        )
        .child(
            div()
                .mt_1()
                .text_size(px(11.0))
                .text_color(rgb(palette.muted_text))
                .child(detail),
        )
}
