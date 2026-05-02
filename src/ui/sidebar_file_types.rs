pub(super) fn is_image_ext(path: &std::path::Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "tiff" | "tif" | "ico"
    )
}

pub(super) fn file_type_icon(name: &str) -> Option<(&'static str, egui::Color32)> {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "rs" => Some(("R", egui::Color32::from_rgb(230, 140, 60))),
        "js" | "jsx" | "mjs" | "cjs" => Some(("J", egui::Color32::from_rgb(240, 220, 80))),
        "ts" | "tsx" => Some(("T", egui::Color32::from_rgb(70, 140, 230))),
        "py" | "pyi" => Some(("P", egui::Color32::from_rgb(80, 140, 220))),
        "go" => Some(("G", egui::Color32::from_rgb(80, 200, 200))),
        "json" | "jsonc" => Some(("{", egui::Color32::from_rgb(240, 220, 80))),
        "md" | "mdx" => Some(("#", egui::Color32::from_rgb(100, 200, 120))),
        "toml" | "yaml" | "yml" => Some(("*", egui::Color32::from_rgb(180, 140, 220))),
        "html" | "htm" => Some(("<", egui::Color32::from_rgb(230, 120, 80))),
        "css" | "scss" | "sass" | "less" => Some(("S", egui::Color32::from_rgb(80, 160, 230))),
        "sh" | "bash" | "zsh" | "fish" => Some(("$", egui::Color32::from_rgb(130, 200, 130))),
        "c" | "h" => Some(("C", egui::Color32::from_rgb(100, 160, 230))),
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Some(("C", egui::Color32::from_rgb(130, 100, 230))),
        "java" => Some(("J", egui::Color32::from_rgb(230, 100, 80))),
        "rb" => Some(("R", egui::Color32::from_rgb(220, 70, 70))),
        "swift" => Some(("S", egui::Color32::from_rgb(230, 120, 60))),
        "lua" => Some(("L", egui::Color32::from_rgb(80, 80, 230))),
        "sql" => Some(("Q", egui::Color32::from_rgb(200, 180, 80))),
        "lock" => Some(("L", egui::Color32::from_rgb(120, 120, 130))),
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "svg" | "ico" => {
            Some(("I", egui::Color32::from_rgb(180, 130, 220)))
        }
        _ => None,
    }
}
