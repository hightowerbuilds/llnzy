use std::collections::HashMap;

use crate::editor::perf;
use crate::editor::syntax::{HighlightGroup, HighlightSpan};
use crate::lsp::{DiagSeverity, FileDiagnostic};

#[allow(clippy::too_many_arguments)]
pub(super) fn render_minimap(
    response: &egui::Response,
    painter: &egui::Painter,
    buf: &crate::editor::buffer::Buffer,
    diagnostics: Option<&[FileDiagnostic]>,
    syntax_colors: &HashMap<HighlightGroup, [u8; 3]>,
    highlight_spans: &[Vec<HighlightSpan>],
    visible_doc_lines: &[usize],
    rect: egui::Rect,
    line_count: usize,
    visible_line_count: usize,
    visible_lines: usize,
    max_scroll: usize,
    syntax_start_line: usize,
    syntax_end_line: usize,
    scroll_line: usize,
    scroll_target: &mut Option<f32>,
) {
    let minimap_w = 50.0_f32;
    let minimap_x = rect.right() - minimap_w;
    if line_count <= 1 || !perf::minimap_enabled(line_count) {
        return;
    }

    if response.clicked() {
        if let Some(click_pos) = response.interact_pointer_pos() {
            if click_pos.x >= minimap_x && click_pos.x <= rect.right() {
                let rel_y = (click_pos.y - rect.top()) / rect.height();
                let target_line = (rel_y * visible_line_count as f32).clamp(0.0, max_scroll as f32);
                let centered =
                    (target_line - visible_lines as f32 / 2.0).clamp(0.0, max_scroll as f32);
                *scroll_target = Some(centered);
            }
        }
    }

    painter.rect_filled(
        egui::Rect::from_min_size(
            egui::pos2(minimap_x, rect.top()),
            egui::Vec2::new(minimap_w, rect.height()),
        ),
        0.0,
        egui::Color32::from_rgba_unmultiplied(20, 22, 28, 200),
    );

    let track_h = rect.height();
    let top_doc_line = visible_doc_lines.get(scroll_line).copied().unwrap_or(0);
    let view_top = (top_doc_line as f32 / line_count as f32) * track_h;
    let view_h = (visible_lines as f32 / line_count as f32) * track_h;
    painter.rect_filled(
        egui::Rect::from_min_size(
            egui::pos2(minimap_x, rect.top() + view_top),
            egui::Vec2::new(minimap_w, view_h.max(4.0)),
        ),
        0.0,
        egui::Color32::from_rgba_unmultiplied(80, 120, 200, 30),
    );

    let line_h = (track_h / line_count as f32).max(0.5).min(2.0);
    let sample_step = if line_count > 2000 {
        line_count / 1000
    } else {
        1
    };
    for line_idx in (0..line_count).step_by(sample_step.max(1)) {
        let y = rect.top() + (line_idx as f32 / line_count as f32) * track_h;
        let line_text = buf.line(line_idx);
        if line_text.trim().is_empty() {
            continue;
        }

        let color = if line_idx < syntax_end_line && line_idx >= syntax_start_line {
            let span_idx = line_idx - syntax_start_line;
            highlight_spans
                .get(span_idx)
                .and_then(|spans| spans.first())
                .map(|s| {
                    let rgb =
                        crate::editor::syntax::group_color_with_overrides(s.group, syntax_colors);
                    egui::Color32::from_rgba_unmultiplied(rgb[0], rgb[1], rgb[2], 120)
                })
                .unwrap_or(egui::Color32::from_rgba_unmultiplied(150, 155, 165, 60))
        } else {
            egui::Color32::from_rgba_unmultiplied(150, 155, 165, 40)
        };

        let text_w = (line_text.len() as f32 * 0.4).min(minimap_w - 4.0);
        painter.rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(minimap_x + 2.0, y),
                egui::Vec2::new(text_w, line_h),
            ),
            0.0,
            color,
        );
    }

    if let Some(diags) = diagnostics {
        for diag in diags {
            let dy = rect.top() + (diag.line as f32 / line_count as f32) * track_h;
            let color = match diag.severity {
                DiagSeverity::Error => egui::Color32::from_rgb(255, 80, 80),
                DiagSeverity::Warning => egui::Color32::from_rgb(230, 180, 50),
                _ => continue,
            };
            painter.rect_filled(
                egui::Rect::from_min_size(egui::pos2(minimap_x, dy), egui::Vec2::new(3.0, 2.0)),
                0.0,
                color,
            );
        }
    }
}
