use super::line_render::editor_line;
use super::*;
use gpui::{img, ObjectFit, StyledImage};

impl Render for EditorPrototype {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = self.snapshot(self.focus_handle.is_focused(window));
        let content = div().flex_1().flex().flex_col();
        let content = if self.show_chrome {
            content.child(editor_header(&snapshot))
        } else {
            content
        }
        .child(editor_file_tabs(&snapshot, cx))
        .child(editor_body(&snapshot, cx.entity(), cx))
        .child(status_bar(&snapshot));

        let root = div()
            .size_full()
            .flex()
            .flex_col()
            .bg(snapshot.appearance.background_color())
            .text_color(snapshot.appearance.foreground_color())
            .font_family("Inter")
            .key_context("EditorPrototype")
            .track_focus(&self.focus_handle(cx))
            .on_key_down(cx.listener(Self::on_editor_key_down))
            .on_action(cx.listener(Self::move_left))
            .on_action(cx.listener(Self::move_right))
            .on_action(cx.listener(Self::move_up))
            .on_action(cx.listener(Self::move_down))
            .on_action(cx.listener(Self::move_word_left))
            .on_action(cx.listener(Self::move_word_right))
            .on_action(cx.listener(Self::select_word_left))
            .on_action(cx.listener(Self::select_word_right))
            .on_action(cx.listener(Self::move_home))
            .on_action(cx.listener(Self::move_end))
            .on_action(cx.listener(Self::move_line_start))
            .on_action(cx.listener(Self::move_line_end))
            .on_action(cx.listener(Self::select_line_start))
            .on_action(cx.listener(Self::select_line_end))
            .on_action(cx.listener(Self::move_document_start))
            .on_action(cx.listener(Self::move_document_end))
            .on_action(cx.listener(Self::select_document_start))
            .on_action(cx.listener(Self::select_document_end))
            .on_action(cx.listener(Self::page_up))
            .on_action(cx.listener(Self::page_down))
            .on_action(cx.listener(Self::select_page_up))
            .on_action(cx.listener(Self::select_page_down))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_up))
            .on_action(cx.listener(Self::select_down))
            .on_action(cx.listener(Self::select_home))
            .on_action(cx.listener(Self::select_end))
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::delete_word_backward))
            .on_action(cx.listener(Self::delete_word_forward))
            .on_action(cx.listener(Self::delete_to_line_start))
            .on_action(cx.listener(Self::delete_to_line_end))
            .on_action(cx.listener(Self::enter))
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::shift_tab))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::select_word))
            .on_action(cx.listener(Self::select_line))
            .on_action(cx.listener(Self::delete_line))
            .on_action(cx.listener(Self::duplicate_line_or_selection_action))
            .on_action(cx.listener(Self::move_line_up))
            .on_action(cx.listener(Self::move_line_down))
            .on_action(cx.listener(Self::toggle_line_comment_action))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::save))
            .on_action(cx.listener(Self::undo))
            .on_action(cx.listener(Self::redo))
            .on_action(cx.listener(Self::find))
            .on_action(cx.listener(Self::find_next))
            .on_action(cx.listener(Self::find_previous))
            .on_action(cx.listener(Self::go_to_line))
            .on_action(cx.listener(Self::close_find_action));

        if self.show_chrome {
            root.child(header()).child(content)
        } else {
            root.child(content)
        }
    }
}

fn header() -> impl IntoElement {
    div()
        .h(px(44.0))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px_4()
        .border_b_1()
        .border_color(rgb(EDITOR_BORDER))
        .bg(rgb(EDITOR_CHROME_BG))
        .child(
            div()
                .font_weight(gpui::FontWeight::BOLD)
                .text_size(px(15.0))
                .child("LLNZY GPUI Editor"),
        )
        .child(
            div()
                .text_size(px(12.0))
                .text_color(rgb(EDITOR_MUTED_FG))
                .child("EditorState-backed code editing"),
        )
}

fn editor_header(snapshot: &EditorSnapshot) -> impl IntoElement {
    div()
        .h(px(48.0))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px_4()
        .border_b_1()
        .border_color(rgb(EDITOR_BORDER))
        .bg(rgb(EDITOR_CHROME_BG))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_size(px(13.0)).child(snapshot.title.clone()))
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(rgb(EDITOR_MUTED_FG))
                        .child(snapshot.subtitle.clone()),
                ),
        )
        .child(
            div()
                .rounded_sm()
                .border_1()
                .border_color(rgb(EDITOR_BORDER))
                .px_2()
                .py_1()
                .text_size(px(11.0))
                .text_color(rgb(EDITOR_TEXT_FG))
                .child(snapshot.language.clone()),
        )
}

fn editor_file_tabs(_snapshot: &EditorSnapshot, _cx: &mut Context<EditorPrototype>) -> gpui::Div {
    div()
}

fn editor_body(
    snapshot: &EditorSnapshot,
    input: Entity<EditorPrototype>,
    cx: &mut Context<EditorPrototype>,
) -> gpui::Div {
    if let Some(preview) = snapshot.image_preview.clone() {
        return image_preview_body(preview, &snapshot.appearance);
    }

    match (snapshot.markdown, snapshot.markdown_mode) {
        (true, MarkdownViewMode::Preview) => div()
            .flex_1()
            .w_full()
            .overflow_hidden()
            .child(markdown_preview_body(snapshot)),
        (true, MarkdownViewMode::Split) => div()
            .flex_1()
            .w_full()
            .flex()
            .overflow_hidden()
            .child(editor_source_body(snapshot, input, cx).w(relative(0.5)))
            .child(div().w(px(1.0)).h_full().bg(rgb(EDITOR_BORDER)))
            .child(
                div()
                    .w(relative(0.5))
                    .h_full()
                    .overflow_hidden()
                    .child(markdown_preview_body(snapshot)),
            ),
        _ => editor_source_body(snapshot, input, cx),
    }
}

fn editor_source_body(
    snapshot: &EditorSnapshot,
    input: Entity<EditorPrototype>,
    cx: &mut Context<EditorPrototype>,
) -> gpui::Div {
    let lines = snapshot
        .lines
        .iter()
        .fold(div().flex().flex_col().py_3(), |column, line| {
            column.child(editor_line(
                line.number,
                &line.text,
                &line.highlights,
                &line.search_matches,
                line.diagnostic.as_ref(),
                snapshot
                    .cursor
                    .filter(|cursor| cursor.line + 1 == line.number),
                snapshot.cursor_visible,
                snapshot.selection,
                snapshot.scroll_col,
                &snapshot.appearance,
            ))
        });
    let ruler_layer = editor_ruler_layer(&snapshot.appearance, snapshot.scroll_col);
    let input_layer = div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .cursor(CursorStyle::IBeam)
        .child(EditorInputElement {
            input: input.clone(),
        });

    div()
        .relative()
        .flex_1()
        .w_full()
        .overflow_hidden()
        .bg(snapshot.appearance.background_color())
        .cursor(CursorStyle::IBeam)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(EditorPrototype::on_editor_mouse_down),
        )
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(EditorPrototype::on_editor_mouse_up),
        )
        .on_mouse_up_out(
            MouseButton::Left,
            cx.listener(EditorPrototype::on_editor_mouse_up),
        )
        .on_mouse_move(cx.listener(EditorPrototype::on_editor_mouse_move))
        .on_scroll_wheel(cx.listener(EditorPrototype::on_editor_scroll))
        .child(lines)
        .child(ruler_layer)
        .child(input_layer)
        .when(snapshot.search_active, |body| {
            body.child(find_overlay(snapshot, cx))
        })
        .when(snapshot.go_to_line_active, |body| {
            body.child(go_to_line_overlay(snapshot, cx))
        })
        .when(snapshot.rename_active, |body| {
            body.child(rename_symbol_overlay(snapshot, cx))
        })
        .when_some(snapshot.external_change.clone(), |body, change| {
            body.child(external_change_overlay(change, cx))
        })
        .when_some(snapshot.degraded_notice.clone(), |body, notice| {
            body.child(degraded_mode_overlay(notice))
        })
        .when_some(snapshot.lsp_panel.clone(), |body, panel| {
            body.child(language_panel_overlay(panel, cx))
        })
}

fn image_preview_body(
    preview: EditorImagePreviewSnapshot,
    appearance: &EditorAppearance,
) -> gpui::Div {
    let detail = preview
        .dimensions
        .map(|(width, height)| format!("{width} x {height}"))
        .unwrap_or_else(|| "image dimensions unavailable".to_string());
    let size = preview
        .file_size
        .map(format_file_size)
        .unwrap_or_else(|| "size unavailable".to_string());

    div()
        .relative()
        .flex_1()
        .w_full()
        .h_full()
        .overflow_hidden()
        .bg(appearance.background_color())
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .p_4()
                .child(img(preview.path).size_full().object_fit(ObjectFit::Contain)),
        )
        .child(
            div()
                .absolute()
                .left(px(12.0))
                .bottom(px(12.0))
                .max_w(px(420.0))
                .rounded_sm()
                .border_1()
                .border_color(rgb(EDITOR_BORDER))
                .bg(rgba(0x10131add))
                .px_2()
                .py_1()
                .text_size(px(11.0))
                .text_color(appearance.muted_color())
                .overflow_hidden()
                .whitespace_nowrap()
                .child(format!("{detail} | {size}")),
        )
}

fn markdown_preview_body(snapshot: &EditorSnapshot) -> impl IntoElement {
    let blocks = snapshot.markdown_preview.clone().unwrap_or_else(|| {
        vec![MarkdownPreviewBlock {
            kind: MarkdownPreviewBlockKind::Paragraph,
            text: "No markdown preview available".to_string(),
        }]
    });

    let content = blocks.into_iter().fold(
        div().flex().flex_col().gap_2().px_5().py_4(),
        |column, block| column.child(markdown_preview_block(block, &snapshot.appearance)),
    );

    div()
        .id("markdown-preview")
        .relative()
        .flex_1()
        .w_full()
        .h_full()
        .overflow_y_scroll()
        .track_scroll(&snapshot.markdown_preview_scroll)
        .scrollbar_width(px(8.0))
        .bg(snapshot.appearance.background_color())
        .child(content)
}

fn markdown_preview_block(
    block: MarkdownPreviewBlock,
    appearance: &EditorAppearance,
) -> impl IntoElement {
    match block.kind {
        MarkdownPreviewBlockKind::Heading(level) => {
            let size = match level {
                1 => 26.0,
                2 => 21.0,
                3 => 18.0,
                _ => 15.0,
            };
            div()
                .w_full()
                .text_size(px(size))
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(appearance.foreground_color())
                .child(block.text)
        }
        MarkdownPreviewBlockKind::Paragraph => div()
            .w_full()
            .text_size(px(14.0))
            .line_height(px(22.0))
            .text_color(appearance.foreground_color())
            .child(block.text),
        MarkdownPreviewBlockKind::Bullet => div()
            .w_full()
            .flex()
            .gap_2()
            .text_size(px(14.0))
            .line_height(px(22.0))
            .text_color(appearance.foreground_color())
            .child(div().text_color(appearance.muted_color()).child("•"))
            .child(div().flex_1().child(block.text)),
        MarkdownPreviewBlockKind::Quote => div()
            .w_full()
            .border_l_1()
            .border_color(appearance.muted_color())
            .pl_3()
            .text_size(px(14.0))
            .line_height(px(22.0))
            .text_color(appearance.muted_color())
            .child(block.text),
        MarkdownPreviewBlockKind::Code => div()
            .w_full()
            .rounded_sm()
            .border_1()
            .border_color(rgb(EDITOR_BORDER))
            .bg(rgb(0x10131a))
            .px_3()
            .py_2()
            .font_family("Berkeley Mono")
            .text_size(px(12.0))
            .line_height(px(18.0))
            .text_color(appearance.foreground_color())
            .child(block.text),
    }
}

fn editor_ruler_layer(appearance: &EditorAppearance, scroll_col: usize) -> gpui::Div {
    if appearance.rulers.is_empty() {
        return div();
    }

    let mut layer = div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .overflow_hidden();

    for ruler in appearance.rulers.iter().copied() {
        if ruler < scroll_col {
            continue;
        }
        let x = appearance.line_number_width
            + appearance.char_width * ruler.saturating_sub(scroll_col) as f32;
        layer = layer.child(
            div()
                .absolute()
                .top_0()
                .left(x)
                .w(px(1.0))
                .h_full()
                .bg(appearance.ruler_color()),
        );
    }

    layer
}

fn language_panel_overlay(
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

fn degraded_mode_overlay(notice: String) -> impl IntoElement {
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

fn external_change_overlay(
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

fn find_overlay(snapshot: &EditorSnapshot, cx: &mut Context<EditorPrototype>) -> impl IntoElement {
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

fn go_to_line_overlay(
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

fn rename_symbol_overlay(
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

fn status_bar(snapshot: &EditorSnapshot) -> impl IntoElement {
    let left = if let Some(preview) = &snapshot.image_preview {
        preview
            .dimensions
            .map(|(width, height)| format!("Image {width}x{height}"))
            .unwrap_or_else(|| "Image preview".to_string())
    } else if let Some(cursor) = snapshot.cursor {
        format!("Ln {}, Col {}", cursor.line + 1, cursor.col + 1)
    } else if snapshot.sample {
        "sample fallback".to_string()
    } else {
        "EditorState active buffer".to_string()
    };
    let diagnostics = diagnostic_status(&snapshot.diagnostics);
    let lsp = [snapshot.lsp_status.clone(), diagnostics]
        .into_iter()
        .filter(|status| !status.is_empty())
        .collect::<Vec<_>>()
        .join(" | ");
    let right = snapshot
        .status_message
        .clone()
        .or_else(|| snapshot.load_error.clone())
        .or_else(|| snapshot.cursor_diagnostic_message.clone())
        .unwrap_or_else(|| "Ready".to_string());

    let position = if snapshot.image_preview.is_some() {
        left
    } else {
        format!(
            "{left} | {}-{} / {}{}",
            snapshot.first_line_number,
            snapshot
                .first_line_number
                .saturating_add(snapshot.lines.len().saturating_sub(1)),
            snapshot.total_lines,
            if lsp.is_empty() {
                String::new()
            } else {
                format!(" | {lsp}")
            }
        )
    };

    div()
        .h(px(24.0))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px_3()
        .border_t_1()
        .border_color(rgb(EDITOR_BORDER))
        .bg(rgb(EDITOR_CHROME_BG))
        .text_size(px(11.0))
        .text_color(snapshot.appearance.muted_color())
        .child(position)
        .child(right)
}
