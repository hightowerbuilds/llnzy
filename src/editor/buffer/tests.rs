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
fn unicode_position_corpus_round_trips_all_character_offsets() {
    for text in [
        "",
        "hello",
        "aé文z",
        "a😀b",
        "line one\nline two",
        "line one\n𝄞 music\nemoji 😀\n",
        "क्ष and flags 🇺🇸",
    ] {
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), text);

        for char_idx in 0..=buf.len_chars() {
            let pos = buf.char_to_pos(char_idx);
            assert_eq!(
                buf.pos_to_char(pos),
                char_idx,
                "position round-trip failed for {text:?} at char {char_idx}"
            );
        }
    }
}

#[derive(Clone, Copy)]
enum ScenarioEdit {
    Insert(Position, &'static str),
    Delete(Position, Position),
    Replace(Position, Position, &'static str),
}

#[test]
fn edit_sequences_undo_and_redo_to_exact_text() {
    for (edits, expected) in [
        (
            &[
                ScenarioEdit::Insert(Position::new(0, 0), "hello\nworld"),
                ScenarioEdit::Replace(Position::new(1, 0), Position::new(1, 5), "🌍"),
                ScenarioEdit::Insert(Position::new(0, 5), "!"),
                ScenarioEdit::Delete(Position::new(0, 0), Position::new(0, 1)),
            ][..],
            "ello!\n🌍",
        ),
        (
            &[
                ScenarioEdit::Insert(Position::new(0, 0), "a\nb\nc"),
                ScenarioEdit::Delete(Position::new(2, 1), Position::new(0, 1)),
                ScenarioEdit::Insert(Position::new(0, 1), "é\n𝄞"),
            ],
            "aé\n𝄞",
        ),
        (
            &[
                ScenarioEdit::Insert(Position::new(0, 0), "alpha\nbeta\ngamma"),
                ScenarioEdit::Replace(Position::new(2, 5), Position::new(1, 0), "BETA\nΓ"),
                ScenarioEdit::Insert(Position::new(0, 5), "\ninserted"),
            ],
            "alpha\ninserted\nBETA\nΓ",
        ),
    ] {
        let mut buf = Buffer::empty();
        for edit in edits {
            apply_scenario_edit(&mut buf, *edit);
        }
        assert_eq!(buf.text(), expected);

        for _ in 0..edits.len() {
            assert!(buf.undo().is_some());
        }
        assert_eq!(buf.text(), "");

        for _ in 0..edits.len() {
            assert!(buf.redo().is_some());
        }
        assert_eq!(buf.text(), expected);
    }
}

fn apply_scenario_edit(buf: &mut Buffer, edit: ScenarioEdit) {
    match edit {
        ScenarioEdit::Insert(pos, text) => buf.insert(pos, text),
        ScenarioEdit::Delete(start, end) => buf.delete(start, end),
        ScenarioEdit::Replace(start, end, text) => buf.replace(start, end, text),
    }
}

#[test]
fn line_ending_detection() {
    assert_eq!(LineEnding::detect("a\nb\nc\n"), LineEnding::Lf);
    assert_eq!(LineEnding::detect("a\r\nb\r\nc\r\n"), LineEnding::CrLf);
    assert_eq!(LineEnding::detect("a\rb\rc\r"), LineEnding::Cr);
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
fn line_handles_non_contiguous_rope_slice() {
    let prefix = "a".repeat(16_384);
    let suffix = "β".repeat(16_384);
    let line_text = format!("{prefix}中{suffix}");
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), &format!("{line_text}\nnext"));

    assert!(
        buf.rope.line(0).as_str().is_none(),
        "test fixture should span multiple rope chunks"
    );
    assert_eq!(buf.line(0).as_ref(), line_text);
    assert_eq!(buf.line_len(0), line_text.chars().count());
    assert_eq!(
        buf.char_at(Position::new(0, prefix.chars().count())),
        Some('中')
    );
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
fn failed_policy_save_does_not_transform_buffer() {
    let missing_parent =
        std::env::temp_dir().join(format!("llnzy-policy-missing-parent-{}", unique_suffix()));
    let path = missing_parent.join("file.txt");

    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "unsaved   ");
    buf.trim_trailing_whitespace = Some(true);
    buf.insert_final_newline = Some(true);

    assert!(buf.save_to(&path).is_err());
    assert!(buf.is_modified());
    assert!(buf.path().is_none());
    assert_eq!(buf.text(), "unsaved   ");
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
fn editorconfig_save_trims_trailing_whitespace_and_inserts_final_newline() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "llnzy-editorconfig-trim-{}-{}.txt",
        std::process::id(),
        unique_suffix()
    ));

    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "alpha   \n beta\t");
    buf.trim_trailing_whitespace = Some(true);
    buf.insert_final_newline = Some(true);

    buf.save_to(&path).unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "alpha\n beta\n");
    assert_eq!(buf.text(), "alpha\n beta\n");
    assert!(!buf.is_modified());

    let _ = std::fs::remove_file(&path);
}

#[test]
fn editorconfig_save_can_remove_final_newlines() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "llnzy-editorconfig-no-final-{}-{}.txt",
        std::process::id(),
        unique_suffix()
    ));

    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "alpha\n\n");
    buf.insert_final_newline = Some(false);

    buf.save_to(&path).unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "alpha");
    assert_eq!(buf.text(), "alpha");
    assert!(!buf.is_modified());

    let _ = std::fs::remove_file(&path);
}

#[test]
fn editorconfig_save_honors_eol_override() {
    use crate::editor::editorconfig::EndOfLine;

    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "llnzy-editorconfig-eol-{}-{}.txt",
        std::process::id(),
        unique_suffix()
    ));

    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "alpha\nbeta\n");
    buf.eol_override = Some(EndOfLine::Cr);

    buf.save_to(&path).unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "alpha\rbeta\r");
    assert_eq!(buf.text(), "alpha\nbeta\n");
    assert_eq!(buf.line_ending(), LineEnding::Cr);

    let loaded = Buffer::from_file(&path).unwrap();
    assert_eq!(loaded.line_ending(), LineEnding::Cr);
    assert_eq!(loaded.text(), "alpha\nbeta\n");

    let _ = std::fs::remove_file(&path);
}

#[test]
fn editorconfig_save_writes_and_loads_utf8_bom() {
    use crate::editor::editorconfig::Charset;

    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "llnzy-editorconfig-bom-{}-{}.txt",
        std::process::id(),
        unique_suffix()
    ));

    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), "hello");
    buf.charset_override = Some(Charset::Utf8Bom);

    buf.save_to(&path).unwrap();

    let raw = std::fs::read(&path).unwrap();
    assert!(raw.starts_with(b"\xEF\xBB\xBF"));
    let loaded = Buffer::from_file(&path).unwrap();
    assert_eq!(loaded.text(), "hello");

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
fn move_line_preserves_non_contiguous_rope_slice() {
    let long_line = "x".repeat(16_384);
    let mut buf = Buffer::empty();
    buf.insert(Position::new(0, 0), &format!("{long_line}\nshort"));

    assert!(
        buf.rope.line(0).as_str().is_none(),
        "test fixture should span multiple rope chunks"
    );
    buf.move_line_down(0);
    assert_eq!(buf.line(0), "short");
    assert_eq!(buf.line(1).as_ref(), long_line);
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

#[test]
fn empty_buffer_defaults_to_code_kind() {
    let buf = Buffer::empty();
    assert_eq!(buf.kind(), BufferKind::Code);
    assert!(!buf.kind().is_prose());
}

#[test]
fn empty_prose_buffer_reports_prose_kind() {
    let buf = Buffer::empty_prose();
    assert_eq!(buf.kind(), BufferKind::Prose);
    assert!(buf.kind().is_prose());
}

#[test]
fn prose_buffer_supports_normal_edits_without_changing_kind() {
    let mut buf = Buffer::empty_prose();
    buf.insert(Position::new(0, 0), "hello world");
    assert_eq!(buf.line(0), "hello world");
    assert_eq!(buf.kind(), BufferKind::Prose);
    assert!(buf.is_modified());
}

fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}
