use crate::path_utils::{
    extension_matches, path_extension_matches, CONFIG_ICON_EXTS, CPP_EXTS, CSS_ICON_EXTS, C_EXTS,
    GO_EXTS, HTML_EXTS, IMAGE_ICON_EXTS, JAVASCRIPT_EXTS, JSON_CODE_EXTS, MARKDOWN_ICON_EXTS,
    PREVIEW_IMAGE_EXTS, PYTHON_EXTS, RUST_EXTS, SHELL_ICON_EXTS, TYPESCRIPT_ICON_EXTS,
};

pub(super) fn is_image_ext(path: &std::path::Path) -> bool {
    path_extension_matches(path, PREVIEW_IMAGE_EXTS)
}

pub(super) fn file_type_icon(name: &str) -> Option<(&'static str, egui::Color32)> {
    let ext = name.rsplit('.').next().unwrap_or("");
    if extension_matches(ext, RUST_EXTS) {
        Some(("R", egui::Color32::from_rgb(230, 140, 60)))
    } else if extension_matches(ext, JAVASCRIPT_EXTS) {
        Some(("J", egui::Color32::from_rgb(240, 220, 80)))
    } else if extension_matches(ext, TYPESCRIPT_ICON_EXTS) {
        Some(("T", egui::Color32::from_rgb(70, 140, 230)))
    } else if extension_matches(ext, PYTHON_EXTS) {
        Some(("P", egui::Color32::from_rgb(80, 140, 220)))
    } else if extension_matches(ext, GO_EXTS) {
        Some(("G", egui::Color32::from_rgb(80, 200, 200)))
    } else if extension_matches(ext, JSON_CODE_EXTS) {
        Some(("{", egui::Color32::from_rgb(240, 220, 80)))
    } else if extension_matches(ext, MARKDOWN_ICON_EXTS) {
        Some(("#", egui::Color32::from_rgb(100, 200, 120)))
    } else if extension_matches(ext, CONFIG_ICON_EXTS) {
        Some(("*", egui::Color32::from_rgb(180, 140, 220)))
    } else if extension_matches(ext, HTML_EXTS) {
        Some(("<", egui::Color32::from_rgb(230, 120, 80)))
    } else if extension_matches(ext, CSS_ICON_EXTS) {
        Some(("S", egui::Color32::from_rgb(80, 160, 230)))
    } else if extension_matches(ext, SHELL_ICON_EXTS) {
        Some(("$", egui::Color32::from_rgb(130, 200, 130)))
    } else if extension_matches(ext, C_EXTS) {
        Some(("C", egui::Color32::from_rgb(100, 160, 230)))
    } else if extension_matches(ext, CPP_EXTS) {
        Some(("C", egui::Color32::from_rgb(130, 100, 230)))
    } else if ext.eq_ignore_ascii_case("java") {
        Some(("J", egui::Color32::from_rgb(230, 100, 80)))
    } else if ext.eq_ignore_ascii_case("rb") {
        Some(("R", egui::Color32::from_rgb(220, 70, 70)))
    } else if ext.eq_ignore_ascii_case("swift") {
        Some(("S", egui::Color32::from_rgb(230, 120, 60)))
    } else if ext.eq_ignore_ascii_case("lua") {
        Some(("L", egui::Color32::from_rgb(80, 80, 230)))
    } else if ext.eq_ignore_ascii_case("sql") {
        Some(("Q", egui::Color32::from_rgb(200, 180, 80)))
    } else if ext.eq_ignore_ascii_case("lock") {
        Some(("L", egui::Color32::from_rgb(120, 120, 130)))
    } else if extension_matches(ext, IMAGE_ICON_EXTS) {
        Some(("I", egui::Color32::from_rgb(180, 130, 220)))
    } else {
        None
    }
}
