use std::path::{Path, PathBuf};

use super::{
    RectElement, SketchDocument, SketchElement, SketchSymbolKind, StrokeElement, SymbolElement,
    TextElement,
};

pub fn default_export_file_name(active_name: Option<&str>) -> String {
    default_export_file_name_with_extension(active_name, "svg")
}

pub fn default_jpeg_export_file_name(active_name: Option<&str>) -> String {
    default_export_file_name_with_extension(active_name, "jpg")
}

fn default_export_file_name_with_extension(active_name: Option<&str>, extension: &str) -> String {
    let stem = active_name
        .map(sanitize_export_stem)
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| {
            let secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .unwrap_or(0);
            format!("sketch-{secs}")
        });
    format!("{stem}.{extension}")
}

pub fn export_svg_to_path(
    document: &SketchDocument,
    path: &Path,
    canvas_size: [f32; 2],
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("Could not create export folder: {err}"))?;
    }
    let svg = document_to_svg(document, canvas_size, canvas_size);
    std::fs::write(path, svg).map_err(|err| format!("Could not write sketch export: {err}"))
}

pub struct JpegExportResult {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
}

pub fn export_jpeg_to_path(
    document: &SketchDocument,
    path: &Path,
    canvas_size: [f32; 2],
) -> Result<JpegExportResult, String> {
    let source_width = canvas_size[0].max(1.0);
    let source_height = canvas_size[1].max(1.0);
    let output_width = source_width.round().max(1.0) as u32;
    let output_height = source_height.round().max(1.0) as u32;
    let output_size = [output_width as f32, output_height as f32];

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("Could not create export folder: {err}"))?;
    }

    let svg = document_to_svg(document, canvas_size, output_size);
    let mut options = usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    let tree = usvg::Tree::from_data(svg.as_bytes(), &options)
        .map_err(|err| format!("Could not prepare sketch export: {err}"))?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(output_width, output_height)
        .ok_or_else(|| "Could not allocate sketch export image".to_string())?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::default(),
        &mut pixmap.as_mut(),
    );

    let rgb = pixmap_to_rgb_image(pixmap.data(), output_width, output_height)?;
    let file = std::fs::File::create(path)
        .map_err(|err| format!("Could not write sketch export: {err}"))?;
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(file, 86);
    encoder
        .encode_image(&image::DynamicImage::ImageRgb8(rgb))
        .map_err(|err| format!("Could not encode sketch JPEG: {err}"))?;

    Ok(JpegExportResult {
        path: path.to_path_buf(),
        width: output_width,
        height: output_height,
    })
}

fn document_to_svg(
    document: &SketchDocument,
    canvas_size: [f32; 2],
    output_size: [f32; 2],
) -> String {
    let view_width = canvas_size[0].max(1.0);
    let view_height = canvas_size[1].max(1.0);
    let width = output_size[0].max(1.0);
    let height = output_size[1].max(1.0);
    let mut out = String::new();
    out.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width:.0}" height="{height:.0}" viewBox="0 0 {view_width:.0} {view_height:.0}">"#
    ));
    out.push_str(r##"<rect width="100%" height="100%" fill="#101216"/>"##);
    for element in &document.elements {
        match element {
            SketchElement::Stroke(stroke) => push_stroke(&mut out, stroke),
            SketchElement::Rectangle(rect) => push_rect(&mut out, rect),
            SketchElement::Text(text) => push_text(&mut out, text),
            SketchElement::Image(image) => out.push_str(&format!(
                r#"<image href="{}" x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" preserveAspectRatio="none"/>"#,
                escape_attr(&image.path),
                image.x,
                image.y,
                image.w,
                image.h
            )),
            SketchElement::Symbol(symbol) => push_symbol(&mut out, symbol),
        }
    }
    out.push_str("</svg>\n");
    out
}

fn pixmap_to_rgb_image(data: &[u8], width: u32, height: u32) -> Result<image::RgbImage, String> {
    let expected = width as usize * height as usize * 4;
    if data.len() != expected {
        return Err("Sketch export renderer returned an unexpected pixel buffer".to_string());
    }

    let mut rgb = image::RgbImage::new(width, height);
    for (pixel, rgba) in rgb.pixels_mut().zip(data.chunks_exact(4)) {
        let alpha = rgba[3] as f32 / 255.0;
        let blend = |channel: u8| -> u8 {
            (channel as f32 * alpha + 16.0 * (1.0 - alpha))
                .round()
                .clamp(0.0, 255.0) as u8
        };
        *pixel = image::Rgb([blend(rgba[0]), blend(rgba[1]), blend(rgba[2])]);
    }
    Ok(rgb)
}

fn push_stroke(out: &mut String, stroke: &StrokeElement) {
    if stroke.points.len() < 2 {
        return;
    }
    let points = stroke
        .points
        .iter()
        .map(|point| format!("{:.1},{:.1}", point.x, point.y))
        .collect::<Vec<_>>()
        .join(" ");
    out.push_str(&format!(
        r#"<polyline points="{points}" fill="none" stroke="{}" stroke-width="{:.1}" stroke-linecap="round" stroke-linejoin="round"/>"#,
        rgba(stroke.style.stroke_color),
        stroke.style.stroke_width
    ));
}

fn push_rect(out: &mut String, rect: &RectElement) {
    let fill = rect
        .style
        .fill_color
        .map(rgba)
        .unwrap_or_else(|| "none".to_string());
    out.push_str(&format!(
        r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="2" fill="{fill}" stroke="{}" stroke-width="{:.1}"/>"#,
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        rgba(rect.style.stroke_color),
        rect.style.stroke_width
    ));
}

fn push_text(out: &mut String, text: &TextElement) {
    if text.text.is_empty() {
        return;
    }
    out.push_str(&format!(
        r#"<text x="{:.1}" y="{:.1}" fill="{}" font-family="sans-serif" font-size="{:.1}">{}</text>"#,
        text.x,
        text.y + text.style.font_size,
        rgba(text.style.stroke_color),
        text.style.font_size,
        escape_text(&text.text)
    ));
}

fn push_symbol(out: &mut String, symbol: &SymbolElement) {
    push_symbol_svg(
        out,
        symbol.kind,
        symbol.x,
        symbol.y,
        symbol.w,
        symbol.h,
        symbol.style.stroke_color,
    );
}

fn push_symbol_svg(
    out: &mut String,
    kind: SketchSymbolKind,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: [u8; 4],
) {
    let stroke = rgba(color);
    match kind {
        SketchSymbolKind::Database => {
            out.push_str(&format!(
                r#"<ellipse cx="{:.1}" cy="{:.1}" rx="{:.1}" ry="10" fill="none" stroke="{stroke}" stroke-width="2"/><path d="M {:.1} {:.1} V {:.1} M {:.1} {:.1} V {:.1}" fill="none" stroke="{stroke}" stroke-width="2"/><ellipse cx="{:.1}" cy="{:.1}" rx="{:.1}" ry="10" fill="none" stroke="{stroke}" stroke-width="2"/>"#,
                x + w * 0.5,
                y + 12.0,
                w * 0.38,
                x + w * 0.12,
                y + 12.0,
                y + h - 14.0,
                x + w * 0.88,
                y + 12.0,
                y + h - 14.0,
                x + w * 0.5,
                y + h - 14.0,
                w * 0.38
            ));
        }
        SketchSymbolKind::Decision => out.push_str(&format!(
            r#"<polygon points="{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" fill="none" stroke="{stroke}" stroke-width="2"/>"#,
            x + w * 0.5,
            y,
            x + w,
            y + h * 0.5,
            x + w * 0.5,
            y + h,
            x,
            y + h * 0.5
        )),
        _ => {
            out.push_str(&format!(
                r#"<rect x="{x:.1}" y="{y:.1}" width="{w:.1}" height="{h:.1}" rx="6" fill="none" stroke="{stroke}" stroke-width="2"/>"#
            ));
        }
    }
    out.push_str(&format!(
        r#"<text x="{:.1}" y="{:.1}" fill="{stroke}" font-family="sans-serif" font-size="11" text-anchor="middle">{}</text>"#,
        x + w * 0.5,
        y + h + 14.0,
        escape_text(kind.label())
    ));
}

fn rgba(color: [u8; 4]) -> String {
    if color[3] == 255 {
        format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2])
    } else {
        format!(
            "rgba({}, {}, {}, {:.3})",
            color[0],
            color[1],
            color[2],
            color[3] as f32 / 255.0
        )
    }
}

fn sanitize_export_stem(name: &str) -> String {
    name.chars()
        .filter(|ch| ch.is_alphanumeric() || matches!(ch, '-' | '_' | ' '))
        .collect::<String>()
        .trim()
        .replace(' ', "-")
}

fn escape_attr(value: &str) -> String {
    escape_text(value).replace('"', "&quot;")
}

fn escape_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sketch::{ImageElement, SketchStyle, TextElement};

    #[test]
    fn export_name_uses_clean_active_name() {
        assert_eq!(
            default_export_file_name(Some(" API Flow! ")),
            "API-Flow.svg"
        );
        assert_eq!(
            default_jpeg_export_file_name(Some(" API Flow! ")),
            "API-Flow.jpg"
        );
    }

    #[test]
    fn svg_export_preserves_text() {
        let mut document = SketchDocument::default();
        document.elements.push(SketchElement::Text(TextElement {
            x: 10.0,
            y: 12.0,
            w: 100.0,
            h: 30.0,
            text: "A < B".to_string(),
            style: SketchStyle::default(),
        }));
        let svg = document_to_svg(&document, [200.0, 120.0], [200.0, 120.0]);
        assert!(svg.contains("A &lt; B"));
    }

    #[test]
    fn jpeg_export_preserves_canvas_size() {
        let dir = std::env::temp_dir().join("llnzy_test_sketch_jpeg_export");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("sketch.jpg");
        let mut document = SketchDocument::default();
        document.elements.push(SketchElement::Text(TextElement {
            x: 10.0,
            y: 12.0,
            w: 100.0,
            h: 30.0,
            text: "JPEG".to_string(),
            style: SketchStyle::default(),
        }));

        let result = export_jpeg_to_path(&document, &path, [1400.0, 700.0]).unwrap();

        assert_eq!(result.width, 1400);
        assert_eq!(result.height, 700);
        assert!(path.is_file());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn jpeg_export_includes_image_layers() {
        let dir = std::env::temp_dir().join("llnzy_test_sketch_jpeg_image_export");
        let _ = std::fs::create_dir_all(&dir);
        let source = dir.join("source.png");
        let output = dir.join("sketch.jpg");
        let mut source_image = image::RgbImage::new(16, 16);
        for pixel in source_image.pixels_mut() {
            *pixel = image::Rgb([240, 32, 24]);
        }
        source_image.save(&source).unwrap();

        let mut document = SketchDocument::default();
        document.elements.push(SketchElement::Image(ImageElement {
            x: 0.0,
            y: 0.0,
            w: 16.0,
            h: 16.0,
            original_w: 16.0,
            original_h: 16.0,
            path: source.to_string_lossy().into_owned(),
        }));

        let result = export_jpeg_to_path(&document, &output, [16.0, 16.0]).unwrap();
        let exported = image::open(&result.path).unwrap().to_rgb8();
        let pixel = exported.get_pixel(8, 8);

        assert!(pixel[0] > 180, "red channel should preserve image layer");
        assert!(pixel[1] < 90, "green channel should preserve image layer");
        assert!(pixel[2] < 90, "blue channel should preserve image layer");
        let _ = std::fs::remove_file(source);
        let _ = std::fs::remove_file(output);
    }
}
