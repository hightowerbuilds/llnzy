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
use std::sync::{Arc, OnceLock};

use gpui::prelude::*;
use gpui::{div, img, px, relative, rgb, rgba, ClipboardItem, Context, RenderImage, Rgba};

use crate::academy::{Course, CourseLibrary, Exercise};
use crate::academy_progress::AcademyProgress;
use crate::config::Config;
use crate::editor::markdown::{parse_markdown_blocks, MarkdownBlock, MarkdownBlockKind};

use super::WorkspacePrototype;
use crate::ui::{button, interactive, ButtonVariant};
use crate::ui_theme::{ControlSize, Typography, UiTheme};

/// Everything the Academy surface reads, assembled by the workspace
/// render pass. The library is shared rather than cloned because a course
/// carries every lesson body.
pub(super) struct AcademyContext {
    pub(super) library: Option<Rc<CourseLibrary>>,
    pub(super) course: Option<String>,
    pub(super) lesson: Option<String>,
    pub(super) progress: AcademyProgress,
    pub(super) practice: super::academy_actions::AcademyPracticeState,
}

/// What sits behind an ambient surface (the Academy or Home) this frame:
/// nothing, or the workspace background image, mounted on the surface's
/// own pane or shared across a joined group.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum SurfaceBackdrop {
    None,
    Image,
}

impl SurfaceBackdrop {
    /// A card or panel fill over this backdrop: the flat theme color with
    /// nothing behind the surface, and translucent over the image, with
    /// enough cover left to keep text legible over a photo.
    pub(super) fn panel_fill(self, color: u32) -> Rgba {
        crate::ui::surface_fill(color, self == Self::Image)
    }
}

/// Panel fills for the Academy, resolved once per render.
///
/// The surface is reading material: prose, code, and cards stacked on a
/// flat theme fill. Put a photo behind that flat fill and you see nothing,
/// so over an image the fills go translucent instead (see
/// `SurfaceBackdrop::panel_fill`). Code blocks sit above panels in both
/// cases, because misread code costs more than a muted photo.
#[derive(Clone, Copy)]
struct AcademyChrome {
    panel: Rgba,
    code: Rgba,
}

impl AcademyChrome {
    fn new(palette: UiTheme, backdrop: SurfaceBackdrop) -> Self {
        let code = match backdrop {
            SurfaceBackdrop::None => rgb(palette.editor_bg),
            SurfaceBackdrop::Image => translucent(palette.editor_bg, 0xe6),
        };
        Self {
            panel: backdrop.panel_fill(palette.panel_bg),
            code,
        }
    }
}

/// Widen a `0xRRGGBB` theme color into `0xRRGGBBAA` at the given alpha.
fn translucent(color: u32, alpha: u32) -> Rgba {
    rgba((color << 8) | alpha)
}

/// Course insignia, chosen by the manifest's `language` field so a new
/// course gets the right logo without touching this file's catalog (there
/// isn't one). Unknown languages render without a logo rather than a
/// placeholder.
pub(super) fn course_logo(language: &str) -> Option<Arc<RenderImage>> {
    static RUST: OnceLock<Option<Arc<RenderImage>>> = OnceLock::new();
    static JAVASCRIPT: OnceLock<Option<Arc<RenderImage>>> = OnceLock::new();
    match language.to_ascii_lowercase().as_str() {
        "rust" => RUST
            .get_or_init(|| render_png(include_bytes!("../../assets/academy/rust-logo.png")))
            .clone(),
        "javascript" | "js" => JAVASCRIPT
            .get_or_init(|| render_png(include_bytes!("../../assets/academy/javascript-logo.png")))
            .clone(),
        _ => None,
    }
}

/// Use a native badge for TypeScript so its course and progress rows are
/// distinguishable from JavaScript without another bitmap asset.
pub(super) fn course_insignia(language: &str, size: f32) -> impl IntoElement {
    if matches!(language.to_ascii_lowercase().as_str(), "typescript" | "ts") {
        return div()
            .size(px(size))
            .flex_none()
            .flex()
            .items_end()
            .justify_end()
            .pr(px(2.0))
            .bg(rgb(0x3178c6))
            .text_color(rgb(0xffffff))
            .text_size(px(size * 0.5))
            .child("TS");
    }
    match course_logo(language) {
        Some(logo) => div().child(img(logo).size(px(size)).flex_none()),
        None => div(),
    }
}

/// Course logos are decoded once and retained by `course_logo`; normal
/// render passes only clone their shared image handles.
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

/// `backdrop` describes the image behind this surface, mounted either on
/// its own pane or shared across a joined group. Anything but `None` means
/// the surface drops its full-bleed fill and lets the panels carry the
/// contrast instead.
pub(super) fn academy_surface(
    config: &Config,
    academy: AcademyContext,
    backdrop: SurfaceBackdrop,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let palette = UiTheme::from_config(config);
    let chrome = AcademyChrome::new(palette, backdrop);

    let mut content = div()
        .id(gpui::SharedString::from(format!(
            "academy-scroll-{}-{}",
            academy.course.as_deref().unwrap_or("catalog"),
            academy.lesson.as_deref().unwrap_or("overview")
        )))
        .flex_1()
        .min_w(px(0.0))
        .px_3()
        .h_full()
        .flex()
        .flex_col()
        .items_center()
        .pt_6()
        .pb_6()
        .overflow_y_scroll();

    if backdrop == SurfaceBackdrop::None {
        content = content.bg(rgb(palette.editor_bg));
    }

    let Some(library) = academy.library.as_deref() else {
        return content
            .child(academy_heading(palette, chrome))
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
                        &academy.practice,
                        palette,
                        chrome,
                        cx,
                    ));
                }
                None => {
                    content = content.child(academy_heading(palette, chrome)).child(
                        academy_course_detail(course, &academy.progress, palette, chrome, cx),
                    );
                }
            }
        }
        None => {
            let mut grid = div().w_full().max_w(px(880.0)).flex().flex_col().gap_2();
            let mut any = false;
            for course in library.courses() {
                any = true;
                grid = grid.child(academy_course_card(
                    course,
                    &academy.progress,
                    palette,
                    chrome,
                    cx,
                ));
            }
            content = content.child(academy_heading(palette, chrome));
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

fn academy_heading(palette: UiTheme, chrome: AcademyChrome) -> impl IntoElement {
    div()
        .w_full()
        .max_w(px(880.0))
        .rounded_sm()
        .bg(chrome.panel)
        .p_4()
        .mb_3()
        .flex()
        .flex_col()
        .child(
            div()
                .text_size(px(Typography::PAGE_TITLE))
                .text_color(rgb(palette.active_text))
                .child("Courses"),
        )
        .child(
            div()
                .mt_1()
                .text_size(px(Typography::CONTROL))
                .text_color(rgb(palette.muted_text))
                .child("Choose a course and start learning at your own pace."),
        )
}

/// Centered muted message for the empty and error states.
fn academy_notice(palette: UiTheme, message: &str) -> impl IntoElement {
    div()
        .w_full()
        .max_w(px(520.0))
        .mb_6()
        .text_size(px(Typography::CONTROL))
        .text_color(rgb(palette.muted_text))
        .child(message.to_string())
}

/// A course row opens its overview through the same pointer/keyboard action.
fn academy_course_card(
    course: &Course,
    progress: &AcademyProgress,
    palette: UiTheme,
    chrome: AcademyChrome,
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

    let mut card = interactive(
        gpui::SharedString::from(format!("academy-course-{id}")),
        palette,
        cx.listener(move |this, _, _, cx| this.select_academy_course(id.clone(), cx)),
    )
    .hover(move |style| style.bg(rgb(palette.hover_bg)))
    .active(move |style| style.bg(rgb(palette.pressed_bg)))
    .w_full()
    .min_w(px(0.0))
    .bg(chrome.panel)
    .p_4()
    .child(
        div()
            .flex()
            .items_center()
            .justify_between()
            .flex_wrap()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(course_insignia(&course.manifest.language, 28.0))
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
                    .border_color(rgb(palette.border))
                    .px_2()
                    .py_1()
                    .text_size(px(Typography::CAPTION))
                    .text_color(rgb(palette.muted_text))
                    .child(course.manifest.language.clone()),
            ),
    )
    .child(
        div()
            .mt_2()
            .text_size(px(Typography::CONTROL))
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
        div()
            .mt_2()
            .text_size(px(Typography::CONTROL))
            .text_color(rgb(palette.accent))
            .child("View lessons →"),
    );

    card
}

/// A single muted "label: value" style meta line for a card.
fn academy_meta_row(palette: UiTheme, line: impl Into<String>) -> impl IntoElement {
    div()
        .text_size(px(Typography::CAPTION))
        .text_color(rgb(palette.muted_text))
        .child(line.into())
}

/// The selected-course view: modules in manifest order, each listing its
/// lessons with completion state. Lesson rows open the reader.
fn academy_course_detail(
    course: &Course,
    progress: &AcademyProgress,
    palette: UiTheme,
    chrome: AcademyChrome,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let (completed, total) = course_progress(course, progress);
    let fraction = if total == 0 {
        0.0
    } else {
        completed as f32 / total as f32
    };

    let mut panel = div()
        .w_full()
        .max_w(px(720.0))
        .mb_6()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(chrome.panel)
        .p_4()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .flex_wrap()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(course_insignia(&course.manifest.language, 24.0))
                        .child(
                            div()
                                .text_size(px(16.0))
                                .text_color(rgb(palette.active_text))
                                .child(course.manifest.title.clone()),
                        ),
                )
                .child(
                    div()
                        .text_size(px(Typography::CAPTION))
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

    if let Some(book) = &course.manifest.book {
        if let Some(url) = &book.url {
            let url = url.clone();
            panel = panel.child(
                button(
                    "academy-read-book",
                    format!("Read {} online for free ↗", book.title),
                    palette,
                    ButtonVariant::Ghost,
                    ControlSize::Compact,
                    move |_, _, cx| cx.open_url(&url),
                )
                .mt_3(),
            );
        }
    }

    for module in &course.manifest.modules {
        let heading = match module.chapter {
            Some(chapter) => format!("Chapter {chapter} · {}", module.title),
            None => module.title.clone(),
        };
        let mut list = div().mt_4().flex().flex_col().gap_1().child(
            div()
                .text_size(px(Typography::CAPTION))
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
    palette: UiTheme,
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

    interactive(
        gpui::SharedString::from(format!("academy-lesson-{course}-{lesson}")),
        palette,
        cx.listener(move |this, _, _, cx| {
            this.select_academy_lesson(course.clone(), lesson.clone(), cx)
        }),
    )
    .hover(move |style| style.bg(rgb(palette.hover_bg)))
    .active(move |style| style.bg(rgb(palette.pressed_bg)))
    .w_full()
    .min_w(px(0.0))
    .flex()
    .items_center()
    .justify_between()
    .flex_wrap()
    .gap_2()
    .px_2()
    .py_2()
    .child(
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .text_size(px(Typography::CAPTION))
                    .text_color(rgb(marker_color))
                    .child(marker),
            )
            .child(
                div()
                    .text_size(px(Typography::CAPTION))
                    .text_color(rgb(palette.muted_text))
                    .child(lesson_id.to_string()),
            )
            .child(
                div()
                    .text_size(px(Typography::CONTROL))
                    .text_color(rgb(palette.sidebar_text))
                    .child(title.to_string()),
            ),
    )
    .child(
        div()
            .text_size(px(Typography::CAPTION))
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
    practice: &super::academy_actions::AcademyPracticeState,
    palette: UiTheme,
    chrome: AcademyChrome,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let course_id = course.manifest.id.clone();
    let done = progress.is_lesson_complete(&course.manifest.id, &lesson.id);
    let ordered = course.lesson_ids_in_order();
    let position = ordered.iter().position(|id| *id == lesson.id);

    let mut panel = div()
        .w_full()
        .max_w(px(720.0))
        .mb_6()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(chrome.panel)
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
                .flex_wrap()
                .gap_2()
                .child(
                    div()
                        .text_size(px(Typography::CAPTION))
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
                        .text_size(px(Typography::CAPTION))
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
                    .text_size(px(Typography::CAPTION))
                    .text_color(rgb(palette.muted_text))
                    .child(concept.clone()),
            );
        }
        panel = panel.child(chips);
    }

    // The whole body as one copy, for readers who want the lesson in their
    // own notes rather than one command at a time. Sourced from the raw
    // markdown, not the parsed blocks, so what lands on the clipboard is
    // the lesson as written.
    panel = panel.child(div().mt_4().flex().justify_end().child(academy_copy_button(
        "academy-copy-lesson",
        "Copy lesson",
        lesson.body.clone(),
        palette,
        cx,
    )));

    let mut body = div()
        .mt_2()
        .flex()
        .flex_col()
        .gap_3()
        .font_family(Typography::READING_FONT_FAMILY);
    for (index, block) in parse_markdown_blocks(&lesson.body).iter().enumerate() {
        body = body.child(academy_markdown_block(index, block, palette, chrome, cx));
    }
    panel = panel.child(academy_study_tools(
        course, lesson, progress, practice, palette, cx,
    ));
    panel = panel.child(body);

    for (index, exercise) in lesson.meta.exercises.iter().enumerate() {
        panel = panel.child(academy_exercise(
            course, lesson, index, exercise, practice, palette, cx,
        ));
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
        Some(id) => div().child(academy_nav_button(
            &course_id,
            &id,
            "← Previous",
            palette,
            cx,
        )),
        None => div(),
    });
    nav = nav.child(match next {
        Some(id) => div().child(academy_nav_button(&course_id, &id, "Next →", palette, cx)),
        None => div(),
    });

    panel.child(nav)
}

fn academy_nav_button(
    course_id: &str,
    lesson_id: &str,
    label: &str,
    palette: UiTheme,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Stateful<gpui::Div> {
    let course = course_id.to_string();
    let lesson = lesson_id.to_string();
    button(
        gpui::SharedString::from(format!("academy-nav-{course}-{lesson}")),
        label.to_owned(),
        palette,
        ButtonVariant::Secondary,
        ControlSize::Regular,
        cx.listener(move |this, _, _, cx| {
            this.select_academy_lesson(course.clone(), lesson.clone(), cx)
        }),
    )
}

/// One exercise block: the prompt, the files it ships, and the command
/// that will grade it.
fn academy_exercise(
    course: &Course,
    lesson: &crate::academy::Lesson,
    index: usize,
    exercise: &Exercise,
    practice: &super::academy_actions::AcademyPracticeState,
    palette: UiTheme,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let key = super::academy_actions::exercise_key(&course.manifest.id, &lesson.id, index);
    let mut controls = div()
        .id(gpui::SharedString::from(format!("academy-controls-{key}")))
        .mt_3()
        .flex()
        .flex_wrap()
        .gap_2();
    let c = course.manifest.id.clone();
    let l = lesson.id.clone();
    controls = controls.child(academy_action(
        "Open practice",
        palette,
        cx,
        move |this, window, cx| {
            this.academy_prepare(c.clone(), l.clone(), index, window, cx);
        },
    ));
    let c = course.manifest.id.clone();
    let l = lesson.id.clone();
    controls = controls.child(academy_action(
        "Check work",
        palette,
        cx,
        move |this, _, cx| {
            this.academy_run_check(c.clone(), l.clone(), index, cx);
        },
    ));
    let reveal_key = key.clone();
    controls = controls.child(academy_action(
        "Compare solution",
        palette,
        cx,
        move |this, _, cx| {
            if !this.academy_practice.revealed.remove(&reveal_key) {
                this.academy_practice.revealed.insert(reveal_key.clone());
            }
            cx.notify();
        },
    ));
    let mut feedback = div()
        .id(gpui::SharedString::from(key.clone()))
        .mt_3()
        .flex()
        .flex_col()
        .gap_2();
    if let Some(directory) = practice.directories.get(&key) {
        feedback = feedback.child(
            div()
                .text_size(px(Typography::CONTROL))
                .child(format!("Practice folder: {}", directory.display())),
        );
    }
    if let Some(result) = practice.results.get(&key) {
        feedback = feedback.child(div().child(result.message.clone()));
        feedback = feedback.child(academy_output("Expected", &result.expected, palette, cx));
        feedback = feedback.child(academy_output("Actual", &result.actual, palette, cx));
        if !result.stdout.is_empty() && result.stdout != result.actual {
            feedback = feedback.child(academy_output(
                "Program output",
                &result.stdout,
                palette,
                cx,
            ));
        }
        if !result.stderr.is_empty() {
            feedback = feedback.child(academy_output("Details", &result.stderr, palette, cx));
        }
        if result.truncated {
            feedback =
                feedback.child("Output was shortened. Reduce repeated logging and check again.");
        }
    }
    if practice.revealed.contains(&key) {
        feedback = feedback.child("Try the lesson hint first. Compare one function at a time and explain the difference. Your files are unchanged.");
        for file in &exercise.files {
            if file.starter != file.solution {
                feedback = feedback.child(academy_output(
                    &format!("Solution: {}", file.path),
                    &file.solution,
                    palette,
                    cx,
                ));
            }
        }
    }
    let command = exercise.check.command.join(" ");
    let mut files = div().mt_2().flex().flex_wrap().gap_1();
    for file in &exercise.files {
        files = files.child(
            div()
                .rounded_sm()
                .border_1()
                .border_color(rgb(palette.border))
                .px_2()
                .py_1()
                .text_size(px(Typography::CAPTION))
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
                .text_size(px(Typography::CAPTION))
                .text_color(rgb(palette.muted_text))
                .child(format!("EXERCISE {}", index + 1)),
        )
        .child(
            div()
                .mt_1()
                .text_size(px(Typography::BODY))
                .text_color(rgb(palette.active_text))
                .child(exercise.prompt.clone()),
        )
        .child(files)
        .child(controls)
        .child(feedback)
        .child(
            // The grading command is something the reader runs, so it gets
            // the same copy affordance as a command in the prose.
            div()
                .mt_2()
                .flex()
                .items_center()
                .justify_between()
                .flex_wrap()
                .gap_2()
                .child(
                    div()
                        .text_size(px(Typography::CAPTION))
                        .text_color(rgb(palette.muted_text))
                        .child(format!("Checked by: {command}")),
                )
                .child(academy_copy_button(
                    ("academy-copy-check", index),
                    "Copy command",
                    command,
                    palette,
                    cx,
                )),
        )
}

/// Lesson body text size. Prose is the product on this surface, so it is
/// sized for sustained reading rather than to match the workspace chrome
/// around it; headings and code scale from here.
const ACADEMY_BODY_TEXT: f32 = Typography::READING;

/// A button that puts `payload` on the system clipboard.
///
/// GPUI 0.2.2 has no selectable static text — `InteractiveText` offers
/// click, hover, and tooltip but no selection state — so copying out of a
/// lesson goes through explicit affordances like this one rather than
/// dragging across the prose.
fn academy_copy_button(
    id: impl Into<gpui::ElementId>,
    label: &str,
    payload: String,
    palette: UiTheme,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Stateful<gpui::Div> {
    button(
        id,
        label.to_owned(),
        palette,
        ButtonVariant::Ghost,
        ControlSize::Compact,
        cx.listener(move |_this, _, _, cx| {
            cx.write_to_clipboard(ClipboardItem::new_string(payload.clone()))
        }),
    )
}

/// Render one parsed markdown block with the workspace palette. Academy
/// keeps its own compact renderer rather than the editor's preview styles
/// because the lesson body sits inside workspace chrome, not a page.
///
/// `index` only has to be unique within the lesson: it disambiguates the
/// per-block copy buttons' element ids.
fn academy_markdown_block(
    index: usize,
    block: &MarkdownBlock,
    palette: UiTheme,
    chrome: AcademyChrome,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    match block.kind {
        MarkdownBlockKind::Heading(level) => {
            let size = match level {
                1 => ACADEMY_BODY_TEXT * 1.625,
                2 => ACADEMY_BODY_TEXT * 1.3125,
                3 => ACADEMY_BODY_TEXT * 1.125,
                _ => ACADEMY_BODY_TEXT,
            };
            div()
                .mt_4()
                .text_size(px(size))
                .text_color(rgb(palette.active_text))
                .child(block.text.clone())
        }
        MarkdownBlockKind::Paragraph => div()
            .text_size(px(ACADEMY_BODY_TEXT))
            .text_color(rgb(palette.sidebar_text))
            .child(block.text.clone())
            .child(div().flex().justify_end().child(academy_copy_button(
                ("academy-copy-paragraph", index),
                "Copy paragraph",
                block.text.clone(),
                palette,
                cx,
            ))),
        MarkdownBlockKind::Bullet => div()
            .pl_3()
            .text_size(px(ACADEMY_BODY_TEXT))
            .text_color(rgb(palette.sidebar_text))
            .child(format!("• {}", block.text)),
        MarkdownBlockKind::Quote => div()
            .pl_3()
            .border_l_2()
            .border_color(rgb(palette.accent))
            .text_size(px(ACADEMY_BODY_TEXT))
            .text_color(rgb(palette.muted_text))
            .child(block.text.clone()),
        MarkdownBlockKind::Code => {
            // A command the reader is meant to run is labelled and framed
            // differently from a source sample they are meant to study —
            // the copy button is the point of the former.
            let is_command = block.is_shell_command();
            let label = if is_command {
                "COMMAND".to_string()
            } else {
                block.lang.as_deref().unwrap_or("code").to_ascii_uppercase()
            };
            let border = if is_command {
                palette.accent
            } else {
                palette.border
            };
            div()
                .rounded_sm()
                .bg(chrome.code)
                .border_1()
                .border_color(rgb(border))
                .p_2()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .mb_1()
                        .child(
                            div()
                                .text_size(px(Typography::CAPTION))
                                .text_color(rgb(palette.muted_text))
                                .child(label),
                        )
                        .child(academy_copy_button(
                            ("academy-copy-block", index),
                            if is_command { "Copy command" } else { "Copy" },
                            block.text.clone(),
                            palette,
                            cx,
                        )),
                )
                .child(
                    div()
                        .font_family("Menlo")
                        .text_size(px(ACADEMY_BODY_TEXT - 2.0))
                        .text_color(rgb(palette.sidebar_text))
                        .child(block.text.clone()),
                )
        }
    }
}

/// Thin two-tone bar: track in panel border color, fill in accent.
fn academy_progress_bar(fraction: f32, palette: UiTheme) -> impl IntoElement {
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
    palette: UiTheme,
    label: &str,
    cx: &mut Context<WorkspacePrototype>,
    action: impl Fn(&mut WorkspacePrototype, &mut Context<WorkspacePrototype>) + 'static,
) -> gpui::Stateful<gpui::Div> {
    button(
        "academy-back",
        label.to_owned(),
        palette,
        ButtonVariant::Ghost,
        ControlSize::Compact,
        cx.listener(move |this, _, _, cx| action(this, cx)),
    )
    .mt_3()
}

fn academy_action(
    label: &str,
    palette: UiTheme,
    cx: &mut Context<WorkspacePrototype>,
    action: impl Fn(&mut WorkspacePrototype, &mut gpui::Window, &mut Context<WorkspacePrototype>)
        + 'static,
) -> gpui::Stateful<gpui::Div> {
    button(
        gpui::SharedString::from(format!("academy-action-{label}")),
        label.to_owned(),
        palette,
        ButtonVariant::Secondary,
        ControlSize::Compact,
        cx.listener(move |this, _, window, cx| action(this, window, cx)),
    )
}

fn academy_output(
    label: &str,
    text: &str,
    palette: UiTheme,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    let mut preview = text.chars().take(8_000).collect::<String>();
    if preview.len() < text.len() {
        preview.push_str("\n… Preview shortened. Copy includes the full captured text.");
    }
    div()
        .w_full()
        .min_w(px(0.0))
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .p_2()
        .child(
            div()
                .flex()
                .justify_between()
                .child(label.to_owned())
                .child(academy_copy_button(
                    gpui::SharedString::from(format!("copy-output-{label}")),
                    "Copy",
                    text.to_owned(),
                    palette,
                    cx,
                )),
        )
        .child(
            div()
                .font_family("Menlo")
                .text_size(px(Typography::BODY))
                .child(preview),
        )
}

fn academy_study_tools(
    course: &Course,
    lesson: &crate::academy::Lesson,
    progress: &AcademyProgress,
    practice: &super::academy_actions::AcademyPracticeState,
    palette: UiTheme,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    let c = course.manifest.id.clone();
    let l = lesson.id.clone();
    let read_label = if progress.is_lesson_read(&c, &l) {
        "Read ✓"
    } else {
        "Mark as read"
    };
    let mut row = div()
        .mt_4()
        .flex()
        .flex_wrap()
        .gap_2()
        .child(academy_action(
            read_label,
            palette,
            cx,
            move |this, _, cx| {
                this.academy_progress.record_lesson_read(&c, &l);
                this.persist_academy_progress();
                cx.notify();
            },
        ));
    let title = format!(
        "{} · {} · {}",
        course.manifest.title, lesson.id, lesson.meta.title
    );
    row = row.child(academy_action(
        "Ask in my notes",
        palette,
        cx,
        move |this, window, cx| {
            this.open_or_activate_surface(super::WorkspaceSurface::Home, window, cx);
            this.notepad.update(cx, |notes, cx| {
                notes.capture_lesson_question(
                    title.clone(),
                    format!("Lesson: {title}\n\nMy question:\n"),
                    window,
                    cx,
                )
            });
        },
    ));
    let language = course.manifest.language.clone();
    row = row.child(academy_action(
        "Recheck setup",
        palette,
        cx,
        move |this, _, cx| {
            this.academy_check_readiness(language.clone(), cx);
        },
    ));
    row = row.child(academy_action(
        "Retry saving progress",
        palette,
        cx,
        |this, _, cx| {
            this.academy_practice.notice = None;
            this.persist_academy_progress();
            cx.notify();
        },
    ));
    let mut panel = div().mt_4().flex().flex_col().gap_2().child(row)
        .child(div().text_size(px(Typography::CONTROL)).child("Reading and verified practice are tracked separately. Pass every exercise to complete this lesson."));
    if practice.busy {
        panel = panel.child("Working…");
    }
    if practice.checking_setup.contains(&course.manifest.language) {
        panel = panel.child("Checking your course tools…");
    }
    if let Some(notice) = &practice.notice {
        panel = panel.child(div().text_color(rgb(palette.warning)).child(notice.clone()));
    }
    if let Some(tools) = practice.readiness.get(&course.manifest.language) {
        for tool in tools {
            panel = panel.child(div().text_size(px(Typography::BODY)).child(format!(
                "{} · {} · {}",
                tool.program,
                if tool.available {
                    "Ready"
                } else if tool.required {
                    "Setup needed"
                } else {
                    "Optional editor help unavailable"
                },
                tool.guidance
            )));
        }
    }
    panel
}
