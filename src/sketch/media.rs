use std::path::{Path, PathBuf};

pub fn fit_image_size(width: u32, height: u32, max_side: f32) -> (f32, f32) {
    let w = width.max(1) as f32;
    let h = height.max(1) as f32;
    let scale = (max_side / w.max(h)).min(1.0);
    ((w * scale).max(1.0), (h * scale).max(1.0))
}

pub fn import_sketch_image(source: &Path) -> Result<(PathBuf, u32, u32), String> {
    let (width, height) =
        image::image_dimensions(source).map_err(|err| format!("Cannot decode image: {err}"))?;
    let Some(dir) = sketch_images_dir() else {
        return Err("Could not resolve Sketcher image library".to_string());
    };
    std::fs::create_dir_all(&dir)
        .map_err(|err| format!("Could not create Sketcher image library: {err}"))?;
    let filename = source
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("sketch-image.png");
    let destination = unique_destination(&dir, filename);
    std::fs::copy(source, &destination).map_err(|err| format!("Could not import image: {err}"))?;
    Ok((destination, width, height))
}

fn sketch_images_dir() -> Option<PathBuf> {
    crate::platform::paths::current_paths().map(|paths| paths.sketches_dir().join("images"))
}

fn unique_destination(dir: &Path, filename: &str) -> PathBuf {
    let source = Path::new(filename);
    let stem = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("sketch-image");
    let ext = source
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("png");
    let mut candidate = dir.join(format!("{stem}.{ext}"));
    let mut index = 1;
    while candidate.exists() {
        candidate = dir.join(format!("{stem}_{index}.{ext}"));
        index += 1;
    }
    candidate
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_image_size_scales_large_images_down() {
        assert_eq!(fit_image_size(2000, 1000, 400.0), (400.0, 200.0));
        assert_eq!(fit_image_size(100, 50, 400.0), (100.0, 50.0));
    }
}
