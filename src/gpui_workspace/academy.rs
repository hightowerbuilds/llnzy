//! Academy course picker. This is the launch surface for LLNZY Code
//! Academy: it lists the two launch courses described in the future
//! roadmaps (`academy-course-javascript-typescript.md` and
//! `academy-course-rust.md`), routes a click to the workspace's
//! `select_academy_course`, and previews the selected course's module
//! list. It is a picker only — no course runtime, no lessons rendered
//! here; the panel says so explicitly.

use gpui::prelude::*;
use gpui::{div, px, rgb, Context, MouseButton, MouseDownEvent};

use crate::config::Config;

use super::{WorkspacePalette, WorkspacePrototype};

/// Identifies one Academy course. Mirrors the course ids the roadmaps
/// define (`js-ts`, `rust`) but as a Rust-friendly enum the workspace can
/// store and compare.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AcademyCourseId {
    JavaScriptTypeScript,
    Rust,
}

/// Everything the picker card and the preview panel need to draw one
/// course. All copy is lifted from the roadmap docs so the surface never
/// drifts from the plan.
struct AcademyCourse {
    id: AcademyCourseId,
    title: &'static str,
    badge: &'static str,
    description: &'static str,
    lessons: &'static str,
    project: &'static str,
    prerequisite: &'static str,
}

/// The launch catalog, in display order.
fn academy_courses() -> [AcademyCourse; 2] {
    [
        AcademyCourse {
            id: AcademyCourseId::JavaScriptTypeScript,
            title: "JavaScript / TypeScript",
            badge: "JS → TS",
            description: "Act I teaches JavaScript on the real node \
runtime; Act II re-covers the same ground in TypeScript and goes deeper. \
Ends with the final project ledger, a zero-dependency TS CLI.",
            lessons: "18 lessons + final project",
            project: "ledger",
            prerequisite: "requires node >= 20",
        },
        AcademyCourse {
            id: AcademyCourseId::Rust,
            title: "Rust",
            badge: "RUST",
            description: "Ownership and moves, borrows and references, \
structs and enums, pattern matching, Option and Result, collections, \
iterators and closures, traits and generics, testing. Ends with the final \
project sift, a std-only search CLI.",
            lessons: "18 lessons + final project",
            project: "sift",
            prerequisite: "requires rustup",
        },
    ]
}

/// One preview group: a heading plus the roadmap's lesson titles.
struct AcademyModuleGroup {
    heading: &'static str,
    modules: &'static [&'static str],
}

/// Act I / Act II module titles from the JS/TS roadmap.
fn js_ts_module_groups() -> [AcademyModuleGroup; 2] {
    [
        AcademyModuleGroup {
            heading: "ACT I — JAVASCRIPT (LESSONS 0–9)",
            modules: &[
                "L0 · Toolchain and workspace verify",
                "L1 · Values, types, coercion",
                "L2 · Control flow",
                "L3 · Functions, scope, closures",
                "L4 · Arrays and objects",
                "L5 · map / filter / reduce",
                "L6 · Error handling",
                "L7 · Modules",
                "L8 · Promises and async/await",
                "L9 · Deterministic async patterns",
            ],
        },
        AcademyModuleGroup {
            heading: "ACT II — TYPESCRIPT (LESSONS 10–17)",
            modules: &[
                "L10 · Why TypeScript",
                "L11 · Annotations and inference",
                "L12 · Narrowing",
                "L13 · Unions, literals, exhaustive switch",
                "L14 · interface vs type",
                "L15 · Generics and utility types",
                "L16 · Typing async and typed boundaries",
                "L17 · Typed refactor of an Act I artifact",
            ],
        },
    ]
}

/// Lesson titles from the Rust roadmap, lessons 0–17.
fn rust_module_groups() -> [AcademyModuleGroup; 1] {
    [AcademyModuleGroup {
        heading: "MODULES (LESSONS 0–17)",
        modules: &[
            "L0 · Toolchain & Hello Cargo",
            "L1 · Variables & Mutability",
            "L2 · Types & Control Flow",
            "L3 · Functions",
            "L4 · Ownership & Moves",
            "L5 · Borrows & References",
            "L6 · Slices & Strings",
            "L7 · Structs",
            "L8 · Enums",
            "L9 · Pattern Matching",
            "L10 · Option",
            "L11 · Result & ?",
            "L12 · Collections",
            "L13 · Iterators & Closures",
            "L14 · Traits & Generics",
            "L15 · Trait Objects",
            "L16 · Modules & Layout",
            "L17 · Testing",
        ],
    }]
}

pub(super) fn academy_surface(
    config: &Config,
    selected_course: Option<AcademyCourseId>,
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
        .overflow_y_scroll()
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
                    "Pick a course. Lessons run in the editor and the \
real terminal you already use.",
                ),
        );

    match selected_course {
        Some(id) => {
            content = content
                .child(academy_course_detail(id, palette, cx))
                .child(academy_preview_note(palette));
        }
        None => {
            let mut grid = div().flex().flex_wrap().gap_3().justify_center();
            for course in academy_courses() {
                grid = grid.child(academy_course_card(course, palette, cx));
            }
            content = content.child(grid);
        }
    }

    content
}

/// One clickable course card. The whole card is the click target; the
/// "Open" pill is an affordance, not a separate button.
fn academy_course_card(
    course: AcademyCourse,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let id = course.id;
    div()
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
                this.select_academy_course(id, cx);
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
                        .text_size(px(15.0))
                        .text_color(rgb(palette.active_text))
                        .child(course.title),
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
                        .child(course.badge),
                ),
        )
        .child(
            div()
                .mt_2()
                .text_size(px(12.0))
                .text_color(rgb(palette.sidebar_text))
                .child(course.description),
        )
        .child(
            div()
                .mt_3()
                .flex()
                .flex_col()
                .gap_1()
                .child(academy_meta_row(palette, course.lessons))
                .child(academy_meta_row(
                    palette,
                    format!("Final project: {}", course.project),
                ))
                .child(academy_meta_row(palette, course.prerequisite)),
        )
        .child(
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
        )
}

/// A single muted "label: value" style meta line for a card.
fn academy_meta_row(palette: WorkspacePalette, line: impl Into<String>) -> impl IntoElement {
    div()
        .text_size(px(11.0))
        .text_color(rgb(palette.muted_text))
        .child(line.into())
}

/// The selected-course preview panel: back affordance, course title, and
/// the roadmap's module titles grouped the way the roadmap groups them.
fn academy_course_detail(
    id: AcademyCourseId,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let course = academy_courses()
        .into_iter()
        .find(|course| course.id == id)
        .expect("academy_surface only passes ids from the catalog");

    let mut panel = div()
        .w(px(720.0))
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
                        .text_size(px(16.0))
                        .text_color(rgb(palette.active_text))
                        .child(course.title),
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
                        .child(course.badge),
                ),
        )
        .child(
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
                    cx.listener(|this, _: &MouseDownEvent, _window, cx| {
                        this.clear_academy_course_selection(cx);
                    }),
                )
                .child("← Back to courses"),
        );

    let groups: Vec<AcademyModuleGroup> = match id {
        AcademyCourseId::JavaScriptTypeScript => js_ts_module_groups().into_iter().collect(),
        AcademyCourseId::Rust => rust_module_groups().into_iter().collect(),
    };

    for group in groups {
        let mut list = div().mt_4().flex().flex_col().gap_1().child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(palette.muted_text))
                .child(group.heading),
        );
        for module in group.modules {
            list = list.child(
                div()
                    .w_full()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .px_2()
                    .py_1()
                    .text_size(px(12.0))
                    .text_color(rgb(palette.sidebar_text))
                    .child(module.to_string()),
            );
        }
        panel = panel.child(list);
    }

    panel
}

/// The muted "this is a preview" disclaimer under the detail panel.
fn academy_preview_note(palette: WorkspacePalette) -> impl IntoElement {
    div()
        .mt_3()
        .mb_6()
        .w(px(720.0))
        .text_size(px(11.0))
        .text_color(rgb(palette.muted_text))
        .child(
            "Preview — the course runtime ships with the Academy \
roadmap: lessons, exercises, and checks arrive next.",
        )
}
