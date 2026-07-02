use gpui::prelude::*;
use gpui::{div, px, rgb, Context, MouseButton, MouseDownEvent};

use crate::{config::Config, sketch::SketchToolbarPosition, theme::builtin_themes};

use super::{
    AppearancePage, ErrorLogFilter, WorkspacePalette, WorkspacePrototype, ACTIVE_TEXT, BORDER,
    EDITOR_BG, MUTED_TEXT, PANEL_BG, QUEUE_GREEN, SIDEBAR_TEXT,
};

mod editor_section;
mod error_log;
mod shader_palettes;
mod terminal_section;
mod widgets;

pub(super) use terminal_section::gpui_terminal_background_reference;

use editor_section::{editor_appearance_controls, editor_settings_controls};
use error_log::{error_log_clear_modal, settings_error_log_row};
use terminal_section::terminal_appearance_controls;
use widgets::{
    appearance_button, appearance_button_palette, color_strip, control_label,
    control_label_palette, effect_toggle_button_palette, metric_readout, metric_row,
    metric_row_palette,
};

// Monospace families. `None` means "use the system default", which is what
// gpui hands the terminal when `config.font_family` is unset.
pub(super) const TERMINAL_MONO_FONT_CHOICES: &[(&str, Option<&str>)] = &[
    ("Default", None),
    ("Menlo", Some("Menlo")),
    ("Courier", Some("Courier")),
];

// Proportional families used in Display mode. The flow renderer shapes each
// row as a single line so glyphs use their natural advance widths.
pub(super) const TERMINAL_DISPLAY_FONT_CHOICES: &[(&str, &str)] = &[
    ("Atkinson Hyperlegible", "Atkinson Hyperlegible"),
    ("Helvetica", "Helvetica"),
    ("Georgia", "Georgia"),
    ("Palatino", "Palatino"),
    ("Verdana", "Verdana"),
];

/// Whether `family` is one of the curated Display-mode (proportional)
/// families. Used when switching modes to clear a stranded font selection.
pub(super) fn is_display_font(family: &str) -> bool {
    TERMINAL_DISPLAY_FONT_CHOICES
        .iter()
        .any(|(_, candidate)| *candidate == family)
}

pub(super) fn appearances_surface(
    config: Config,
    page: AppearancePage,
    sketch_toolbar_position: SketchToolbarPosition,
    terminal_background_import_error: Option<String>,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    div()
        .flex_1()
        .h_full()
        .flex()
        .flex_col()
        .bg(rgb(EDITOR_BG))
        .child(
            div()
                .h(px(52.0))
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .px_4()
                .border_b_1()
                .border_color(rgb(BORDER))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_size(px(18.0))
                                .text_color(rgb(ACTIVE_TEXT))
                                .child("Appearances"),
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(rgb(MUTED_TEXT))
                                .child("Theme, terminal, editor, and canvas presentation"),
                        ),
                )
                .child(appearance_page_nav(page, cx)),
        )
        .child(
            div()
                .flex_1()
                .flex()
                .gap_3()
                .p_4()
                .overflow_hidden()
                .child(appearance_theme_column(&config, cx))
                .child(appearance_controls_column(
                    config,
                    page,
                    sketch_toolbar_position,
                    terminal_background_import_error,
                    cx,
                )),
        )
}

fn appearance_page_nav(
    page: AppearancePage,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let mut nav = div().flex().items_center().gap_1();
    for target in AppearancePage::ALL {
        nav = nav.child(appearance_page_button(target, page, cx));
    }
    nav
}

fn appearance_page_button(
    target: AppearancePage,
    page: AppearancePage,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let active = target == page;
    appearance_button(target.title().to_string(), active, cx, move |this, cx| {
        this.set_appearance_page(target, cx);
    })
}

fn appearance_theme_column(
    config: &Config,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let mut themes = div()
        .id("appearance-themes-scroll")
        .w(px(320.0))
        .h_full()
        .flex()
        .flex_col()
        .gap_2()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(PANEL_BG))
        .p_3()
        .overflow_y_scroll()
        .scrollbar_width(px(8.0))
        .child(
            div()
                .text_size(px(13.0))
                .text_color(rgb(MUTED_TEXT))
                .child("THEMES"),
        )
        .child(color_strip([
            config.colors.background,
            config.colors.foreground,
            config.colors.cursor,
            config.colors.selection,
            config.colors.ansi[1],
            config.colors.ansi[2],
            config.colors.ansi[4],
            config.colors.ansi[5],
        ]));

    for theme in builtin_themes().into_iter().take(6) {
        let theme_name = theme.name.clone();
        themes = themes.child(
            div()
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .rounded_sm()
                .bg(rgb(0x252935))
                .p_2()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_size(px(13.0))
                                .text_color(rgb(ACTIVE_TEXT))
                                .child(theme.name.clone()),
                        )
                        .child(color_strip([
                            theme.colors.background,
                            theme.colors.foreground,
                            theme.colors.cursor,
                            theme.colors.ansi[1],
                        ])),
                )
                .child(appearance_button(
                    "Apply".to_string(),
                    false,
                    cx,
                    move |this, cx| {
                        this.apply_builtin_theme(&theme_name, cx);
                    },
                )),
        );
    }

    themes
}

fn app_theme_section(
    content: gpui::Div,
    config: &Config,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    let mut section = div()
        .w_full()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_size(px(13.0))
                .text_color(rgb(palette.muted_text))
                .child("THEMES"),
        )
        .child(color_strip([
            config.colors.background,
            config.colors.foreground,
            config.colors.cursor,
            config.colors.selection,
            config.colors.ansi[1],
            config.colors.ansi[2],
            config.colors.ansi[4],
            config.colors.ansi[5],
        ]));

    for theme in builtin_themes() {
        let active = theme.colors.background == config.colors.background
            && theme.colors.foreground == config.colors.foreground
            && theme.colors.cursor == config.colors.cursor;
        let theme_name = theme.name.clone();
        section = section.child(
            div()
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .rounded_sm()
                .border_1()
                .border_color(rgb(if active {
                    palette.queue_green
                } else {
                    palette.border
                }))
                .bg(rgb(palette.panel_bg))
                .p_3()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_size(px(13.0))
                                .text_color(rgb(palette.active_text))
                                .child(theme.name.clone()),
                        )
                        .child(color_strip([
                            theme.colors.background,
                            theme.colors.foreground,
                            theme.colors.cursor,
                            theme.colors.ansi[1],
                        ])),
                )
                .child(appearance_button_palette(
                    if active {
                        "Applied".to_string()
                    } else {
                        "Apply".to_string()
                    },
                    active,
                    palette,
                    cx,
                    move |this, cx| {
                        this.apply_builtin_theme(&theme_name, cx);
                    },
                )),
        );
    }

    content.child(section)
}

fn appearance_controls_column(
    config: Config,
    page: AppearancePage,
    sketch_toolbar_position: SketchToolbarPosition,
    terminal_background_import_error: Option<String>,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let content = div()
        .flex_1()
        .h_full()
        .flex()
        .flex_col()
        .gap_3()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(0x15151c))
        .p_4()
        .child(
            div()
                .text_size(px(18.0))
                .text_color(rgb(ACTIVE_TEXT))
                .child(page.title()),
        );

    let content = match page {
        AppearancePage::Terminal => {
            terminal_appearance_controls(content, config, terminal_background_import_error, cx)
        }
        AppearancePage::Editor => editor_appearance_controls(content, config, cx),
        AppearancePage::Stacker => stacker_settings_controls(content),
        AppearancePage::Sketch => sketch_appearance_controls(content, sketch_toolbar_position, cx),
        AppearancePage::App => {
            let palette = WorkspacePalette::from_config(&config);
            app_settings_controls(content, config, 2, palette, cx)
        }
        AppearancePage::Advanced => advanced_settings_controls(
            content,
            false,
            ErrorLogFilter::All,
            Vec::new(),
            WorkspacePalette::from_config(&config),
            cx,
        ),
    };

    content
        .id("appearance-controls-scroll")
        .overflow_y_scroll()
        .scrollbar_width(px(8.0))
}

fn sketch_appearance_controls(
    content: gpui::Div,
    toolbar_position: SketchToolbarPosition,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    content.child(
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(control_label("Toolbar"))
            .child(appearance_button(
                "Top".to_string(),
                toolbar_position == SketchToolbarPosition::Top,
                cx,
                |this, cx| this.set_sketch_toolbar_position(SketchToolbarPosition::Top, cx),
            ))
            .child(appearance_button(
                "Left".to_string(),
                toolbar_position == SketchToolbarPosition::Left,
                cx,
                |this, cx| this.set_sketch_toolbar_position(SketchToolbarPosition::Left, cx),
            ))
            .child(appearance_button(
                "Right".to_string(),
                toolbar_position == SketchToolbarPosition::Right,
                cx,
                |this, cx| this.set_sketch_toolbar_position(SketchToolbarPosition::Right, cx),
            )),
    )
}

fn app_settings_controls(
    content: gpui::Div,
    config: Config,
    joined_tab_limit: usize,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    app_theme_section(content, &config, palette, cx)
        .child(metric_row_palette(
            "App Font Size",
            format!("{:.0}px", config.font_size),
            palette,
            cx,
            |this, cx| this.adjust_font_size(-1.0, cx),
            |this, cx| this.adjust_font_size(1.0, cx),
        ))
        .child(metric_row_palette(
            "Selection Alpha",
            format!("{:.0}%", config.colors.selection_alpha * 100.0),
            palette,
            cx,
            |this, cx| this.adjust_selection_alpha(-0.05, cx),
            |this, cx| this.adjust_selection_alpha(0.05, cx),
        ))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(control_label_palette("Time-of-Day Warmth", palette))
                .child(appearance_button_palette(
                    if config.time_of_day_enabled {
                        "On".to_string()
                    } else {
                        "Off".to_string()
                    },
                    config.time_of_day_enabled,
                    palette,
                    cx,
                    |this, cx| this.toggle_time_of_day(cx),
                )),
        )
        .child(settings_join_limit_row_palette(
            joined_tab_limit,
            palette,
            cx,
        ))
}

fn stacker_settings_controls(content: gpui::Div) -> gpui::Div {
    content
        .child(metric_readout("Prompt Queue", "Default".to_string()))
        .child(metric_readout("Formatting", "Default".to_string()))
}

fn advanced_settings_controls(
    content: gpui::Div,
    error_log_expanded: bool,
    error_log_filter: ErrorLogFilter,
    error_entries: Vec<crate::error_log::LogEntry>,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    content.child(settings_error_log_row(
        error_log_expanded,
        error_log_filter,
        error_entries,
        palette,
        cx,
    ))
}

pub(super) fn markdown_appearance_controls(
    content: gpui::Div,
    config: Config,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    let editor_font = config
        .editor
        .font_size
        .unwrap_or((config.font_size - 2.0).max(10.0));
    content
        .child(markdown_preview_style_controls(&config, cx))
        .child(metric_row(
            "Preview Font Size",
            format!("{editor_font:.0}px"),
            cx,
            |this, cx| this.adjust_editor_font_size(-1.0, cx),
            |this, cx| this.adjust_editor_font_size(1.0, cx),
        ))
        .child(metric_row(
            "Preview Line Height",
            format!("{:.2}x", config.editor.line_height),
            cx,
            |this, cx| this.adjust_editor_line_height(-0.05, cx),
            |this, cx| this.adjust_editor_line_height(0.05, cx),
        ))
        .child(metric_readout(
            "Preview Width",
            "Matches editor pane or split pane".to_string(),
        ))
        .child(
            div()
                .mt_2()
                .text_size(px(13.0))
                .text_color(rgb(SIDEBAR_TEXT))
                .child("Markdown preview uses editor font, line height, and theme colors while keeping Source, Preview, and Split mode state separate."),
        )
}

fn markdown_preview_style_controls(
    config: &Config,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let active_style = config.editor.markdown_preview_style;
    let mut buttons = div().flex().flex_wrap().gap_2();
    for style in crate::config::MarkdownPreviewStyle::all() {
        let active = style == active_style;
        buttons = buttons.child(
            div()
                .h(px(30.0))
                .px_3()
                .flex()
                .items_center()
                .justify_center()
                .rounded_sm()
                .border_1()
                .border_color(rgb(if active { 0x47785f } else { BORDER }))
                .bg(rgb(if active { 0x183725 } else { 0x242632 }))
                .text_size(px(12.0))
                .text_color(rgb(if active { QUEUE_GREEN } else { SIDEBAR_TEXT }))
                .cursor_pointer()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                        this.set_markdown_preview_style(style, cx);
                    }),
                )
                .child(style.label()),
        );
    }

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_size(px(12.0))
                .text_color(rgb(MUTED_TEXT))
                .child("Preview Style"),
        )
        .child(buttons)
}

#[expect(
    clippy::too_many_arguments,
    reason = "Settings surface wires several independent workspace preferences into GPUI"
)]
pub(super) fn settings_surface(
    config: Config,
    page: AppearancePage,
    sketch_toolbar_position: SketchToolbarPosition,
    terminal_background_import_error: Option<String>,
    editor_word_wrap: bool,
    joined_tab_limit: usize,
    error_log_expanded: bool,
    error_log_filter: ErrorLogFilter,
    pending_clear_error_log: bool,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let palette = WorkspacePalette::from_config(&config);
    let error_entries = crate::error_log::global().recent(1000);

    let content = settings_controls_column(
        config.clone(),
        page,
        sketch_toolbar_position,
        terminal_background_import_error,
        editor_word_wrap,
        joined_tab_limit,
        error_log_expanded,
        error_log_filter,
        error_entries,
        cx,
    );

    let root_content = div()
        .id("settings-surface")
        .flex_1()
        .h_full()
        .flex()
        .flex_col()
        .bg(rgb(palette.editor_bg))
        .child(
            div()
                .h(px(52.0))
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .px_4()
                .border_b_1()
                .border_color(rgb(palette.border))
                .child(
                    div().flex().items_center().gap_2().child(
                        div()
                            .text_size(px(18.0))
                            .text_color(rgb(palette.active_text))
                            .child("Settings"),
                    ),
                )
                .child(appearance_page_nav(page, cx)),
        )
        .child(content);

    let mut root = div()
        .relative()
        .size_full()
        .flex()
        .flex_col()
        .child(root_content);

    if pending_clear_error_log {
        root = root.child(error_log_clear_modal(palette, cx));
    }

    root
}

#[expect(
    clippy::too_many_arguments,
    reason = "Settings controls dispatch a flat view model across sub-tabs"
)]
fn settings_controls_column(
    config: Config,
    page: AppearancePage,
    sketch_toolbar_position: SketchToolbarPosition,
    terminal_background_import_error: Option<String>,
    editor_word_wrap: bool,
    joined_tab_limit: usize,
    error_log_expanded: bool,
    error_log_filter: ErrorLogFilter,
    error_entries: Vec<crate::error_log::LogEntry>,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let palette = WorkspacePalette::from_config(&config);
    let body = div()
        .flex_1()
        .h_full()
        .flex()
        .gap_3()
        .p_4()
        .overflow_hidden();

    if page == AppearancePage::App {
        let content = div()
            .flex_1()
            .h_full()
            .flex()
            .flex_col()
            .gap_4()
            .p_4()
            .child(
                div()
                    .text_size(px(18.0))
                    .text_color(rgb(palette.active_text))
                    .child(page.title()),
            );
        let content = app_settings_controls(content, config, joined_tab_limit, palette, cx);
        return body.child(
            content
                .id("settings-controls-scroll")
                .overflow_y_scroll()
                .scrollbar_width(px(8.0)),
        );
    }

    let content = div()
        .flex_1()
        .h_full()
        .flex()
        .flex_col()
        .gap_3()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.panel_bg))
        .p_4()
        .child(
            div()
                .text_size(px(18.0))
                .text_color(rgb(palette.active_text))
                .child(page.title()),
        );

    let content = match page {
        AppearancePage::Terminal => {
            terminal_appearance_controls(content, config, terminal_background_import_error, cx)
        }
        AppearancePage::Editor => editor_settings_controls(content, config, editor_word_wrap, cx),
        AppearancePage::Stacker => stacker_settings_controls(content),
        AppearancePage::Sketch => sketch_appearance_controls(content, sketch_toolbar_position, cx),
        AppearancePage::App => {
            app_settings_controls(content, config, joined_tab_limit, palette, cx)
        }
        AppearancePage::Advanced => advanced_settings_controls(
            content,
            error_log_expanded,
            error_log_filter,
            error_entries,
            palette,
            cx,
        ),
    };

    body.child(
        content
            .id("settings-controls-scroll")
            .overflow_y_scroll()
            .scrollbar_width(px(8.0)),
    )
}

fn settings_join_limit_row_palette(
    current_limit: usize,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let mut choices = div().flex().items_center().gap_1();
    for limit in [2_u8, 3, 4] {
        choices = choices.child(appearance_button_palette(
            limit.to_string(),
            current_limit == limit as usize,
            palette,
            cx,
            move |this, cx| this.set_joined_tab_limit(limit, cx),
        ));
    }

    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .px_4()
        .py_3()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(rgb(palette.active_text))
                        .child("Joined tab limit"),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(rgb(palette.muted_text))
                        .child(
                            "Choose whether joined tab groups can hold two, three, or four tabs.",
                        ),
                ),
        )
        .child(choices)
}

pub(super) fn settings_toggle_row(
    title: &'static str,
    description: &'static str,
    active: bool,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
    on_click: impl Fn(&mut WorkspacePrototype, &mut Context<WorkspacePrototype>) + 'static,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .px_4()
        .py_3()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(rgb(palette.active_text))
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(rgb(palette.muted_text))
                        .child(description),
                ),
        )
        .child(effect_toggle_button_palette(
            "", active, palette, cx, on_click,
        ))
}
