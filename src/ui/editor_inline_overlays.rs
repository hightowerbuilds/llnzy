use crate::editor::BufferView;
use crate::lsp::{CompletionItem, SignatureInfo};

#[allow(clippy::too_many_arguments)]
pub(super) fn render_inline_lsp_overlays(
    painter: &egui::Painter,
    hover_text: Option<&str>,
    completions: Option<(&[&CompletionItem], usize)>,
    signature_help: Option<&SignatureInfo>,
    view: &BufferView,
    visible_window: &[usize],
    rect: egui::Rect,
    gutter_width: f32,
    text_margin: f32,
    char_width: f32,
    line_height: f32,
    h_offset: f32,
) {
    render_hover_tooltip(
        painter,
        hover_text,
        view,
        visible_window,
        rect,
        gutter_width,
        text_margin,
        char_width,
        line_height,
        h_offset,
    );
    render_signature_help(
        painter,
        signature_help,
        view,
        visible_window,
        rect,
        gutter_width,
        text_margin,
        char_width,
        line_height,
        h_offset,
    );
    render_completion_popup(
        painter,
        completions,
        view,
        visible_window,
        rect,
        gutter_width,
        text_margin,
        char_width,
        line_height,
        h_offset,
    );
}

#[allow(clippy::too_many_arguments)]
fn render_hover_tooltip(
    painter: &egui::Painter,
    hover_text: Option<&str>,
    view: &BufferView,
    visible_window: &[usize],
    rect: egui::Rect,
    gutter_width: f32,
    text_margin: f32,
    char_width: f32,
    line_height: f32,
    h_offset: f32,
) {
    let Some(hover) = hover_text else { return };
    let Some(cursor_visible_offset) = visible_window
        .iter()
        .position(|&line| line == view.cursor.pos.line)
    else {
        return;
    };

    let vis_y = cursor_visible_offset as f32 * line_height;
    let tooltip_x =
        rect.left() + gutter_width + text_margin + view.cursor.pos.col as f32 * char_width
            - h_offset;
    let tooltip_y = rect.top() + vis_y - 4.0;
    let max_w = (rect.width() - gutter_width - 40.0).max(200.0);
    let lines: Vec<&str> = hover.lines().take(12).collect();
    let tooltip_h = lines.len() as f32 * 16.0 + 8.0;
    let tooltip_y = if tooltip_y - tooltip_h < rect.top() {
        rect.top() + vis_y + line_height + 4.0
    } else {
        tooltip_y - tooltip_h
    };

    let bg_rect = egui::Rect::from_min_size(
        egui::pos2(tooltip_x.max(rect.left() + gutter_width), tooltip_y),
        egui::Vec2::new(max_w, tooltip_h),
    );
    painter.rect_filled(bg_rect, 4.0, egui::Color32::from_rgb(40, 42, 54));
    painter.rect_stroke(
        bg_rect,
        4.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 85, 100)),
    );

    for (i, line) in lines.iter().enumerate() {
        painter.text(
            egui::pos2(bg_rect.left() + 6.0, bg_rect.top() + 4.0 + i as f32 * 16.0),
            egui::Align2::LEFT_TOP,
            line,
            egui::FontId::monospace(12.0),
            egui::Color32::from_rgb(200, 205, 215),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn render_signature_help(
    painter: &egui::Painter,
    signature_help: Option<&SignatureInfo>,
    view: &BufferView,
    visible_window: &[usize],
    rect: egui::Rect,
    gutter_width: f32,
    text_margin: f32,
    char_width: f32,
    line_height: f32,
    h_offset: f32,
) {
    let Some(sig) = signature_help else { return };
    let Some(cursor_visible_offset) = visible_window
        .iter()
        .position(|&line| line == view.cursor.pos.line)
    else {
        return;
    };

    let vis_y = cursor_visible_offset as f32 * line_height;
    let sig_x = rect.left() + gutter_width + text_margin + view.cursor.pos.col as f32 * char_width
        - h_offset;
    let sig_y = rect.top() + vis_y - 4.0;
    let label = &sig.label;
    let sig_h = 20.0;
    let sig_y = if sig_y - sig_h < rect.top() {
        rect.top() + vis_y + line_height + 4.0
    } else {
        sig_y - sig_h
    };

    let max_w = (rect.width() - gutter_width - 40.0).max(200.0);
    let bg_rect = egui::Rect::from_min_size(
        egui::pos2(sig_x.max(rect.left() + gutter_width), sig_y),
        egui::Vec2::new(max_w, sig_h),
    );
    painter.rect_filled(bg_rect, 4.0, egui::Color32::from_rgb(35, 38, 52));
    painter.rect_stroke(
        bg_rect,
        4.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 75, 95)),
    );

    if sig.active_parameter < sig.parameters.len() {
        let active_param = &sig.parameters[sig.active_parameter];
        if let Some(start) = label.find(active_param) {
            let before = &label[..start];
            let after = &label[start + active_param.len()..];
            let x = bg_rect.left() + 6.0;
            let dim_color = egui::Color32::from_rgb(170, 175, 190);
            let highlight_color = egui::Color32::from_rgb(255, 220, 100);
            let sig_font = egui::FontId::monospace(12.0);

            painter.text(
                egui::pos2(x, bg_rect.top() + 3.0),
                egui::Align2::LEFT_TOP,
                before,
                sig_font.clone(),
                dim_color,
            );
            let before_w = before.len() as f32 * 7.2;
            painter.text(
                egui::pos2(x + before_w, bg_rect.top() + 3.0),
                egui::Align2::LEFT_TOP,
                active_param,
                sig_font.clone(),
                highlight_color,
            );
            let param_w = active_param.len() as f32 * 7.2;
            painter.text(
                egui::pos2(x + before_w + param_w, bg_rect.top() + 3.0),
                egui::Align2::LEFT_TOP,
                after,
                sig_font,
                dim_color,
            );
            return;
        }
    }

    painter.text(
        egui::pos2(bg_rect.left() + 6.0, bg_rect.top() + 3.0),
        egui::Align2::LEFT_TOP,
        label,
        egui::FontId::monospace(12.0),
        egui::Color32::from_rgb(200, 205, 215),
    );
}

#[allow(clippy::too_many_arguments)]
fn render_completion_popup(
    painter: &egui::Painter,
    completions: Option<(&[&CompletionItem], usize)>,
    view: &BufferView,
    visible_window: &[usize],
    rect: egui::Rect,
    gutter_width: f32,
    text_margin: f32,
    char_width: f32,
    line_height: f32,
    h_offset: f32,
) {
    let Some((items, selected)) = completions else {
        return;
    };
    if items.is_empty()
        || !visible_window
            .iter()
            .any(|&line| line == view.cursor.pos.line)
    {
        return;
    }

    let vis_y = visible_window
        .iter()
        .position(|&line| line == view.cursor.pos.line)
        .unwrap_or(0) as f32
        * line_height;
    let popup_x =
        rect.left() + gutter_width + text_margin + view.cursor.pos.col as f32 * char_width
            - h_offset;
    let popup_y = rect.top() + vis_y + line_height + 2.0;
    let item_h = 20.0;
    let popup_w = 320.0;
    let popup_h = (items.len() as f32 * item_h).min(200.0) + 4.0;
    let popup_x = popup_x
        .min(rect.right() - popup_w - 4.0)
        .max(rect.left() + gutter_width);

    let bg = egui::Rect::from_min_size(
        egui::pos2(popup_x, popup_y),
        egui::Vec2::new(popup_w, popup_h),
    );
    painter.rect_filled(bg, 4.0, egui::Color32::from_rgb(30, 32, 42));
    painter.rect_stroke(
        bg,
        4.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 65, 80)),
    );

    for (i, item) in items.iter().enumerate() {
        let y = popup_y + 2.0 + i as f32 * item_h;
        if y + item_h > popup_y + popup_h {
            break;
        }

        if i == selected {
            painter.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(popup_x + 2.0, y),
                    egui::Vec2::new(popup_w - 4.0, item_h),
                ),
                2.0,
                egui::Color32::from_rgb(50, 80, 130),
            );
        }

        let kind_char = match item.kind {
            Some(lsp_types::CompletionItemKind::FUNCTION)
            | Some(lsp_types::CompletionItemKind::METHOD) => "f",
            Some(lsp_types::CompletionItemKind::VARIABLE) => "v",
            Some(lsp_types::CompletionItemKind::CLASS)
            | Some(lsp_types::CompletionItemKind::STRUCT) => "S",
            Some(lsp_types::CompletionItemKind::MODULE) => "M",
            Some(lsp_types::CompletionItemKind::KEYWORD) => "k",
            Some(lsp_types::CompletionItemKind::FIELD)
            | Some(lsp_types::CompletionItemKind::PROPERTY) => "p",
            Some(lsp_types::CompletionItemKind::CONSTANT) => "C",
            Some(lsp_types::CompletionItemKind::ENUM_MEMBER) => "e",
            Some(lsp_types::CompletionItemKind::INTERFACE) => "I",
            Some(lsp_types::CompletionItemKind::TYPE_PARAMETER) => "T",
            _ => " ",
        };
        painter.text(
            egui::pos2(popup_x + 6.0, y + 2.0),
            egui::Align2::LEFT_TOP,
            kind_char,
            egui::FontId::monospace(11.0),
            egui::Color32::from_rgb(120, 130, 160),
        );

        let label_color = if i == selected {
            egui::Color32::WHITE
        } else {
            egui::Color32::from_rgb(200, 205, 215)
        };
        painter.text(
            egui::pos2(popup_x + 22.0, y + 2.0),
            egui::Align2::LEFT_TOP,
            &item.label,
            egui::FontId::monospace(12.0),
            label_color,
        );

        if let Some(detail) = &item.detail {
            let short = if detail.len() > 30 {
                &detail[..30]
            } else {
                detail
            };
            painter.text(
                egui::pos2(popup_x + popup_w - 8.0, y + 2.0),
                egui::Align2::RIGHT_TOP,
                short,
                egui::FontId::monospace(10.0),
                egui::Color32::from_rgb(100, 105, 120),
            );
        }
    }
}
