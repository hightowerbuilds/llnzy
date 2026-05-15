//! A GPUI `Element` that renders an effect frame and submits it via
//! `Window::paint_surface`. Modelled on gpui's own `elements/surface.rs`,
//! adapted to (a) regenerate the CVPixelBuffer every frame from
//! `EffectsHost::shared` and (b) self-schedule the next frame via
//! `request_animation_frame` so the effect animates continuously.
//!
//! The element itself is cheap to construct -- it carries no GPU state. The
//! wgpu device + render pipeline live in `EffectsHost::shared`, a
//! process-wide singleton lazily initialized on first paint.

use gpui::{
    relative, App, Bounds, Element, ElementId, GlobalElementId, IntoElement, LayoutId, Pixels,
    Style, Window,
};

use super::host::{app_time_seconds, EffectKind, EffectParams, EffectsHost};

pub struct EffectsElement {
    kind: EffectKind,
    params: EffectParams,
}

impl EffectsElement {
    pub fn new() -> Self {
        Self {
            kind: EffectKind::Smoke,
            params: EffectParams::default(),
        }
    }

    /// Select which shader to render. Defaults to `EffectKind::Smoke`.
    pub fn with_kind(mut self, kind: EffectKind) -> Self {
        self.kind = kind;
        self
    }

    /// Override the intensity slider value. Defaults to `EffectParams::default`.
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.params.intensity = intensity;
        self
    }

    /// Override the three palette stops. Each is RGB (sRGB byte triple); the
    /// `[3]` slot is uniform padding only (the shader ignores it).
    pub fn with_palette(mut self, c1: [u8; 3], c2: [u8; 3], c3: [u8; 3]) -> Self {
        self.params.palette = [srgb8_to_vec4(c1), srgb8_to_vec4(c2), srgb8_to_vec4(c3)];
        self
    }
}

impl Default for EffectsElement {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoElement for EffectsElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for EffectsElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

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
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        // Render at backing-pixel resolution so GPUI's bilinear sampler ends
        // up doing identity sampling on Retina displays. The CVPixelBuffer's
        // `width × height` matches the actual device pixels GPUI will draw
        // into.
        let scale = window.scale_factor();
        let pixel_width = (f32::from(bounds.size.width) * scale).round().max(1.0) as u32;
        let pixel_height = (f32::from(bounds.size.height) * scale).round().max(1.0) as u32;

        let kind = self.kind;
        let params = self.params;
        let frame = EffectsHost::with_shared(|host| {
            host.render_frame(kind, app_time_seconds(), pixel_width, pixel_height, params)
        })
        .flatten();
        if let Some(buffer) = frame {
            window.paint_surface(bounds, buffer);
        }

        window.request_animation_frame();
    }
}

/// Convert an sRGB byte triplet (0..255) into a 4-component float vector
/// (RGB in [0, 1], padded with 0 for WGSL vec4 alignment). The shader writes
/// output as linear into an `Rgba8Unorm` texture, so passing values typed in
/// sRGB hex gets reasonably close to what the palette swatches show.
fn srgb8_to_vec4(rgb: [u8; 3]) -> [f32; 4] {
    [
        rgb[0] as f32 / 255.0,
        rgb[1] as f32 / 255.0,
        rgb[2] as f32 / 255.0,
        0.0,
    ]
}
