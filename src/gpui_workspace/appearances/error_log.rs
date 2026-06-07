use gpui::prelude::*;
use gpui::{div, px, rgb, rgba, Context, FontWeight, MouseButton, MouseDownEvent};

use crate::gpui_workspace::{ErrorLogFilter, WorkspacePalette, WorkspacePrototype};

use super::widgets::appearance_button_palette;

pub(super) fn settings_error_log_row(
    expanded: bool,
    filter: ErrorLogFilter,
    entries: Vec<crate::error_log::LogEntry>,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let (error_count, warn_count) =
        entries
            .iter()
            .fold((0usize, 0usize), |(errs, warns), entry| match entry.level {
                crate::error_log::LogLevel::Error => (errs + 1, warns),
                crate::error_log::LogLevel::Warn => (errs, warns + 1),
                crate::error_log::LogLevel::Info => (errs, warns),
            });

    let count_summary = format!(
        "{} entries  ·  {} errors, {} warnings",
        entries.len(),
        error_count,
        warn_count,
    );

    let chevron = if expanded { "▾" } else { "▸" };

    let header = div()
        .id("error-log-header")
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .px_4()
        .py_3()
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _: &MouseDownEvent, _window, cx| {
                this.toggle_error_log_expanded(cx);
            }),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(rgb(palette.active_text))
                        .child("Error Log"),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(rgb(palette.muted_text))
                        .child(count_summary),
                ),
        )
        .child(
            div()
                .text_size(px(14.0))
                .text_color(rgb(palette.muted_text))
                .child(chevron),
        );

    let mut row = div().flex().flex_col().child(header);

    if expanded {
        let filtered: Vec<_> = entries
            .into_iter()
            .filter(|entry| filter.includes(entry.level))
            .collect();
        let has_entries = !filtered.is_empty();

        let filter_group = div()
            .flex()
            .items_center()
            .gap_1()
            .child(error_log_filter_button(
                "All",
                ErrorLogFilter::All,
                filter,
                palette,
                cx,
            ))
            .child(error_log_filter_button(
                "Warn+",
                ErrorLogFilter::WarnAndError,
                filter,
                palette,
                cx,
            ))
            .child(error_log_filter_button(
                "Errors",
                ErrorLogFilter::ErrorOnly,
                filter,
                palette,
                cx,
            ));

        let actions = div()
            .flex()
            .items_center()
            .gap_2()
            .child(appearance_button_palette(
                "Copy All".to_string(),
                false,
                palette,
                cx,
                |this, cx| {
                    this.copy_error_log(cx);
                },
            ))
            .child(appearance_button_palette(
                "Clear".to_string(),
                false,
                palette,
                cx,
                |this, cx| {
                    this.request_clear_error_log(cx);
                },
            ));

        let toolbar = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .px_4()
            .py_2()
            .border_t_1()
            .border_color(rgb(palette.border))
            .child(filter_group)
            .child(actions);
        row = row.child(toolbar);
        if has_entries {
            row = row.child(error_log_list(filtered, palette, cx));
        } else {
            let empty_label = match filter {
                ErrorLogFilter::All => "No errors recorded this session.",
                ErrorLogFilter::WarnAndError => "No warnings or errors match the filter.",
                ErrorLogFilter::ErrorOnly => "No errors match the filter.",
            };
            row = row.child(
                div()
                    .px_4()
                    .py_6()
                    .border_t_1()
                    .border_color(rgb(palette.border))
                    .text_size(px(12.0))
                    .text_color(rgb(palette.muted_text))
                    .child(empty_label),
            );
        }
    }

    row
}

fn error_log_filter_button(
    label: &'static str,
    target: ErrorLogFilter,
    active_filter: ErrorLogFilter,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let active = target == active_filter;
    appearance_button_palette(label.to_string(), active, palette, cx, move |this, cx| {
        this.set_error_log_filter(target, cx);
    })
}

fn error_log_list(
    entries: Vec<crate::error_log::LogEntry>,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let mut list = div()
        .id("error-log-list")
        .flex()
        .flex_col()
        .max_h(px(420.0))
        .overflow_y_scroll()
        .scrollbar_width(px(8.0))
        .border_t_1()
        .border_color(rgb(palette.border));

    // Render newest first so users see the most recent failure at the top.
    // Use enumerate after reversing so each row gets a stable id for
    // GPUI's interactive element book-keeping.
    for (idx, entry) in entries.into_iter().rev().enumerate() {
        list = list.child(error_log_entry_row(idx, entry, palette, cx));
    }
    list
}

fn error_log_entry_row(
    idx: usize,
    entry: crate::error_log::LogEntry,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let [lr, lg, lb] = entry.level.color();
    let level_color = ((lr as u32) << 16) | ((lg as u32) << 8) | (lb as u32);
    let level_label = entry.level.label().trim().to_string();

    let timestamp = entry.timestamp_label();

    let module = entry
        .module
        .clone()
        .unwrap_or_else(|| "<unknown module>".to_string());

    let source_hint = match (entry.file.as_ref(), entry.line) {
        (Some(file), Some(line)) => Some(format!("{file}:{line}")),
        (Some(file), None) => Some(file.clone()),
        _ => None,
    };

    // Capture the source coordinates before the row consumes `entry`
    // below — we use them in the click handler to jump to the file.
    let jump_target = match (entry.file.as_ref(), entry.line) {
        (Some(file), Some(line)) => Some((file.clone(), line)),
        _ => None,
    };

    let mut metadata_row = div()
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .text_size(px(10.0))
                .text_color(rgb(level_color))
                .child(level_label),
        )
        .child(
            div()
                .text_size(px(10.0))
                .text_color(rgb(palette.muted_text))
                .whitespace_nowrap()
                .child(timestamp),
        )
        .child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(palette.sidebar_text))
                .child(module),
        );

    if let Some(hint) = source_hint {
        metadata_row = metadata_row.child(
            div()
                .text_size(px(10.0))
                .text_color(rgb(palette.muted_text))
                .child(hint),
        );
    }

    let mut row = div()
        .id(("error-log-row", idx))
        .flex()
        .flex_col()
        .gap_1()
        .px_4()
        .py_2()
        .border_b_1()
        .border_color(rgb(palette.border))
        .child(metadata_row)
        .child(
            div()
                .text_size(px(12.0))
                .text_color(rgb(palette.active_text))
                .child(entry.message),
        );

    if let Some((file, line)) = jump_target {
        row = row.cursor_pointer().on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                this.open_error_log_source(file.clone(), line, window, cx);
            }),
        );
    }

    row
}

/// Scrim + centered card asking the user to confirm clearing the
/// persisted error log. Same look-and-feel as Stacker's delete modal.
pub(super) fn error_log_clear_modal(
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let scrim = div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .bg(rgba(0x00000099))
        .flex()
        .items_center()
        .justify_center()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _: &MouseDownEvent, _window, cx| {
                this.cancel_clear_error_log(cx);
            }),
        );

    let card = div()
        .w(px(380.0))
        .flex()
        .flex_col()
        .gap_3()
        .p_5()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.panel_bg))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_this, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
            }),
        )
        .child(
            div()
                .text_size(px(15.0))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(palette.active_text))
                .child("Clear error log"),
        )
        .child(
            div()
                .text_size(px(13.0))
                .text_color(rgb(palette.muted_text))
                .child(
                    "Drop every in-memory entry and truncate the persisted log on disk. \
                     Past sessions will no longer replay. This cannot be undone.",
                ),
        )
        .child(
            div()
                .flex()
                .justify_end()
                .gap_2()
                .pt_2()
                .child(appearance_button_palette(
                    "Cancel".to_string(),
                    false,
                    palette,
                    cx,
                    |this, cx| {
                        this.cancel_clear_error_log(cx);
                    },
                ))
                .child(appearance_button_palette(
                    "Clear".to_string(),
                    true,
                    palette,
                    cx,
                    |this, cx| {
                        this.confirm_clear_error_log(cx);
                    },
                )),
        );

    scrim.child(card)
}
