const S: f32 = 16.0;

pub(super) fn label(text: &str) -> egui::RichText {
    egui::RichText::new(text).size(S)
}
