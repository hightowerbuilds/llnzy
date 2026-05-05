use super::serialization::sanitize_sketch_name;
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
fn rectangle_can_constrain_to_square() {
    let mut state = SketchState::default();
    state.begin_rectangle(point(0.0, 0.0));
    state.update_rectangle_with_modifiers(point(20.0, 5.0), true, false);

    assert!(state.finish_rectangle());
    let SketchElement::Rectangle(rect) = &state.document.elements[0] else {
        panic!("expected rectangle");
    };
    assert_eq!(rect.w, 20.0);
    assert_eq!(rect.h, 20.0);
}

#[test]
fn rectangle_can_draw_from_center() {
    let mut state = SketchState::default();
    state.begin_rectangle(point(10.0, 10.0));
    state.update_rectangle_with_modifiers(point(20.0, 25.0), false, true);

    assert!(state.finish_rectangle());
    let SketchElement::Rectangle(rect) = &state.document.elements[0] else {
        panic!("expected rectangle");
    };
    assert_eq!(rect.x, 0.0);
    assert_eq!(rect.y, -5.0);
    assert_eq!(rect.w, 20.0);
    assert_eq!(rect.h, 30.0);
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
fn existing_text_box_can_be_edited() {
    let mut state = SketchState::default();
    let index = state.add_text_box(point(10.0, 10.0));
    state.update_text_draft("old".to_string());
    state.commit_text_draft();

    assert!(state.edit_text_box(index));
    state.update_text_draft("new".to_string());
    state.commit_text_draft();

    let SketchElement::Text(text) = &state.document.elements[0] else {
        panic!("expected text box");
    };
    assert_eq!(text.text, "new");
}

#[test]
fn selected_rectangle_can_move() {
    let mut state = SketchState::default();
    state.begin_rectangle(point(0.0, 0.0));
    state.update_rectangle(point(20.0, 20.0));
    state.finish_rectangle();
    state.selected = Some(0);

    assert!(state.begin_move_selected(point(5.0, 5.0)));
    assert!(state.update_move_selected(point(15.0, 25.0)));
    assert!(state.finish_move_selected());

    let SketchElement::Rectangle(rect) = &state.document.elements[0] else {
        panic!("expected rectangle");
    };
    assert_eq!(rect.x, 10.0);
    assert_eq!(rect.y, 20.0);
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

#[test]
fn new_sketch_clears_everything() {
    let mut state = SketchState::default();
    state.begin_stroke(point(1.0, 1.0));
    state.append_stroke_point(point(10.0, 10.0));
    state.finish_stroke();
    state.active_sketch_name = Some("test".to_string());

    state.new_sketch();

    assert!(state.document.elements.is_empty());
    assert!(state.active_sketch_name.is_none());
    assert!(!state.can_undo());
    assert!(!state.can_redo());
}

#[test]
fn save_and_load_via_path() {
    let dir = std::env::temp_dir().join("llnzy_test_sketch");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("roundtrip_test.json");
    let mut document = SketchDocument::default();
    document.elements.push(SketchElement::Text(TextElement {
        x: 5.0,
        y: 10.0,
        w: 100.0,
        h: 40.0,
        text: "named sketch".to_string(),
        style: SketchStyle::default(),
    }));

    save_document_to_path(&document, &path).unwrap();
    let loaded = load_document_from_path(&path).unwrap();

    assert_eq!(loaded, document);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sanitize_sketch_name_removes_special_chars() {
    assert_eq!(sanitize_sketch_name("  my sketch!@#  "), "my sketch");
    assert_eq!(sanitize_sketch_name("good-name_1"), "good-name_1");
    assert_eq!(sanitize_sketch_name(""), "");
}
