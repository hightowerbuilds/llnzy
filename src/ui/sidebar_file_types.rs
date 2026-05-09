pub(super) fn is_image_ext(path: &std::path::Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    matches_ext(
        ext,
        &[
            "png", "jpg", "jpeg", "gif", "bmp", "webp", "tiff", "tif", "ico",
        ],
    )
}

pub(super) fn file_type_icon(name: &str) -> Option<(&'static str, egui::Color32)> {
    let ext = name.rsplit('.').next().unwrap_or("");
    if ext.eq_ignore_ascii_case("rs") {
        Some(("R", egui::Color32::from_rgb(230, 140, 60)))
    } else if matches_ext(ext, &["js", "jsx", "mjs", "cjs"]) {
        Some(("J", egui::Color32::from_rgb(240, 220, 80)))
    } else if matches_ext(ext, &["ts", "tsx"]) {
        Some(("T", egui::Color32::from_rgb(70, 140, 230)))
    } else if matches_ext(ext, &["py", "pyi"]) {
        Some(("P", egui::Color32::from_rgb(80, 140, 220)))
    } else if ext.eq_ignore_ascii_case("go") {
        Some(("G", egui::Color32::from_rgb(80, 200, 200)))
    } else if matches_ext(ext, &["json", "jsonc"]) {
        Some(("{", egui::Color32::from_rgb(240, 220, 80)))
    } else if matches_ext(ext, &["md", "mdx"]) {
        Some(("#", egui::Color32::from_rgb(100, 200, 120)))
    } else if matches_ext(ext, &["toml", "yaml", "yml"]) {
        Some(("*", egui::Color32::from_rgb(180, 140, 220)))
    } else if matches_ext(ext, &["html", "htm"]) {
        Some(("<", egui::Color32::from_rgb(230, 120, 80)))
    } else if matches_ext(ext, &["css", "scss", "sass", "less"]) {
        Some(("S", egui::Color32::from_rgb(80, 160, 230)))
    } else if matches_ext(ext, &["sh", "bash", "zsh", "fish"]) {
        Some(("$", egui::Color32::from_rgb(130, 200, 130)))
    } else if matches_ext(ext, &["c", "h"]) {
        Some(("C", egui::Color32::from_rgb(100, 160, 230)))
    } else if matches_ext(ext, &["cpp", "cc", "cxx", "hpp", "hxx"]) {
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
    } else if matches_ext(
        ext,
        &["png", "jpg", "jpeg", "gif", "bmp", "webp", "svg", "ico"],
    ) {
        Some(("I", egui::Color32::from_rgb(180, 130, 220)))
    } else {
        None
    }
}

fn matches_ext(ext: &str, candidates: &[&str]) -> bool {
    candidates
        .iter()
        .any(|candidate| ext.eq_ignore_ascii_case(candidate))
}
