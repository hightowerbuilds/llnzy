//! Home keeps learning and writing in the first viewport.
use std::{path::PathBuf, rc::Rc};

use gpui::{div, prelude::*, px, rgb, Context};

use super::{
    academy::{course_insignia, course_progress, SurfaceBackdrop},
    sidebar::project_display_name,
    WorkspacePrototype,
};
use crate::{
    academy::{Course, CourseLibrary},
    academy_progress::AcademyProgress,
    config::Config,
    ui::{button, interactive, section_heading, ButtonVariant},
    ui_theme::{ControlSize, Typography, UiTheme},
};

/// Available width is the containing pane's width, so split panes receive the
/// same compact composition as narrow windows. The image remains mounted by the pane.
#[expect(
    clippy::too_many_arguments,
    reason = "Home composes projects, courses, and notes from separate workspace state"
)]
pub(super) fn home_surface(
    notepad: gpui::Entity<super::notepad::Notepad>,
    workspace_root: Option<PathBuf>,
    recent_projects: Vec<PathBuf>,
    config: &Config,
    academy_library: Option<Rc<CourseLibrary>>,
    academy_progress: &AcademyProgress,
    backdrop: SurfaceBackdrop,
    available_width: f32,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let theme = UiTheme::from_config(config);
    let narrow = available_width < 760.0;
    let library = academy_library.as_deref();
    let continuation = home_continue(library, academy_progress, theme, cx);
    let has_continuation = continuation.is_some();
    let mut entry = div()
        .id("home-course-entry")
        .flex()
        .flex_col()
        .gap_3()
        .child(section_heading("Courses", theme));
    if let Some(continuation) = continuation {
        entry = entry.child(continuation);
    } else {
        entry = entry.child(
            div()
                .text_size(px(Typography::CONTROL))
                .text_color(rgb(theme.muted_text))
                .child("Choose a course to get started."),
        );
    }
    // On a narrow pane, one real course remains directly accessible above
    // writing. The complete catalog follows the notepad, never precedes it.
    let first_course = library.and_then(|library| library.courses().next());
    if narrow && !has_continuation {
        if let Some(course) = first_course {
            entry = entry.child(home_course_row(course, academy_progress, theme, cx));
        } else {
            entry = entry.child(empty_courses(theme));
        }
    }
    let mut catalog = div().flex().flex_col().gap_1().w_full();
    let mut any = false;
    if let Some(library) = library {
        for course in library.courses() {
            any = true;
            catalog = catalog.child(home_course_row(course, academy_progress, theme, cx));
        }
    }
    if !any {
        catalog = catalog.child(empty_courses(theme));
    }
    let mut projects = div()
        .mt_6()
        .flex()
        .flex_col()
        .gap_1()
        .child(section_heading("Recent projects", theme));
    let mut recent = recent_projects;
    if let Some(root) = workspace_root {
        if !recent.contains(&root) {
            recent.insert(0, root);
        }
    }
    for project in recent.iter().take(4) {
        let path = project.clone();
        projects = projects.child(
            interactive(
                gpui::SharedString::from(format!("home-project-{}", project.display())),
                theme,
                cx.listener(move |this, _, _, cx| this.open_project(path.clone(), cx)),
            )
            .hover(move |style| style.bg(rgb(theme.hover_bg)))
            .active(move |style| style.bg(rgb(theme.pressed_bg)))
            .w_full()
            .px_2()
            .py_2()
            .text_size(px(Typography::CONTROL))
            .text_color(rgb(theme.muted_text))
            .child(div().truncate().child(project_display_name(project))),
        );
    }
    if recent.is_empty() {
        projects = projects.child(
            div()
                .py_2()
                .text_size(px(Typography::CAPTION))
                .text_color(rgb(theme.muted_text))
                .child("Opened projects will appear here."),
        );
    }
    // Courses and writing are separate containers with a gap between them
    // rather than two halves of one panel split by a rule. Each carries its
    // own fill and border so it reads as a card on the chrome, with or
    // without an image behind it.
    let container = |theme: UiTheme| {
        div()
            .rounded_sm()
            .border_1()
            .border_color(rgb(theme.border))
            .bg(backdrop.panel_fill(theme.panel_bg))
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
    };
    let mut body = div().w_full().min_w(px(0.0)).flex().gap_6();
    if narrow {
        body = body
            .flex_col()
            .child(container(theme).child(entry))
            .child(container(theme).child(notepad))
            .child(
                container(theme)
                    .child(section_heading("All courses", theme))
                    .child(catalog)
                    .child(projects),
            );
    } else {
        body = body
            .items_start()
            .child(
                container(theme)
                    .w(px(320.0))
                    .flex_shrink_0()
                    .child(entry)
                    .child(catalog)
                    .child(projects),
            )
            .child(container(theme).flex_1().min_w(px(0.0)).child(notepad));
    }
    div()
        .id("home-scroll")
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .overflow_y_scroll()
        .when(backdrop == SurfaceBackdrop::None, |surface| {
            surface.bg(rgb(theme.chrome_bg))
        })
        .p_4()
        .when(!narrow, |surface| surface.p_6())
        .text_color(rgb(theme.sidebar_text))
        .text_size(px(Typography::BODY))
        .child(
            div()
                .w_full()
                .max_w(px(1200.0))
                .mx_auto()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_3()
                        .child(
                            div()
                                .text_size(px(Typography::PAGE_TITLE))
                                .text_color(rgb(theme.active_text))
                                .child("Home"),
                        )
                        .child(button(
                            "home-open-project",
                            "Open project",
                            theme,
                            ButtonVariant::Ghost,
                            ControlSize::Compact,
                            cx.listener(|this, _, _, cx| this.pick_open_project(cx)),
                        )),
                )
                .child(body),
        )
}

fn empty_courses(theme: UiTheme) -> gpui::Div {
    div()
        .py_2()
        .text_size(px(Typography::CONTROL))
        .text_color(rgb(theme.muted_text))
        .child("No courses installed. Courses will appear here when available.")
}

fn home_continue(
    library: Option<&CourseLibrary>,
    progress: &AcademyProgress,
    theme: UiTheme,
    cx: &mut Context<WorkspacePrototype>,
) -> Option<gpui::Div> {
    let (course, lesson) = resumable_lesson(library, progress)?;
    Some(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_size(px(Typography::BODY))
                    .text_color(rgb(theme.active_text))
                    .child(course.manifest.title.clone()),
            )
            .child(
                div()
                    .text_size(px(Typography::CONTROL))
                    .text_color(rgb(theme.muted_text))
                    .child(lesson.meta.title.clone()),
            )
            .child(button(
                "home-continue",
                "Continue lesson",
                theme,
                ButtonVariant::Primary,
                ControlSize::Regular,
                cx.listener(|this, _, window, cx| this.continue_academy_learning(window, cx)),
            )),
    )
}

fn home_course_row(
    course: &Course,
    progress: &AcademyProgress,
    theme: UiTheme,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let (completed, total) = course_progress(course, progress);
    let course_id = course.manifest.id.clone();
    interactive(
        gpui::SharedString::from(format!("home-course-{course_id}")),
        theme,
        cx.listener(move |this, _, window, cx| {
            this.open_academy_course(course_id.clone(), window, cx)
        }),
    )
    .hover(move |style| style.bg(rgb(theme.hover_bg)))
    .active(move |style| style.bg(rgb(theme.pressed_bg)))
    .w_full()
    .min_w(px(0.0))
    .flex()
    .items_center()
    .gap_3()
    .px_2()
    .py_3()
    .child(course_insignia(&course.manifest.language, 22.0))
    .child(
        div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_size(px(Typography::CONTROL))
                    .text_color(rgb(theme.active_text))
                    .child(course.manifest.title.clone()),
            )
            .child(
                div()
                    .text_size(px(Typography::CAPTION))
                    .text_color(rgb(theme.muted_text))
                    .child(if total == 0 {
                        "No lessons yet".into()
                    } else if completed == 0 {
                        format!("{total} lessons")
                    } else {
                        format!("{completed} of {total} complete")
                    }),
            ),
    )
    .child(
        div()
            .flex_shrink_0()
            .text_size(px(Typography::CAPTION))
            .text_color(rgb(theme.muted_text))
            .child(if completed == 0 {
                "Start →"
            } else {
                "Open →"
            }),
    )
}

/// Stored progress can outlive an installed course or lesson. Only offer a
/// continuation when both IDs resolve, without clearing the student's progress.
fn resumable_lesson<'a>(
    library: Option<&'a CourseLibrary>,
    progress: &AcademyProgress,
) -> Option<(&'a Course, &'a crate::academy::Lesson)> {
    let location = progress.last_location()?;
    let course = library?.course(&location.course_id)?;
    let lesson = course.lessons.get(&location.lesson_id)?;
    Some((course, lesson))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> CourseLibrary {
        CourseLibrary::load(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/academy/courses"),
        )
        .expect("bundled course catalog")
    }

    #[test]
    fn continuation_resolves_the_saved_course_and_lesson() {
        let library = catalog();
        let course = library.courses().next().expect("a bundled course");
        let lesson = course.lesson_ids_in_order()[0];
        let mut progress = AcademyProgress::default();
        progress.record_location(&course.manifest.id, lesson, Some(PathBuf::from("practice")));
        let before = progress.clone();
        let (resumed_course, resumed_lesson) = resumable_lesson(Some(&library), &progress).unwrap();
        assert_eq!(resumed_course.manifest.id, course.manifest.id);
        assert_eq!(resumed_lesson.id, lesson);
        assert_eq!(
            progress, before,
            "rendering must preserve practice and completion data"
        );
    }

    #[test]
    fn stale_or_unavailable_catalog_omits_continue_without_discarding_progress() {
        let library = catalog();
        let course = library.courses().next().unwrap();
        let mut progress = AcademyProgress::default();
        assert!(resumable_lesson(Some(&library), &progress).is_none());
        for (course_id, lesson_id) in [
            ("course-removed-from-disk", "L00"),
            (course.manifest.id.as_str(), "lesson-removed-from-disk"),
        ] {
            progress.record_location(course_id, lesson_id, None);
            let before = progress.clone();
            assert!(resumable_lesson(Some(&library), &progress).is_none());
            assert!(resumable_lesson(None, &progress).is_none());
            assert_eq!(progress, before);
        }
    }
}
