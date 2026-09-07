//! The Code Academy surface: course picker, lesson list, and lesson
//! reader.
//!
//! Everything drawn here comes from the loaded `academy::CourseLibrary` —
//! the course manifests and lesson files on disk are the single source of
//! truth for titles, ordering, and lesson counts. Nothing about a course
//! is duplicated in this file, because a hardcoded catalog is exactly how
//! the surface and the shipped courses drift apart.
//!
//! The three views are chosen by the workspace's selection state: no
//! course selected draws the picker, a course draws its module/lesson
//! list, and a course plus a lesson draws the reader.

use std::rc::Rc;
use std::sync::Arc;

use gpui::prelude::*;
use gpui::{div, img, px, relative, rgb, Context, MouseButton, MouseDownEvent, RenderImage};

use crate::academy::{Course, CourseLibrary, Exercise};
use crate::academy_progress::AcademyProgress;
use crate::config::Config;
use crate::editor::markdown::{parse_markdown_blocks, MarkdownBlock, MarkdownBlockKind};

use super::{WorkspacePalette, WorkspacePrototype};

/// Everything the Academy surface reads, assembled by the workspace
/// render pass. The library is shared rather than cloned because a course
/// carries every lesson body.
pub(super) struct AcademyContext {
    pub(super) library: Option<Rc<CourseLibrary>>,
    pub(super) course: Option<String>,
    pub(super) lesson: Option<String>,
    pub(super) progress: AcademyProgress,
}

/// Course insignia, chosen by the manifest's `language` field so a new
/// course gets the right logo without touching this file's catalog (there
/// isn't one). Unknown languages render without a logo rather than a
/// placeholder.
pub(super) fn course_logo(language: &str) -> Option<Arc<RenderImage>> {
    let bytes: &'static [u8] = match language.to_ascii_lowercase().as_str() {
        "rust" => include_bytes!("../../assets/academy/rust-logo.png"),
        "javascript" | "typescript" | "js" | "ts" => {
            include_bytes!("../../assets/academy/javascript-logo.png")
        }
        _ => return None,
    };
    render_png(bytes)
}

/// Decode an embedded PNG once per call; callers keep the returned
/// `RenderImage` in the element tree only for the frame it is built in,
/// which is the same lifetime model as any GPUI image source.
fn render_png(bytes: &'static [u8]) -> Option<Arc<RenderImage>> {
    let decoded = image::load_from_memory(bytes).ok()?;
    let rgba = decoded.to_rgba8();
    let frame = image::Frame::new(rgba);
    Some(Arc::new(RenderImage::new(vec![frame])))
}

/// Completed and total lesson counts for one course.
pub(super) fn course_progress(course: &Course, progress: &AcademyProgress) -> (usize, usize) {
    let ids = course.lesson_ids_in_order();
    (
        progress.completed_lessons(&course.manifest.id, &ids),
        ids.len(),
    )
}

pub(super) fn academy_surface(
    config: &Config,
    academy: AcademyContext,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let palette = WorkspacePalette::from_config(config);

    let mut content = div()
        .id("academy-surface-scroll")
        .flex_1()
        .h_full()
        .flex()
        .flex_col()
        .items_center()
        .bg(rgb(palette.editor_bg))
        .pt(px(48.0))
        .overflow_y_scroll();

    let Some(library) = academy.library.as_deref() else {
        return content
            .child(academy_heading(palette))
            .child(academy_notice(
                palette,
                "No courses found. The bundled courses directory is missing from \
this build — reinstall the app, or run from a source checkout.",
            ));
    };

    // A selected lesson draws the reader; a selected course draws its
    // lesson list; neither draws the picker. A stale selection (course
    // removed from disk between renders) falls through to the picker.
    let selected = academy.course.as_deref().and_then(|id| library.course(id));

    match selected {
        Some(course) => {
            let lesson = academy
                .lesson
                .as_deref()
                .and_then(|id| course.lessons.get(id));
            match lesson {
                Some(lesson) => {
                    content = content.child(academy_lesson_reader(
                        course,
                        lesson,
                        &academy.progress,
                        palette,
                        cx,
                    ));
                }
                None => {
                    content = content
                        .child(academy_heading(palette))
                        .child(academy_course_detail(
                            course,
                            &academy.progress,
                            palette,
                            cx,
                        ));
                }
            }
        }
        None => {
            let mut grid = div().flex().flex_wrap().gap_3().justify_center();
            let mut any = false;
            for course in library.courses() {
                any = true;
                grid = grid.child(academy_course_card(course, &academy.progress, palette, cx));
            }
            content = content.child(academy_heading(palette));
            content = if any {
                content.child(grid)
            } else {
                content.child(academy_notice(
                    palette,
                    "The courses directory is empty. Add a course directory \
with a course.toml manifest to see it here.",
                ))
            };
        }
    }

    content
}

fn academy_heading(palette: WorkspacePalette) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .child(
            div()
                .text_size(px(18.0))
                .text_color(rgb(palette.active_text))
                .child("Code Academy"),
        )
        .child(
            div()
                .mt_1()
                .mb_5()
                .text_size(px(12.0))
                .text_color(rgb(palette.muted_text))
                .child(
                    "Pick a course. Lessons run in the editor and the real \
terminal you already use.",
                ),
        )
}

/// Centered muted message for the empty and error states.
fn academy_notice(palette: WorkspacePalette, message: &str) -> impl IntoElement {
    div()
        .w(px(520.0))
        .mb_6()
        .text_size(px(12.0))
        .text_color(rgb(palette.muted_text))
        .child(message.to_string())
}

/// One clickable course card. The whole card is the click target; the
/// "Open" pill is an affordance, not a separate button.
fn academy_course_card(
    course: &Course,
    progress: &AcademyProgress,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let id = course.manifest.id.clone();
    let (completed, total) = course_progress(course, progress);
    let modules = course.manifest.modules.len();
    let book = course
        .manifest
        .book
        .as_ref()
        .map(|book| format!("Follows {}, {}e", book.title, book.edition));

    let mut card = div()
        .w(px(320.0))
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.panel_bg))
        .p_3()
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                this.select_academy_course(id.clone(), cx);
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
                        .child(match course_logo(&course.manifest.language) {
                            Some(logo) => {
                                div().child(img(Arc::clone(&logo)).size(px(28.0)).flex_none())
                            }
                            None => div(),
                        })
                        .child(
                            div()
                                .text_size(px(15.0))
                                .text_color(rgb(palette.active_text))
                                .child(course.manifest.title.clone()),
                        ),
                )
                .child(
                    div()
                        .rounded_sm()
                        .border_1()
                        .border_color(rgb(palette.queue_green))
                        .px_2()
                        .py_1()
                        .text_size(px(10.0))
                        .text_color(rgb(palette.queue_green))
                        .child(course.manifest.language.to_uppercase()),
                ),
        )
        .child(
            div()
                .mt_2()
                .text_size(px(12.0))
                .text_color(rgb(palette.sidebar_text))
                .child(course.manifest.description.clone()),
        );

    let mut meta = div()
        .mt_3()
        .flex()
        .flex_col()
        .gap_1()
        .child(academy_meta_row(
            palette,
            format!("{total} lessons across {modules} modules"),
        ));
    if let Some(book) = book {
        meta = meta.child(academy_meta_row(palette, book));
    }
    meta = meta.child(academy_meta_row(
        palette,
        format!("{completed} of {total} complete"),
    ));

    card = card.child(meta).child(
        div().mt_3().flex().justify_end().child(
            div()
                .rounded_sm()
                .bg(rgb(palette.accent))
                .px_3()
                .py_1()
                .text_size(px(12.0))
                .text_color(rgb(palette.active_text))
                .child("Open"),
        ),
    );

    card
}

/// A single muted "label: value" style meta line for a card.
fn academy_meta_row(palette: WorkspacePalette, line: impl Into<String>) -> impl IntoElement {
    div()
        .text_size(px(11.0))
        .text_color(rgb(palette.muted_text))
        .child(line.into())
}

/// The selected-course view: modules in manifest order, each listing its
/// lessons with completion state. Lesson rows open the reader.
fn academy_course_detail(
    course: &Course,
    progress: &AcademyProgress,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let (completed, total) = course_progress(course, progress);
    let fraction = if total == 0 {
        0.0
    } else {
        completed as f32 / total as f32
    };

    let mut panel = div()
        .w(px(720.0))
        .mb_6()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.accent))
        .bg(rgb(palette.panel_bg))
        .p_4()
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
                        .child(match course_logo(&course.manifest.language) {
                            Some(logo) => {
                                div().child(img(Arc::clone(&logo)).size(px(24.0)).flex_none())
                            }
                            None => div(),
                        })
                        .child(
                            div()
                                .text_size(px(16.0))
                                .text_color(rgb(palette.active_text))
                                .child(course.manifest.title.clone()),
                        ),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(rgb(palette.muted_text))
                        .child(format!("{completed}/{total} lessons")),
                ),
        )
        .child(academy_progress_bar(fraction, palette))
        .child(academy_back_row(
            palette,
            "← Back to courses",
            cx,
            |this, cx| {
                this.clear_academy_course_selection(cx);
            },
        ));

    for module in &course.manifest.modules {
        let heading = match module.chapter {
            Some(chapter) => format!("CHAPTER {chapter} — {}", module.title.to_uppercase()),
            None => module.title.to_uppercase(),
        };
        let mut list = div().mt_4().flex().flex_col().gap_1().child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(palette.muted_text))
                .child(heading),
        );
        for lesson_id in &module.lessons {
            let Some(lesson) = course.lessons.get(lesson_id) else {
                continue;
            };
            let done = progress.is_lesson_complete(&course.manifest.id, lesson_id);
            list = list.child(academy_lesson_row(
                &course.manifest.id,
                lesson_id,
                &lesson.meta.title,
                lesson.meta.exercises.len(),
                done,
                palette,
                cx,
            ));
        }
        panel = panel.child(list);
    }

    panel
}

/// One lesson row in the course view. Clicking opens the reader.
fn academy_lesson_row(
    course_id: &str,
    lesson_id: &str,
    title: &str,
    exercises: usize,
    done: bool,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let course = course_id.to_string();
    let lesson = lesson_id.to_string();
    let marker = if done { "●" } else { "○" };
    let marker_color = if done {
        palette.queue_green
    } else {
        palette.muted_text
    };

    div()
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .rounded_sm()
        .border_1()
        .border_color(rgb(if done {
            palette.queue_green
        } else {
            palette.border
        }))
        .px_2()
        .py_1()
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                this.select_academy_lesson(course.clone(), lesson.clone(), cx);
            }),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(rgb(marker_color))
                        .child(marker),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(rgb(palette.muted_text))
                        .child(lesson_id.to_string()),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(rgb(palette.sidebar_text))
                        .child(title.to_string()),
                ),
        )
        .child(
            div()
                .text_size(px(10.0))
                .text_color(rgb(palette.muted_text))
                .child(if exercises == 1 {
                    "1 exercise".to_string()
                } else {
                    format!("{exercises} exercises")
                }),
        )
}

/// The lesson reader: concepts, the markdown body, and each exercise.
fn academy_lesson_reader(
    course: &Course,
    lesson: &crate::academy::Lesson,
    progress: &AcademyProgress,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let course_id = course.manifest.id.clone();
    let done = progress.is_lesson_complete(&course.manifest.id, &lesson.id);
    let ordered = course.lesson_ids_in_order();
    let position = ordered.iter().position(|id| *id == lesson.id);

    let mut panel = div()
        .w(px(720.0))
        .mb_6()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.panel_bg))
        .p_4()
        .child(academy_back_row(
            palette,
            &format!("← {}", course.manifest.title),
            cx,
            move |this, cx| {
                this.clear_academy_lesson_selection(cx);
            },
        ))
        .child(
            div()
                .mt_3()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(rgb(palette.muted_text))
                        .child(match position {
                            Some(index) => {
                                format!("{} · LESSON {} OF {}", lesson.id, index + 1, ordered.len())
                            }
                            None => lesson.id.clone(),
                        }),
                )
                .child(if done {
                    div()
                        .rounded_sm()
                        .border_1()
                        .border_color(rgb(palette.queue_green))
                        .px_2()
                        .py_1()
                        .text_size(px(10.0))
                        .text_color(rgb(palette.queue_green))
                        .child("COMPLETE")
                } else {
                    div()
                }),
        )
        .child(
            div()
                .mt_1()
                .text_size(px(20.0))
                .text_color(rgb(palette.active_text))
                .child(lesson.meta.title.clone()),
        );

    if !lesson.meta.concepts.is_empty() {
        let mut chips = div().mt_2().flex().flex_wrap().gap_1();
        for concept in &lesson.meta.concepts {
            chips = chips.child(
                div()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .px_2()
                    .py_1()
                    .text_size(px(10.0))
                    .text_color(rgb(palette.muted_text))
                    .child(concept.clone()),
            );
        }
        panel = panel.child(chips);
    }

    let mut body = div().mt_4().flex().flex_col().gap_2();
    for block in parse_markdown_blocks(&lesson.body) {
        body = body.child(academy_markdown_block(&block, palette));
    }
    panel = panel.child(body);

    for (index, exercise) in lesson.meta.exercises.iter().enumerate() {
        panel = panel.child(academy_exercise(index, exercise, palette));
    }

    // Sequential navigation: the reader is the only place the student
    // moves between lessons, so both directions live here.
    let prev = position
        .and_then(|index| index.checked_sub(1))
        .and_then(|index| ordered.get(index))
        .map(|id| id.to_string());
    let next = position
        .map(|index| index + 1)
        .and_then(|index| ordered.get(index))
        .map(|id| id.to_string());

    let mut nav = div().mt_5().flex().items_center().justify_between().gap_2();
    nav = nav.child(match prev {
        Some(id) => academy_nav_button(&course_id, &id, "← Previous", palette, cx),
        None => div(),
    });
    nav = nav.child(match next {
        Some(id) => academy_nav_button(&course_id, &id, "Next →", palette, cx),
        None => div(),
    });

    panel.child(nav)
}

fn academy_nav_button(
    course_id: &str,
    lesson_id: &str,
    label: &str,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    let course = course_id.to_string();
    let lesson = lesson_id.to_string();
    div()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .px_3()
        .py_1()
        .text_size(px(11.0))
        .text_color(rgb(palette.sidebar_text))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                this.select_academy_lesson(course.clone(), lesson.clone(), cx);
            }),
        )
        .child(label.to_string())
}

/// One exercise block: the prompt, the files it ships, and the command
/// that will grade it.
fn academy_exercise(
    index: usize,
    exercise: &Exercise,
    palette: WorkspacePalette,
) -> impl IntoElement {
    let mut files = div().mt_2().flex().flex_wrap().gap_1();
    for file in &exercise.files {
        files = files.child(
            div()
                .rounded_sm()
                .border_1()
                .border_color(rgb(palette.border))
                .px_2()
                .py_1()
                .text_size(px(10.0))
                .text_color(rgb(palette.sidebar_text))
                .child(file.path.clone()),
        );
    }

    div()
        .mt_4()
        .w_full()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.accent))
        .p_3()
        .child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(palette.muted_text))
                .child(format!("EXERCISE {}", index + 1)),
        )
        .child(
            div()
                .mt_1()
                .text_size(px(13.0))
                .text_color(rgb(palette.active_text))
                .child(exercise.prompt.clone()),
        )
        .child(files)
        .child(
            div()
                .mt_2()
                .text_size(px(10.0))
                .text_color(rgb(palette.muted_text))
                .child(format!("Checked by: {}", exercise.check.command.join(" "))),
        )
}

/// Render one parsed markdown block with the workspace palette. Academy
/// keeps its own compact renderer rather than the editor's preview styles
/// because the lesson body sits inside workspace chrome, not a page.
fn academy_markdown_block(block: &MarkdownBlock, palette: WorkspacePalette) -> gpui::Div {
    match block.kind {
        MarkdownBlockKind::Heading(level) => {
            let size = match level {
                1 => 20.0,
                2 => 16.0,
                3 => 14.0,
                _ => 13.0,
            };
            div()
                .mt_3()
                .text_size(px(size))
                .text_color(rgb(palette.active_text))
                .child(block.text.clone())
        }
        MarkdownBlockKind::Paragraph => div()
            .text_size(px(13.0))
            .text_color(rgb(palette.sidebar_text))
            .child(block.text.clone()),
        MarkdownBlockKind::Bullet => div()
            .pl_3()
            .text_size(px(13.0))
            .text_color(rgb(palette.sidebar_text))
            .child(format!("• {}", block.text)),
        MarkdownBlockKind::Quote => div()
            .pl_3()
            .border_l_2()
            .border_color(rgb(palette.accent))
            .text_size(px(13.0))
            .text_color(rgb(palette.muted_text))
            .child(block.text.clone()),
        MarkdownBlockKind::Code => div()
            .rounded_sm()
            .bg(rgb(palette.editor_bg))
            .border_1()
            .border_color(rgb(palette.border))
            .p_2()
            .font_family("Menlo")
            .text_size(px(12.0))
            .text_color(rgb(palette.sidebar_text))
            .child(block.text.clone()),
    }
}

/// Thin two-tone bar: track in panel border color, fill in accent.
fn academy_progress_bar(fraction: f32, palette: WorkspacePalette) -> impl IntoElement {
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

/// Shared back affordance for the course and lesson views.
fn academy_back_row(
    palette: WorkspacePalette,
    label: &str,
    cx: &mut Context<WorkspacePrototype>,
    action: impl Fn(&mut WorkspacePrototype, &mut Context<WorkspacePrototype>) + 'static,
) -> gpui::Div {
    div()
        .mt_3()
        .w_auto()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .px_2()
        .py_1()
        .text_size(px(11.0))
        .text_color(rgb(palette.sidebar_text))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                action(this, cx);
            }),
        )
        .child(label.to_string())
}
