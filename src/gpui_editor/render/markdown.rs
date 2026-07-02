use super::super::*;

use crate::config::MarkdownPreviewStyle;

/// Everything one preview style needs. Styles are typographic only — every
/// color derives from the active app theme so all three follow light/dark
/// themes; `two_tone_ink` is the one color *relationship* a style may add.
struct MarkdownStyleTable {
    /// `None` falls back to the editor appearance font.
    body_font: Option<&'static str>,
    heading_font: Option<&'static str>,
    code_font: Option<&'static str>,
    /// Newspaper ink hierarchy: headlines at full theme foreground, body
    /// slightly softened toward the background (NYT sets #121212 headlines
    /// over #363636 body — the same relationship, theme-relative).
    two_tone_ink: bool,
    /// Square corners read print; rounded reads app.
    code_rounded: bool,
    body_size_scale: f32,
    line_height_scale: f32,
    /// Vertical gap between blocks, in units of the base font size.
    block_gap_em: f32,
    max_width: f32,
    /// H1/H2/H3 size multipliers (H4+ render at body size, bold).
    heading_scales: [f32; 3],
    h1_centered: bool,
    /// LaTeX `\maketitle` sets the title in regular weight, not bold.
    h1_bold: bool,
    /// Hairline rule under H1 and above H2 (newspaper section rules).
    heading_rules: bool,
    /// Prefix H2/H3 with "1  " / "1.1  " section numbers (research paper;
    /// article class numbers sections without a trailing period).
    numbered_headings: bool,
    quote_italic: bool,
    /// Symmetric indent instead of the web left bar (LaTeX quotation /
    /// newspaper pull-quote). False keeps the left bar.
    quote_indented: bool,
    /// Quote size relative to body: 0.9 for a paper abstract, 1.25 for a
    /// newspaper pull-quote.
    quote_size_scale: f32,
    /// Hairline rules above and below the quote (pull-quote framing).
    quote_rules: bool,
    bullet_marker: &'static str,
}

fn markdown_style_table(style: MarkdownPreviewStyle) -> MarkdownStyleTable {
    match style {
        MarkdownPreviewStyle::Default => MarkdownStyleTable {
            body_font: None,
            heading_font: None,
            code_font: None,
            two_tone_ink: false,
            code_rounded: true,
            body_size_scale: 1.0,
            line_height_scale: 1.0,
            block_gap_em: 1.0,
            max_width: 820.0,
            heading_scales: [2.0, 1.5, 1.2],
            h1_centered: false,
            h1_bold: true,
            heading_rules: false,
            numbered_headings: false,
            quote_italic: false,
            quote_indented: false,
            quote_size_scale: 1.0,
            quote_rules: false,
            bullet_marker: "•",
        },
        // Digital article-page typography from verified production values:
        // Georgia body (NYT/WSJ's own fallback for Imperial/Exchange),
        // NYT's two-tone ink hierarchy and hairline rules (theme-relative),
        // ~600px measure, 1.5 leading, H1 at 2.125x (Guardian headline
        // scale), and indented italic pull-quotes framed by rules instead
        // of a blog-style left bar.
        MarkdownPreviewStyle::Newspaper => MarkdownStyleTable {
            body_font: Some("Georgia"),
            heading_font: Some("Georgia"),
            code_font: Some("Menlo"),
            two_tone_ink: true,
            code_rounded: false,
            body_size_scale: 1.1,
            line_height_scale: 1.2,
            block_gap_em: 0.8,
            max_width: 600.0,
            heading_scales: [2.125, 1.5, 1.19],
            h1_centered: false,
            h1_bold: true,
            heading_rules: true,
            numbered_headings: false,
            quote_italic: true,
            quote_indented: true,
            quote_size_scale: 1.25,
            quote_rules: true,
            bullet_marker: "•",
        },
        // LaTeX article-class typography (the arXiv default): Charter body
        // (the screen-friendly macOS stand-in for Computer Modern, itself a
        // popular LaTeX paper font), the verified \normalsize..\LARGE size
        // ladder (1.2x per step: 1.2 / 1.44 / 1.728), uniform theme ink,
        // dense ~1.3 leading, numbered sections, abstract-style quotes.
        MarkdownPreviewStyle::ResearchPaper => MarkdownStyleTable {
            body_font: Some("Charter"),
            heading_font: Some("Charter"),
            code_font: Some("Menlo"),
            two_tone_ink: false,
            code_rounded: false,
            body_size_scale: 0.94,
            line_height_scale: 0.93,
            block_gap_em: 0.65,
            max_width: 660.0,
            heading_scales: [1.73, 1.44, 1.2],
            h1_centered: true,
            h1_bold: false,
            heading_rules: false,
            numbered_headings: true,
            quote_italic: false,
            quote_indented: true,
            quote_size_scale: 0.9,
            quote_rules: false,
            bullet_marker: "•",
        },
    }
}

pub(super) fn markdown_preview_body(snapshot: &EditorSnapshot) -> impl IntoElement {
    let blocks = snapshot.markdown_preview.clone().unwrap_or_else(|| {
        vec![MarkdownPreviewBlock {
            kind: MarkdownPreviewBlockKind::Paragraph,
            text: "No markdown preview available".to_string(),
        }]
    });
    let table = markdown_style_table(snapshot.appearance.markdown_preview_style);
    let base_size = markdown_preview_font_size(&snapshot.appearance) * table.body_size_scale;
    let body_font = table
        .body_font
        .map(str::to_string)
        .unwrap_or_else(|| snapshot.appearance.font_family.clone());
    let page_bg = snapshot.appearance.background_color();

    // Left-aligned column with comfortable line length and generous
    // bottom padding so the last paragraph scrolls clear of the pane edge.
    // Section numbering (research style) is assigned in document order.
    let mut section_counters = [0usize; 2];
    let column = blocks.into_iter().fold(
        div()
            .flex()
            .flex_col()
            .flex_none()
            .h_auto()
            .gap(px(base_size * table.block_gap_em))
            .w_full()
            .max_w(px(table.max_width))
            .pl_8()
            .pr_4()
            .pt_8()
            .pb_24()
            .font_family(body_font),
        |column, block| {
            let heading_prefix = heading_number_prefix(&table, &block, &mut section_counters);
            column.child(markdown_preview_block(
                block,
                &snapshot.appearance,
                &table,
                heading_prefix,
            ))
        },
    );

    div()
        .id("markdown-preview")
        .flex_1()
        .min_h(px(0.0))
        .w_full()
        .overflow_y_scroll()
        .track_scroll(&snapshot.markdown_preview_scroll)
        .scrollbar_width(px(10.0))
        .bg(page_bg)
        .child(column)
}

/// "1." / "1.1" numbering for H2/H3 under the research-paper convention
/// (H1 is the unnumbered title). Returns `None` for other blocks/styles.
fn heading_number_prefix(
    table: &MarkdownStyleTable,
    block: &MarkdownPreviewBlock,
    counters: &mut [usize; 2],
) -> Option<String> {
    if !table.numbered_headings {
        return None;
    }
    // Article-class style: number then a quad of space, no trailing period
    // ("1  Introduction", "1.1  Motivation").
    match block.kind {
        MarkdownPreviewBlockKind::Heading(2) => {
            counters[0] += 1;
            counters[1] = 0;
            Some(format!("{}\u{2003}", counters[0]))
        }
        MarkdownPreviewBlockKind::Heading(3) => {
            counters[1] += 1;
            Some(format!("{}.{}\u{2003}", counters[0].max(1), counters[1]))
        }
        _ => None,
    }
}

fn markdown_preview_block(
    block: MarkdownPreviewBlock,
    appearance: &EditorAppearance,
    table: &MarkdownStyleTable,
    heading_prefix: Option<String>,
) -> impl IntoElement {
    let base_size = markdown_preview_font_size(appearance) * table.body_size_scale;
    let base_line_height = (markdown_preview_line_height(appearance)
        * table.body_size_scale
        * table.line_height_scale)
        .max(base_size * 1.2);
    let heading_color = appearance.foreground_color();
    let body_color = if table.two_tone_ink {
        appearance.preview_soft_foreground()
    } else {
        heading_color
    };
    let muted_color = appearance.muted_color();
    let rule_color = appearance.preview_rule_color();
    match block.kind {
        MarkdownPreviewBlockKind::Heading(level) => {
            let (scale, top_pad) = match level {
                1 => (table.heading_scales[0], 4.0),
                2 => (table.heading_scales[1], 4.0),
                3 => (table.heading_scales[2], 2.0),
                _ => (1.0, 0.0),
            };
            let size = (base_size * scale).clamp(base_size, 40.0);
            let text = match heading_prefix {
                Some(prefix) => format!("{prefix}{}", block.text),
                None => block.text,
            };
            let weight = if level == 1 && !table.h1_bold {
                gpui::FontWeight::NORMAL
            } else {
                gpui::FontWeight::BOLD
            };
            let mut heading = div()
                .w_full()
                .pt(px(top_pad))
                .text_size(px(size))
                .line_height(px((size * 1.25).max(base_line_height)))
                .font_weight(weight)
                .text_color(heading_color);
            if let Some(font) = table.heading_font {
                heading = heading.font_family(font.to_string());
            }
            if table.h1_centered && level == 1 {
                heading = heading.text_center();
            }
            if table.heading_rules && level <= 2 {
                heading = heading.pb(px(6.0)).border_b_1().border_color(rule_color);
            }
            heading.child(text)
        }
        MarkdownPreviewBlockKind::Paragraph => div()
            .w_full()
            .text_size(px(base_size))
            .line_height(px(base_line_height))
            .text_color(body_color)
            .child(block.text),
        MarkdownPreviewBlockKind::Bullet => div()
            .w_full()
            .flex()
            .gap_3()
            .text_size(px(base_size))
            .line_height(px(base_line_height))
            .text_color(body_color)
            .child(
                div()
                    .w(px((base_size * 1.1).max(14.0)))
                    .text_color(muted_color)
                    .child(table.bullet_marker),
            )
            .child(div().flex_1().child(block.text)),
        MarkdownPreviewBlockKind::Quote => {
            let quote_size = base_size * table.quote_size_scale;
            let quote_line_height =
                (base_line_height * table.quote_size_scale).max(quote_size * 1.3);
            let mut quote = if table.quote_indented {
                // Symmetric indent, body ink, no bar: the LaTeX quotation /
                // abstract block or, scaled up, a newspaper pull-quote.
                div()
                    .w_full()
                    .px(px(32.0))
                    .py_1()
                    .text_size(px(quote_size))
                    .line_height(px(quote_line_height))
                    .text_color(body_color)
            } else {
                div()
                    .w_full()
                    .border_l_2()
                    .border_color(muted_color)
                    .pl_4()
                    .py_1()
                    .text_size(px(quote_size))
                    .line_height(px(quote_line_height))
                    .text_color(muted_color)
            };
            if table.quote_rules {
                quote = quote
                    .border_t_1()
                    .border_b_1()
                    .border_color(rule_color)
                    .py_3();
            }
            if table.quote_italic {
                quote = quote.italic();
            }
            quote.child(block.text)
        }
        MarkdownPreviewBlockKind::Code => {
            let code_bg = appearance.preview_code_background();
            let code_border = appearance.preview_code_border();
            let code_font = table
                .code_font
                .map(str::to_string)
                .unwrap_or_else(|| appearance.font_family.clone());
            let code = div().w_full();
            let code = if table.code_rounded {
                code.rounded_md()
            } else {
                code
            };
            code.border_1()
                .border_color(code_border)
                .bg(code_bg)
                .px_4()
                .py_3()
                .font_family(code_font)
                .text_size(px((base_size * 0.9).max(11.0)))
                .line_height(px((base_line_height * 0.92).max(base_size * 1.25)))
                .text_color(body_color)
                .child(block.text)
        }
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
