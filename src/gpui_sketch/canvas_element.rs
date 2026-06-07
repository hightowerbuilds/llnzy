use std::path::PathBuf;

use gpui::prelude::*;
use gpui::{
    div, img, px, relative, rgb, rgba, App, Bounds, ContentMask, DispatchPhase, Element, ElementId,
    Entity, GlobalElementId, LayoutId, ObjectFit, PaintQuad, Path as GpuiPath, Pixels, Point,
    ScrollWheelEvent, Style, StyledImage, TextAlign, Window, WrappedLine,
};

use crate::sketch::{normalize_zoom_scale, sketch_to_canvas_point, ImageElement, SketchPoint, SketchState};

use super::paint::build_canvas_paint;
use super::SketchSurface;

pub(super) struct SketchCanvasElement {
    pub(super) surface: Entity<SketchSurface>,
    pub(super) layer: SketchCanvasLayer,
}

#[derive(Clone, Copy)]
pub(super) enum SketchCanvasLayer {
    Background,
    Foreground,
}

pub(super) fn sketch_canvas_layer(
    surface: Entity<SketchSurface>,
    layer: SketchCanvasLayer,
) -> impl IntoElement {
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .child(SketchCanvasElement { surface, layer })
}

pub(super) struct SketchPrepaintState {
    pub(super) quads: Vec<PaintQuad>,
    pub(super) paths: Vec<(GpuiPath<Pixels>, u32)>,
    pub(super) text: Vec<SketchPaintText>,
}

pub(super) struct SketchPaintText {
    pub(super) line: WrappedLine,
    pub(super) origin: Point<Pixels>,
    pub(super) line_height: Pixels,
}

#[derive(Clone, Copy)]
pub(super) struct SketchCanvasFrame {
    pub(super) bounds: Bounds<Pixels>,
    pub(super) pad_offset: SketchPoint,
    pub(super) zoom_scale: f32,
}

impl IntoElement for SketchCanvasElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for SketchCanvasElement {
    type RequestLayoutState = ();
    type PrepaintState = SketchPrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        style.size.height = relative(1.0).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let surface = self.surface.read(cx);
        build_canvas_paint(
            &surface.state,
            bounds,
            self.layer,
            surface.light_mode,
            window,
        )
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.surface.update(cx, |surface, _cx| {
            surface.last_bounds = Some(bounds);
            surface.state.last_canvas_size =
                [bounds.size.width / px(1.0), bounds.size.height / px(1.0)];
        });

        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            for quad in prepaint.quads.drain(..) {
                window.paint_quad(quad);
            }
            for (path, color) in prepaint.paths.drain(..) {
                window.paint_path(path, rgba(color));
            }
            for text in prepaint.text.drain(..) {
                text.line
                    .paint(
                        text.origin,
                        text.line_height,
                        TextAlign::Left,
                        Some(bounds),
                        window,
                        cx,
                    )
                    .ok();
            }
        });

        if matches!(self.layer, SketchCanvasLayer::Foreground) {
            let surface = self.surface.clone();
            window.on_mouse_event(move |event: &ScrollWheelEvent, phase, _window, cx| {
                if phase == DispatchPhase::Bubble && bounds.contains(&event.position) {
                    let delta = event.delta.pixel_delta(px(16.0));
                    surface.update(cx, |surface, cx| {
                        surface.zoom_from_scroll(event.position, delta.y / px(1.0), cx);
                    });
                }
            });
        }
    }
}

pub(super) fn sketch_image_layer(state: &SketchState) -> gpui::Div {
    state
        .document
        .elements
        .iter()
        .filter_map(|element| match element {
            crate::sketch::SketchElement::Image(image) => Some(image.clone()),
            _ => None,
        })
        .fold(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .overflow_hidden(),
            |layer, image| {
                layer.child(sketch_image_element(
                    image,
                    state.pad_offset,
                    state.zoom_scale,
                ))
            },
        )
}

pub(super) fn sketch_image_element(
    image: ImageElement,
    pad_offset: SketchPoint,
    zoom_scale: f32,
) -> impl IntoElement {
    let zoom_scale = normalize_zoom_scale(zoom_scale);
    let origin = sketch_to_canvas_point(SketchPoint::new(image.x, image.y), pad_offset, zoom_scale);
    div()
        .absolute()
        .left(px(origin.x))
        .top(px(origin.y))
        .w(px((image.w.max(1.0) * zoom_scale).max(1.0)))
        .h(px((image.h.max(1.0) * zoom_scale).max(1.0)))
        .overflow_hidden()
        .border_1()
        .border_color(rgb(0x111722))
        .child(
            img(PathBuf::from(image.path))
                .size_full()
                .object_fit(ObjectFit::Fill),
        )
}
