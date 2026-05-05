use super::*;

#[test]
fn empty_buffer_has_one_line() {
    let buf = Buffer::empty();
    // Ropey considers empty text as 1 line.
    assert_eq!(buf.line_count(), 1);
    assert_eq!(buf.line(0), "");
    assert!(!buf.is_modified());
}

#[test]
fn insert_text() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "hello");
    assert_eq!(buf.line(0), "hello");
    assert!(buf.is_modified());
}

#[test]
fn insert_multiline() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "line1\nline2\nline3");
    assert_eq!(buf.line_count(), 3);
    assert_eq!(buf.line(0), "line1");
    assert_eq!(buf.line(1), "line2");
    assert_eq!(buf.line(2), "line3");
}

#[test]
fn insert_char_at_position() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "hllo");
    buf.insert_char(Position::new(0, 1), 'e');
    assert_eq!(buf.line(0), "hello");
}

#[test]
fn delete_range() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "hello world");
    buf.delete(Position::new(0, 5), Position::new(0, 11));
    assert_eq!(buf.line(0), "hello");
}

#[test]
fn delete_across_lines() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "hello\nworld");
    buf.delete(Position::new(0, 3), Position::new(1, 2));
    assert_eq!(buf.line(0), "helrld");
    assert_eq!(buf.line_count(), 1);
}

#[test]
fn replace_range() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "hello world");
    buf.replace(Position::new(0, 6), Position::new(0, 11), "rust");
    assert_eq!(buf.line(0), "hello rust");
}

#[test]
fn records_last_edit_for_replace() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "hello world");
    let _ = buf.take_last_edit();

    buf.replace(Position::new(0, 6), Position::new(0, 11), "rust");
    assert_eq!(
        buf.take_last_edit(),
        Some(BufferEdit {
            start: Position::new(0, 6),
            old_end: Position::new(0, 11),
            new_end: Position::new(0, 10),
            new_text: "rust".to_string(),
        })
    );
}

#[test]
fn undo_insert() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "hello");
    assert_eq!(buf.line(0), "hello");

    let pos = buf.undo();
    assert!(pos.is_some());
    assert_eq!(buf.line(0), "");
    assert!(!buf.is_modified());
}

#[test]
fn undo_delete() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "hello");
    buf.delete(Position::new(0, 0), Position::new(0, 5));
    assert_eq!(buf.text(), "");

    buf.undo();
    assert_eq!(buf.line(0), "hello");
}

#[test]
fn records_last_edit_for_undo_and_redo() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "hello");
    let _ = buf.take_last_edit();

    buf.undo();
    assert_eq!(
        buf.take_last_edit(),
        Some(BufferEdit {
            start: Position::new(0, 0),
            old_end: Position::new(0, 5),
            new_end: Position::new(0, 0),
            new_text: String::new(),
        })
    );

    buf.redo();
    assert_eq!(
        buf.take_last_edit(),
        Some(BufferEdit {
            start: Position::new(0, 0),
            old_end: Position::new(0, 0),
            new_end: Position::new(0, 5),
            new_text: "hello".to_string(),
        })
    );
}

#[test]
fn redo_after_undo() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "hello");
    buf.undo();
    assert_eq!(buf.text(), "");

    buf.redo();
    assert_eq!(buf.line(0), "hello");
}

#[test]
fn undo_redo_multiple() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "a");
    buf.insert(Position::new(0, 1), "b");
    buf.insert(Position::new(0, 2), "c");
    assert_eq!(buf.line(0), "abc");

    buf.undo();
    assert_eq!(buf.line(0), "ab");
    buf.undo();
    assert_eq!(buf.line(0), "a");
    buf.redo();
    assert_eq!(buf.line(0), "ab");
}

#[test]
fn pos_to_char_and_back() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "hello\nworld");

    assert_eq!(buf.pos_to_char(Position::new(0, 0)), 0);
    assert_eq!(buf.pos_to_char(Position::new(0, 5)), 5);
    assert_eq!(buf.pos_to_char(Position::new(1, 0)), 6);
    assert_eq!(buf.pos_to_char(Position::new(1, 3)), 9);

    assert_eq!(buf.char_to_pos(0), Position::new(0, 0));
    assert_eq!(buf.char_to_pos(6), Position::new(1, 0));
    assert_eq!(buf.char_to_pos(9), Position::new(1, 3));
}

#[test]
fn line_ending_detection() {
    assert_eq!(LineEnding::detect("a\nb\nc\n"), LineEnding::Lf);
    assert_eq!(LineEnding::detect("a\r\nb\r\nc\r\n"), LineEnding::CrLf);
    assert_eq!(LineEnding::detect("a\r\nb\nc\n"), LineEnding::Lf);
}

#[test]
fn indent_style_detection() {
    assert_eq!(
        IndentStyle::detect("  a\n  b\n  c\n"),
        IndentStyle::Spaces(2)
    );
    assert_eq!(
        IndentStyle::detect("    a\n    b\n"),
        IndentStyle::Spaces(4)
    );
    assert_eq!(IndentStyle::detect("\ta\n\tb\n"), IndentStyle::Tabs);
}

#[test]
fn file_name_untitled_when_no_path() {
    let buf = Buffer::empty();
    assert_eq!(buf.file_name(), "untitled");
}

#[test]
fn char_at_position() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "hello");
    assert_eq!(buf.char_at(Position::new(0, 0)), Some('h'));
    assert_eq!(buf.char_at(Position::new(0, 4)), Some('o'));
    assert_eq!(buf.char_at(Position::new(0, 5)), None);
    assert_eq!(buf.char_at(Position::new(1, 0)), None);
}

#[test]
fn line_indent_extraction() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "    indented\nnot indented\n\t\ttabs");
    assert_eq!(buf.line_indent(0), "    ");
    assert_eq!(buf.line_indent(1), "");
    assert_eq!(buf.line_indent(2), "\t\t");
}

#[test]
fn save_and_reload_round_trip() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("llnzy-test-{}.txt", std::process::id()));

    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "hello\nworld");
    buf.save_to(&path).unwrap();
    assert!(!buf.is_modified());

    let loaded = Buffer::from_file(&path).unwrap();
    assert_eq!(loaded.line(0), "hello");
    assert_eq!(loaded.line(1), "world");
    assert!(!loaded.is_modified());

    let _ = std::fs::remove_file(&path);
}

#[test]
fn save_preserves_undo_history() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("llnzy-undo-save-{}.txt", std::process::id()));
    std::fs::write(&path, "hello").unwrap();

    let mut buf = Buffer::from_file(&path).unwrap();
    buf.insert(Position::new(0, 5), " world");
    buf.save().unwrap();
    assert!(!buf.is_modified());

    assert!(buf.undo().is_some());
    assert_eq!(buf.text(), "hello");
    assert!(buf.is_modified());

    let _ = std::fs::remove_file(&path);
}

#[test]
fn failed_save_keeps_buffer_modified() {
    let missing_parent =
        std::env::temp_dir().join(format!("llnzy-missing-parent-{}", std::process::id()));
    let path = missing_parent.join("file.txt");

    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "unsaved");

    assert!(buf.save_to(&path).is_err());
    assert!(buf.is_modified());
    assert!(buf.path().is_none());
    assert_eq!(buf.text(), "unsaved");
}

#[test]
fn crlf_preserved_on_save() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("llnzy-crlf-{}.txt", std::process::id()));

    std::fs::write(&path, "line1\r\nline2\r\n").unwrap();

    let mut buf = Buffer::from_file(&path).unwrap();
    assert_eq!(buf.line_ending(), LineEnding::CrLf);
    assert_eq!(buf.line(0), "line1");

    buf.insert(Position::new(1, 5), "!");
    buf.save().unwrap();

    let raw = std::fs::read_to_string(&path).unwrap();
    assert!(raw.contains("\r\n"), "CRLF should be preserved");

    let _ = std::fs::remove_file(&path);
}

#[test]
fn large_fixture_edit_save_roundtrip_keeps_buffer_usable() {
    let dir = std::env::temp_dir().join(format!("llnzy-buffer-large-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = crate::editor::stress_fixtures::write_rust_file(
        &dir,
        "large.rs",
        crate::editor::stress_fixtures::LARGE_MINIMAP_LINE_COUNT,
    );
    let mut buf = Buffer::from_file(&path).unwrap();

    assert_eq!(
        buf.line_count(),
        crate::editor::stress_fixtures::LARGE_MINIMAP_LINE_COUNT + 1
    );

    buf.insert(Position::new(0, 0), "// edited\n");
    assert!(buf.is_modified());
    buf.save().unwrap();

    let loaded = Buffer::from_file(&path).unwrap();
    assert!(loaded.text().starts_with("// edited\n"));
    assert!(!loaded.is_modified());

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn delete_reversed_range() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "hello");
    buf.delete(Position::new(0, 5), Position::new(0, 0));
    assert_eq!(buf.text(), "");
}

#[test]
fn delete_line_middle() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "aaa\nbbb\nccc");
    buf.delete_line(1);
    assert_eq!(buf.line(0), "aaa");
    assert_eq!(buf.line(1), "ccc");
    assert_eq!(buf.line_count(), 2);
}

#[test]
fn delete_line_first() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "aaa\nbbb");
    buf.delete_line(0);
    assert_eq!(buf.line(0), "bbb");
    assert_eq!(buf.line_count(), 1);
}

#[test]
fn delete_line_last() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "aaa\nbbb");
    buf.delete_line(1);
    assert_eq!(buf.line(0), "aaa");
    assert_eq!(buf.line_count(), 1);
}

#[test]
fn delete_only_line() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "hello");
    buf.delete_line(0);
    assert_eq!(buf.text(), "");
}

#[test]
fn duplicate_line() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "aaa\nbbb");
    buf.duplicate_line(0);
    assert_eq!(buf.line(0), "aaa");
    assert_eq!(buf.line(1), "aaa");
    assert_eq!(buf.line(2), "bbb");
    assert_eq!(buf.line_count(), 3);
}

#[test]
fn move_line_up() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "aaa\nbbb\nccc");
    buf.move_line_up(1);
    assert_eq!(buf.line(0), "bbb");
    assert_eq!(buf.line(1), "aaa");
    assert_eq!(buf.line(2), "ccc");
}

#[test]
fn move_line_down() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "aaa\nbbb\nccc");
    buf.move_line_down(0);
    assert_eq!(buf.line(0), "bbb");
    assert_eq!(buf.line(1), "aaa");
    assert_eq!(buf.line(2), "ccc");
}

#[test]
fn move_line_up_at_top_returns_none() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "aaa\nbbb");
    assert!(buf.move_line_up(0).is_none());
}

#[test]
fn move_line_down_at_bottom_returns_none() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "aaa\nbbb");
    assert!(buf.move_line_down(1).is_none());
}

#[test]
fn text_range() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "hello world");
    assert_eq!(
        buf.text_range(Position::new(0, 0), Position::new(0, 5)),
        "hello"
    );
    assert_eq!(
        buf.text_range(Position::new(0, 6), Position::new(0, 11)),
        "world"
    );
}

#[test]
fn indent_lines() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "a\nb\nc");
    buf.indent_lines(0, 2);
    assert_eq!(buf.line(0), "    a");
    assert_eq!(buf.line(1), "    b");
    assert_eq!(buf.line(2), "    c");
}

#[test]
fn dedent_lines() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "    a\n    b\n    c");
    buf.dedent_lines(0, 2);
    assert_eq!(buf.line(0), "a");
    assert_eq!(buf.line(1), "b");
    assert_eq!(buf.line(2), "c");
}

#[test]
fn dedent_partial() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "  a\n      b");
    buf.dedent_lines(0, 1);
    assert_eq!(buf.line(0), "a");
    assert_eq!(buf.line(1), "  b");
}

#[test]
fn toggle_line_comments_adds_prefix_after_indent() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "fn main() {\n    println!();\n}");
    buf.toggle_line_comments(0, 2, "//");
    assert_eq!(buf.line(0), "// fn main() {");
    assert_eq!(buf.line(1), "    // println!();");
    assert_eq!(buf.line(2), "// }");
}

#[test]
fn toggle_line_comments_removes_existing_prefix() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "// a\n    // b\n//c");
    buf.toggle_line_comments(0, 2, "//");
    assert_eq!(buf.line(0), "a");
    assert_eq!(buf.line(1), "    b");
    assert_eq!(buf.line(2), "c");
}

#[test]
fn toggle_line_comments_ignores_blank_lines() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "a\n\n    b");
    buf.toggle_line_comments(0, 2, "#");
    assert_eq!(buf.line(0), "# a");
    assert_eq!(buf.line(1), "");
    assert_eq!(buf.line(2), "    # b");
}

#[test]
fn toggle_block_comment_wraps_and_unwraps_selection() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "let value = 1;");
    let start = Position::new(0, 4);
    let end = Position::new(0, 9);
    let (start, end) = buf.toggle_block_comment(start, end, "/*", "*/");
    assert_eq!(buf.line(0), "let /*value*/ = 1;");

    buf.toggle_block_comment(
        Position::new(0, start.col - 2),
        Position::new(0, end.col + 2),
        "/*",
        "*/",
    );
    assert_eq!(buf.line(0), "let value = 1;");
}

#[test]
fn matching_bracket_finds_pair_at_cursor() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "fn main() { call(1); }");
    assert_eq!(
        buf.matching_bracket(Position::new(0, 7)),
        Some((Position::new(0, 7), Position::new(0, 8)))
    );
    assert_eq!(
        buf.matching_bracket(Position::new(0, 10)),
        Some((Position::new(0, 10), Position::new(0, 21)))
    );
}

#[test]
fn matching_bracket_handles_nested_pairs() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "outer(inner())");
    assert_eq!(
        buf.matching_bracket(Position::new(0, 5)),
        Some((Position::new(0, 5), Position::new(0, 13)))
    );
    assert_eq!(
        buf.matching_bracket(Position::new(0, 11)),
        Some((Position::new(0, 11), Position::new(0, 12)))
    );
}

#[test]
fn matching_bracket_crosses_lines() {
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "{\n    value\n}");
    assert_eq!(
        buf.matching_bracket(Position::new(0, 0)),
        Some((Position::new(0, 0), Position::new(2, 0)))
    );
}
