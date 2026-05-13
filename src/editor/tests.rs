use super::*;

fn temp_file(name: &str, contents: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "llnzy_editor_state_{}_{}",
        std::process::id(),
        name
    ));
    std::fs::write(&path, contents).unwrap();
    path
}

#[test]
fn open_defers_tree_sitter_parse_to_background() {
    let path = temp_file("async_parse.rs", "fn main() {\n    println!(\"hi\");\n}\n");

    let mut editor = EditorState::new();
    let buffer_id = editor.open(path.clone()).unwrap();
    assert_eq!(editor.index_for_id(buffer_id), Some(0));
    assert!(editor.views[0].tree.is_none());
    assert!(editor.views[0].tree_dirty);

    editor.reparse_active();
    for _ in 0..100 {
        editor.reparse_active();
        if editor.views[0].tree.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    assert!(editor.views[0].tree.is_some());
    let _ = std::fs::remove_file(path);
}

#[test]
fn large_syntax_fixture_skips_tree_sitter_parse() {
    let dir = std::env::temp_dir().join(format!("llnzy-editor-large-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = stress_fixtures::write_rust_file(
        &dir,
        "large.rs",
        stress_fixtures::LARGE_SYNTAX_LINE_COUNT,
    );
    let mut editor = EditorState::new();
    let buffer_id = editor.open(path).unwrap();

    editor.reparse_active();

    let idx = editor.index_for_id(buffer_id).unwrap();
    assert!(!editor.active_parse_pending());
    assert!(editor.views[idx].tree_dirty);
    assert!(editor.views[idx].tree.is_none());

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn large_buffer_reparse_clears_existing_syntax_state() {
    let source = "fn main() {}\n";
    let path = temp_file("large_clears_existing_tree.rs", source);
    let mut editor = EditorState::new();
    let buffer_id = editor.open(path.clone()).unwrap();
    let idx = editor.index_for_id(buffer_id).unwrap();

    editor.views[idx].tree = editor.syntax.parse("rust", source);
    editor.views[idx].parse_pending = true;
    editor.views[idx].pending_tree_edit = Some(input_edit_from_positions(
        source,
        buffer::Position::new(0, 0),
        buffer::Position::new(0, 0),
        "// ",
    ));
    editor.views[idx].folded_ranges.push(FoldRange {
        start_line: 0,
        end_line: 1,
    });
    assert!(editor.views[idx].tree.is_some());

    let extra_lines = "let value = 1;\n".repeat(perf::SYNTAX_LINE_LIMIT + 1);
    editor.buffers[idx].insert(buffer::Position::new(1, 0), &extra_lines);
    editor.active = idx;
    editor.reparse_active();

    assert!(!editor.views[idx].parse_pending);
    assert!(editor.views[idx].pending_tree_edit.is_none());
    assert!(editor.views[idx].tree.is_none());
    assert!(editor.views[idx].folded_ranges.is_empty());
    assert!(editor.views[idx].tree_dirty);

    let _ = std::fs::remove_file(path);
}

#[test]
fn late_parse_result_does_not_restore_tree_for_large_buffer() {
    let source = "fn main() {}\n";
    let path = temp_file("large_rejects_late_parse.rs", source);
    let mut editor = EditorState::new();
    let buffer_id = editor.open(path.clone()).unwrap();
    let idx = editor.index_for_id(buffer_id).unwrap();

    editor.views[idx].parse_pending = true;
    editor.views[idx].parse_generation = 7;
    let parsed_tree = editor.syntax.parse("rust", source);
    let extra_lines = "let value = 1;\n".repeat(perf::SYNTAX_LINE_LIMIT + 1);
    editor.buffers[idx].insert(buffer::Position::new(1, 0), &extra_lines);

    editor.apply_parse_result(ParseResult {
        buffer_id,
        generation: 7,
        path: Some(path.clone()),
        lang_id: "rust",
        tree: parsed_tree,
        line_count: 1,
        used_incremental: false,
    });

    assert!(!editor.views[idx].parse_pending);
    assert!(editor.views[idx].tree.is_none());

    let _ = std::fs::remove_file(path);
}

#[test]
fn dirty_parsed_buffer_plans_incremental_reparse_with_previous_tree() {
    let source = "fn main() {\n    let value = 1;\n}\n";
    let path = temp_file("incremental_plan.rs", source);
    let mut editor = EditorState::new();
    let buffer_id = editor.open(path.clone()).unwrap();
    let idx = editor.index_for_id(buffer_id).unwrap();

    editor.views[idx].tree = editor.syntax.parse("rust", source);
    assert!(editor.views[idx].tree.is_some());

    let old_source = editor.buffers[idx].text();
    let start = crate::editor::buffer::Position::new(1, 16);
    editor.buffers[idx].insert(start, " + 1");
    editor.active = idx;
    assert!(editor.record_active_incremental_edit(&old_source, start, start, " + 1"));

    match plan_syntax_reparse(editor.buffers[idx].line_count(), &editor.views[idx]) {
        SyntaxReparsePlan::Incremental {
            lang_id, old_tree, ..
        } => {
            assert_eq!(lang_id, "rust");
            assert_eq!(old_tree.root_node().kind(), "source_file");
        }
        SyntaxReparsePlan::Fresh { .. } => {
            panic!("parsed dirty buffer should retain its previous tree for reparse")
        }
        SyntaxReparsePlan::Skip => panic!("small dirty Rust buffer should be parsed"),
    }

    let _ = std::fs::remove_file(path);
}

#[test]
fn incremental_reparse_applies_pending_tree_edit() {
    let source = "fn main() {\n    let value = 1;\n}\n";
    let path = temp_file("incremental_reparse.rs", source);
    let mut editor = EditorState::new();
    let buffer_id = editor.open(path.clone()).unwrap();
    let idx = editor.index_for_id(buffer_id).unwrap();

    editor.reparse_active();
    for _ in 0..100 {
        editor.reparse_active();
        if editor.views[idx].tree.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(editor.views[idx].tree.is_some());

    let old_source = editor.buffers[idx].text();
    let start = crate::editor::buffer::Position::new(1, 17);
    editor.buffers[idx].insert(start, " + 1");
    editor.active = idx;
    assert!(editor.record_active_incremental_edit(&old_source, start, start, " + 1"));

    editor.reparse_active();
    for _ in 0..100 {
        editor.reparse_active();
        if !editor.active_parse_pending() && editor.views[idx].last_parse_used_incremental {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    assert!(editor.views[idx].tree.is_some());
    assert!(editor.views[idx].last_parse_used_incremental);

    let _ = std::fs::remove_file(path);
}

#[test]
fn input_edit_uses_byte_columns_for_unicode_text() {
    let source = "fn main() {\n    let café = 1;\n}\n";
    let start = crate::editor::buffer::Position::new(1, 8);
    let end = crate::editor::buffer::Position::new(1, 12);

    let edit = input_edit_from_positions(source, start, end, "value");

    assert_eq!(edit.start_position, Point { row: 1, column: 8 });
    assert_eq!(edit.old_end_position, Point { row: 1, column: 13 });
    assert_eq!(edit.new_end_position, Point { row: 1, column: 13 });
    assert_eq!(edit.new_end_byte, edit.start_byte + "value".len());
}

#[test]
fn buffer_position_utf16_conversion_uses_character_indices() {
    use crate::stacker::utf16::{char_index_to_utf16_index, utf16_index_to_char_index};

    let mut buffer = Buffer::empty();
    buffer.insert(
        crate::editor::buffer::Position::new(0, 0),
        "a\u{1f600}\nbc\u{1d11e}d",
    );
    let text = buffer.text();

    let before_emoji = crate::editor::buffer::Position::new(0, 1);
    let after_emoji = crate::editor::buffer::Position::new(0, 2);
    let before_g_clef = crate::editor::buffer::Position::new(1, 2);
    let after_g_clef = crate::editor::buffer::Position::new(1, 3);

    assert_eq!(
        char_index_to_utf16_index(&text, buffer.pos_to_char(before_emoji)),
        1
    );
    assert_eq!(
        char_index_to_utf16_index(&text, buffer.pos_to_char(after_emoji)),
        3
    );
    assert_eq!(
        char_index_to_utf16_index(&text, buffer.pos_to_char(before_g_clef)),
        6
    );
    assert_eq!(
        char_index_to_utf16_index(&text, buffer.pos_to_char(after_g_clef)),
        8
    );

    assert_eq!(
        buffer.char_to_pos(utf16_index_to_char_index(&text, 2)),
        after_emoji
    );
    assert_eq!(
        buffer.char_to_pos(utf16_index_to_char_index(&text, 7)),
        after_g_clef
    );
}

#[test]
fn dirty_buffer_without_tree_plans_fresh_parse() {
    let mut view = BufferView {
        lang_id: Some("rust"),
        tree_dirty: true,
        tree: None,
        ..Default::default()
    };

    match plan_syntax_reparse(3, &view) {
        SyntaxReparsePlan::Fresh { lang_id } => assert_eq!(lang_id, "rust"),
        SyntaxReparsePlan::Incremental { .. } => {
            panic!("missing previous tree should fall back to a fresh parse")
        }
        SyntaxReparsePlan::Skip => panic!("dirty Rust buffer should not be skipped"),
    }

    view.parse_pending = true;
    assert!(matches!(
        plan_syntax_reparse(3, &view),
        SyntaxReparsePlan::Skip
    ));
}

#[test]
fn large_dirty_buffer_skips_even_when_previous_tree_exists() {
    let source = "fn main() {}\n";
    let mut syntax = SyntaxEngine::new();
    let view = BufferView {
        lang_id: Some("rust"),
        tree_dirty: true,
        tree: syntax.parse("rust", source),
        ..Default::default()
    };

    assert!(matches!(
        plan_syntax_reparse(stress_fixtures::LARGE_SYNTAX_LINE_COUNT, &view),
        SyntaxReparsePlan::Skip
    ));
}

#[test]
fn buffer_ids_remain_stable_when_indexes_shift() {
    let first = temp_file("first.txt", "first");
    let second = temp_file("second.txt", "second");

    let mut editor = EditorState::new();
    let first_id = editor.open(first.clone()).unwrap();
    let second_id = editor.open(second.clone()).unwrap();

    assert_ne!(first_id, second_id);
    assert_eq!(editor.index_for_id(first_id), Some(0));
    assert_eq!(editor.index_for_id(second_id), Some(1));

    assert!(editor.close_id(first_id));

    assert_eq!(editor.index_for_id(first_id), None);
    assert_eq!(editor.index_for_id(second_id), Some(0));
    assert_eq!(
        editor
            .buffer_for_id(second_id)
            .and_then(|buffer| buffer.path().map(PathBuf::from)),
        Some(second.clone())
    );

    let _ = std::fs::remove_file(first);
    let _ = std::fs::remove_file(second);
}

#[test]
fn switch_to_id_selects_the_matching_buffer_after_index_shift() {
    let first = temp_file("switch_first.txt", "first");
    let second = temp_file("switch_second.txt", "second");

    let mut editor = EditorState::new();
    let first_id = editor.open(first.clone()).unwrap();
    let second_id = editor.open(second.clone()).unwrap();
    assert!(editor.close_id(first_id));

    assert!(editor.switch_to_id(second_id));
    assert_eq!(editor.active_buffer_id(), Some(second_id));
    assert_eq!(editor.active, 0);

    let _ = std::fs::remove_file(first);
    let _ = std::fs::remove_file(second);
}

#[test]
fn closing_buffer_before_active_preserves_active_buffer_identity() {
    let first = temp_file("close_before_active_first.txt", "first");
    let second = temp_file("close_before_active_second.txt", "second");
    let third = temp_file("close_before_active_third.txt", "third");

    let mut editor = EditorState::new();
    let first_id = editor.open(first.clone()).unwrap();
    let second_id = editor.open(second.clone()).unwrap();
    let third_id = editor.open(third.clone()).unwrap();
    assert!(editor.switch_to_id(second_id));

    assert!(editor.close_id(first_id));

    assert_eq!(editor.active_buffer_id(), Some(second_id));
    assert_eq!(editor.index_for_id(second_id), Some(0));
    assert_eq!(editor.index_for_id(third_id), Some(1));

    let _ = std::fs::remove_file(first);
    let _ = std::fs::remove_file(second);
    let _ = std::fs::remove_file(third);
}

#[test]
fn active_buffer_view_returns_buffer_id_buffer_and_view() {
    let path = temp_file("active_buffer_view.rs", "fn main() {}\n");
    let mut editor = EditorState::new();
    let buffer_id = editor.open(path.clone()).unwrap();

    let (active_id, buffer, view) = editor.active_buffer_view().unwrap();

    assert_eq!(active_id, buffer_id);
    assert_eq!(buffer.path(), Some(path.as_path()));
    assert_eq!(view.lang_id, Some("rust"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn registry_resolves_and_updates_paths_by_id() {
    let original = temp_file("registry_original.txt", "first");
    let renamed = std::env::temp_dir().join(format!(
        "llnzy_editor_state_{}_registry_renamed.txt",
        std::process::id()
    ));

    let mut editor = EditorState::new();
    let id = editor.open(original.clone()).unwrap();

    assert_eq!(editor.id_for_path(&original), Some(id));
    assert!(editor.update_path(id, renamed.clone()));
    assert_eq!(editor.id_for_path(&original), None);
    assert_eq!(editor.id_for_path(&renamed), Some(id));

    let _ = std::fs::remove_file(original);
    let _ = std::fs::remove_file(renamed);
}

#[test]
fn dirty_buffer_ids_reports_modified_buffers_by_identity() {
    let clean = temp_file("dirty_clean.txt", "clean");
    let dirty = temp_file("dirty_modified.txt", "dirty");

    let mut editor = EditorState::new();
    let clean_id = editor.open(clean.clone()).unwrap();
    let dirty_id = editor.open(dirty.clone()).unwrap();
    editor
        .buffer_for_id_mut(dirty_id)
        .unwrap()
        .insert(crate::editor::buffer::Position::new(0, 5), "!");

    assert_eq!(editor.dirty_buffer_ids(), vec![dirty_id]);
    assert!(!editor.dirty_buffer_ids().contains(&clean_id));

    let _ = std::fs::remove_file(clean);
    let _ = std::fs::remove_file(dirty);
}
