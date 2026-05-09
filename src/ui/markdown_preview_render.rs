use std::path::{Path, PathBuf};

use super::markdown_preview::{inline_display_text, MarkdownPreviewTheme};
use super::markdown_preview_parse::MarkdownBlock;

pub(super) fn render_markdown_blocks(
    ui: &mut egui::Ui,
    blocks: &[MarkdownBlock],
    base_dir: Option<&Path>,
    theme: MarkdownPreviewTheme,
) {
    for (block_index, block) in blocks.iter().enumerate() {
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
            MarkdownBlock::Table { headers, rows } => {
                render_table(ui, block_index, headers, rows, theme)
            }
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
    block_index: usize,
    headers: &[String],
    rows: &[Vec<String>],
    theme: MarkdownPreviewTheme,
) {
    egui::Frame::none()
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(56, 62, 74)))
        .rounding(egui::Rounding::same(6.0))
        .inner_margin(egui::Margin::symmetric(10.0, 8.0))
        .show(ui, |ui| {
            egui::Grid::new(("markdown_table", block_index))
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
