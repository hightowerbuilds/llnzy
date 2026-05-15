use gpui::prelude::*;
use gpui::{
    div, fill, point, px, rgb, rgba, size, Bounds, Font, PaintQuad, Pixels, SharedString, TextRun,
    Window,
};

use super::effects::rgba_u32;
use super::{
    CellMetrics, FALLBACK_CELL_WIDTH, TERMINAL_ACCENT, TERMINAL_BORDER, TERMINAL_MUTED,
    TERMINAL_PADDING,
};
use crate::config::{Config, CursorStyle};
use crate::session::Session;

pub(super) fn terminal_header(
    title: String,
    subtitle: String,
    status_message: Option<String>,
    uses_background_image: bool,
) -> impl IntoElement {
    let mut header = div()
        .h(px(42.0))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px_3()
        .border_b_1()
        .border_color(rgb(TERMINAL_BORDER));

    header = if uses_background_image {
        header.bg(rgba(rgba_u32([0x12, 0x12, 0x17], 0.74)))
    } else {
        header.bg(rgb(0x121217))
    };

    header
        .child(
            div()
                .flex()
                .flex_col()
                .child(div().text_size(px(13.0)).child(title))
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(rgb(TERMINAL_MUTED))
                        .child(subtitle),
                ),
        )
        .child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(if status_message.is_some() {
                    TERMINAL_ACCENT
                } else {
                    TERMINAL_MUTED
                }))
                .child(status_message.unwrap_or_else(|| "Cmd-R restart".into())),
        )
}

pub(super) fn terminal_grid_size(bounds: Bounds<Pixels>, metrics: CellMetrics) -> (u16, u16) {
    let width = (bounds.size.width - px(TERMINAL_PADDING * 2.0)).max(px(metrics.advance));
    let height = (bounds.size.height - px(TERMINAL_PADDING * 2.0)).max(px(metrics.line_height));
    let cols = (width / px(metrics.advance))
        .floor()
        .max(1.0)
        .min(u16::MAX as f32) as u16;
    let rows = (height / px(metrics.line_height))
        .floor()
        .max(1.0)
        .min(u16::MAX as f32) as u16;
    (cols, rows)
}

pub(super) fn cursor_quad(
    terminal_bounds: Bounds<Pixels>,
    row: usize,
    col: usize,
    config: &Config,
    metrics: CellMetrics,
    row_offsets: Option<&[f32]>,
) -> PaintQuad {
    // In Display mode the cursor's x and width come from the row's actual
    // shaped offsets. Wide chars (or proportional glyphs) make the block
    // cursor naturally as wide as the glyph it sits on. Fall back to
    // `metrics.advance` whenever the offset table is missing (Monospace
    // mode, or before the first paint).
    let (x, cell_width) = if let Some(offsets) = row_offsets {
        let start = offsets
            .get(col)
            .copied()
            .unwrap_or(col as f32 * metrics.advance);
        let next = offsets
            .get(col + 1)
            .copied()
            .unwrap_or(start + metrics.advance);
        let width = (next - start).max(2.0);
        (TERMINAL_PADDING + start, width)
    } else {
        (
            TERMINAL_PADDING + col as f32 * metrics.advance,
            metrics.advance,
        )
    };
    let y = TERMINAL_PADDING + row as f32 * metrics.line_height;
    let (cursor_x, cursor_y, cursor_w, cursor_h) = match config.cursor_style {
        CursorStyle::Block => (x, y, cell_width, metrics.line_height),
        CursorStyle::Beam => (x, y, 2.0, metrics.line_height),
        CursorStyle::Underline => (x, y + metrics.line_height - 2.0, cell_width, 2.0),
    };

    fill(
        Bounds::new(
            point(
                terminal_bounds.left() + px(cursor_x),
                terminal_bounds.top() + px(cursor_y),
            ),
            size(px(cursor_w), px(cursor_h)),
        ),
        rgba(rgba_u32(config.cursor_color(), 1.0)),
    )
}

/// Binary-search a row's column offset table for the column whose pixel
/// interval contains `x`. Used by `point_to_grid` in Display mode to map
/// clicks back to grid columns through the actual shaped widths instead of
/// dividing by `metrics.advance`.
pub(super) fn col_for_local_x(offsets: &[f32], x: f32) -> usize {
    if offsets.len() <= 1 || x <= offsets[0] {
        return 0;
    }
    // `offsets` is monotonically non-decreasing; partition_point gives the
    // first index whose offset is strictly greater than `x`. The clicked
    // column is one before that.
    let upper = offsets.partition_point(|&offset| offset <= x);
    upper.saturating_sub(1).min(offsets.len() - 2)
}

/// Display-mode equivalent of `selection_rects`. Pulls cell ranges from
/// alacritty and translates them into pixel rects via each row's column
/// offset table.
pub(super) fn display_mode_selection_rects(
    session: &Session,
    row_offsets: &[Vec<f32>],
    metrics: CellMetrics,
) -> Vec<(f32, f32, f32, f32, [f32; 4])> {
    let color = [
        0x38 as f32 / 255.0,
        0xbd as f32 / 255.0,
        0xf8 as f32 / 255.0,
        0.35,
    ];
    session
        .terminal
        .selection_cells()
        .into_iter()
        .filter_map(|(row, col_start, col_end)| {
            let offsets = row_offsets.get(row)?;
            let x = *offsets.get(col_start)?;
            let right = *offsets.get(col_end + 1)?;
            Some((
                x,
                row as f32 * metrics.line_height,
                (right - x).max(0.0),
                metrics.line_height,
                color,
            ))
        })
        .collect()
}

/// Display-mode equivalent of `background_rects`. Walks the visible grid and
/// coalesces adjacent cells with the same non-default background color,
/// using the row's column offset table for pixel widths.
pub(super) fn display_mode_background_rects(
    session: &Session,
    config: &Config,
    row_offsets: &[Vec<f32>],
    metrics: CellMetrics,
) -> Vec<(f32, f32, f32, f32, [f32; 4])> {
    let (cols, rows) = session.terminal.size();
    let bg_f = config.bg();
    let default_bg = [
        (bg_f[0] * 255.0) as u8,
        (bg_f[1] * 255.0) as u8,
        (bg_f[2] * 255.0) as u8,
    ];
    let mut rects = Vec::new();

    for row in 0..rows {
        let Some(offsets) = row_offsets.get(row) else {
            continue;
        };
        let mut col = 0;
        while col < cols {
            let bg = session.terminal.resolve_bg_with_attrs(row, col, config);
            if bg != default_bg {
                let start_col = col;
                while col < cols && session.terminal.resolve_bg_with_attrs(row, col, config) == bg {
                    col += 1;
                }
                let x = offsets[start_col];
                let right = offsets[col];
                rects.push((
                    x,
                    row as f32 * metrics.line_height,
                    (right - x).max(0.0),
                    metrics.line_height,
                    [
                        bg[0] as f32 / 255.0,
                        bg[1] as f32 / 255.0,
                        bg[2] as f32 / 255.0,
                        1.0,
                    ],
                ));
            } else {
                col += 1;
            }
        }
    }

    rects
}

/// Display-mode equivalent of `decoration_rects` + `url_decoration_rects`.
/// Each cell's decorations (underline, strikethrough, undercurl, etc.) are
/// drawn at the cell's actual shaped extent rather than `metrics.advance`.
pub(super) fn display_mode_decoration_rects(
    session: &Session,
    config: &Config,
    row_offsets: &[Vec<f32>],
    metrics: CellMetrics,
) -> Vec<(f32, f32, f32, f32, [f32; 4])> {
    use alacritty_terminal::term::cell::Flags;

    let (cols, rows) = session.terminal.size();
    let cell_h = metrics.line_height;
    let mut rects = Vec::new();

    for row in 0..rows {
        let Some(offsets) = row_offsets.get(row) else {
            continue;
        };
        let y = row as f32 * cell_h;
        for col in 0..cols {
            let flags = session.terminal.cell_flags(row, col);
            let needs_decoration = flags.intersects(
                Flags::UNDERLINE
                    | Flags::DOUBLE_UNDERLINE
                    | Flags::UNDERCURL
                    | Flags::DOTTED_UNDERLINE
                    | Flags::DASHED_UNDERLINE
                    | Flags::STRIKEOUT,
            );
            if !needs_decoration {
                continue;
            }
            let fg = session.terminal.resolve_fg_with_attrs(row, col, config);
            let color = [
                fg[0] as f32 / 255.0,
                fg[1] as f32 / 255.0,
                fg[2] as f32 / 255.0,
                1.0,
            ];
            let x = offsets[col];
            let w = (offsets[col + 1] - x).max(0.0);

            if flags.contains(Flags::UNDERLINE) {
                rects.push((x, y + cell_h - 2.0, w, 1.0, color));
            } else if flags.contains(Flags::DOUBLE_UNDERLINE) {
                rects.push((x, y + cell_h - 4.0, w, 1.0, color));
                rects.push((x, y + cell_h - 1.0, w, 1.0, color));
            } else if flags.contains(Flags::UNDERCURL) {
                let segments = 4;
                let seg_w = w / segments as f32;
                for i in 0..segments {
                    let offset = if i % 2 == 0 { -1.5 } else { 0.5 };
                    rects.push((
                        x + i as f32 * seg_w,
                        y + cell_h - 2.0 + offset,
                        seg_w,
                        1.0,
                        color,
                    ));
                }
            } else if flags.contains(Flags::DOTTED_UNDERLINE) {
                let dot_w = (w / 4.0).max(1.0);
                let mut dx = 0.0;
                while dx < w {
                    rects.push((x + dx, y + cell_h - 2.0, dot_w, 1.0, color));
                    dx += dot_w * 2.0;
                }
            } else if flags.contains(Flags::DASHED_UNDERLINE) {
                let dash_w = (w / 2.0).max(1.0);
                rects.push((x, y + cell_h - 2.0, dash_w, 1.0, color));
            }

            if flags.contains(Flags::STRIKEOUT) {
                rects.push((x, y + cell_h * 0.5, w, 1.0, color));
            }
        }
    }

    rects
}

/// Compute the per-frame cell geometry from the actual terminal font and the
/// configured line-height multiplier. Advance width is measured by shaping the
/// probe character `M` through the same `TextSystem::shape_line` path that
/// renders the actual terminal rows, so the metric matches the rendered
/// glyph width even when font resolution falls back (e.g. Berkeley Mono is
/// not installed and the renderer drops to a system monospace). Line height
/// is `font_size * terminal.line_height`, matching the existing config
/// semantics. Falls back to the legacy 9.0/20.0 pair if shaping fails.
pub(super) fn compute_cell_metrics(
    window: &Window,
    base_font: &Font,
    font_size: Pixels,
    config: &Config,
) -> CellMetrics {
    let advance = measure_cell_advance(window, base_font, font_size);
    let line_height_multiplier = config.line_height.max(1.0);
    let line_height = (f32::from(font_size) * line_height_multiplier).max(1.0);
    CellMetrics {
        advance,
        line_height,
    }
}

fn measure_cell_advance(window: &Window, base_font: &Font, font_size: Pixels) -> f32 {
    // Shape a multi-character probe through the same pipeline that paints
    // terminal rows, then divide by the probe length. This averages out any
    // per-glyph offset (kerning, hinting) and matches the per-cell advance
    // that `shape_text` will use for actual content. Ligatures are disabled
    // on the probe font so contextual alternates cannot compress the probe
    // string into a shorter measured width than the real, mostly
    // non-ligaturable content rendered by terminal output.
    const PROBE: &str = "MMMMMMMMMM";
    let mut probe_font = base_font.clone();
    probe_font.features = gpui::FontFeatures::disable_ligatures();
    let probe_string: SharedString = PROBE.into();
    let run = TextRun {
        len: probe_string.len(),
        font: probe_font,
        color: gpui::black(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let shaped = window
        .text_system()
        .shape_line(probe_string, font_size, &[run], None);
    let total_width = f32::from(shaped.width);
    let per_char = total_width / PROBE.len() as f32;
    if per_char > 0.0 {
        per_char
    } else {
        FALLBACK_CELL_WIDTH
    }
}
