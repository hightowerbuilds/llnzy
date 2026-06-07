use super::geometry::{
    canvas_to_sketch_point, export_frame_size, normalize_zoom_scale, pad_offset_for_zoom_anchor,
    sketch_to_canvas_point,
};
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
fn pasted_text_box_commits_text_and_can_undo() {
    let mut state = SketchState::default();
    let index = state
        .paste_text_box("service boundary", point(24.0, 32.0))
        .expect("expected pasted text");

    assert_eq!(state.selected, Some(index));
    let SketchElement::Text(text) = &state.document.elements[index] else {
        panic!("expected text box");
    };
    assert_eq!(text.text, "service boundary");
    assert!(text.w >= DEFAULT_TEXT_W);
    assert!(state.undo());
    assert!(state.document.elements.is_empty());
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
fn selected_rectangle_can_resize_from_corner() {
    let mut state = SketchState::default();
    state.begin_rectangle(point(10.0, 10.0));
    state.update_rectangle(point(40.0, 30.0));
    state.finish_rectangle();
    state.selected = Some(0);

    assert!(state.begin_resize_selected(ResizeHandle::BottomRight, point(40.0, 30.0)));
    assert!(state.update_resize_selected(point(70.0, 55.0)));
    assert!(state.finish_resize_selected());

    let SketchElement::Rectangle(rect) = &state.document.elements[0] else {
        panic!("expected rectangle");
    };
    assert_eq!(rect.x, 10.0);
    assert_eq!(rect.y, 10.0);
    assert_eq!(rect.w, 60.0);
    assert_eq!(rect.h, 45.0);
    assert!(state.undo());
    let SketchElement::Rectangle(rect) = &state.document.elements[0] else {
        panic!("expected rectangle");
    };
    assert_eq!(rect.w, 30.0);
    assert_eq!(rect.h, 20.0);
}

#[test]
fn selected_symbol_can_resize_from_corner() {
    let mut state = SketchState::default();
    let index = state.add_symbol(SketchSymbolKind::Database, point(30.0, 40.0));

    assert!(state.begin_resize_selected(ResizeHandle::TopLeft, point(30.0, 40.0)));
    assert!(state.update_resize_selected(point(10.0, 15.0)));
    assert!(state.finish_resize_selected());

    let SketchElement::Symbol(symbol) = &state.document.elements[index] else {
        panic!("expected symbol");
    };
    assert_eq!(symbol.x, 10.0);
    assert_eq!(symbol.y, 15.0);
    assert_eq!(symbol.w, DEFAULT_SYMBOL_W + 20.0);
    assert_eq!(symbol.h, DEFAULT_SYMBOL_H + 25.0);
}

#[test]
fn resize_handle_hit_test_uses_selected_resizable_element() {
    let mut state = SketchState::default();
    state.begin_rectangle(point(10.0, 10.0));
    state.update_rectangle(point(40.0, 30.0));
    state.finish_rectangle();
    state.selected = Some(0);

    assert_eq!(
        state.selected_resize_handle_at(point(40.0, 30.0)),
        Some(ResizeHandle::BottomRight)
    );
    assert_eq!(state.selected_resize_handle_at(point(25.0, 20.0)), None);
}

#[test]
fn image_resize_handle_can_be_hit_tested() {
    let mut state = SketchState::default();
    state
        .document
        .elements
        .push(SketchElement::Image(ImageElement {
            x: 12.0,
            y: 14.0,
            w: 200.0,
            h: 100.0,
            original_w: 200.0,
            original_h: 100.0,
            path: "/tmp/image.png".to_string(),
        }));
    state.selected = Some(0);

    assert_eq!(
        state.selected_resize_handle_at(point(212.0, 114.0)),
        Some(ResizeHandle::BottomRight)
    );
}

#[test]
fn grab_tool_preserves_current_selection() {
    let mut state = SketchState::default();
    let index = state.add_symbol(SketchSymbolKind::Database, point(30.0, 40.0));

    state.set_tool(SketchTool::Grab);

    assert_eq!(state.selected, Some(index));
    assert_eq!(state.tool, SketchTool::Grab);
}

#[test]
fn pad_offset_converts_between_canvas_and_sketch_coordinates() {
    let offset = point(120.0, -45.0);
    let screen = point(200.0, 80.0);
    let sketch = canvas_to_sketch_point(screen, offset, 1.0);

    assert_eq!(sketch, point(80.0, 125.0));
    assert_eq!(sketch_to_canvas_point(sketch, offset, 1.0), screen);
}

#[test]
fn zoom_scale_converts_between_canvas_and_sketch_coordinates() {
    let offset = point(120.0, -45.0);
    let screen = point(320.0, 155.0);
    let sketch = canvas_to_sketch_point(screen, offset, 2.0);

    assert_eq!(sketch, point(100.0, 100.0));
    assert_eq!(sketch_to_canvas_point(sketch, offset, 2.0), screen);
}

#[test]
fn zoom_anchor_keeps_same_sketch_point_under_cursor() {
    let offset = point(120.0, -45.0);
    let anchor = point(320.0, 155.0);
    let sketch_before = canvas_to_sketch_point(anchor, offset, 2.0);
    let next_offset = pad_offset_for_zoom_anchor(offset, anchor, 2.0, 3.0);
    let sketch_after = canvas_to_sketch_point(anchor, next_offset, 3.0);

    assert_eq!(sketch_before, point(100.0, 100.0));
    assert_eq!(sketch_after, sketch_before);
}

#[test]
fn zoom_scale_is_clamped_to_supported_range() {
    assert_eq!(normalize_zoom_scale(0.01), 0.25);
    assert_eq!(normalize_zoom_scale(8.0), 4.0);
    assert_eq!(normalize_zoom_scale(f32::NAN), 1.0);
}

#[test]
fn export_frame_size_is_fixed_full_hd() {
    assert_eq!(export_frame_size([1400.0, 700.0]), [1920.0, 1080.0]);
    assert_eq!(export_frame_size([0.0, -10.0]), [1920.0, 1080.0]);
}

#[test]
fn resize_from_painted_handle_does_not_jump_by_handle_offset() {
    let mut state = SketchState::default();
    state.begin_rectangle(point(10.0, 10.0));
    state.update_rectangle(point(40.0, 30.0));
    state.finish_rectangle();
    state.selected = Some(0);
    let painted_handle = point(
        40.0 + state.appearance.effective_handle_size() * 0.7,
        30.0 + state.appearance.effective_handle_size() * 0.7,
    );

    assert_eq!(
        state.selected_resize_handle_at(painted_handle),
        Some(ResizeHandle::BottomRight)
    );
    assert!(state.begin_resize_selected(ResizeHandle::BottomRight, painted_handle));
    assert!(state.update_resize_selected(point(painted_handle.x + 10.0, painted_handle.y + 5.0)));

    let SketchElement::Rectangle(rect) = &state.document.elements[0] else {
        panic!("expected rectangle");
    };
    assert_eq!(rect.w, 40.0);
    assert_eq!(rect.h, 25.0);
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
fn symbol_elements_can_be_added_selected_and_hit_tested() {
    let mut state = SketchState::default();
    let index = state.add_symbol(SketchSymbolKind::Database, point(30.0, 40.0));

    assert_eq!(state.selected, Some(index));
    assert_eq!(state.hit_test(point(50.0, 60.0)), Some(index));
}

#[test]
fn selected_image_resizes_proportionally() {
    let mut state = SketchState::default();
    state
        .document
        .elements
        .push(SketchElement::Image(ImageElement {
            x: 0.0,
            y: 0.0,
            w: 200.0,
            h: 100.0,
            original_w: 400.0,
            original_h: 200.0,
            path: "/tmp/image.png".to_string(),
        }));
    state.selected = Some(0);

    assert!(state.resize_selected_image_to_scale(0.25));
    let SketchElement::Image(image) = &state.document.elements[0] else {
        panic!("expected image");
    };
    assert_eq!(image.w, 100.0);
    assert_eq!(image.h, 50.0);
}

#[test]
fn sketch_appearance_defaults_preserve_current_canvas_behavior() {
    let state = SketchState::default();

    assert_eq!(
        state.appearance.canvas_background_mode,
        SketchCanvasBackgroundMode::Theme
    );
    assert_eq!(state.appearance.grid_mode, SketchGridMode::Hidden);
    assert_eq!(
        state.appearance.toolbar_position,
        SketchToolbarPosition::Top
    );
    assert!(!state.appearance.grid_visible());
    assert_eq!(
        state.appearance.selection_outline_color,
        [60, 130, 255, 255]
    );
    assert!(state.appearance.canvas_border_visible);
    assert!(!state.appearance.canvas_shadow_visible);
}

#[test]
fn sketch_appearance_normalizes_numeric_controls() {
    let mut state = SketchState::default();
    state.set_appearance(SketchAppearanceSettings {
        grid_mode: SketchGridMode::Lines,
        grid_spacing: 1.0,
        grid_opacity: 2.0,
        handle_size: 100.0,
        ..SketchAppearanceSettings::default()
    });

    assert_eq!(state.appearance.effective_grid_spacing(), 4.0);
    assert_eq!(state.appearance.effective_grid_opacity(), 1.0);
    assert_eq!(state.appearance.effective_handle_size(), 24.0);
    assert!(state.appearance.grid_visible());
}

#[test]
fn sketch_appearance_settings_round_trip_via_path() {
    let dir = std::env::temp_dir().join("llnzy_test_sketch_appearance");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("appearance.json");
    let settings = SketchAppearanceSettings {
        canvas_background_mode: SketchCanvasBackgroundMode::Solid,
        canvas_background_color: [12, 24, 36, 255],
        grid_mode: SketchGridMode::Dots,
        grid_spacing: 32.0,
        grid_opacity: 0.4,
        selection_outline_color: [220, 120, 80, 255],
        handle_size: 8.0,
        canvas_border_visible: false,
        canvas_shadow_visible: true,
        toolbar_position: SketchToolbarPosition::Left,
    };

    save_appearance_settings_to_path(&settings, &path).unwrap();
    let loaded = load_appearance_settings_from_path(&path).unwrap();

    assert_eq!(loaded, settings);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sketch_appearance_settings_load_missing_fields_from_defaults() {
    let dir = std::env::temp_dir().join("llnzy_test_sketch_appearance_partial");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("appearance.json");
    std::fs::write(
        &path,
        r#"{
  "canvas_background_mode": "solid",
  "canvas_background_color": [1, 2, 3, 255]
}"#,
    )
    .unwrap();

    let loaded = load_appearance_settings_from_path(&path).unwrap();

    assert_eq!(
        loaded.canvas_background_mode,
        SketchCanvasBackgroundMode::Solid
    );
    assert_eq!(loaded.canvas_background_color, [1, 2, 3, 255]);
    assert_eq!(loaded.grid_mode, SketchGridMode::Hidden);
    assert_eq!(loaded.toolbar_position, SketchToolbarPosition::Top);
    assert_eq!(loaded.effective_grid_spacing(), 24.0);
    assert!(loaded.canvas_border_visible);
    let _ = std::fs::remove_file(&path);
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
