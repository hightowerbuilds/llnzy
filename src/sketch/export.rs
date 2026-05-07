use std::path::Path;

use super::{
    RectElement, SketchDocument, SketchElement, SketchPoint, SketchSymbolKind, StrokeElement,
    SymbolElement, TextElement,
};

pub fn default_export_file_name(active_name: Option<&str>) -> String {
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
    format!("{stem}.svg")
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
    let svg = document_to_svg(document, canvas_size);
    std::fs::write(path, svg).map_err(|err| format!("Could not write sketch export: {err}"))
}

fn document_to_svg(document: &SketchDocument, canvas_size: [f32; 2]) -> String {
    let width = canvas_size[0].max(1.0);
    let height = canvas_size[1].max(1.0);
    let mut out = String::new();
    out.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width:.0}" height="{height:.0}" viewBox="0 0 {width:.0} {height:.0}">"#
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

#[allow(dead_code)]
fn point_attr(point: SketchPoint) -> String {
    format!("{:.1},{:.1}", point.x, point.y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sketch::{SketchStyle, TextElement};

    #[test]
    fn export_name_uses_clean_active_name() {
        assert_eq!(
            default_export_file_name(Some(" API Flow! ")),
            "API-Flow.svg"
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
        let svg = document_to_svg(&document, [200.0, 120.0]);
        assert!(svg.contains("A &lt; B"));
    }
}
