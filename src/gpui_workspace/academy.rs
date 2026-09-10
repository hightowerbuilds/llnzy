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
use gpui::{
    div, img, px, relative, rgb, rgba, ClipboardItem, Context, MouseButton, MouseDownEvent,
    RenderImage, Rgba,
};

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

/// What sits behind the Academy surface this frame.
///
/// `Blurred` is the surface's own layer. `Sharp` happens when the Academy
/// is joined to a terminal: the two panes share one image so it reads as
/// continuous, and the terminal has no business being blurred, so the
/// Academy takes the sharp image and leans on panel opacity instead.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum AcademyBackdrop {
    None,
    Sharp,
    Blurred,
}

/// Panel fills for the Academy, resolved once per render.
///
/// The surface is reading material: prose, code, and cards stacked on a
/// flat theme fill. Put a photo behind that flat fill and you see nothing,
/// so over an image the fills go translucent instead. How translucent
/// depends on what the image is doing: a blurred backdrop carries no
/// competing detail, so the panels can be sheerer and let more of it
/// through, which is the whole point of blurring it. A sharp photo needs
/// more cover to keep text legible. Code blocks sit above panels in both
/// cases, because misread code costs more than a muted photo.
#[derive(Clone, Copy)]
struct AcademyChrome {
    panel: Rgba,
    code: Rgba,
}

impl AcademyChrome {
    fn new(palette: WorkspacePalette, backdrop: AcademyBackdrop) -> Self {
        match backdrop {
            AcademyBackdrop::None => Self {
                panel: rgb(palette.panel_bg),
                code: rgb(palette.editor_bg),
            },
            AcademyBackdrop::Sharp => Self {
                panel: translucent(palette.panel_bg, 0xd9),
                code: translucent(palette.editor_bg, 0xe6),
            },
            AcademyBackdrop::Blurred => Self {
                panel: translucent(palette.panel_bg, 0xbf),
                code: translucent(palette.editor_bg, 0xd9),
            },
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
    let bytes: &'static [u8] = match language.to_ascii_lowercase().as_str() {
        "rust" => include_bytes!("../../assets/academy/rust-logo.png"),
        "javascript" | "js" => {
            include_bytes!("../../assets/academy/javascript-logo.png")
        }
        _ => return None,
    };
    render_png(bytes)
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

/// `backdrop` describes the image behind this surface, mounted either on
/// its own pane or shared across a joined group. Anything but `None` means
/// the surface drops its full-bleed fill and lets the panels carry the
/// contrast instead.
pub(super) fn academy_surface(
    config: &Config,
    academy: AcademyContext,
    backdrop: AcademyBackdrop,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let palette = WorkspacePalette::from_config(config);
    let chrome = AcademyChrome::new(palette, backdrop);

    let mut content = div()
        .id("academy-surface-scroll")
        .flex_1()
        .h_full()
        .flex()
        .flex_col()
        .items_center()
        .pt(px(48.0))
        .overflow_y_scroll();

    if backdrop == AcademyBackdrop::None {
        content = content.bg(rgb(palette.editor_bg));
    }

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
                        chrome,
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
                            chrome,
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
                grid = grid.child(academy_course_card(
                    course,
                    &academy.progress,
                    palette,
                    chrome,
                    cx,
                ));
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

    let mut card = div()
        .w(px(320.0))
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(chrome.panel)
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
        .w(px(720.0))
        .mb_6()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.accent))
        .bg(chrome.panel)
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
    chrome: AcademyChrome,
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

    let mut body = div().mt_2().flex().flex_col().gap_3();
    for (index, block) in parse_markdown_blocks(&lesson.body).iter().enumerate() {
        body = body.child(academy_markdown_block(index, block, palette, chrome, cx));
    }
    panel = panel.child(body);

    for (index, exercise) in lesson.meta.exercises.iter().enumerate() {
        panel = panel.child(academy_exercise(index, exercise, palette, cx));
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
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
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
            // The grading command is something the reader runs, so it gets
            // the same copy affordance as a command in the prose.
            div()
                .mt_2()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .text_size(px(10.0))
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
const ACADEMY_BODY_TEXT: f32 = 16.0;

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
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex_none()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .px_2()
        .py(px(1.0))
        .text_size(px(11.0))
        .text_color(rgb(palette.sidebar_text))
        .cursor_pointer()
        .hover(|style| style.border_color(rgb(palette.accent)))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |_this, _: &MouseDownEvent, _window, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(payload.clone()));
            }),
        )
        .child(label.to_string())
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
    palette: WorkspacePalette,
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
            .child(block.text.clone()),
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
                                .text_size(px(10.0))
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
