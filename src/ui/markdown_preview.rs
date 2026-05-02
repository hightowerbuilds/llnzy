use std::path::{Path, PathBuf};

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
                            let text = buf.text();
                            let blocks = parse_markdown_blocks(&text);
                            let base_dir = buf.path().and_then(|path| path.parent());
                            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                                render_markdown_blocks(ui, &blocks, base_dir, theme);
                            });
                        });
                });
            });
    });
}

#[derive(Debug, PartialEq, Eq)]
enum MarkdownBlock {
    Paragraph(String),
    Heading {
        level: usize,
        text: String,
    },
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    Blockquote(String),
    ListItem {
        marker: String,
        text: String,
        indent_level: usize,
    },
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    Image {
        alt: String,
        target: String,
    },
    HorizontalRule,
    Blank,
}

fn parse_markdown_blocks(source: &str) -> Vec<MarkdownBlock> {
    let mut blocks = Vec::new();
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
            flush_parsed_paragraph(&mut blocks, &mut paragraph);
            blocks.push(MarkdownBlock::Blank);
            continue;
        }

        if let Some(fence) = code_fence(trimmed) {
            flush_parsed_paragraph(&mut blocks, &mut paragraph);
            let mut code = String::new();
            for code_line in lines.by_ref() {
                if code_line.trim_start().starts_with(fence.marker) {
                    break;
                }
                code.push_str(code_line);
                code.push('\n');
            }
            blocks.push(MarkdownBlock::CodeBlock {
                language: fence.language.map(ToOwned::to_owned),
                code: code.trim_end_matches('\n').to_string(),
            });
            continue;
        }

        if let Some((level, text)) = heading(trimmed) {
            flush_parsed_paragraph(&mut blocks, &mut paragraph);
            blocks.push(MarkdownBlock::Heading {
                level,
                text: text.to_string(),
            });
            continue;
        }

        if is_horizontal_rule(trimmed) {
            flush_parsed_paragraph(&mut blocks, &mut paragraph);
            blocks.push(MarkdownBlock::HorizontalRule);
            continue;
        }

        if let Some(text) = trimmed.strip_prefix("> ") {
            flush_parsed_paragraph(&mut blocks, &mut paragraph);
            blocks.push(MarkdownBlock::Blockquote(text.to_string()));
            continue;
        }

        if let Some((marker, text, indent_level)) = list_item(line) {
            flush_parsed_paragraph(&mut blocks, &mut paragraph);
            blocks.push(MarkdownBlock::ListItem {
                marker: marker.to_string(),
                text: text.to_string(),
                indent_level,
            });
            continue;
        }

        if let Some((alt, target)) = image(trimmed) {
            flush_parsed_paragraph(&mut blocks, &mut paragraph);
            blocks.push(MarkdownBlock::Image {
                alt: alt.to_string(),
                target: target.to_string(),
            });
            continue;
        }

        if is_table_separator_line(lines.peek().copied()) && looks_like_table_row(trimmed) {
            flush_parsed_paragraph(&mut blocks, &mut paragraph);
            let headers = split_table_row(trimmed);
            lines.next();
            let mut rows = Vec::new();
            while let Some(next) = lines.peek().copied() {
                if !looks_like_table_row(next.trim()) {
                    break;
                }
                rows.push(split_table_row(next.trim()));
                lines.next();
            }
            blocks.push(MarkdownBlock::Table { headers, rows });
            continue;
        }

        paragraph.push(trimmed.to_string());
    }

    flush_parsed_paragraph(&mut blocks, &mut paragraph);
    blocks
}

fn render_markdown_blocks(
    ui: &mut egui::Ui,
    blocks: &[MarkdownBlock],
    base_dir: Option<&Path>,
    theme: MarkdownPreviewTheme,
) {
    for block in blocks {
        match block {
            MarkdownBlock::Paragraph(text) => render_paragraph(ui, text, theme),
            MarkdownBlock::Heading { level, text } => render_heading(ui, *level, text, theme),
            MarkdownBlock::CodeBlock { language, code } => {
                render_code_block(ui, language.as_deref(), code, theme)
            }
            MarkdownBlock::Blockquote(text) => render_blockquote(ui, text, theme),
            MarkdownBlock::ListItem {
                marker,
                text,
                indent_level,
            } => render_list_item(ui, marker, text, *indent_level, theme),
            MarkdownBlock::Table { headers, rows } => render_table(ui, headers, rows, theme),
            MarkdownBlock::Image { alt, target } => render_image(ui, alt, target, base_dir, theme),
            MarkdownBlock::HorizontalRule => {
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(12.0);
            }
            MarkdownBlock::Blank => ui.add_space(6.0),
        }
    }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CodeFence<'a> {
    marker: &'static str,
    language: Option<&'a str>,
}

fn code_fence(line: &str) -> Option<CodeFence<'_>> {
    if let Some(info) = line.strip_prefix("```") {
        Some(CodeFence {
            marker: "```",
            language: code_fence_language(info),
        })
    } else if let Some(info) = line.strip_prefix("~~~") {
        Some(CodeFence {
            marker: "~~~",
            language: code_fence_language(info),
        })
    } else {
        None
    }
}

fn code_fence_language(info: &str) -> Option<&str> {
    let language = info.split_whitespace().next()?;
    (!language.is_empty()).then_some(language)
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

fn list_item(line: &str) -> Option<(&str, &str, usize)> {
    let indent_width = line
        .chars()
        .take_while(|ch| matches!(ch, ' ' | '\t'))
        .map(|ch| if ch == '\t' { 4 } else { 1 })
        .sum::<usize>();
    let indent_level = indent_width / 2;
    let line = line.trim_start();

    for marker in ["- ", "* ", "+ "] {
        if let Some(text) = line.strip_prefix(marker) {
            return Some(("•", text, indent_level));
        }
    }

    let dot = line.find(". ")?;
    if dot == 0 || dot > 3 || !line[..dot].chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    Some((&line[..=dot], line[dot + 2..].trim_start(), indent_level))
}

fn image(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix("![")?;
    let alt_end = rest.find("](")?;
    let target_end = rest[alt_end + 2..].find(')')?;
    let target = normalize_image_target(&rest[alt_end + 2..alt_end + 2 + target_end]);
    if target.is_empty() {
        return None;
    }
    Some((&rest[..alt_end], target))
}

fn normalize_image_target(target: &str) -> &str {
    let target = target.trim();
    if let Some(stripped) = target
        .strip_prefix('<')
        .and_then(|value| value.strip_suffix('>'))
    {
        return stripped.trim();
    }
    target.split_whitespace().next().unwrap_or("")
}

fn looks_like_table_row(line: &str) -> bool {
    line.contains('|') && split_table_row(line).len() >= 2
}

fn is_table_separator_line(line: Option<&str>) -> bool {
    let Some(line) = line.map(str::trim) else {
        return false;
    };
    if !line.contains('|') {
        return false;
    }
    let cells = split_table_row(line);
    cells.len() >= 2
        && cells.iter().all(|cell| {
            let cell = cell.trim();
            let cell = cell.strip_prefix(':').unwrap_or(cell);
            let cell = cell.strip_suffix(':').unwrap_or(cell);
            cell.len() >= 3 && cell.chars().all(|ch| ch == '-')
        })
}

fn split_table_row(line: &str) -> Vec<String> {
    let line = line.trim().trim_matches('|');
    line.split('|')
        .map(|cell| inline_display_text(cell.trim()))
        .collect()
}

fn flush_parsed_paragraph(blocks: &mut Vec<MarkdownBlock>, paragraph: &mut Vec<String>) {
    if paragraph.is_empty() {
        return;
    }
    blocks.push(MarkdownBlock::Paragraph(paragraph.join(" ")));
    paragraph.clear();
}

fn resolve_local_image_path(base_dir: Option<&Path>, target: &str) -> Option<PathBuf> {
    if target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("data:")
        || target.starts_with('#')
    {
        return None;
    }
    let path = Path::new(target);
    if path.is_absolute() {
        return Some(path.to_path_buf());
    }
    Some(base_dir?.join(path))
}

fn load_local_image_texture(
    ui: &mut egui::Ui,
    image_path: &Path,
) -> Option<(egui::TextureHandle, egui::Vec2)> {
    let image = match image::open(image_path) {
        Ok(image) => image.thumbnail(1400, 1400).to_rgba8(),
        Err(err) => {
            log::warn!("Failed to load Markdown preview image: {err}");
            return None;
        }
    };
    let size = [image.width() as usize, image.height() as usize];
    let display_size = egui::vec2(size[0] as f32, size[1] as f32);
    let pixels = image.into_raw();
    Some((
        ui.ctx().load_texture(
            format!("markdown_preview_image:{}", image_path.display()),
            egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
            Default::default(),
        ),
        display_size,
    ))
}

fn render_paragraph(ui: &mut egui::Ui, text: &str, theme: MarkdownPreviewTheme) {
    let text = inline_display_text(text);
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

fn render_code_block(
    ui: &mut egui::Ui,
    language: Option<&str>,
    code: &str,
    theme: MarkdownPreviewTheme,
) {
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(22, 24, 30))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 54, 66)))
        .rounding(egui::Rounding::same(6.0))
        .inner_margin(egui::Margin::symmetric(14.0, 12.0))
        .show(ui, |ui| {
            if let Some(language) = language {
                ui.label(
                    egui::RichText::new(language)
                        .monospace()
                        .size(11.0)
                        .color(theme.muted),
                );
                ui.add_space(6.0);
            }
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

fn render_list_item(
    ui: &mut egui::Ui,
    marker: &str,
    text: &str,
    indent_level: usize,
    theme: MarkdownPreviewTheme,
) {
    ui.horizontal_top(|ui| {
        ui.set_min_height(22.0);
        ui.add_space((indent_level as f32 * 18.0).min(96.0));
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

fn render_table(
    ui: &mut egui::Ui,
    headers: &[String],
    rows: &[Vec<String>],
    theme: MarkdownPreviewTheme,
) {
    egui::Frame::none()
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(56, 62, 74)))
        .rounding(egui::Rounding::same(6.0))
        .inner_margin(egui::Margin::symmetric(10.0, 8.0))
        .show(ui, |ui| {
            egui::Grid::new(format!("markdown_table_{:p}", headers.as_ptr()))
                .striped(true)
                .spacing(egui::vec2(18.0, 8.0))
                .show(ui, |ui| {
                    for header in headers {
                        ui.label(
                            egui::RichText::new(header)
                                .size(14.0)
                                .strong()
                                .color(theme.text),
                        );
                    }
                    ui.end_row();
                    for row in rows {
                        for index in 0..headers.len() {
                            ui.label(
                                egui::RichText::new(row.get(index).map_or("", String::as_str))
                                    .size(14.0)
                                    .color(theme.text),
                            );
                        }
                        ui.end_row();
                    }
                });
        });
    ui.add_space(14.0);
}

fn render_image(
    ui: &mut egui::Ui,
    alt: &str,
    target: &str,
    base_dir: Option<&Path>,
    theme: MarkdownPreviewTheme,
) {
    if let Some(image_path) = resolve_local_image_path(base_dir, target) {
        if let Some((texture, image_size)) = load_local_image_texture(ui, &image_path) {
            let max_width = ui.available_width().max(1.0);
            let scale = (max_width / image_size.x).min(1.0);
            let display_size = image_size * scale;
            ui.image(egui::load::SizedTexture::new(texture.id(), display_size));
            if !alt.is_empty() {
                ui.label(egui::RichText::new(alt).size(12.0).color(theme.muted));
            }
            ui.add_space(14.0);
            return;
        }
    }

    render_image_placeholder(ui, alt, target, theme);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_markdown_extensions_case_insensitively() {
        assert!(is_markdown_path(Path::new("README.md")));
        assert!(is_markdown_path(Path::new("notes.MDX")));
        assert!(is_markdown_path(Path::new("post.markdown")));
        assert!(!is_markdown_path(Path::new("main.rs")));
        assert!(!is_markdown_path(Path::new("Makefile")));
    }

    #[test]
    fn markdown_mode_cycles_source_preview_split() {
        assert_eq!(MarkdownViewMode::Source.cycle(), MarkdownViewMode::Preview);
        assert_eq!(MarkdownViewMode::Preview.cycle(), MarkdownViewMode::Split);
        assert_eq!(MarkdownViewMode::Split.cycle(), MarkdownViewMode::Source);
    }

    #[test]
    fn inline_display_text_preserves_readable_link_targets() {
        assert_eq!(
            inline_display_text("See [docs](https://example.com) for `code`."),
            "See docs (https://example.com) for code."
        );
    }

    #[test]
    fn parses_tables_as_structured_blocks() {
        let blocks = parse_markdown_blocks(
            "| Feature | Status |\n| --- | :---: |\n| Tables | **ok** |\n| Images | pending |\n",
        );

        assert_eq!(
            blocks,
            vec![MarkdownBlock::Table {
                headers: vec!["Feature".to_string(), "Status".to_string()],
                rows: vec![
                    vec!["Tables".to_string(), "ok".to_string()],
                    vec!["Images".to_string(), "pending".to_string()],
                ],
            }]
        );
    }

    #[test]
    fn parses_nested_list_indentation_without_losing_ordered_markers() {
        let blocks = parse_markdown_blocks("- top\n  - child\n    1. ordered child\n");

        assert_eq!(
            blocks,
            vec![
                MarkdownBlock::ListItem {
                    marker: "•".to_string(),
                    text: "top".to_string(),
                    indent_level: 0,
                },
                MarkdownBlock::ListItem {
                    marker: "•".to_string(),
                    text: "child".to_string(),
                    indent_level: 1,
                },
                MarkdownBlock::ListItem {
                    marker: "1.".to_string(),
                    text: "ordered child".to_string(),
                    indent_level: 2,
                },
            ]
        );
    }

    #[test]
    fn parses_local_images_and_normalizes_titles() {
        assert_eq!(
            image("![Screenshot](assets/screen.png \"Preview\")"),
            Some(("Screenshot", "assets/screen.png"))
        );

        let blocks = parse_markdown_blocks("![Logo](./images/logo.png)\n");
        assert_eq!(
            blocks,
            vec![MarkdownBlock::Image {
                alt: "Logo".to_string(),
                target: "./images/logo.png".to_string(),
            }]
        );
    }

    #[test]
    fn resolves_relative_image_paths_against_markdown_directory() {
        assert_eq!(
            resolve_local_image_path(Some(Path::new("/repo/docs")), "assets/screen.png"),
            Some(PathBuf::from("/repo/docs/assets/screen.png"))
        );
        assert_eq!(
            resolve_local_image_path(Some(Path::new("/repo/docs")), "https://example.com/a.png"),
            None
        );
        assert_eq!(resolve_local_image_path(None, "assets/screen.png"), None);
    }

    #[test]
    fn preserves_code_fence_language_labels() {
        let blocks = parse_markdown_blocks("```rust\nfn main() {}\n```\n");

        assert_eq!(
            blocks,
            vec![MarkdownBlock::CodeBlock {
                language: Some("rust".to_string()),
                code: "fn main() {}".to_string(),
            }]
        );
    }
}
