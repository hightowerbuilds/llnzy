use std::path::Path;

use crate::editor::buffer::Buffer;
use crate::editor::MarkdownViewMode;
use crate::path_utils::{path_extension_matches, MARKDOWN_EXTS};
use egui::scroll_area::ScrollAreaOutput;

use super::markdown_preview_parse::parse_markdown_blocks;
use super::markdown_preview_render::render_markdown_blocks;

const PREVIEW_MAX_PAGE_WIDTH: f32 = 840.0;
const PREVIEW_RIGHT_GUTTER: f32 = 18.0;
const PREVIEW_SURFACE_X_PADDING: f32 = 34.0;
const PREVIEW_SURFACE_Y_PADDING: f32 = 30.0;
const MODE_BAR_HEIGHT: f32 = 24.0;

#[derive(Clone, Copy)]
pub(crate) struct MarkdownPreviewTheme {
    pub background: egui::Color32,
    pub surface: egui::Color32,
    pub text: egui::Color32,
    pub muted: egui::Color32,
    pub accent: egui::Color32,
}

pub(crate) fn is_markdown_path(path: &Path) -> bool {
    path_extension_matches(path, MARKDOWN_EXTS)
}

pub(crate) fn render_markdown_mode_bar(
    ui: &mut egui::Ui,
    mode: &mut MarkdownViewMode,
    theme: MarkdownPreviewTheme,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.set_min_height(MODE_BAR_HEIGHT);
        mode_bar_label(ui, "MD", 24.0, theme);
        mode_bar_label(ui, "→", 18.0, theme);

        mode_button(ui, mode, MarkdownViewMode::Source, "Source", theme);
        mode_button(ui, mode, MarkdownViewMode::Preview, "Preview", theme);
        mode_button(ui, mode, MarkdownViewMode::Split, "Split", theme);
    });
    ui.add_space(6.0);
}

fn mode_bar_label(ui: &mut egui::Ui, text: &str, width: f32, theme: MarkdownPreviewTheme) {
    let rich_text = egui::RichText::new(text)
        .size(12.0)
        .color(theme.muted)
        .monospace();
    ui.add_sized(
        [width, MODE_BAR_HEIGHT],
        egui::Label::new(rich_text).selectable(false),
    );
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
                .min_size(egui::vec2(58.0, MODE_BAR_HEIGHT)),
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
    let text = buf.text();
    let base_dir = buf.path().and_then(|path| path.parent());
    render_markdown_text(
        ui,
        &text,
        base_dir,
        markdown_preview_scroll_salt(buf),
        theme,
    );
}

pub(crate) fn render_markdown_text(
    ui: &mut egui::Ui,
    text: &str,
    base_dir: Option<&Path>,
    scroll_salt: impl std::hash::Hash,
    theme: MarkdownPreviewTheme,
) {
    egui::Frame::none().fill(theme.background).show(ui, |ui| {
        let output = egui::ScrollArea::vertical()
            .id_salt(scroll_salt)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let layout = markdown_preview_layout(ui.available_width());
                ui.horizontal_top(|ui| {
                    ui.add_space(layout.left_margin);
                    egui::Frame::none()
                        .fill(theme.surface)
                        .inner_margin(egui::Margin::symmetric(
                            PREVIEW_SURFACE_X_PADDING,
                            PREVIEW_SURFACE_Y_PADDING,
                        ))
                        .show(ui, |ui| {
                            ui.set_width(layout.page_width);
                            let blocks = parse_markdown_blocks(text);
                            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                                render_markdown_blocks(ui, &blocks, base_dir, theme);
                            });
                        });
                    ui.add_space(layout.right_gutter);
                });
            });
        apply_markdown_preview_drag_cursor(ui.ctx(), &output);
    });
}

fn markdown_preview_scroll_salt(buf: &Buffer) -> String {
    buf.path()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| "untitled".to_string())
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct MarkdownPreviewLayout {
    page_width: f32,
    left_margin: f32,
    right_gutter: f32,
}

fn markdown_preview_layout(available_width: f32) -> MarkdownPreviewLayout {
    let right_gutter = PREVIEW_RIGHT_GUTTER.min((available_width * 0.2).max(0.0));
    let content_width = (available_width - right_gutter).max(1.0);
    let page_width = content_width.min(PREVIEW_MAX_PAGE_WIDTH);
    let left_margin = ((content_width - page_width) * 0.5).max(0.0);
    MarkdownPreviewLayout {
        page_width,
        left_margin,
        right_gutter,
    }
}

fn apply_markdown_preview_drag_cursor(ctx: &egui::Context, output: &ScrollAreaOutput<()>) {
    let scrollable = output.content_size.y > output.inner_rect.height() + 0.5;
    let cursor = ctx.input(|input| {
        markdown_preview_cursor_for_drag(
            scrollable,
            input
                .pointer
                .press_origin()
                .is_some_and(|pos| output.inner_rect.contains(pos)),
            input.pointer.primary_down() && input.pointer.is_decidedly_dragging(),
        )
    });
    if let Some(cursor) = cursor {
        ctx.set_cursor_icon(cursor);
    }
}

fn markdown_preview_cursor_for_drag(
    scrollable: bool,
    press_origin_in_preview: bool,
    dragging: bool,
) -> Option<egui::CursorIcon> {
    (scrollable && press_origin_in_preview && dragging).then_some(egui::CursorIcon::Grabbing)
}

pub(super) fn inline_display_text(input: &str) -> String {
    let without_links = replace_markdown_links(input);
    without_links.replace(['`', '*', '_'], "")
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
    use std::path::PathBuf;

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
    fn preview_scroll_salt_tracks_markdown_file_path() {
        let mut left = Buffer::empty();
        let mut right = Buffer::empty();
        left.set_path(PathBuf::from("/tmp/left.md"));
        right.set_path(PathBuf::from("/tmp/right.md"));

        assert_ne!(
            markdown_preview_scroll_salt(&left),
            markdown_preview_scroll_salt(&right)
        );
        assert_eq!(
            markdown_preview_scroll_salt(&left),
            "/tmp/left.md".to_string()
        );
    }

    #[test]
    fn preview_layout_reserves_right_gutter_in_narrow_panes() {
        let layout = markdown_preview_layout(320.0);

        assert_eq!(layout.right_gutter, PREVIEW_RIGHT_GUTTER);
        assert_eq!(layout.left_margin, 0.0);
        assert_eq!(layout.page_width, 320.0 - PREVIEW_RIGHT_GUTTER);
    }

    #[test]
    fn preview_layout_centers_page_after_reserving_right_gutter() {
        let layout = markdown_preview_layout(1200.0);

        assert_eq!(layout.right_gutter, PREVIEW_RIGHT_GUTTER);
        assert_eq!(layout.page_width, PREVIEW_MAX_PAGE_WIDTH);
        assert_eq!(
            layout.left_margin,
            ((1200.0 - PREVIEW_RIGHT_GUTTER - PREVIEW_MAX_PAGE_WIDTH) * 0.5)
        );
    }

    #[test]
    fn preview_drag_cursor_only_shows_while_dragging_scrollable_preview() {
        assert_eq!(
            markdown_preview_cursor_for_drag(true, true, true),
            Some(egui::CursorIcon::Grabbing)
        );
        assert_eq!(markdown_preview_cursor_for_drag(false, true, true), None);
        assert_eq!(markdown_preview_cursor_for_drag(true, false, true), None);
        assert_eq!(markdown_preview_cursor_for_drag(true, true, false), None);
    }

    #[test]
    fn inline_display_text_preserves_readable_link_targets() {
        assert_eq!(
            inline_display_text("See [docs](https://example.com) for `code`."),
            "See docs (https://example.com) for code."
        );
    }
}
