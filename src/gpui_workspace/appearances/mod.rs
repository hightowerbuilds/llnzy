use gpui::prelude::*;
use gpui::{div, px, rgb, Context, MouseButton, MouseDownEvent};

use crate::{config::Config, theme::builtin_themes};

use super::{
    ErrorLogFilter, SettingsPage, WorkspacePalette, WorkspacePrototype, ACTIVE_TEXT, BORDER,
    EDITOR_BG, MUTED_TEXT, PANEL_BG, QUEUE_GREEN, SIDEBAR_TEXT,
};

mod editor_section;
mod error_log;
mod terminal_section;
mod widgets;

pub(super) use terminal_section::gpui_terminal_background_reference;

use editor_section::editor_appearance_controls;
use error_log::{error_log_clear_modal, settings_error_log_row};
use terminal_section::terminal_appearance_controls;
use widgets::{
    appearance_button, appearance_button_palette, color_strip, control_label_palette,
    effect_toggle_button_palette, glass_fill, metric_readout, metric_row, metric_row_palette,
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
                                .child("Theme, terminal, and editor presentation"),
                        ),
                ),
        )
        .child(
            div()
                .flex_1()
                .flex()
                .gap_3()
                .p_4()
                .overflow_hidden()
                .child(appearance_theme_column(&config, cx))
                .child(appearance_all_controls_column(config, cx)),
        )
}

/// The standalone Appearances surface shows every visual section in one
/// scroll (theme column + terminal/editor/app appearance sections), now that
/// Settings owns the three-tab split.
fn appearance_all_controls_column(
    config: Config,
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
                .child("Appearances"),
        );

    let content = settings_appearances_controls(content, config, None, false, cx);

    content
        .id("appearance-controls-scroll")
        .overflow_y_scroll()
        .scrollbar_width(px(8.0))
}

/// Settings groups controls under Home, Courses, and Terminal.
fn settings_page_nav(page: SettingsPage, cx: &mut Context<WorkspacePrototype>) -> impl IntoElement {
    let mut nav = div().flex().items_center().gap_1();
    for target in SettingsPage::ALL {
        let active = target == page;
        nav = nav.child(appearance_button(
            target.title().to_string(),
            active,
            cx,
            move |this, cx| this.set_settings_page(target, cx),
        ));
    }
    nav
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
                .bg(glass_fill(palette.panel_bg))
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

fn advanced_settings_controls(
    content: gpui::Div,
    joined_tab_limit: usize,
    error_log_expanded: bool,
    error_log_filter: ErrorLogFilter,
    error_entries: Vec<crate::error_log::LogEntry>,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    content
        .child(settings_join_limit_row_palette(
            joined_tab_limit,
            palette,
            cx,
        ))
        .child(settings_error_log_row(
            error_log_expanded,
            error_log_filter,
            error_entries,
            palette,
            cx,
        ))
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
                .bg(glass_fill(if active { 0x183725 } else { 0x242632 }))
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
    page: SettingsPage,
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
        terminal_background_import_error,
        editor_word_wrap,
        joined_tab_limit,
        error_log_expanded,
        error_log_filter,
        error_entries,
        cx,
    );

    let background = crate::gpui_terminal::workspace_background_layer_blurred(&config);
    let has_background = background.is_some();
    let root_content = div()
        .id("settings-surface")
        .relative()
        .flex_1()
        .h_full()
        .flex()
        .flex_col()
        .when(!has_background, |surface| {
            surface.bg(rgb(palette.editor_bg))
        })
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
                .when(has_background, |header| {
                    header.bg(glass_fill(palette.editor_bg))
                })
                .child(settings_page_nav(page, cx)),
        )
        .child(content);

    let mut root = div()
        .relative()
        .size_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .children(background)
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
    page: SettingsPage,
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

    let content = div()
        .flex_1()
        .h_full()
        .flex()
        .flex_col()
        .gap_3()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(glass_fill(palette.panel_bg))
        .p_4()
        .child(
            div()
                .text_size(px(18.0))
                .text_color(rgb(palette.active_text))
                .child(page.title()),
        );

    let content = match page {
        SettingsPage::Home => {
            let content = app_appearance_controls(
                content.child(settings_section_label("APP APPEARANCE")),
                &config,
                cx,
            );
            advanced_settings_controls(
                content.child(settings_section_label("WORKSPACE & DIAGNOSTICS")),
                joined_tab_limit,
                error_log_expanded,
                error_log_filter,
                error_entries,
                palette,
                cx,
            )
        }
        SettingsPage::Courses => {
            let content = editor_appearance_controls(
                content.child(settings_section_label("EDITOR APPEARANCE")),
                config.clone(),
                cx,
            );
            editor_behavior_appearance_controls(
                content.child(settings_section_label("EDITING & MARKDOWN")),
                config,
                editor_word_wrap,
                cx,
            )
        }
        SettingsPage::Terminal => {
            let content = terminal_appearance_controls(
                content.child(settings_section_label("TERMINAL APPEARANCE")),
                config.clone(),
                terminal_background_import_error,
                cx,
            );
            settings_terminal_controls(
                content.child(settings_section_label("TERMINAL BEHAVIOR")),
                &config,
                cx,
            )
        }
    };

    body.child(
        content
            .id("settings-controls-scroll")
            .overflow_y_scroll()
            .scrollbar_width(px(8.0)),
    )
}

/// Combined controls for the standalone Appearances surface.
fn settings_appearances_controls(
    content: gpui::Div,
    config: Config,
    terminal_background_import_error: Option<String>,
    editor_word_wrap: bool,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    content
        .child(settings_section_label("TERMINAL"))
        .child(terminal_appearance_controls(
            div().flex().flex_col().gap_3(),
            config.clone(),
            terminal_background_import_error,
            cx,
        ))
        .child(settings_section_label("EDITOR"))
        .child(editor_appearance_controls(
            div().flex().flex_col().gap_3(),
            config.clone(),
            cx,
        ))
        .child(settings_section_label("EDITOR BEHAVIOR"))
        .child(editor_behavior_appearance_controls(
            div().flex().flex_col().gap_3(),
            config.clone(),
            editor_word_wrap,
            cx,
        ))
        .child(settings_section_label("APP"))
        .child(app_appearance_controls(
            div().flex().flex_col().gap_3(),
            &config,
            cx,
        ))
}

/// Visual presentation rows that used to live under the Settings "App" page.
fn app_appearance_controls(
    content: gpui::Div,
    config: &Config,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    let palette = WorkspacePalette::from_config(config);
    app_theme_section(content, config, palette, cx)
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
}

/// Word wrap + markdown preview controls that used to live under the
/// Settings "Editor" page.
fn editor_behavior_appearance_controls(
    content: gpui::Div,
    config: Config,
    editor_word_wrap: bool,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    let palette = WorkspacePalette::from_config(&config);
    content
        .child(settings_toggle_row(
            "Word wrap",
            "Wraps long source lines in JavaScript, Markdown, and other text files.",
            editor_word_wrap,
            palette,
            cx,
            |this, cx| this.toggle_editor_word_wrap(cx),
        ))
        .child(markdown_appearance_controls(
            div().flex().flex_col().gap_3(),
            config,
            cx,
        ))
}

/// Terminal-behavior (non-visual) settings for the Terminal tab.
fn settings_terminal_controls(
    content: gpui::Div,
    config: &Config,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    content.child(metric_row(
        "Scrollback Lines",
        format!("{}", config.terminal.scrollback_lines),
        cx,
        |this, cx| this.adjust_terminal_scrollback(-1000, cx),
        |this, cx| this.adjust_terminal_scrollback(1000, cx),
    ))
}

fn settings_section_label(label: &'static str) -> impl IntoElement {
    div()
        .mt_2()
        .text_size(px(12.0))
        .text_color(rgb(0x8d94a3))
        .child(label)
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
