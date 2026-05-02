use std::path::Path;

use crate::editor::buffer::Buffer;
use crate::editor::MarkdownViewMode;

#[derive(Clone, Copy)]
pub(crate) struct MarkdownPreviewTheme {
    pub background: egui::Color32,
    pub surface: egui::Color32,
    pub text: egui::Color32,
    pub muted: egui::Color32,
    pub accent: egui::Color32,
}

pub(crate) fn is_markdown_path(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(ext.as_str(), "md" | "mdx" | "markdown")
}

pub(crate) fn render_markdown_mode_bar(
    ui: &mut egui::Ui,
    mode: &mut MarkdownViewMode,
    theme: MarkdownPreviewTheme,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.label(
            egui::RichText::new("Markdown")
                .size(12.0)
                .color(theme.muted)
                .monospace(),
        );
        ui.add_space(8.0);

        mode_button(ui, mode, MarkdownViewMode::Source, "Source", theme);
        mode_button(ui, mode, MarkdownViewMode::Preview, "Preview", theme);
        mode_button(ui, mode, MarkdownViewMode::Split, "Split", theme);
    });
    ui.add_space(6.0);
}

fn mode_button(
    ui: &mut egui::Ui,
    mode: &mut MarkdownViewMode,
    button_mode: MarkdownViewMode,
    label: &str,
    theme: MarkdownPreviewTheme,
) {
    let active = *mode == button_mode;
    let fill = if active {
        theme.accent
    } else {
        egui::Color32::from_rgb(38, 40, 48)
    };
    let text_color = if active {
        egui::Color32::WHITE
    } else {
        theme.text
    };
    if ui
        .add(
            egui::Button::new(egui::RichText::new(label).size(12.0).color(text_color))
                .fill(fill)
                .min_size(egui::vec2(58.0, 24.0)),
        )
        .clicked()
    {
        *mode = button_mode;
    }
}

pub(crate) fn render_markdown_preview(
    ui: &mut egui::Ui,
    buf: &Buffer,
    theme: MarkdownPreviewTheme,
) {
    egui::Frame::none().fill(theme.background).show(ui, |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let available_width = ui.available_width();
                let page_width = available_width.min(840.0);
                let side_margin = ((available_width - page_width) * 0.5).max(0.0);
                ui.horizontal_top(|ui| {
                    ui.add_space(side_margin);
                    egui::Frame::none()
                        .fill(theme.surface)
                        .inner_margin(egui::Margin::symmetric(34.0, 30.0))
                        .show(ui, |ui| {
                            ui.set_width(page_width);
                            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                                render_markdown_lines(ui, &buf.text(), theme);
                            });
                        });
                });
            });
    });
}

fn render_markdown_lines(ui: &mut egui::Ui, source: &str, theme: MarkdownPreviewTheme) {
    let mut paragraph = Vec::new();
    let mut lines = source.lines().peekable();
    let mut at_start = true;

    while let Some(line) = lines.next() {
        if at_start && line.trim() == "---" {
            skip_frontmatter(&mut lines);
            at_start = false;
            continue;
        }
        at_start = false;

        let trimmed = line.trim();
        if trimmed.is_empty() {
            flush_paragraph(ui, &mut paragraph, theme);
            ui.add_space(6.0);
            continue;
        }

        if let Some(fence) = code_fence_marker(trimmed) {
            flush_paragraph(ui, &mut paragraph, theme);
            let mut code = String::new();
            for code_line in lines.by_ref() {
                if code_line.trim_start().starts_with(fence) {
                    break;
                }
                code.push_str(code_line);
                code.push('\n');
            }
            render_code_block(ui, code.trim_end_matches('\n'), theme);
            continue;
        }

        if let Some((level, text)) = heading(trimmed) {
            flush_paragraph(ui, &mut paragraph, theme);
            render_heading(ui, level, text, theme);
            continue;
        }

        if is_horizontal_rule(trimmed) {
            flush_paragraph(ui, &mut paragraph, theme);
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(12.0);
            continue;
        }

        if let Some(text) = trimmed.strip_prefix("> ") {
            flush_paragraph(ui, &mut paragraph, theme);
            render_blockquote(ui, text, theme);
            continue;
        }

        if let Some((marker, text)) = list_item(trimmed) {
            flush_paragraph(ui, &mut paragraph, theme);
            render_list_item(ui, marker, text, theme);
            continue;
        }

        if let Some((alt, target)) = image(trimmed) {
            flush_paragraph(ui, &mut paragraph, theme);
            render_image_placeholder(ui, alt, target, theme);
            continue;
        }

        paragraph.push(trimmed.to_string());
    }

    flush_paragraph(ui, &mut paragraph, theme);
}

fn skip_frontmatter<'a, I>(lines: &mut std::iter::Peekable<I>)
where
    I: Iterator<Item = &'a str>,
{
    for line in lines.by_ref() {
        if line.trim() == "---" {
            break;
        }
    }
}

fn code_fence_marker(line: &str) -> Option<&'static str> {
    if line.starts_with("```") {
        Some("```")
    } else if line.starts_with("~~~") {
        Some("~~~")
    } else {
        None
    }
}

fn heading(line: &str) -> Option<(usize, &str)> {
    let hashes = line.chars().take_while(|ch| *ch == '#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let text = line.get(hashes..)?.trim_start();
    if text.is_empty() || text.len() == line.len() - hashes {
        return None;
    }
    Some((hashes, text.trim_end_matches('#').trim_end()))
}

fn is_horizontal_rule(line: &str) -> bool {
    let compact: String = line.chars().filter(|ch| !ch.is_whitespace()).collect();
    compact.len() >= 3
        && (compact.chars().all(|ch| ch == '-')
            || compact.chars().all(|ch| ch == '*')
            || compact.chars().all(|ch| ch == '_'))
}

fn list_item(line: &str) -> Option<(&str, &str)> {
    for marker in ["- ", "* ", "+ "] {
        if let Some(text) = line.strip_prefix(marker) {
            return Some(("•", text));
        }
    }

    let dot = line.find(". ")?;
    if dot == 0 || dot > 3 || !line[..dot].chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    Some((&line[..=dot], line[dot + 2..].trim_start()))
}

fn image(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix("![")?;
    let alt_end = rest.find("](")?;
    let target_end = rest[alt_end + 2..].find(')')?;
    Some((
        &rest[..alt_end],
        &rest[alt_end + 2..alt_end + 2 + target_end],
    ))
}

fn flush_paragraph(ui: &mut egui::Ui, paragraph: &mut Vec<String>, theme: MarkdownPreviewTheme) {
    if paragraph.is_empty() {
        return;
    }
    let text = inline_display_text(&paragraph.join(" "));
    ui.add(
        egui::Label::new(
            egui::RichText::new(text)
                .size(16.0)
                .line_height(Some(22.0))
                .color(theme.text),
        )
        .wrap(),
    );
    ui.add_space(10.0);
    paragraph.clear();
}

fn render_heading(ui: &mut egui::Ui, level: usize, text: &str, theme: MarkdownPreviewTheme) {
    let size = match level {
        1 => 34.0,
        2 => 26.0,
        3 => 21.0,
        4 => 18.0,
        _ => 16.0,
    };
    let top_space = match level {
        1 => 0.0,
        2 => 18.0,
        _ => 12.0,
    };
    ui.add_space(top_space);
    ui.add(
        egui::Label::new(
            egui::RichText::new(inline_display_text(text))
                .size(size)
                .strong()
                .color(theme.text),
        )
        .wrap(),
    );
    ui.add_space(if level <= 2 { 12.0 } else { 8.0 });
}

fn render_code_block(ui: &mut egui::Ui, code: &str, theme: MarkdownPreviewTheme) {
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(22, 24, 30))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 54, 66)))
        .rounding(egui::Rounding::same(6.0))
        .inner_margin(egui::Margin::symmetric(14.0, 12.0))
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(code)
                        .monospace()
                        .size(13.0)
                        .color(theme.text),
                )
                .wrap(),
            );
        });
    ui.add_space(14.0);
}

fn render_blockquote(ui: &mut egui::Ui, text: &str, theme: MarkdownPreviewTheme) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(3.0, 42.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 1.5, theme.accent);
        ui.add_space(10.0);
        ui.add(
            egui::Label::new(
                egui::RichText::new(inline_display_text(text))
                    .italics()
                    .size(15.0)
                    .color(theme.muted),
            )
            .wrap(),
        );
    });
    ui.add_space(12.0);
}

fn render_list_item(ui: &mut egui::Ui, marker: &str, text: &str, theme: MarkdownPreviewTheme) {
    ui.horizontal_top(|ui| {
        ui.set_min_height(22.0);
        ui.add_sized(
            egui::vec2(28.0, 20.0),
            egui::Label::new(
                egui::RichText::new(marker)
                    .size(15.0)
                    .color(theme.muted)
                    .monospace(),
            ),
        );
        ui.add(
            egui::Label::new(
                egui::RichText::new(inline_display_text(text))
                    .size(16.0)
                    .color(theme.text),
            )
            .wrap(),
        );
    });
    ui.add_space(5.0);
}

fn render_image_placeholder(
    ui: &mut egui::Ui,
    alt: &str,
    target: &str,
    theme: MarkdownPreviewTheme,
) {
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(30, 34, 40))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(62, 70, 82)))
        .rounding(egui::Rounding::same(6.0))
        .inner_margin(egui::Margin::symmetric(16.0, 14.0))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(if alt.is_empty() { "Image" } else { alt })
                    .size(14.0)
                    .strong()
                    .color(theme.text),
            );
            ui.label(
                egui::RichText::new(target)
                    .size(12.0)
                    .color(theme.muted)
                    .monospace(),
            );
        });
    ui.add_space(14.0);
}

fn inline_display_text(input: &str) -> String {
    let without_links = replace_markdown_links(input);
    without_links
        .replace("**", "")
        .replace("__", "")
        .replace('`', "")
        .replace('*', "")
        .replace('_', "")
}

fn replace_markdown_links(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(open) = rest.find('[') {
        out.push_str(&rest[..open]);
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find("](") else {
            out.push_str(&rest[open..]);
            return out;
        };
        let after_close = &after_open[close + 2..];
        let Some(target_end) = after_close.find(')') else {
            out.push_str(&rest[open..]);
            return out;
        };
        let label = &after_open[..close];
        let target = &after_close[..target_end];
        out.push_str(label);
        if !target.is_empty() {
            out.push_str(" (");
            out.push_str(target);
            out.push(')');
        }
        rest = &after_close[target_end + 1..];
    }
    out.push_str(rest);
    out
}
