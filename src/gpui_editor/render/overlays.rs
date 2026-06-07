use super::super::*;

pub(super) fn language_panel_overlay(
    panel: GpuiLspPanel,
    cx: &mut Context<EditorPrototype>,
) -> impl IntoElement {
    let selected = panel.selected;
    let anchor = panel.anchor.unwrap_or(GpuiLspPanelAnchor {
        top: px(44.0),
        left: px(92.0),
    });
    let items = panel.items.into_iter().take(14).enumerate().fold(
        div().flex().flex_col().gap_1(),
        |column, (index, item)| {
            let is_actionable = !matches!(&item.action, GpuiLspPanelAction::None);
            let is_selected = index == selected;
            column.child(
                div()
                    .w_full()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_size(px(11.0))
                    .text_color(rgb(EDITOR_TEXT_FG))
                    .px_1()
                    .py_1()
                    .rounded_sm()
                    .bg(if is_selected {
                        rgb(0x2d374a)
                    } else {
                        rgba(0x00000000)
                    })
                    .when(is_actionable, |row| {
                        row.cursor(CursorStyle::PointingHand)
                            .hover(|style| style.bg(rgb(0x2d374a)))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |editor, _: &MouseDownEvent, _window, cx| {
                                    cx.stop_propagation();
                                    editor.activate_lsp_panel_item(index, cx);
                                }),
                            )
                    })
                    .child(item.label),
            )
        },
    );

    div()
        .absolute()
        .top(anchor.top)
        .left(anchor.left)
        .w(px(420.0))
        .max_h(px(360.0))
        .overflow_hidden()
        .flex()
        .flex_col()
        .gap_2()
        .px_2()
        .py_2()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x3d4a5f))
        .bg(rgb(0x202432))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_this, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
            }),
        )
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_size(px(12.0))
                        .text_color(rgb(EDITOR_TEXT_FG))
                        .child(panel.title),
                )
                .child(find_button("x", cx, |editor, cx| {
                    editor.close_lsp_panel(cx);
                })),
        )
        .child(items)
}

pub(super) fn degraded_mode_overlay(notice: String) -> impl IntoElement {
    div()
        .absolute()
        .top(px(46.0))
        .left(px(84.0))
        .h(px(30.0))
        .max_w(px(420.0))
        .flex()
        .items_center()
        .px_2()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x665f3a))
        .bg(rgb(0x252313))
        .text_size(px(11.0))
        .text_color(rgb(0xe1d58a))
        .overflow_hidden()
        .whitespace_nowrap()
        .child(notice)
}

pub(super) fn external_change_overlay(
    change: ExternalFileChangeSnapshot,
    cx: &mut Context<EditorPrototype>,
) -> impl IntoElement {
    div()
        .absolute()
        .top(px(8.0))
        .left(px(84.0))
        .h(px(36.0))
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x765b2f))
        .bg(rgb(0x2a2419))
        .text_size(px(12.0))
        .text_color(rgb(EDITOR_TEXT_FG))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_this, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
            }),
        )
        .child(
            div()
                .max_w(px(280.0))
                .overflow_hidden()
                .whitespace_nowrap()
                .child(format!("{} changed on disk", change.file_name)),
        )
        .child(find_button("Reload", cx, |editor, cx| {
            editor.reload_external_change(cx);
        }))
        .child(find_button("Keep Local", cx, |editor, cx| {
            editor.keep_local_external_change(cx);
        }))
}

pub(super) fn find_overlay(snapshot: &EditorSnapshot, cx: &mut Context<EditorPrototype>) -> impl IntoElement {
    let height = if snapshot.search_replace_mode {
        px(66.0)
    } else {
        px(34.0)
    };
    div()
        .absolute()
        .top(px(8.0))
        .right(px(12.0))
        .h(height)
        .w(px(470.0))
        .flex()
        .flex_col()
        .justify_center()
        .gap_1()
        .px_2()
        .py_1()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x3d4a5f))
        .bg(rgb(0x202432))
        .text_size(px(12.0))
        .text_color(rgb(EDITOR_TEXT_FG))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_this, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
            }),
        )
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .gap_1()
                .child(find_text_field(
                    "Find",
                    snapshot.search_query.clone(),
                    snapshot.search_input_target == EditorSearchInputTarget::Query,
                    EditorSearchInputTarget::Query,
                    cx,
                ))
                .child(
                    div()
                        .w(px(54.0))
                        .text_align(gpui::TextAlign::Center)
                        .text_color(rgb(EDITOR_MUTED_FG))
                        .child(snapshot.search_status.clone()),
                )
                .child(find_button("Prev", cx, |editor, cx| {
                    editor.move_search_focus(EditorSearchDirection::Previous, cx);
                }))
                .child(find_button("Next", cx, |editor, cx| {
                    editor.move_search_focus(EditorSearchDirection::Next, cx);
                }))
                .child(find_button("Repl", cx, |editor, cx| {
                    editor.toggle_replace_mode(cx);
                }))
                .child(find_button("x", cx, |editor, cx| {
                    editor.close_find(cx);
                })),
        )
        .when(snapshot.search_replace_mode, |overlay| {
            overlay.child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(find_text_field(
                        "Replace",
                        snapshot.search_replacement.clone(),
                        snapshot.search_input_target == EditorSearchInputTarget::Replacement,
                        EditorSearchInputTarget::Replacement,
                        cx,
                    ))
                    .child(find_button("Replace", cx, |editor, cx| {
                        editor.replace_focused_search_match(cx);
                    }))
                    .child(find_button("All", cx, |editor, cx| {
                        editor.replace_all_search_matches(cx);
                    })),
            )
        })
}

pub(super) fn go_to_line_overlay(
    snapshot: &EditorSnapshot,
    cx: &mut Context<EditorPrototype>,
) -> impl IntoElement {
    div()
        .absolute()
        .top(px(8.0))
        .right(px(12.0))
        .h(px(34.0))
        .w(px(300.0))
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .py_1()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x3d4a5f))
        .bg(rgb(0x202432))
        .text_size(px(12.0))
        .text_color(rgb(EDITOR_TEXT_FG))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_this, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
            }),
        )
        .child(go_to_line_text_field(
            snapshot.go_to_line_input.clone(),
            snapshot.total_lines,
        ))
        .child(find_button("Go", cx, |editor, cx| {
            editor.submit_go_to_line(cx);
        }))
        .child(find_button("x", cx, |editor, cx| {
            editor.close_go_to_line(cx);
        }))
}

pub(super) fn rename_symbol_overlay(
    snapshot: &EditorSnapshot,
    cx: &mut Context<EditorPrototype>,
) -> impl IntoElement {
    div()
        .absolute()
        .top(px(8.0))
        .right(px(12.0))
        .h(px(34.0))
        .w(px(360.0))
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .py_1()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x3d4a5f))
        .bg(rgb(0x202432))
        .text_size(px(12.0))
        .text_color(rgb(EDITOR_TEXT_FG))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_this, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
            }),
        )
        .child(rename_symbol_text_field(snapshot.rename_input.clone()))
        .child(find_button("Rename", cx, |editor, cx| {
            editor.submit_lsp_rename(cx);
        }))
        .child(find_button("x", cx, |editor, cx| {
            editor.close_lsp_rename(cx);
        }))
}

fn rename_symbol_text_field(input: String) -> impl IntoElement {
    let empty = input.is_empty();
    let label = if empty {
        "New symbol name".to_string()
    } else {
        input
    };

    div()
        .flex_1()
        .h(px(24.0))
        .flex()
        .items_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x78a6d8))
        .bg(rgb(0x141d2b))
        .px_2()
        .overflow_hidden()
        .whitespace_nowrap()
        .text_color(rgb(if empty { EDITOR_DIM_FG } else { EDITOR_TEXT_FG }))
        .cursor(CursorStyle::IBeam)
        .child(label)
}

fn go_to_line_text_field(input: String, total_lines: usize) -> impl IntoElement {
    let empty = input.is_empty();
    let label = if empty {
        format!("Line 1-{total_lines}")
    } else {
        input
    };

    div()
        .flex_1()
        .h(px(24.0))
        .flex()
        .items_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x78a6d8))
        .bg(rgb(0x141d2b))
        .px_2()
        .overflow_hidden()
        .whitespace_nowrap()
        .text_color(rgb(if empty { EDITOR_DIM_FG } else { EDITOR_TEXT_FG }))
        .cursor(CursorStyle::IBeam)
        .child(label)
}

fn find_text_field(
    placeholder: &'static str,
    text: String,
    active: bool,
    target: EditorSearchInputTarget,
    cx: &mut Context<EditorPrototype>,
) -> impl IntoElement {
    let empty = text.is_empty();
    div()
        .flex_1()
        .h(px(24.0))
        .flex()
        .items_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(if active { 0x78a6d8 } else { 0x48546a }))
        .bg(rgb(if active { 0x141d2b } else { 0x111722 }))
        .px_2()
        .overflow_hidden()
        .whitespace_nowrap()
        .text_color(rgb(if empty { EDITOR_DIM_FG } else { EDITOR_TEXT_FG }))
        .cursor(CursorStyle::IBeam)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |editor, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
                editor.set_search_input_target(target, cx);
            }),
        )
        .child(if empty { placeholder.to_string() } else { text })
}

fn find_button(
    label: &'static str,
    cx: &mut Context<EditorPrototype>,
    handler: fn(&mut EditorPrototype, &mut Context<EditorPrototype>),
) -> impl IntoElement {
    div()
        .h(px(24.0))
        .min_w(px(28.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x48546a))
        .bg(rgb(0x151b26))
        .px_2()
        .text_size(px(11.0))
        .text_color(rgb(EDITOR_TEXT_FG))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0x273247)))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
                handler(this, cx);
            }),
        )
        .child(label)
}

