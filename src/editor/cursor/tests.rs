use super::*;
use crate::editor::buffer::{Buffer, Position};

fn buf_with(text: &str) -> Buffer {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), text);
    buf
}

#[test]
fn new_cursor_at_origin() {
    let c = EditorCursor::new();
    assert_eq!(c.pos, Position::new(0, 0));
    assert!(!c.has_selection());
}

#[test]
fn move_right_within_line() {
    let buf = buf_with("hello");
    let mut c = EditorCursor::new();
    c.move_right(&buf, false);
    assert_eq!(c.pos, Position::new(0, 1));
}

#[test]
fn move_right_wraps_to_next_line() {
    let buf = buf_with("ab\ncd");
    let mut c = EditorCursor::at(0, 2);
    c.move_right(&buf, false);
    assert_eq!(c.pos, Position::new(1, 0));
}

#[test]
fn move_right_at_end_of_buffer_stays() {
    let buf = buf_with("ab");
    let mut c = EditorCursor::at(0, 2);
    c.move_right(&buf, false);
    assert_eq!(c.pos, Position::new(0, 2));
}

#[test]
fn move_left_within_line() {
    let buf = buf_with("hello");
    let mut c = EditorCursor::at(0, 3);
    c.move_left(&buf, false);
    assert_eq!(c.pos, Position::new(0, 2));
}

#[test]
fn move_left_wraps_to_prev_line() {
    let buf = buf_with("ab\ncd");
    let mut c = EditorCursor::at(1, 0);
    c.move_left(&buf, false);
    assert_eq!(c.pos, Position::new(0, 2));
}

#[test]
fn move_left_at_start_stays() {
    let buf = buf_with("hello");
    let mut c = EditorCursor::new();
    c.move_left(&buf, false);
    assert_eq!(c.pos, Position::new(0, 0));
}

#[test]
fn move_down_preserves_desired_col() {
    let buf = buf_with("long line here\nhi\nlong again here");
    let mut c = EditorCursor::at(0, 10);
    c.move_down(&buf, false);
    assert_eq!(c.pos, Position::new(1, 2));
    c.move_down(&buf, false);
    assert_eq!(c.pos, Position::new(2, 10));
}

#[test]
fn move_up_at_first_line_goes_to_start() {
    let buf = buf_with("hello");
    let mut c = EditorCursor::at(0, 3);
    c.move_up(&buf, false);
    assert_eq!(c.pos, Position::new(0, 0));
}

#[test]
fn move_word_right() {
    let buf = buf_with("hello world_test foo");
    let mut c = EditorCursor::new();
    c.move_word_right(&buf, false);
    assert_eq!(c.pos.col, 6);
    c.move_word_right(&buf, false);
    assert_eq!(c.pos.col, 17);
}

#[test]
fn move_word_left() {
    let buf = buf_with("hello world");
    let mut c = EditorCursor::at(0, 11);
    c.move_word_left(&buf, false);
    assert_eq!(c.pos.col, 6);
    c.move_word_left(&buf, false);
    assert_eq!(c.pos.col, 0);
}

#[test]
fn move_home_to_first_non_ws() {
    let buf = buf_with("    indented");
    let mut c = EditorCursor::at(0, 8);
    c.move_home(&buf, false);
    assert_eq!(c.pos.col, 4);
    c.move_home(&buf, false);
    assert_eq!(c.pos.col, 0);
}

#[test]
fn move_end() {
    let buf = buf_with("hello");
    let mut c = EditorCursor::new();
    c.move_end(&buf, false);
    assert_eq!(c.pos.col, 5);
}

#[test]
fn shift_right_creates_selection() {
    let buf = buf_with("hello");
    let mut c = EditorCursor::new();
    c.move_right(&buf, true);
    c.move_right(&buf, true);
    assert!(c.has_selection());
    let (start, end) = c.selection().unwrap();
    assert_eq!(start, Position::new(0, 0));
    assert_eq!(end, Position::new(0, 2));
}

#[test]
fn move_without_shift_clears_selection() {
    let buf = buf_with("hello");
    let mut c = EditorCursor::new();
    c.move_right(&buf, true);
    c.move_right(&buf, true);
    assert!(c.has_selection());
    c.move_right(&buf, false);
    assert!(!c.has_selection());
}

#[test]
fn select_word() {
    let buf = buf_with("hello world");
    let mut c = EditorCursor::at(0, 2);
    c.select_word(&buf);
    let (start, end) = c.selection().unwrap();
    assert_eq!(start, Position::new(0, 0));
    assert_eq!(end, Position::new(0, 5));
}

#[test]
fn select_line() {
    let buf = buf_with("hello\nworld");
    let mut c = EditorCursor::at(0, 2);
    c.select_line(&buf);
    let (start, end) = c.selection().unwrap();
    assert_eq!(start, Position::new(0, 0));
    assert_eq!(end, Position::new(1, 0));
}

#[test]
fn select_all() {
    let buf = buf_with("hello\nworld");
    let mut c = EditorCursor::new();
    c.select_all(&buf);
    let (start, end) = c.selection().unwrap();
    assert_eq!(start, Position::new(0, 0));
    assert_eq!(end, Position::new(1, 5));
}

#[test]
fn go_to_line() {
    let buf = buf_with("a\nb\nc\nd");
    let mut c = EditorCursor::new();
    c.go_to_line(3, &buf);
    assert_eq!(c.pos, Position::new(2, 0));
}

#[test]
fn clamp_out_of_bounds() {
    let buf = buf_with("hi");
    let mut c = EditorCursor::at(10, 50);
    c.clamp(&buf);
    assert_eq!(c.pos.line, 0);
    assert_eq!(c.pos.col, 2);
}

#[test]
fn page_down() {
    let text = (0..50)
        .map(|i| format!("line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let buf = buf_with(&text);
    let mut c = EditorCursor::new();
    c.move_page_down(&buf, 20, false);
    assert_eq!(c.pos.line, 20);
}

#[test]
fn page_up_clamps_to_zero() {
    let buf = buf_with("a\nb\nc");
    let mut c = EditorCursor::at(1, 0);
    c.move_page_up(&buf, 20, false);
    assert_eq!(c.pos.line, 0);
}

#[test]
fn empty_buffer_movements_stay_at_origin() {
    let buf = Buffer::empty();
    let mut c = EditorCursor::new();

    c.move_right(&buf, false);
    c.move_left(&buf, false);
    c.move_up(&buf, false);
    c.move_down(&buf, false);
    c.move_home(&buf, false);
    c.move_end(&buf, false);
    c.move_page_up(&buf, 100, false);
    c.move_page_down(&buf, 100, false);
    c.move_to_start(false);
    c.move_to_end(&buf, false);
    c.clamp(&buf);

    assert_eq!(c.pos, Position::new(0, 0));
    assert!(!c.has_selection());
}

#[test]
fn vertical_movement_restores_column_on_long_lines() {
    let buf = buf_with("01234567890123456789\nx\n01234567890123456789");
    let mut c = EditorCursor::at(0, 18);

    c.move_down(&buf, false);
    assert_eq!(c.pos, Position::new(1, 1));
    c.move_down(&buf, false);
    assert_eq!(c.pos, Position::new(2, 18));
}

#[test]
fn document_start_and_end_extend_selection() {
    let buf = buf_with("abc\ndef");
    let mut c = EditorCursor::at(0, 2);

    c.move_to_end(&buf, true);
    assert_eq!(
        c.selection(),
        Some((Position::new(0, 2), Position::new(1, 3)))
    );

    c.move_to_start(true);
    assert_eq!(
        c.selection(),
        Some((Position::new(0, 0), Position::new(0, 2)))
    );

    c.move_to_end(&buf, false);
    assert_eq!(c.pos, Position::new(1, 3));
    assert!(!c.has_selection());
}

#[test]
fn page_down_saturates_and_clamps_to_document_end() {
    let buf = buf_with("a\nbb\nccc");
    let mut c = EditorCursor::at(0, 10);

    c.move_page_down(&buf, usize::MAX, false);

    assert_eq!(c.pos, Position::new(2, 3));
    assert_eq!(c.desired_col, Some(10));
}

#[test]
fn page_movement_extends_selection_from_anchor() {
    let buf = buf_with("aa\nbb\ncc\ndd");
    let mut c = EditorCursor::at(1, 1);

    c.move_page_down(&buf, 2, true);

    assert_eq!(c.pos, Position::new(3, 1));
    assert_eq!(
        c.selection(),
        Some((Position::new(1, 1), Position::new(3, 1)))
    );
}

#[test]
fn clamp_clamps_anchors_and_dedups_extra_cursors() {
    let buf = buf_with("hi\nx");
    let mut c = EditorCursor::at(5, 5);
    c.anchor = Some(Position::new(9, 9));
    c.extra_cursors = vec![
        CursorRange {
            pos: Position::new(1, 50),
            anchor: Some(Position::new(10, 10)),
        },
        CursorRange {
            pos: Position::new(0, 20),
            anchor: None,
        },
        CursorRange {
            pos: Position::new(0, 2),
            anchor: None,
        },
    ];

    c.clamp(&buf);

    assert_eq!(c.pos, Position::new(1, 1));
    assert_eq!(c.anchor, Some(Position::new(1, 1)));
    assert_eq!(
        c.extra_cursors,
        vec![CursorRange {
            pos: Position::new(0, 2),
            anchor: None,
        }]
    );
}

#[test]
fn occurrences_use_character_positions_and_avoid_primary_duplicate() {
    let buf = buf_with("éx éx");
    let mut c = EditorCursor::new();

    c.select_all_occurrences(&buf, "éx");
    assert_eq!(
        c.selection(),
        Some((Position::new(0, 0), Position::new(0, 2)))
    );
    assert_eq!(
        c.extra_cursors,
        vec![CursorRange {
            pos: Position::new(0, 5),
            anchor: Some(Position::new(0, 3)),
        }]
    );

    assert!(!c.add_next_occurrence(&buf, "éx"));
    assert_eq!(c.extra_cursors.len(), 1);
}
