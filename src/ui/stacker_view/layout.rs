pub(super) const S: f32 = 14.0;
pub(super) const MIN_EDITOR_FONT_SIZE: f32 = 12.0;
pub(super) const MAX_EDITOR_FONT_SIZE: f32 = 24.0;
pub(super) const ATKINSON: &str = "Atkinson Hyperlegible";
pub(super) const MUTED: egui::Color32 = egui::Color32::from_rgb(130, 130, 145);
pub(super) const HEADING_COLOR: egui::Color32 = egui::Color32::from_rgb(200, 200, 210);
pub(super) const DIM: egui::Color32 = egui::Color32::from_rgb(90, 92, 105);
pub(super) const ROW_BG: egui::Color32 = egui::Color32::from_rgb(30, 30, 30);
pub(super) const ROW_HOVER: egui::Color32 = egui::Color32::from_rgb(42, 42, 42);
pub(super) const PANEL_BG: egui::Color32 = egui::Color32::from_rgb(28, 28, 28);
pub(super) const NOTE_BG: egui::Color32 = PANEL_BG;
pub(super) const NOTE_TEXT: egui::Color32 = egui::Color32::from_rgb(240, 248, 255);
pub(super) const QUEUE_GREEN: egui::Color32 = egui::Color32::from_rgb(106, 255, 144);
pub(super) const NOTE_PADDING: f32 = 34.0;
pub(super) const EDITOR_BOTTOM_GAP: f32 = 20.0;

pub(super) fn small(text: &str) -> egui::RichText {
    egui::RichText::new(text).size(S)
}

pub(super) fn stacker_editor_font(font_size: f32) -> egui::FontId {
    egui::FontId::new(font_size, egui::FontFamily::Name(ATKINSON.into()))
}

pub(super) fn header_label(text: &str) -> egui::Label {
    egui::Label::new(egui::RichText::new(text).size(11.0).color(DIM).strong())
}

/// Truncate to first line, capped at `max_chars`.
pub(super) fn truncate_line(text: &str, max_chars: usize) -> String {
    let first_line = text.lines().next().unwrap_or("");
    if first_line.len() > max_chars {
        format!("{}...", &first_line[..max_chars])
    } else if text.lines().count() > 1 {
        format!("{}...", first_line)
    } else {
        first_line.to_string()
    }
}
