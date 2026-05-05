//! App-agnostic graphics engine data model.
//!
//! This module is the boundary the app will migrate toward: feature surfaces
//! describe visual layers, and the renderer/engine decides how to draw them.

use std::fmt;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn is_empty(self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const TRANSPARENT: Self = Self::rgba(0.0, 0.0, 0.0, 0.0);
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::rgba(r, g, b, 1.0)
    }

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::TRANSPARENT
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LayerId(String);

impl LayerId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for LayerId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for LayerId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for LayerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendMode {
    Normal,
    Additive,
}

impl Default for BlendMode {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayerStyle {
    pub opacity: f32,
    pub clip: Option<Rect>,
    pub blend_mode: BlendMode,
    pub effects: EffectStack,
}

impl LayerStyle {
    pub fn visible() -> Self {
        Self {
            opacity: 1.0,
            clip: None,
            blend_mode: BlendMode::Normal,
            effects: EffectStack::default(),
        }
    }
}

impl Default for LayerStyle {
    fn default() -> Self {
        Self::visible()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EffectStack {
    pub passes: Vec<EffectPass>,
    pub mask: Option<EffectMask>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EffectPass {
    Bloom {
        intensity: f32,
    },
    Crt {
        curvature: f32,
        scanline_strength: f32,
    },
    Blur {
        radius: f32,
    },
    ColorGrade {
        saturation: f32,
        contrast: f32,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum EffectMask {
    /// Normalized UV rect [left, top, right, bottom] in the render target.
    UvRect([f32; 4]),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub id: LayerId,
    pub z_index: i32,
    pub style: LayerStyle,
    pub kind: LayerKind,
}

impl Layer {
    pub fn new(id: impl Into<LayerId>, z_index: i32, kind: LayerKind) -> Self {
        Self {
            id: id.into(),
            z_index,
            style: LayerStyle::visible(),
            kind,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum LayerKind {
    Primitives(Vec<Primitive>),
    Text(Vec<TextRun>),
    Image(ImageLayer),
    CustomGpu(CustomGpuLayer),
    Egui(EguiLayer),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Primitive {
    Rect {
        rect: Rect,
        color: Color,
    },
    StrokeRect {
        rect: Rect,
        color: Color,
        width: f32,
    },
    Line {
        from: [f32; 2],
        to: [f32; 2],
        color: Color,
        width: f32,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextRun {
    pub text: String,
    pub origin: [f32; 2],
    pub size: f32,
    pub color: Color,
    pub font_family: Option<String>,
    pub monospace: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImageLayer {
    pub asset: AssetId,
    pub rect: Rect,
    pub tint: Color,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AssetId(String);

impl AssetId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CustomPassId(String);

impl CustomPassId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CustomGpuLayer {
    pub pass: CustomPassId,
    pub bounds: Rect,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EguiLayer {
    pub bounds: Option<Rect>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FrameBudget {
    pub target_frame_ms: f32,
    pub max_text_runs: usize,
    pub max_primitives: usize,
}

impl Default for FrameBudget {
    fn default() -> Self {
        Self {
            target_frame_ms: 16.67,
            max_text_runs: 50_000,
            max_primitives: 100_000,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HitRegionId(String);

impl HitRegionId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for HitRegionId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for HitRegionId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HitRegion {
    pub id: HitRegionId,
    pub layer_id: LayerId,
    pub bounds: Rect,
    pub z_index: i32,
}

impl HitRegion {
    pub fn new(
        id: impl Into<HitRegionId>,
        layer_id: impl Into<LayerId>,
        bounds: Rect,
        z_index: i32,
    ) -> Self {
        Self {
            id: id.into(),
            layer_id: layer_id.into(),
            bounds,
            z_index,
        }
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        !self.bounds.is_empty()
            && x >= self.bounds.x
            && y >= self.bounds.y
            && x < self.bounds.x + self.bounds.width
            && y < self.bounds.y + self.bounds.height
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EngineFrame {
    pub viewport: Size,
    pub clear_color: Color,
    pub layers: Vec<Layer>,
    pub hit_regions: Vec<HitRegion>,
    pub budget: FrameBudget,
}

impl EngineFrame {
    pub fn new(viewport: Size) -> Self {
        Self {
            viewport,
            clear_color: Color::BLACK,
            layers: Vec::new(),
            hit_regions: Vec::new(),
            budget: FrameBudget::default(),
        }
    }

    pub fn push_layer(&mut self, layer: Layer) {
        self.layers.push(layer);
    }

    pub fn push_hit_region(&mut self, region: HitRegion) {
        self.hit_regions.push(region);
    }

    pub fn sorted_layers(&self) -> Vec<&Layer> {
        let mut layers: Vec<&Layer> = self.layers.iter().collect();
        layers.sort_by_key(|layer| layer.z_index);
        layers
    }

    pub fn hit_test(&self, x: f32, y: f32) -> Option<&HitRegion> {
        self.hit_regions
            .iter()
            .filter(|region| region.contains(x, y))
            .max_by_key(|region| region.z_index)
    }

    pub fn validate(&self) -> Result<(), FrameValidationError> {
        if self.viewport.width <= 0.0 || self.viewport.height <= 0.0 {
            return Err(FrameValidationError::InvalidViewport);
        }

        for layer in &self.layers {
            if layer.id.as_str().trim().is_empty() {
                return Err(FrameValidationError::EmptyLayerId);
            }
            if !(0.0..=1.0).contains(&layer.style.opacity) {
                return Err(FrameValidationError::InvalidOpacity {
                    layer: layer.id.clone(),
                    opacity: layer.style.opacity,
                });
            }
            if let Some(EffectMask::UvRect(rect)) = layer.style.effects.mask {
                if rect[0] < 0.0
                    || rect[1] < 0.0
                    || rect[2] > 1.0
                    || rect[3] > 1.0
                    || rect[0] >= rect[2]
                    || rect[1] >= rect[3]
                {
                    return Err(FrameValidationError::InvalidEffectMask {
                        layer: layer.id.clone(),
                    });
                }
            }
        }

        for region in &self.hit_regions {
            if region.id.as_str().trim().is_empty() {
                return Err(FrameValidationError::EmptyHitRegionId);
            }
            if region.layer_id.as_str().trim().is_empty() {
                return Err(FrameValidationError::EmptyLayerId);
            }
            if region.bounds.is_empty() {
                return Err(FrameValidationError::InvalidHitRegion {
                    region: region.id.clone(),
                });
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FrameValidationError {
    InvalidViewport,
    EmptyLayerId,
    EmptyHitRegionId,
    InvalidOpacity { layer: LayerId, opacity: f32 },
    InvalidEffectMask { layer: LayerId },
    InvalidHitRegion { region: HitRegionId },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorted_layers_orders_by_z_index() {
        let mut frame = EngineFrame::new(Size::new(100.0, 100.0));
        frame.push_layer(Layer::new("overlay", 20, LayerKind::Primitives(Vec::new())));
        frame.push_layer(Layer::new("content", 10, LayerKind::Primitives(Vec::new())));

        let sorted: Vec<&str> = frame
            .sorted_layers()
            .into_iter()
            .map(|layer| layer.id.as_str())
            .collect();

        assert_eq!(sorted, vec!["content", "overlay"]);
    }

    #[test]
    fn validate_rejects_empty_viewport() {
        let frame = EngineFrame::new(Size::new(0.0, 100.0));

        assert_eq!(frame.validate(), Err(FrameValidationError::InvalidViewport));
    }

    #[test]
    fn validate_rejects_empty_layer_id() {
        let mut frame = EngineFrame::new(Size::new(100.0, 100.0));
        frame.push_layer(Layer::new("", 0, LayerKind::Primitives(Vec::new())));

        assert_eq!(frame.validate(), Err(FrameValidationError::EmptyLayerId));
    }

    #[test]
    fn validate_rejects_invalid_opacity() {
        let mut frame = EngineFrame::new(Size::new(100.0, 100.0));
        let mut layer = Layer::new("bad-opacity", 0, LayerKind::Primitives(Vec::new()));
        layer.style.opacity = 1.5;
        frame.push_layer(layer);

        assert_eq!(
            frame.validate(),
            Err(FrameValidationError::InvalidOpacity {
                layer: LayerId::new("bad-opacity"),
                opacity: 1.5,
            })
        );
    }

    #[test]
    fn validate_rejects_invalid_effect_mask() {
        let mut frame = EngineFrame::new(Size::new(100.0, 100.0));
        let mut layer = Layer::new("masked", 0, LayerKind::Primitives(Vec::new()));
        layer.style.effects.mask = Some(EffectMask::UvRect([0.8, 0.0, 0.2, 1.0]));
        frame.push_layer(layer);

        assert_eq!(
            frame.validate(),
            Err(FrameValidationError::InvalidEffectMask {
                layer: LayerId::new("masked")
            })
        );
    }

    #[test]
    fn hit_test_returns_topmost_render_aligned_region() {
        let mut frame = EngineFrame::new(Size::new(100.0, 100.0));
        frame.push_hit_region(HitRegion::new(
            "background",
            "base",
            Rect::new(0.0, 0.0, 100.0, 100.0),
            0,
        ));
        frame.push_hit_region(HitRegion::new(
            "button",
            "overlay",
            Rect::new(20.0, 20.0, 20.0, 20.0),
            10,
        ));

        assert_eq!(
            frame.hit_test(25.0, 25.0).map(|region| region.id.as_str()),
            Some("button")
        );
        assert_eq!(
            frame.hit_test(2.0, 2.0).map(|region| region.id.as_str()),
            Some("background")
        );
        assert!(frame.hit_test(120.0, 2.0).is_none());
    }
}
