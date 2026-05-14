use super::*;
use crate::config::Config;
use alacritty_terminal::term::cell::Flags;

#[test]
fn create_terminal() {
    let term = Terminal::new(80, 24);
    assert_eq!(term.size(), (80, 24));
}

#[test]
fn create_terminal_small() {
    let term = Terminal::new(10, 5);
    assert_eq!(term.size(), (10, 5));
}

#[test]
fn resize_terminal() {
    let mut term = Terminal::new(80, 24);
    term.resize(120, 40);
    assert_eq!(term.size(), (120, 40));
}

#[test]
fn zero_sized_terminal_requests_are_clamped() {
    let mut term = Terminal::new(0, 0);
    assert_eq!(term.size(), (1, 1));

    term.resize(0, 0);
    assert_eq!(term.size(), (1, 1));

    term.process(b"X");
    assert_eq!(term.cell_char(0, 0), 'X');
}

#[test]
fn empty_cells_are_space() {
    let term = Terminal::new(80, 24);
    assert_eq!(term.cell_char(0, 0), ' ');
    assert_eq!(term.cell_char(23, 79), ' ');
}

#[test]
fn process_text_sets_cells() {
    let mut term = Terminal::new(80, 24);
    term.process(b"Hello");
    assert_eq!(term.cell_char(0, 0), 'H');
    assert_eq!(term.cell_char(0, 1), 'e');
    assert_eq!(term.cell_char(0, 2), 'l');
    assert_eq!(term.cell_char(0, 3), 'l');
    assert_eq!(term.cell_char(0, 4), 'o');
    assert_eq!(term.cell_char(0, 5), ' ');
}

#[test]
fn process_newline_moves_to_next_row() {
    let mut term = Terminal::new(80, 24);
    term.process(b"A\r\nB");
    assert_eq!(term.cell_char(0, 0), 'A');
    assert_eq!(term.cell_char(1, 0), 'B');
}

#[test]
fn simple_selection_uses_alacritty_selected_text() {
    let mut term = Terminal::new(80, 24);
    term.process(b"Hello world");

    term.start_selection(0, 0);
    term.update_selection(0, 4);

    assert!(term.has_selection());
    assert_eq!(term.selected_text().as_deref(), Some("Hello"));
}

#[test]
fn forward_drag_selection_uses_full_range() {
    let mut term = Terminal::new(80, 24);
    term.process(b"Hello world");

    term.start_selection(0, 0);
    term.update_selection(0, 10);

    assert_eq!(term.selected_text().as_deref(), Some("Hello world"));
}

#[test]
fn same_cell_drag_update_is_coalesced() {
    let mut term = Terminal::new(80, 24);
    term.process(b"Hello world");

    term.start_selection(0, 0);
    let revision = term.selection_revision();

    assert!(!term.update_selection(0, 0));
    assert_eq!(term.selection_revision(), revision);

    assert!(term.update_selection(0, 1));
    assert!(term.selection_revision() > revision);
}

#[test]
fn selection_revision_changes_when_visible_selection_scrolls() {
    let mut term = Terminal::new(80, 3);
    term.process(b"one\r\ntwo\r\nthree");
    term.start_selection(0, 0);
    term.update_selection(0, 2);
    let revision = term.selection_revision();

    term.scroll(1);

    assert!(term.selection_revision() > revision);
}

#[test]
fn backward_drag_selection_uses_full_range() {
    let mut term = Terminal::new(80, 24);
    term.process(b"Hello world");

    term.start_selection(0, 10);
    term.update_selection(0, 0);

    assert_eq!(term.selected_text().as_deref(), Some("Hello world"));
}

#[test]
fn multiline_drag_selection_uses_full_range() {
    let mut term = Terminal::new(80, 24);
    term.process(b"Hello\r\nworld");

    term.start_selection(0, 0);
    term.update_selection(1, 4);

    assert_eq!(term.selected_text().as_deref(), Some("Hello\nworld"));
}

#[test]
fn mouse_reporting_tui_selection_copies_visible_grid_text() {
    let mut term = Terminal::new(80, 24);
    term.process(b"\x1b[?1000h\x1b[?1006hCodex status\r\nSelect this text");

    assert!(term.mouse_mode());
    assert!(term.sgr_mouse());

    term.start_selection(0, 0);
    term.update_selection(1, 15);

    assert_eq!(
        term.selected_text().as_deref(),
        Some("Codex status\nSelect this text")
    );
}

#[test]
fn clearing_selection_removes_selected_text() {
    let mut term = Terminal::new(80, 24);
    term.process(b"Hello");
    term.start_selection(0, 0);
    term.update_selection(0, 4);

    term.clear_selection();

    assert!(!term.has_selection());
    assert_eq!(term.selected_text(), None);
}

#[test]
fn cursor_starts_at_origin() {
    let term = Terminal::new(80, 24);
    assert_eq!(term.cursor_point(), Some((0, 0)));
}

#[test]
fn cursor_advances_with_text() {
    let mut term = Terminal::new(80, 24);
    term.process(b"ABC");
    assert_eq!(term.cursor_point(), Some((0, 3)));
}

#[test]
fn default_modes() {
    let term = Terminal::new(80, 24);
    assert!(!term.app_cursor());
    assert!(!term.mouse_mode());
    assert!(!term.sgr_mouse());
    assert!(!term.bracketed_paste());
}

#[test]
fn enable_app_cursor_mode() {
    let mut term = Terminal::new(80, 24);
    term.process(b"\x1b[?1h");
    assert!(term.app_cursor());
}

#[test]
fn disable_app_cursor_mode() {
    let mut term = Terminal::new(80, 24);
    term.process(b"\x1b[?1h");
    term.process(b"\x1b[?1l");
    assert!(!term.app_cursor());
}

#[test]
fn enable_bracketed_paste() {
    let mut term = Terminal::new(80, 24);
    term.process(b"\x1b[?2004h");
    assert!(term.bracketed_paste());
}

#[test]
fn enable_mouse_mode() {
    let mut term = Terminal::new(80, 24);
    term.process(b"\x1b[?1000h");
    assert!(term.mouse_mode());
}

#[test]
fn enable_sgr_mouse() {
    let mut term = Terminal::new(80, 24);
    term.process(b"\x1b[?1006h");
    assert!(term.sgr_mouse());
}

#[test]
fn title_event_from_osc() {
    let mut term = Terminal::new(80, 24);
    term.process(b"\x1b]0;My Title\x07");
    let events = term.drain_events();
    let has_title = events
        .iter()
        .any(|e| matches!(e, TerminalEvent::Title(t) if t == "My Title"));
    assert!(has_title);
}

#[test]
fn working_directory_event_from_osc7_file_uri() {
    let mut term = Terminal::new(80, 24);
    term.process(b"\x1b]7;file://localhost/tmp/llnzy%20cwd\x07");
    let events = term.drain_events();
    let has_cwd = events
        .iter()
        .any(|e| matches!(e, TerminalEvent::WorkingDirectory(cwd) if cwd == "/tmp/llnzy cwd"));
    assert!(has_cwd);
}

#[test]
fn bell_event() {
    let mut term = Terminal::new(80, 24);
    term.process(b"\x07");
    let events = term.drain_events();
    let has_bell = events.iter().any(|e| matches!(e, TerminalEvent::Bell));
    assert!(has_bell);
}

#[test]
fn drain_events_empties_queue() {
    let mut term = Terminal::new(80, 24);
    term.process(b"\x07");
    let events = term.drain_events();
    assert!(!events.is_empty());
    let events2 = term.drain_events();
    assert!(events2.is_empty());
}

#[test]
fn bold_flag_set() {
    let mut term = Terminal::new(80, 24);
    term.process(b"\x1b[1mX");
    let flags = term.cell_flags(0, 0);
    assert!(flags.contains(Flags::BOLD));
}

#[test]
fn italic_flag_set() {
    let mut term = Terminal::new(80, 24);
    term.process(b"\x1b[3mX");
    let flags = term.cell_flags(0, 0);
    assert!(flags.contains(Flags::ITALIC));
}

#[test]
fn underline_flag_set() {
    let mut term = Terminal::new(80, 24);
    term.process(b"\x1b[4mX");
    let flags = term.cell_flags(0, 0);
    assert!(flags.contains(Flags::UNDERLINE));
}

#[test]
fn inverse_flag_set() {
    let mut term = Terminal::new(80, 24);
    term.process(b"\x1b[7mX");
    let flags = term.cell_flags(0, 0);
    assert!(flags.contains(Flags::INVERSE));
}

#[test]
fn strikeout_flag_set() {
    let mut term = Terminal::new(80, 24);
    term.process(b"\x1b[9mX");
    let flags = term.cell_flags(0, 0);
    assert!(flags.contains(Flags::STRIKEOUT));
}

#[test]
fn scroll_to_bottom_after_scroll_up() {
    let mut term = Terminal::new(80, 24);
    for _ in 0..30 {
        term.process(b"\r\n");
    }
    term.scroll(5);
    term.scroll_to_bottom();
    assert!(term.cursor_point().is_some());
}

#[test]
fn resolve_fg_bg_normal() {
    let mut term = Terminal::new(80, 24);
    let config = Config::default();
    term.process(b"X");
    let fg = term.resolve_fg_with_attrs(0, 0, &config);
    let bg = term.resolve_bg_with_attrs(0, 0, &config);
    assert_eq!(fg, config.colors.foreground);
    assert_eq!(bg, config.colors.background);
}

#[test]
fn resolve_fg_bg_inverse_swaps() {
    let mut term = Terminal::new(80, 24);
    let config = Config::default();
    term.process(b"\x1b[7mX");
    let fg = term.resolve_fg_with_attrs(0, 0, &config);
    let bg = term.resolve_bg_with_attrs(0, 0, &config);
    assert_eq!(fg, config.colors.background);
    assert_eq!(bg, config.colors.foreground);
}

#[test]
fn wide_char_occupies_two_cells_with_spacer_flag() {
    let mut term = Terminal::new(10, 2);
    // U+4E2D is a fullwidth CJK character. alacritty stores it in column 0
    // and marks column 1 as a WIDE_CHAR_SPACER so terminal rows do not drift.
    term.process("中A".as_bytes());

    assert_eq!(term.cell_char(0, 0), '中');
    assert!(term.cell_flags(0, 0).contains(Flags::WIDE_CHAR));
    assert!(term.cell_flags(0, 1).contains(Flags::WIDE_CHAR_SPACER));
    // The trailing ASCII glyph lands at column 2, not column 1.
    assert_eq!(term.cell_char(0, 2), 'A');
}
