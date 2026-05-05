use crate::engine::{Color, EngineFrame, LayerKind, Primitive, TextRun};

pub(super) fn primitive_rects(
    primitives: &[Primitive],
    layer_opacity: f32,
) -> Vec<(f32, f32, f32, f32, [f32; 4])> {
    let mut rects = Vec::new();
    for primitive in primitives {
        match primitive {
            Primitive::Rect { rect, color } => {
                if !rect.is_empty() {
                    rects.push((
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                        color_with_opacity(*color, layer_opacity),
                    ));
                }
            }
            Primitive::StrokeRect { rect, color, width } => {
                if !rect.is_empty() && *width > 0.0 {
                    let width = width.min(rect.width / 2.0).min(rect.height / 2.0);
                    let color = color_with_opacity(*color, layer_opacity);
                    rects.push((rect.x, rect.y, rect.width, width, color));
                    rects.push((
                        rect.x,
                        rect.y + rect.height - width,
                        rect.width,
                        width,
                        color,
                    ));
                    rects.push((rect.x, rect.y, width, rect.height, color));
                    rects.push((
                        rect.x + rect.width - width,
                        rect.y,
                        width,
                        rect.height,
                        color,
                    ));
                }
            }
            Primitive::Line {
                from,
                to,
                color,
                width,
            } => {
                if *width > 0.0 {
                    let x = from[0].min(to[0]);
                    let y = from[1].min(to[1]);
                    let w = (from[0] - to[0]).abs().max(*width);
                    let h = (from[1] - to[1]).abs().max(*width);
                    rects.push((x, y, w, h, color_with_opacity(*color, layer_opacity)));
                }
            }
        }
    }
    rects
}

pub(super) fn offset_rects(
    rects: &mut [(f32, f32, f32, f32, [f32; 4])],
    offset_x: f32,
    offset_y: f32,
) {
    for rect in rects {
        rect.0 += offset_x;
        rect.1 += offset_y;
    }
}

pub(super) fn offset_highlight_rects(
    search_rects: &[(f32, f32, f32, f32, [f32; 4])],
    selection_rects: &[(f32, f32, f32, f32, [f32; 4])],
    offset_x: f32,
    offset_y: f32,
) -> Vec<(f32, f32, f32, f32, [f32; 4])> {
    let total = search_rects.len() + selection_rects.len();
    if total == 0 {
        return Vec::new();
    }

    let mut rects = Vec::with_capacity(total);
    rects.extend(
        search_rects
            .iter()
            .chain(selection_rects)
            .map(|&(x, y, width, height, color)| {
                (x + offset_x, y + offset_y, width, height, color)
            }),
    );
    rects
}

#[cfg(test)]
fn primitive_layer<'a>(frame: &'a EngineFrame, layer_id: &str) -> &'a [Primitive] {
    frame
        .layers
        .iter()
        .find_map(|layer| {
            if layer.id.as_str() == layer_id {
                if let LayerKind::Primitives(primitives) = &layer.kind {
                    return Some(primitives.as_slice());
                }
            }
            None
        })
        .unwrap_or(&[])
}

pub(super) fn text_layer<'a>(frame: &'a EngineFrame, layer_id: &str) -> &'a [TextRun] {
    frame
        .layers
        .iter()
        .find_map(|layer| {
            if layer.id.as_str() == layer_id {
                if let LayerKind::Text(runs) = &layer.kind {
                    return Some(runs.as_slice());
                }
            }
            None
        })
        .unwrap_or(&[])
}

fn color_with_opacity(color: Color, layer_opacity: f32) -> [f32; 4] {
    [
        color.r,
        color.g,
        color.b,
        color.a * layer_opacity.clamp(0.0, 1.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Size;

    #[test]
    fn primitive_rects_applies_layer_opacity() {
        let rects = primitive_rects(
            &[Primitive::Rect {
                rect: crate::engine::Rect::new(1.0, 2.0, 3.0, 4.0),
                color: Color::rgba(0.1, 0.2, 0.3, 0.5),
            }],
            0.5,
        );

        assert_eq!(rects, vec![(1.0, 2.0, 3.0, 4.0, [0.1, 0.2, 0.3, 0.25])]);
    }

    #[test]
    fn primitive_rects_expands_stroke_rect() {
        let rects = primitive_rects(
            &[Primitive::StrokeRect {
                rect: crate::engine::Rect::new(10.0, 20.0, 30.0, 40.0),
                color: Color::rgba(1.0, 1.0, 1.0, 1.0),
                width: 2.0,
            }],
            1.0,
        );

        assert_eq!(rects.len(), 4);
        assert_eq!(rects[0], (10.0, 20.0, 30.0, 2.0, [1.0, 1.0, 1.0, 1.0]));
    }

    #[test]
    fn primitive_layer_returns_named_primitive_layer() {
        let mut frame = EngineFrame::new(Size::new(100.0, 100.0));
        frame.push_layer(crate::engine::Layer::new(
            "hits",
            10,
            LayerKind::Primitives(vec![Primitive::Rect {
                rect: crate::engine::Rect::new(0.0, 0.0, 1.0, 1.0),
                color: Color::rgba(1.0, 1.0, 1.0, 1.0),
            }]),
        ));

        assert_eq!(primitive_layer(&frame, "hits").len(), 1);
        assert!(primitive_layer(&frame, "missing").is_empty());
    }

    #[test]
    fn text_layer_returns_named_text_layer() {
        let mut frame = EngineFrame::new(Size::new(100.0, 100.0));
        frame.push_layer(crate::engine::Layer::new(
            "label",
            10,
            LayerKind::Text(vec![TextRun {
                text: "hello".to_string(),
                origin: [0.0, 0.0],
                size: 12.0,
                color: Color::rgba(1.0, 1.0, 1.0, 1.0),
                font_family: None,
                monospace: true,
            }]),
        ));

        assert_eq!(text_layer(&frame, "label").len(), 1);
        assert!(text_layer(&frame, "missing").is_empty());
    }

    #[test]
    fn offset_highlight_rects_skips_allocation_when_empty() {
        let rects = offset_highlight_rects(&[], &[], 10.0, 20.0);

        assert!(rects.is_empty());
        assert_eq!(rects.capacity(), 0);
    }

    #[test]
    fn offset_highlight_rects_offsets_search_and_selection_in_order() {
        let search_rects = [(2.0, 3.0, 16.0, 16.0, [1.0, 0.0, 0.0, 0.5])];
        let selection_rects = [(5.0, 7.0, 24.0, 16.0, [0.0, 0.0, 1.0, 0.35])];

        let rects = offset_highlight_rects(&search_rects, &selection_rects, 10.0, 40.0);

        assert_eq!(
            rects,
            vec![
                (12.0, 43.0, 16.0, 16.0, [1.0, 0.0, 0.0, 0.5]),
                (15.0, 47.0, 24.0, 16.0, [0.0, 0.0, 1.0, 0.35]),
            ]
        );
        assert!(rects.capacity() >= rects.len());
    }
}
