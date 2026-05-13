use super::*;

#[test]
fn new_session_uses_prose_buffer() {
    let session = StackerSession::new();
    assert_eq!(session.kind(), BufferKind::Prose);
    assert!(session.is_empty());
    assert!(!session.can_undo());
    assert!(!session.can_redo());
}

#[test]
fn insert_records_undo_and_redo() {
    let mut session = StackerSession::new();
    session.insert_text(StackerSelection::collapsed(0), "hello");
    assert_eq!(session.text(), "hello");

    assert!(session.undo());
    assert_eq!(session.text(), "");

    assert!(session.redo());
    assert_eq!(session.text(), "hello");
}

#[test]
fn set_text_resets_history() {
    let mut session = StackerSession::new();
    session.insert_text(StackerSelection::collapsed(0), "draft");
    session.set_text("saved prompt");

    assert!(!session.can_undo());
    assert!(!session.undo());
    assert_eq!(session.text(), "saved prompt");
    assert_eq!(
        session.selection(),
        StackerSelection::collapsed("saved prompt".chars().count())
    );
}

#[test]
fn replace_all_records_single_history_entry() {
    let mut session = StackerSession::new();
    session.set_text("one");

    assert!(
        session.replace_all_with_history("one two".to_string(), StackerSelection::collapsed(7),)
    );
    assert_eq!(session.text(), "one two");

    assert!(session.undo());
    assert_eq!(session.text(), "one");
}

#[test]
fn undo_redo_availability_tracks_history() {
    let mut session = StackerSession::new();
    assert!(session.is_empty());
    assert!(!session.can_undo());
    assert!(!session.can_redo());

    session.insert_text(StackerSelection::collapsed(0), "draft");
    assert!(!session.is_empty());
    assert!(session.can_undo());
    assert!(!session.can_redo());

    assert!(session.undo());
    assert!(!session.can_undo());
    assert!(session.can_redo());

    assert!(session.redo());
    assert!(session.can_undo());
    assert!(!session.can_redo());
}

#[test]
fn undo_restores_operation_selection_and_redo_restores_post_edit_selection() {
    let mut session = StackerSession::new();
    session.set_text("hello world");
    session.set_selection(StackerSelection::collapsed(0));

    session.replace_range(
        StackerSelection { start: 6, end: 11 },
        "llnzy",
        StackerSelection::collapsed(11),
    );

    assert_eq!(session.text(), "hello llnzy");
    assert_eq!(session.selection(), StackerSelection::collapsed(11));

    assert!(session.undo());
    assert_eq!(session.text(), "hello world");
    assert_eq!(session.selection(), StackerSelection { start: 6, end: 11 });

    assert!(session.redo());
    assert_eq!(session.text(), "hello llnzy");
    assert_eq!(session.selection(), StackerSelection::collapsed(11));
}

#[test]
fn replace_selection_uses_current_selection() {
    let mut session = StackerSession::new();
    session.set_text("hello world");
    session.set_selection(StackerSelection { start: 6, end: 11 });

    let outcome = session.replace_selection("llnzy");

    assert!(outcome.changed);
    assert_eq!(outcome.cursor, 11);
    assert_eq!(session.text(), "hello llnzy");
    assert_eq!(session.selection(), StackerSelection::collapsed(11));
}

#[test]
fn delete_backward_at_doc_start_is_noop() {
    let mut session = StackerSession::new();
    session.set_text("abc");
    session.set_selection(StackerSelection::collapsed(0));

    let outcome = session.delete_backward(StackerSelection::collapsed(0));

    assert!(!outcome.changed);
    assert_eq!(session.text(), "abc");
    assert!(!session.can_undo());
}

#[test]
fn delete_forward_at_doc_end_is_noop() {
    let mut session = StackerSession::new();
    session.set_text("abc");
    let total = session.char_count();
    session.set_selection(StackerSelection::collapsed(total));

    let outcome = session.delete_forward(StackerSelection::collapsed(total));

    assert!(!outcome.changed);
    assert_eq!(session.text(), "abc");
}

#[test]
fn selected_text_returns_substring() {
    let mut session = StackerSession::new();
    session.set_text("hello world");

    let selected = session.selected_text(StackerSelection { start: 6, end: 11 });
    assert_eq!(selected.as_deref(), Some("world"));
}

#[test]
fn select_all_spans_full_text() {
    let mut session = StackerSession::new();
    session.set_text("abcdef");

    let sel = session.select_all();
    assert_eq!(sel, StackerSelection { start: 0, end: 6 });
    assert_eq!(session.selection(), sel);
}

#[test]
fn insert_normalizes_crlf_to_lf() {
    let mut session = StackerSession::new();
    session.insert_text(StackerSelection::collapsed(0), "one\r\ntwo\rthree");
    assert_eq!(session.text(), "one\ntwo\nthree");
}

#[test]
fn unicode_selection_round_trips_via_undo() {
    let mut session = StackerSession::new();
    session.set_text("héllo wörld");
    session.set_selection(StackerSelection { start: 6, end: 11 });

    session.replace_range(
        StackerSelection { start: 6, end: 11 },
        "llnzy",
        StackerSelection::collapsed(11),
    );

    assert_eq!(session.text(), "héllo llnzy");
    assert!(session.undo());
    assert_eq!(session.text(), "héllo wörld");
    assert_eq!(session.selection(), StackerSelection { start: 6, end: 11 });
}

#[test]
fn delete_backward_with_selection_records_undo() {
    let mut session = StackerSession::new();
    session.set_text("hello world");
    session.set_selection(StackerSelection { start: 6, end: 11 });

    let outcome = session.delete_backward(StackerSelection { start: 6, end: 11 });

    assert!(outcome.changed);
    assert_eq!(session.text(), "hello ");
    assert_eq!(session.selection(), StackerSelection::collapsed(6));

    assert!(session.undo());
    assert_eq!(session.text(), "hello world");
    assert_eq!(session.selection(), StackerSelection { start: 6, end: 11 });
}

#[test]
fn fresh_session_has_no_marked_range() {
    let session = StackerSession::new();
    assert!(session.marked_range().is_none());
}

#[test]
fn set_marked_text_at_collapsed_cursor_inserts_and_marks() {
    let mut session = StackerSession::new();
    session.set_text("hello ");
    session.set_selection(StackerSelection::collapsed(6));

    // First setMarkedText: no replacement_range, no existing marked
    // range → composition replaces the current selection.
    session.set_marked_text("wo", StackerSelection::collapsed(2), None);

    assert_eq!(session.text(), "hello wo");
    assert_eq!(
        session.marked_range(),
        Some(StackerSelection { start: 6, end: 8 })
    );
    assert_eq!(session.selection(), StackerSelection::collapsed(8));
}

#[test]
fn second_set_marked_text_replaces_previous_marked_content() {
    let mut session = StackerSession::new();
    session.set_text("hello ");
    session.set_selection(StackerSelection::collapsed(6));
    session.set_marked_text("wo", StackerSelection::collapsed(2), None);
    assert_eq!(session.text(), "hello wo");

    // IME refines composition: replace marked "wo" with "wor".
    session.set_marked_text("wor", StackerSelection::collapsed(3), None);

    assert_eq!(session.text(), "hello wor");
    assert_eq!(
        session.marked_range(),
        Some(StackerSelection { start: 6, end: 9 })
    );
    assert_eq!(session.selection(), StackerSelection::collapsed(9));
}

#[test]
fn unmark_text_commits_composition_in_place() {
    let mut session = StackerSession::new();
    session.set_text("hello ");
    session.set_selection(StackerSelection::collapsed(6));
    session.set_marked_text("world", StackerSelection::collapsed(5), None);
    assert!(session.marked_range().is_some());

    session.unmark_text();

    assert_eq!(session.text(), "hello world");
    assert!(session.marked_range().is_none());
    assert_eq!(session.selection(), StackerSelection::collapsed(11));
}

#[test]
fn set_marked_text_with_replacement_range_overrides_when_unmarked() {
    let mut session = StackerSession::new();
    session.set_text("hello world");
    session.set_selection(StackerSelection::collapsed(0));

    // Insert composition over the explicit range "world".
    session.set_marked_text(
        "llnzy",
        StackerSelection::collapsed(5),
        Some(StackerSelection { start: 6, end: 11 }),
    );

    assert_eq!(session.text(), "hello llnzy");
    assert_eq!(
        session.marked_range(),
        Some(StackerSelection { start: 6, end: 11 })
    );
}

#[test]
fn empty_set_marked_text_clears_marked_range() {
    let mut session = StackerSession::new();
    session.set_text("hello ");
    session.set_selection(StackerSelection::collapsed(6));
    session.set_marked_text("wo", StackerSelection::collapsed(2), None);
    assert_eq!(session.text(), "hello wo");

    // IME aborts composition: setMarkedText: with empty text replaces
    // the marked range with nothing, leaving an unmarked document.
    session.set_marked_text("", StackerSelection::collapsed(0), None);

    assert_eq!(session.text(), "hello ");
    assert!(session.marked_range().is_none());
}

#[test]
fn set_text_clears_marked_range() {
    let mut session = StackerSession::new();
    session.set_text("hello ");
    session.set_selection(StackerSelection::collapsed(6));
    session.set_marked_text("wo", StackerSelection::collapsed(2), None);
    assert!(session.marked_range().is_some());

    session.set_text("brand new");

    assert!(session.marked_range().is_none());
}

#[test]
fn marked_range_tracks_internal_selection_within_composition() {
    let mut session = StackerSession::new();
    session.set_text("");
    session.set_selection(StackerSelection::collapsed(0));

    // Composition with the cursor in the middle of the marked text.
    session.set_marked_text("héllo", StackerSelection::collapsed(2), None);

    assert_eq!(session.text(), "héllo");
    assert_eq!(
        session.marked_range(),
        Some(StackerSelection { start: 0, end: 5 })
    );
    assert_eq!(session.selection(), StackerSelection::collapsed(2));
}

#[test]
fn sync_to_view_mirrors_collapsed_selection() {
    use crate::editor::BufferView;
    let mut session = StackerSession::new();
    session.set_text("hello\nworld");
    session.set_selection(StackerSelection::collapsed(8));

    let mut view = BufferView::default();
    session.sync_to_view(&mut view);

    assert_eq!(view.cursor.pos.line, 1);
    assert_eq!(view.cursor.pos.col, 2);
    assert!(view.cursor.anchor.is_none());
}

#[test]
fn sync_to_view_mirrors_range_selection() {
    use crate::editor::BufferView;
    let mut session = StackerSession::new();
    session.set_text("hello\nworld");
    session.set_selection(StackerSelection { start: 2, end: 8 });

    let mut view = BufferView::default();
    session.sync_to_view(&mut view);

    assert_eq!(view.cursor.pos.line, 1);
    assert_eq!(view.cursor.pos.col, 2);
    let anchor = view.cursor.anchor.expect("anchor present for range");
    assert_eq!(anchor.line, 0);
    assert_eq!(anchor.col, 2);
}

#[test]
fn sync_from_view_writes_collapsed_back() {
    use crate::editor::buffer::Position;
    use crate::editor::BufferView;
    let mut session = StackerSession::new();
    session.set_text("hello\nworld");
    session.set_selection(StackerSelection::collapsed(0));

    let mut view = BufferView::default();
    view.cursor.pos = Position::new(1, 3);
    view.cursor.anchor = None;

    session.sync_from_view(&view);
    assert_eq!(session.selection(), StackerSelection::collapsed(9));
}

#[test]
fn sync_from_view_writes_range_back() {
    use crate::editor::buffer::Position;
    use crate::editor::BufferView;
    let mut session = StackerSession::new();
    session.set_text("hello\nworld");
    session.set_selection(StackerSelection::collapsed(0));

    let mut view = BufferView::default();
    view.cursor.pos = Position::new(1, 3);
    view.cursor.anchor = Some(Position::new(0, 2));

    session.sync_from_view(&view);
    assert_eq!(session.selection(), StackerSelection { start: 2, end: 9 });
}

#[test]
fn sync_round_trip_is_stable() {
    use crate::editor::BufferView;
    let mut session = StackerSession::new();
    session.set_text("first line\nsecond line\nthird");
    session.set_selection(StackerSelection { start: 6, end: 18 });

    let mut view = BufferView::default();
    session.sync_to_view(&mut view);
    // No mutation; reading back should be a no-op.
    let before = session.selection();
    session.sync_from_view(&view);
    assert_eq!(session.selection(), before);
}

#[test]
fn insert_via_session_then_sync_places_view_cursor_after_text() {
    use crate::editor::BufferView;
    let mut session = StackerSession::new();
    session.insert_text(StackerSelection::collapsed(0), "hello\nwor");
    // Session selection should be collapsed at end of inserted text (9 chars).
    assert_eq!(session.selection(), StackerSelection::collapsed(9));

    let mut view = BufferView::default();
    session.sync_to_view(&mut view);
    assert_eq!(view.cursor.pos.line, 1);
    assert_eq!(view.cursor.pos.col, 3);
    assert!(view.cursor.anchor.is_none());
}

#[test]
fn delete_via_session_then_sync_moves_view_cursor_back() {
    use crate::editor::BufferView;
    let mut session = StackerSession::new();
    session.set_text("hello\nworld");
    session.set_selection(StackerSelection::collapsed(8));

    session.delete_backward(session.selection());
    assert_eq!(session.selection(), StackerSelection::collapsed(7));

    let mut view = BufferView::default();
    session.sync_to_view(&mut view);
    assert_eq!(view.cursor.pos.line, 1);
    assert_eq!(view.cursor.pos.col, 1);
}

#[test]
fn mouse_drag_simulated_on_view_then_sync_updates_session() {
    use crate::editor::buffer::Position;
    use crate::editor::BufferView;
    let mut session = StackerSession::new();
    session.set_text("first line\nsecond line\nthird");
    session.set_selection(StackerSelection::collapsed(0));

    // Simulate a drag selection: click at (0, 6), drag to (1, 6).
    let mut view = BufferView::default();
    view.cursor.anchor = Some(Position::new(0, 6));
    view.cursor.pos = Position::new(1, 6);

    session.sync_from_view(&view);
    // 0,6 = char 6 ; 1,6 = char 11 (newline) + 6 = char 17.
    assert_eq!(session.selection(), StackerSelection { start: 6, end: 17 });
}

#[test]
fn sync_round_trip_through_session_mutation_preserves_selection() {
    use crate::editor::BufferView;
    let mut session = StackerSession::new();
    session.insert_text(StackerSelection::collapsed(0), "abcdef");

    let mut view = BufferView::default();
    session.sync_to_view(&mut view);
    let view_pos_before = view.cursor.pos;

    // No view-side change; sync back must not move the session selection.
    session.sync_from_view(&view);
    assert_eq!(session.selection(), StackerSelection::collapsed(6));

    // And re-sync to view yields the same view position.
    session.sync_to_view(&mut view);
    assert_eq!(view.cursor.pos, view_pos_before);
}

#[test]
fn sync_to_view_clears_extra_cursors_and_desired_col() {
    use crate::editor::buffer::Position;
    use crate::editor::cursor::CursorRange;
    use crate::editor::BufferView;
    let mut session = StackerSession::new();
    session.set_text("hello");
    session.set_selection(StackerSelection::collapsed(3));

    let mut view = BufferView::default();
    view.cursor.desired_col = Some(40);
    view.cursor.extra_cursors.push(CursorRange {
        pos: Position::new(0, 0),
        anchor: None,
    });

    session.sync_to_view(&mut view);
    assert_eq!(view.cursor.desired_col, None);
    assert!(view.cursor.extra_cursors.is_empty());
}
