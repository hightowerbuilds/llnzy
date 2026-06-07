use super::super::*;

pub(super) fn markdown_preview_body(snapshot: &EditorSnapshot) -> impl IntoElement {
    let blocks = snapshot.markdown_preview.clone().unwrap_or_else(|| {
        vec![MarkdownPreviewBlock {
            kind: MarkdownPreviewBlockKind::Paragraph,
            text: "No markdown preview available".to_string(),
        }]
    });

    // Left-aligned column with comfortable line length and generous
    // bottom padding so the last paragraph scrolls clear of the pane edge.
    let column = blocks.into_iter().fold(
        div()
            .flex()
            .flex_col()
            .flex_none()
            .h_auto()
            .gap_4()
            .w_full()
            .max_w(px(820.0))
            .pl_8()
            .pr_4()
            .pt_8()
            .pb_24()
            .font_family(snapshot.appearance.font_family.clone()),
        |column, block| column.child(markdown_preview_block(block, &snapshot.appearance)),
    );

    div()
        .id("markdown-preview")
        .flex_1()
        .min_h(px(0.0))
        .w_full()
        .overflow_y_scroll()
        .track_scroll(&snapshot.markdown_preview_scroll)
        .scrollbar_width(px(10.0))
        .bg(snapshot.appearance.background_color())
        .child(column)
}

fn markdown_preview_block(
    block: MarkdownPreviewBlock,
    appearance: &EditorAppearance,
) -> impl IntoElement {
    let base_size = markdown_preview_font_size(appearance);
    let base_line_height = markdown_preview_line_height(appearance);
    match block.kind {
        MarkdownPreviewBlockKind::Heading(level) => {
            let (scale, top_pad) = match level {
                1 => (2.0, 4.0),
                2 => (1.5, 4.0),
                3 => (1.2, 2.0),
                _ => (1.0, 0.0),
            };
            let size = (base_size * scale).clamp(base_size, 40.0);
            div()
                .w_full()
                .pt(px(top_pad))
                .text_size(px(size))
                .line_height(px((size * 1.25).max(base_line_height)))
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(appearance.foreground_color())
                .child(block.text)
        }
        MarkdownPreviewBlockKind::Paragraph => div()
            .w_full()
            .text_size(px(base_size))
            .line_height(px(base_line_height))
            .text_color(appearance.foreground_color())
            .child(block.text),
        MarkdownPreviewBlockKind::Bullet => div()
            .w_full()
            .flex()
            .gap_3()
            .text_size(px(base_size))
            .line_height(px(base_line_height))
            .text_color(appearance.foreground_color())
            .child(
                div()
                    .w(px((base_size * 1.1).max(14.0)))
                    .text_color(appearance.muted_color())
                    .child("•"),
            )
            .child(div().flex_1().child(block.text)),
        MarkdownPreviewBlockKind::Quote => div()
            .w_full()
            .border_l_2()
            .border_color(appearance.muted_color())
            .pl_4()
            .py_1()
            .text_size(px(base_size))
            .line_height(px(base_line_height))
            .text_color(appearance.muted_color())
            .child(block.text),
        MarkdownPreviewBlockKind::Code => div()
            .w_full()
            .rounded_md()
            .border_1()
            .border_color(rgb(EDITOR_BORDER))
            .bg(rgb(0x10131a))
            .px_4()
            .py_3()
            .font_family(appearance.font_family.clone())
            .text_size(px((base_size * 0.9).max(11.0)))
            .line_height(px((base_line_height * 0.92).max(base_size * 1.25)))
            .text_color(appearance.foreground_color())
            .child(block.text),
    }
}

fn markdown_preview_font_size(appearance: &EditorAppearance) -> f32 {
    (appearance.font_size / px(1.0)).clamp(12.0, 24.0)
}

fn markdown_preview_line_height(appearance: &EditorAppearance) -> f32 {
    let font_size = markdown_preview_font_size(appearance);
    let configured = appearance.line_height / px(1.0);
    configured.max(font_size * 1.35)
}

/// Toggle pill that flips between Source (editable) and Preview (read-only).
/// Rendered as an absolute-positioned overlay in the top-right corner of
/// the body so it doesn't consume layout space — the body keeps its full
/// height and any internal scroll continues to work as before.
pub(super) fn markdown_mode_toggle(
    mode: MarkdownViewMode,
    _snapshot: &EditorSnapshot,
    cx: &mut Context<EditorPrototype>,
) -> impl IntoElement {
    div()
        .absolute()
        .top(px(8.0))
        .right(px(12.0))
        .flex()
        .gap_1()
        .child(markdown_mode_button(
            "Preview",
            mode == MarkdownViewMode::Preview,
            MarkdownViewMode::Preview,
            cx,
        ))
        .child(markdown_mode_button(
            "Source",
            mode == MarkdownViewMode::Source,
            MarkdownViewMode::Source,
            cx,
        ))
}

fn markdown_mode_button(
    label: &'static str,
    active: bool,
    target: MarkdownViewMode,
    cx: &mut Context<EditorPrototype>,
) -> impl IntoElement {
    let (bg, border, fg) = if active {
        (0x214966u32, 0x4d8fbfu32, 0xe1e6eeu32)
    } else {
        (0x242632u32, 0x3a3f4bu32, 0x9aa3b3u32)
    };
    div()
        .h(px(26.0))
        .px_3()
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(bg))
        .text_size(px(12.0))
        .text_color(rgb(fg))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                this.set_markdown_mode_from_workspace(target, cx);
            }),
        )
        .child(label)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_preview_metrics_follow_editor_appearance() {
        let mut config = Config::default();
        config.editor.font_size = Some(18.0);
        config.editor.line_height = 1.5;
        let appearance = EditorAppearanceConfig::from_config(&config).for_language(None);

        assert!((markdown_preview_font_size(&appearance) - 18.0).abs() < 0.0001);
        assert!((markdown_preview_line_height(&appearance) - 27.0).abs() < 0.0001);
    }

    #[test]
    fn markdown_preview_metrics_keep_readable_minimums() {
        let mut config = Config::default();
        config.editor.font_size = Some(8.0);
        config.editor.line_height = 1.0;
        let appearance = EditorAppearanceConfig::from_config(&config).for_language(None);

        assert!((markdown_preview_font_size(&appearance) - 12.0).abs() < 0.0001);
        assert!((markdown_preview_line_height(&appearance) - 16.2).abs() < 0.0001);
    }
}
