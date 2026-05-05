use super::CURRENT_VERSION;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SketchTool {
    Select,
    Marker,
    Rectangle,
    Text,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SketchPoint {
    pub x: f32,
    pub y: f32,
}

impl SketchPoint {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn translated(self, dx: f32, dy: f32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SketchStyle {
    pub stroke_color: [u8; 4],
    pub fill_color: Option<[u8; 4]>,
    pub stroke_width: f32,
    pub font_size: f32,
}

impl Default for SketchStyle {
    fn default() -> Self {
        Self {
            stroke_color: [235, 238, 245, 255],
            fill_color: None,
            stroke_width: 3.0,
            font_size: 18.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StrokeElement {
    pub points: Vec<SketchPoint>,
    pub style: SketchStyle,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RectElement {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub style: SketchStyle,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TextElement {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub text: String,
    pub style: SketchStyle,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SketchElement {
    Stroke(StrokeElement),
    Rectangle(RectElement),
    Text(TextElement),
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SketchDocument {
    pub version: u32,
    pub elements: Vec<SketchElement>,
}

impl Default for SketchDocument {
    fn default() -> Self {
        Self {
            version: CURRENT_VERSION,
            elements: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum DraftElement {
    Stroke(StrokeElement),
    Rectangle {
        start: SketchPoint,
        current: SketchPoint,
        constrain_square: bool,
        from_center: bool,
        style: SketchStyle,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextDraft {
    pub index: usize,
    pub text: String,
    pub is_new: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MoveDraft {
    pub index: usize,
    pub last_point: SketchPoint,
    pub moved: bool,
}
