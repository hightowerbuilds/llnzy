use super::line_render::editor_line;
use super::*;
use gpui::{img, ObjectFit, StyledImage};

mod markdown;
mod overlays;
mod status_bar;

use markdown::{markdown_mode_toggle, markdown_preview_body};
use overlays::{
    degraded_mode_overlay, external_change_overlay, find_overlay, go_to_line_overlay,
    language_panel_overlay, rename_symbol_overlay,
};
use status_bar::status_bar;

impl Render for EditorPrototype {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = self.snapshot(self.focus_handle.is_focused(window));
        let content = div().flex_1().min_h(px(0.0)).flex().flex_col();
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

    if !snapshot.markdown {
        return editor_source_body(snapshot, input, cx);
    }

    let mode = snapshot.markdown_mode;
    // Wrap every markdown body in a flex_col container so flex_1 on the
    // inner body element resolves into "fill the remaining height".
    // For Source, this also guarantees the editor source body takes the
    // pane's full size regardless of where else the flex chain breaks.
    let body = match mode {
        MarkdownViewMode::Preview => div()
            .flex_1()
            .min_h(px(0.0))
            .w_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(markdown_preview_body(snapshot)),
        MarkdownViewMode::Source => div()
            .flex_1()
            .min_h(px(0.0))
            .w_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(editor_source_body(snapshot, input, cx)),
        MarkdownViewMode::Split => div()
            .flex_1()
            .min_h(px(0.0))
            .w_full()
            .flex()
            .overflow_hidden()
            .child(
                editor_source_body(snapshot, input, cx)
                    .w(relative(0.5))
                    .min_h(px(0.0)),
            )
            .child(div().w(px(1.0)).h_full().bg(rgb(EDITOR_BORDER)))
            .child(
                div()
                    .w(relative(0.5))
                    .h_full()
                    .min_h(px(0.0))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(markdown_preview_body(snapshot)),
            ),
    };

    body.relative()
        .child(markdown_mode_toggle(mode, snapshot, cx))
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
                line.wrap_start_col,
                line.wrap_cols,
                line.show_line_number,
                &snapshot.appearance,
            ))
        });
    // Rulers mark code-style column boundaries. They are opt-in so the
    // default workspace editor does not show stray vertical divider lines.
    let ruler_layer = if snapshot.markdown || snapshot.appearance.word_wrap {
        div()
    } else {
        editor_ruler_layer(&snapshot.appearance, snapshot.scroll_col)
    };
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
        .min_h(px(0.0))
        .w_full()
        .overflow_hidden()
        .bg(snapshot.appearance.background_color())
        .cursor(CursorStyle::IBeam)
        .on_scroll_wheel(cx.listener(EditorPrototype::on_editor_scroll))
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
            div().absolute().top_0().left_0().size_full().p_4().child(
                img(preview.path)
                    .size_full()
                    .object_fit(ObjectFit::ScaleDown),
            ),
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
