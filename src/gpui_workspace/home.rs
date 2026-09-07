use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use gpui::prelude::*;
use gpui::{div, img, px, relative, rgb, Context, MouseButton, MouseDownEvent};

use crate::academy::CourseLibrary;
use crate::academy_progress::AcademyProgress;
use crate::config::Config;

use super::{
    academy::{course_logo, course_progress},
    sidebar::project_display_name,
    WorkspacePalette, WorkspacePrototype,
};

pub(super) fn home_surface(
    workspace_root: Option<PathBuf>,
    recent_projects: Vec<PathBuf>,
    config: &Config,
    academy_library: Option<Rc<CourseLibrary>>,
    academy_progress: &AcademyProgress,
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
        .child(home_open_project_button(palette, cx))
        .child(home_new_course_button(palette, cx))
        .child(home_academy_progress_section(
            academy_library.as_deref(),
            academy_progress,
            palette,
            cx,
        ));

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

fn home_new_course_button(
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    div()
        .mt_2()
        .w(px(240.0))
        .h(px(42.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.panel_bg))
        .text_size(px(15.0))
        .text_color(rgb(palette.sidebar_text))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _: &MouseDownEvent, window, cx| {
                this.open_academy_from_home(window, cx);
            }),
        )
        .child("Open Course")
}

/// Academy course progress on Home: one row per loaded course showing the
/// language insignia, completed-vs-total lessons, and a progress bar.
/// Clicking a row opens the Academy tab with that course selected. Course
/// identity, titles, and totals all come from the loaded library, so a
/// course added to the courses directory appears here with no code change.
fn home_academy_progress_section(
    library: Option<&CourseLibrary>,
    progress: &AcademyProgress,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let mut rows = div().flex().flex_col().gap_2().w(px(360.0));
    let mut any = false;
    if let Some(library) = library {
        for course in library.courses() {
            any = true;
            let (completed, total) = course_progress(course, progress);
            rows = rows.child(home_academy_progress_row(
                &course.manifest.id,
                &course.manifest.title,
                &course.manifest.language,
                completed,
                total,
                palette,
                cx,
            ));
        }
    }
    if !any {
        rows = rows.child(
            div()
                .py_2()
                .text_size(px(13.0))
                .text_color(rgb(palette.muted_text))
                .child("No courses installed"),
        );
    }

    div()
        .mt_6()
        .w(px(360.0))
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_size(px(12.0))
                .text_color(rgb(palette.muted_text))
                .child("COURSE PROGRESS"),
        )
        .child(rows)
}

fn home_academy_progress_row(
    id: &str,
    title: &str,
    language: &str,
    completed: usize,
    total: usize,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let fraction = if total == 0 {
        0.0
    } else {
        completed as f32 / total as f32
    };
    let percent = (fraction * 100.0).round() as u32;
    let course_id = id.to_string();

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
            cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                this.open_academy_course(course_id.clone(), window, cx);
            }),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(match course_logo(language) {
                            Some(logo) => {
                                div().child(img(Arc::clone(&logo)).size(px(24.0)).flex_none())
                            }
                            None => div(),
                        })
                        .child(
                            div()
                                .text_size(px(13.0))
                                .text_color(rgb(palette.sidebar_text))
                                .child(title.to_string()),
                        ),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(rgb(palette.muted_text))
                        .child(if total == 0 {
                            "No lessons".to_string()
                        } else {
                            format!("{completed}/{total} lessons · {percent}%")
                        }),
                ),
        )
        .child(home_progress_bar(fraction, palette))
}

/// Thin two-tone bar: track in panel border color, fill in accent.
fn home_progress_bar(fraction: f32, palette: WorkspacePalette) -> impl IntoElement {
    div()
        .mt_2()
        .h(px(4.0))
        .w_full()
        .rounded_sm()
        .bg(rgb(palette.border))
        .overflow_hidden()
        .child(
            div()
                .h_full()
                .w(relative(fraction.clamp(0.0, 1.0)))
                .bg(rgb(palette.accent)),
        )
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
