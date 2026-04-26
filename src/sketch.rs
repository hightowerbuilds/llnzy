use std::path::{Path, PathBuf};

const CURRENT_VERSION: u32 = 1;
const MIN_POINTS_FOR_STROKE: usize = 2;
const MIN_RECT_SIZE: f32 = 4.0;
const DEFAULT_TEXT_W: f32 = 180.0;
const DEFAULT_TEXT_H: f32 = 48.0;

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
        style: SketchStyle,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextDraft {
    pub index: usize,
    pub text: String,
}

pub struct SketchState {
    pub document: SketchDocument,
    pub tool: SketchTool,
    pub style: SketchStyle,
    pub draft: Option<DraftElement>,
    pub selected: Option<usize>,
    pub text_draft: Option<TextDraft>,
    undo_stack: Vec<SketchDocument>,
    redo_stack: Vec<SketchDocument>,
    dirty: bool,
}

impl Default for SketchState {
    fn default() -> Self {
        Self {
            document: SketchDocument::default(),
            tool: SketchTool::Marker,
            style: SketchStyle::default(),
            draft: None,
            selected: None,
            text_draft: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            dirty: false,
        }
    }
}

impl SketchState {
    pub fn load_default() -> Self {
        let document = sketch_path()
            .and_then(|path| load_document_from_path(&path).ok())
            .unwrap_or_default();
        Self {
            document,
            ..Self::default()
        }
    }

    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn set_tool(&mut self, tool: SketchTool) {
        self.tool = tool;
        self.draft = None;
        if tool != SketchTool::Select {
            self.selected = None;
        }
    }

    pub fn begin_stroke(&mut self, point: SketchPoint) {
        self.draft = Some(DraftElement::Stroke(StrokeElement {
            points: vec![point],
            style: self.style,
        }));
    }

    pub fn append_stroke_point(&mut self, point: SketchPoint) {
        let Some(DraftElement::Stroke(stroke)) = &mut self.draft else {
            return;
        };
        if stroke
            .points
            .last()
            .is_some_and(|last| distance(*last, point) < 1.5)
        {
            return;
        }
        stroke.points.push(point);
    }

    pub fn finish_stroke(&mut self) -> bool {
        let Some(DraftElement::Stroke(stroke)) = self.draft.take() else {
            return false;
        };
        if stroke.points.len() < MIN_POINTS_FOR_STROKE {
            return false;
        }
        self.push_undo();
        self.document.elements.push(SketchElement::Stroke(stroke));
        self.selected = None;
        self.dirty = true;
        true
    }

    pub fn begin_rectangle(&mut self, point: SketchPoint) {
        self.draft = Some(DraftElement::Rectangle {
            start: point,
            current: point,
            style: self.style,
        });
    }

    pub fn update_rectangle(&mut self, point: SketchPoint) {
        if let Some(DraftElement::Rectangle { current, .. }) = &mut self.draft {
            *current = point;
        }
    }

    pub fn finish_rectangle(&mut self) -> bool {
        let Some(DraftElement::Rectangle {
            start,
            current,
            style,
        }) = self.draft.take()
        else {
            return false;
        };
        let rect = normalized_rect(start, current, style);
        if rect.w < MIN_RECT_SIZE || rect.h < MIN_RECT_SIZE {
            return false;
        }
        self.push_undo();
        self.document.elements.push(SketchElement::Rectangle(rect));
        self.selected = None;
        self.dirty = true;
        true
    }

    pub fn add_text_box(&mut self, point: SketchPoint) -> usize {
        self.push_undo();
        let index = self.document.elements.len();
        self.document
            .elements
            .push(SketchElement::Text(TextElement {
                x: point.x,
                y: point.y,
                w: DEFAULT_TEXT_W,
                h: DEFAULT_TEXT_H,
                text: String::new(),
                style: self.style,
            }));
        self.selected = Some(index);
        self.text_draft = Some(TextDraft {
            index,
            text: String::new(),
        });
        self.dirty = true;
        index
    }

    pub fn update_text_draft(&mut self, text: String) {
        if let Some(draft) = &mut self.text_draft {
            draft.text = text;
        }
    }

    pub fn commit_text_draft(&mut self) {
        let Some(draft) = self.text_draft.take() else {
            return;
        };
        let text = draft.text.trim().to_string();
        if text.is_empty() {
            if draft.index < self.document.elements.len() {
                self.document.elements.remove(draft.index);
                self.selected = None;
                self.dirty = true;
            }
            return;
        }
        if let Some(SketchElement::Text(text_box)) = self.document.elements.get_mut(draft.index) {
            text_box.text = text;
            self.selected = Some(draft.index);
            self.dirty = true;
        }
    }

    pub fn cancel_text_draft(&mut self) {
        let Some(draft) = self.text_draft.take() else {
            return;
        };
        if draft.index < self.document.elements.len() {
            self.document.elements.remove(draft.index);
            self.selected = None;
            self.dirty = true;
        }
    }

    pub fn select_at(&mut self, point: SketchPoint) -> Option<usize> {
        self.selected = self.hit_test(point);
        self.selected
    }

    pub fn delete_selected(&mut self) -> bool {
        let Some(index) = self.selected else {
            return false;
        };
        if index >= self.document.elements.len() {
            self.selected = None;
            return false;
        }
        self.push_undo();
        self.document.elements.remove(index);
        self.selected = None;
        self.text_draft = None;
        self.dirty = true;
        true
    }

    pub fn clear(&mut self) -> bool {
        if self.document.elements.is_empty() {
            return false;
        }
        self.push_undo();
        self.document.elements.clear();
        self.selected = None;
        self.text_draft = None;
        self.draft = None;
        self.dirty = true;
        true
    }

    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.undo_stack.pop() else {
            return false;
        };
        self.redo_stack.push(self.document.clone());
        self.document = previous;
        self.selected = None;
        self.text_draft = None;
        self.draft = None;
        self.dirty = true;
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo_stack.pop() else {
            return false;
        };
        self.undo_stack.push(self.document.clone());
        self.document = next;
        self.selected = None;
        self.text_draft = None;
        self.draft = None;
        self.dirty = true;
        true
    }

    pub fn hit_test(&self, point: SketchPoint) -> Option<usize> {
        self.document
            .elements
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, element)| element_contains(element, point).then_some(index))
    }

    fn push_undo(&mut self) {
        self.undo_stack.push(self.document.clone());
        self.redo_stack.clear();
    }
}

pub fn sketch_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("llnzy").join("sketches").join("scratch.json"))
}

pub fn load_document_from_path(path: &Path) -> Result<SketchDocument, String> {
    let data = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

pub fn save_document_to_path(document: &SketchDocument, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(document).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

pub fn save_default_document(document: &SketchDocument) -> Result<(), String> {
    let Some(path) = sketch_path() else {
        return Ok(());
    };
    save_document_to_path(document, &path)
}

fn normalized_rect(start: SketchPoint, current: SketchPoint, style: SketchStyle) -> RectElement {
    let x = start.x.min(current.x);
    let y = start.y.min(current.y);
    let w = (start.x - current.x).abs();
    let h = (start.y - current.y).abs();
    RectElement { x, y, w, h, style }
}

fn element_contains(element: &SketchElement, point: SketchPoint) -> bool {
    match element {
        SketchElement::Stroke(stroke) => stroke.points.windows(2).any(|pair| {
            distance_to_segment(point, pair[0], pair[1]) <= stroke.style.stroke_width.max(6.0)
        }),
        SketchElement::Rectangle(rect) => {
            point.x >= rect.x
                && point.x <= rect.x + rect.w
                && point.y >= rect.y
                && point.y <= rect.y + rect.h
        }
        SketchElement::Text(text) => {
            point.x >= text.x
                && point.x <= text.x + text.w
                && point.y >= text.y
                && point.y <= text.y + text.h
        }
    }
}

fn distance(a: SketchPoint, b: SketchPoint) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

fn distance_to_segment(point: SketchPoint, start: SketchPoint, end: SketchPoint) -> f32 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq == 0.0 {
        return distance(point, start);
    }
    let t = (((point.x - start.x) * dx + (point.y - start.y) * dy) / len_sq).clamp(0.0, 1.0);
    let projection = SketchPoint::new(start.x + t * dx, start.y + t * dy);
    distance(point, projection)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: f32, y: f32) -> SketchPoint {
        SketchPoint::new(x, y)
    }

    #[test]
    fn marker_stroke_commits_with_two_points() {
        let mut state = SketchState::default();
        state.begin_stroke(point(1.0, 2.0));
        state.append_stroke_point(point(8.0, 9.0));

        assert!(state.finish_stroke());
        assert_eq!(state.document.elements.len(), 1);
        assert!(state.is_dirty());
    }

    #[test]
    fn marker_stroke_ignores_single_point() {
        let mut state = SketchState::default();
        state.begin_stroke(point(1.0, 2.0));

        assert!(!state.finish_stroke());
        assert!(state.document.elements.is_empty());
    }

    #[test]
    fn rectangle_normalizes_drag_direction() {
        let mut state = SketchState::default();
        state.begin_rectangle(point(20.0, 30.0));
        state.update_rectangle(point(5.0, 10.0));

        assert!(state.finish_rectangle());
        let SketchElement::Rectangle(rect) = &state.document.elements[0] else {
            panic!("expected rectangle");
        };
        assert_eq!(rect.x, 5.0);
        assert_eq!(rect.y, 10.0);
        assert_eq!(rect.w, 15.0);
        assert_eq!(rect.h, 20.0);
    }

    #[test]
    fn tiny_rectangle_is_discarded() {
        let mut state = SketchState::default();
        state.begin_rectangle(point(1.0, 1.0));
        state.update_rectangle(point(2.0, 2.0));

        assert!(!state.finish_rectangle());
        assert!(state.document.elements.is_empty());
    }

    #[test]
    fn empty_text_box_is_removed_on_commit() {
        let mut state = SketchState::default();
        state.add_text_box(point(10.0, 10.0));
        state.commit_text_draft();

        assert!(state.document.elements.is_empty());
    }

    #[test]
    fn text_box_commit_keeps_trimmed_text() {
        let mut state = SketchState::default();
        state.add_text_box(point(10.0, 10.0));
        state.update_text_draft("  idea map  ".to_string());
        state.commit_text_draft();

        let SketchElement::Text(text) = &state.document.elements[0] else {
            panic!("expected text box");
        };
        assert_eq!(text.text, "idea map");
    }

    #[test]
    fn undo_redo_round_trip() {
        let mut state = SketchState::default();
        state.begin_stroke(point(1.0, 1.0));
        state.append_stroke_point(point(10.0, 10.0));
        state.finish_stroke();

        assert!(state.undo());
        assert!(state.document.elements.is_empty());
        assert!(state.redo());
        assert_eq!(state.document.elements.len(), 1);
    }

    #[test]
    fn hit_test_returns_topmost_element() {
        let mut state = SketchState::default();
        state.begin_rectangle(point(0.0, 0.0));
        state.update_rectangle(point(100.0, 100.0));
        state.finish_rectangle();
        state.add_text_box(point(10.0, 10.0));
        state.update_text_draft("top".to_string());
        state.commit_text_draft();

        assert_eq!(state.hit_test(point(20.0, 20.0)), Some(1));
    }

    #[test]
    fn serialization_round_trip() {
        let mut document = SketchDocument::default();
        document.elements.push(SketchElement::Text(TextElement {
            x: 1.0,
            y: 2.0,
            w: 100.0,
            h: 40.0,
            text: "hello".to_string(),
            style: SketchStyle::default(),
        }));

        let json = serde_json::to_string(&document).unwrap();
        let decoded: SketchDocument = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded, document);
    }
}
