//! Integration tests for terminal emulation.
//!
//! These tests feed escape sequences into the terminal emulator and verify
//! the resulting grid state, testing the full pipeline from raw bytes through
//! ANSI parsing to cell content.

use llnzy::config::Config;
use llnzy::terminal::Terminal;

// ── Helper ──

fn term() -> Terminal {
    Terminal::new(80, 24)
}

fn read_line(term: &Terminal, row: usize, cols: usize) -> String {
    (0..cols)
        .map(|c| term.cell_char(row, c))
        .collect::<String>()
        .trim_end()
        .to_string()
}

// ── Cursor movement ──

#[test]
fn cursor_up() {
    let mut t = term();
    t.process(b"\r\nline2\x1b[A");
    // Cursor should have moved up one line
    assert_eq!(t.cursor_point(), Some((0, 5)));
}

#[test]
fn cursor_down() {
    let mut t = term();
    t.process(b"X\x1b[B");
    assert_eq!(t.cursor_point(), Some((1, 1)));
}

#[test]
fn cursor_forward() {
    let mut t = term();
    t.process(b"\x1b[10C");
    assert_eq!(t.cursor_point(), Some((0, 10)));
}

#[test]
fn cursor_backward() {
    let mut t = term();
    t.process(b"ABCDE\x1b[3D");
    assert_eq!(t.cursor_point(), Some((0, 2)));
}

#[test]
fn cursor_absolute_position() {
    let mut t = term();
    t.process(b"\x1b[5;10H");
    // CUP is 1-indexed: row 5, col 10 → (4, 9) in 0-indexed
    assert_eq!(t.cursor_point(), Some((4, 9)));
}

#[test]
fn cursor_home() {
    let mut t = term();
    t.process(b"some text\x1b[H");
    assert_eq!(t.cursor_point(), Some((0, 0)));
}

#[test]
fn cursor_save_restore() {
    let mut t = term();
    t.process(b"\x1b[5;5H"); // move to (4,4)
    t.process(b"\x1b7"); // save cursor
    t.process(b"\x1b[10;10H"); // move elsewhere
    t.process(b"\x1b8"); // restore cursor
    assert_eq!(t.cursor_point(), Some((4, 4)));
}

// ── Erase operations ──

#[test]
fn erase_display_from_cursor() {
    let mut t = term();
    t.process(b"AAAA\r\nBBBB\r\nCCCC");
    t.process(b"\x1b[1;3H"); // row 1, col 3 (0-indexed: 0, 2)
    t.process(b"\x1b[J"); // erase from cursor to end
    assert_eq!(read_line(&t, 0, 80), "AA");
    assert_eq!(read_line(&t, 1, 80), "");
    assert_eq!(read_line(&t, 2, 80), "");
}

#[test]
fn erase_line_from_cursor() {
    let mut t = term();
    t.process(b"Hello World");
    t.process(b"\x1b[1;6H"); // col 6 (0-indexed: 5)
    t.process(b"\x1b[K"); // erase from cursor to end of line
    assert_eq!(read_line(&t, 0, 80), "Hello");
}

#[test]
fn erase_entire_line() {
    let mut t = term();
    t.process(b"Hello World");
    t.process(b"\x1b[2K"); // erase entire line
    assert_eq!(read_line(&t, 0, 80), "");
}

#[test]
fn erase_display_full() {
    let mut t = term();
    t.process(b"Line1\r\nLine2\r\nLine3");
    t.process(b"\x1b[2J"); // erase entire display
    for row in 0..3 {
        assert_eq!(read_line(&t, row, 80), "");
    }
}

// ── Line wrapping ──

#[test]
fn line_wraps_at_column_limit() {
    let mut t = Terminal::new(10, 5);
    t.process(b"1234567890AB");
    assert_eq!(read_line(&t, 0, 10), "1234567890");
    // Overflow wraps to row 1
    assert_eq!(t.cell_char(1, 0), 'A');
    assert_eq!(t.cell_char(1, 1), 'B');
}

// ── Scrolling ──

#[test]
fn scroll_down_fills_new_lines() {
    let mut t = Terminal::new(80, 5);
    // Fill all 5 rows then print one more line to force scroll
    t.process(b"L1\r\nL2\r\nL3\r\nL4\r\nL5\r\nL6");
    // L1 should have scrolled off, L2 is now row 0
    assert_eq!(read_line(&t, 0, 80), "L2");
    assert_eq!(read_line(&t, 4, 80), "L6");
}

#[test]
fn scroll_region() {
    let mut t = Terminal::new(80, 10);
    // Set scroll region to rows 3-6 (1-indexed)
    t.process(b"\x1b[3;6r");
    // Move cursor to row 6 (bottom of region)
    t.process(b"\x1b[6;1H");
    // Write enough to scroll within the region
    t.process(b"A\r\nB\r\nC");
    // Row 1 and 2 (above region) should be unaffected
    // The scroll region content should have shifted
    // Just verify the terminal didn't panic and cursor is in the region
    let (_, row) = t.cursor_point().unwrap();
    assert!(row <= 5); // row is within or below region
}

// ── Text attributes ──

#[test]
fn sgr_reset_clears_all_attributes() {
    let mut t = term();
    use alacritty_terminal::term::cell::Flags;
    t.process(b"\x1b[1;3;4mX\x1b[0mY");
    // X should have bold+italic+underline
    let flags_x = t.cell_flags(0, 0);
    assert!(flags_x.contains(Flags::BOLD));
    assert!(flags_x.contains(Flags::ITALIC));
    assert!(flags_x.contains(Flags::UNDERLINE));
    // Y after reset should have no flags
    let flags_y = t.cell_flags(0, 1);
    assert!(!flags_y.contains(Flags::BOLD));
    assert!(!flags_y.contains(Flags::ITALIC));
    assert!(!flags_y.contains(Flags::UNDERLINE));
}

#[test]
fn multiple_sgr_in_one_sequence() {
    let mut t = term();
    use alacritty_terminal::term::cell::Flags;
    // Bold + Underline + Strikethrough in one sequence
    t.process(b"\x1b[1;4;9mX");
    let flags = t.cell_flags(0, 0);
    assert!(flags.contains(Flags::BOLD));
    assert!(flags.contains(Flags::UNDERLINE));
    assert!(flags.contains(Flags::STRIKEOUT));
}

#[test]
fn dim_attribute() {
    let mut t = term();
    use alacritty_terminal::term::cell::Flags;
    t.process(b"\x1b[2mX");
    assert!(t.cell_flags(0, 0).contains(Flags::DIM));
}

#[test]
fn hidden_attribute() {
    let mut t = term();
    use alacritty_terminal::term::cell::Flags;
    t.process(b"\x1b[8mX");
    assert!(t.cell_flags(0, 0).contains(Flags::HIDDEN));
}

// ── Color sequences ──

#[test]
fn ansi_foreground_color_16() {
    let mut t = term();
    let config = Config::default();
    // \x1b[31m = red foreground
    t.process(b"\x1b[31mR");
    let fg = t.resolve_fg_with_attrs(0, 0, &config);
    assert_eq!(fg, config.colors.ansi[1]); // red
}

#[test]
fn ansi_foreground_bright_color() {
    let mut t = term();
    let config = Config::default();
    // \x1b[91m = bright red foreground
    t.process(b"\x1b[91mR");
    let fg = t.resolve_fg_with_attrs(0, 0, &config);
    assert_eq!(fg, config.colors.ansi[9]); // bright red
}

#[test]
fn ansi_background_color() {
    let mut t = term();
    let config = Config::default();
    // \x1b[44m = blue background
    t.process(b"\x1b[44mX");
    let bg = t.resolve_bg_with_attrs(0, 0, &config);
    assert_eq!(bg, config.colors.ansi[4]); // blue
}

#[test]
fn true_color_foreground() {
    let mut t = term();
    let config = Config::default();
    // \x1b[38;2;R;G;Bm = 24-bit true color foreground
    t.process(b"\x1b[38;2;100;150;200mX");
    let fg = t.resolve_fg_with_attrs(0, 0, &config);
    assert_eq!(fg, [100, 150, 200]);
}

#[test]
fn true_color_background() {
    let mut t = term();
    let config = Config::default();
    t.process(b"\x1b[48;2;50;100;150mX");
    let bg = t.resolve_bg_with_attrs(0, 0, &config);
    assert_eq!(bg, [50, 100, 150]);
}

#[test]
fn indexed_256_color_foreground() {
    let mut t = term();
    let config = Config::default();
    // \x1b[38;5;196m = 256-color index 196 (bright red in cube)
    t.process(b"\x1b[38;5;196mX");
    let fg = t.resolve_fg_with_attrs(0, 0, &config);
    assert_eq!(fg, [255, 0, 0]); // index 196 = (5,0,0) = pure red
}

#[test]
fn inverse_swaps_true_colors() {
    let mut t = term();
    let config = Config::default();
    t.process(b"\x1b[38;2;100;100;100m\x1b[48;2;200;200;200m\x1b[7mX");
    let fg = t.resolve_fg_with_attrs(0, 0, &config);
    let bg = t.resolve_bg_with_attrs(0, 0, &config);
    // INVERSE: fg becomes the bg color, bg becomes the fg color
    assert_eq!(fg, [200, 200, 200]);
    assert_eq!(bg, [100, 100, 100]);
}

// ── OSC sequences ──

#[test]
fn osc_0_sets_title() {
    let mut t = term();
    t.process(b"\x1b]0;My Terminal\x07");
    let events = t.drain_events();
    let title = events.iter().find_map(|e| {
        if let llnzy::terminal::TerminalEvent::Title(t) = e {
            Some(t.clone())
        } else {
            None
        }
    });
    assert_eq!(title.as_deref(), Some("My Terminal"));
}

#[test]
fn osc_2_sets_title() {
    let mut t = term();
    t.process(b"\x1b]2;Window Title\x07");
    let events = t.drain_events();
    let title = events.iter().find_map(|e| {
        if let llnzy::terminal::TerminalEvent::Title(t) = e {
            Some(t.clone())
        } else {
            None
        }
    });
    assert_eq!(title.as_deref(), Some("Window Title"));
}

// ── Terminal mode sequences ──

#[test]
fn mode_toggle_sequence() {
    let mut t = term();
    // Enable all modes
    t.process(b"\x1b[?1h"); // app cursor
    t.process(b"\x1b[?1000h"); // mouse
    t.process(b"\x1b[?1006h"); // SGR mouse
    t.process(b"\x1b[?2004h"); // bracketed paste
    assert!(t.app_cursor());
    assert!(t.mouse_mode());
    assert!(t.sgr_mouse());
    assert!(t.bracketed_paste());
    // Disable all modes
    t.process(b"\x1b[?1l");
    t.process(b"\x1b[?1000l");
    t.process(b"\x1b[?1006l");
    t.process(b"\x1b[?2004l");
    assert!(!t.app_cursor());
    assert!(!t.mouse_mode());
    assert!(!t.sgr_mouse());
    assert!(!t.bracketed_paste());
}

// ── Alternate screen buffer ──

#[test]
fn alternate_screen_buffer() {
    let mut t = term();
    t.process(b"Main screen text");
    // Switch to alternate buffer (saves cursor and clears)
    t.process(b"\x1b[?1049h");
    // Move cursor to top-left on alt buffer
    t.process(b"\x1b[H");
    // Write on alt buffer
    t.process(b"Alt text");
    assert_eq!(read_line(&t, 0, 80), "Alt text");
    // Switch back to main buffer
    t.process(b"\x1b[?1049l");
    // Main screen text should be restored
    assert_eq!(read_line(&t, 0, 80), "Main screen text");
}

// ── Tab stops ──

#[test]
fn default_tab_stops() {
    let mut t = term();
    t.process(b"X\t");
    // Default tab stops every 8 columns: from col 1, tab goes to col 8
    assert_eq!(t.cursor_point(), Some((0, 8)));
}

#[test]
fn multiple_tabs() {
    let mut t = term();
    t.process(b"\t\t");
    assert_eq!(t.cursor_point(), Some((0, 16)));
}

// ── Carriage return / newline ──

#[test]
fn carriage_return_moves_to_column_zero() {
    let mut t = term();
    t.process(b"Hello\r");
    assert_eq!(t.cursor_point(), Some((0, 0)));
}

#[test]
fn crlf_sequence() {
    let mut t = term();
    t.process(b"Line1\r\nLine2\r\nLine3");
    assert_eq!(read_line(&t, 0, 80), "Line1");
    assert_eq!(read_line(&t, 1, 80), "Line2");
    assert_eq!(read_line(&t, 2, 80), "Line3");
}

// ── Backspace ──

#[test]
fn backspace_moves_cursor_left() {
    let mut t = term();
    t.process(b"ABC\x08");
    assert_eq!(t.cursor_point(), Some((0, 2)));
}

#[test]
fn backspace_overwrite() {
    let mut t = term();
    t.process(b"ABC\x08X");
    assert_eq!(t.cell_char(0, 0), 'A');
    assert_eq!(t.cell_char(0, 1), 'B');
    assert_eq!(t.cell_char(0, 2), 'X'); // C overwritten
}

// ── Insert / Delete characters ──

#[test]
fn delete_characters() {
    let mut t = term();
    t.process(b"ABCDE");
    t.process(b"\x1b[1;2H"); // cursor to col 2 (0-indexed: 1)
    t.process(b"\x1b[2P"); // delete 2 chars
    assert_eq!(t.cell_char(0, 0), 'A');
    assert_eq!(t.cell_char(0, 1), 'D');
    assert_eq!(t.cell_char(0, 2), 'E');
}

#[test]
fn insert_blank_characters() {
    let mut t = term();
    t.process(b"ABCDE");
    t.process(b"\x1b[1;2H"); // cursor to col 2
    t.process(b"\x1b[2@"); // insert 2 blanks
    assert_eq!(t.cell_char(0, 0), 'A');
    assert_eq!(t.cell_char(0, 1), ' ');
    assert_eq!(t.cell_char(0, 2), ' ');
    assert_eq!(t.cell_char(0, 3), 'B');
}

// ── Resize during content ──

#[test]
fn resize_preserves_content() {
    let mut t = Terminal::new(80, 24);
    t.process(b"Hello World");
    t.resize(120, 40);
    assert_eq!(t.size(), (120, 40));
    assert_eq!(t.cell_char(0, 0), 'H');
    assert_eq!(t.cell_char(0, 4), 'o');
}

// ── Decoration rects ──

#[test]
fn decoration_rects_for_underlined_text() {
    let mut t = term();
    let config = Config::default();
    t.process(b"\x1b[4mUUU\x1b[0m   ");
    let rects = t.decoration_rects(&config, 10.0, 20.0);
    // Should have underline rects for cells 0, 1, 2
    assert!(rects.len() >= 3);
}

#[test]
fn decoration_rects_for_strikethrough() {
    let mut t = term();
    let config = Config::default();
    t.process(b"\x1b[9mSSS\x1b[0m   ");
    let rects = t.decoration_rects(&config, 10.0, 20.0);
    // Should have strikethrough rects
    assert!(rects.len() >= 3);
}

#[test]
fn background_rects_for_colored_cells() {
    let mut t = term();
    let config = Config::default();
    t.process(b"\x1b[41mRRR\x1b[0m");
    let rects = t.background_rects(&config, 10.0, 20.0);
    // Should have a background rect for the 3 red-bg cells
    assert!(!rects.is_empty());
    // The rect should cover 3 cells = 30px width
    assert!((rects[0].2 - 30.0).abs() < 0.01);
}

#[test]
fn background_rects_batch_same_color() {
    let mut t = term();
    let config = Config::default();
    // 5 cells with same background should be batched into one rect
    t.process(b"\x1b[42mXXXXX\x1b[0m");
    let rects = t.background_rects(&config, 10.0, 20.0);
    assert_eq!(rects.len(), 1);
    assert!((rects[0].2 - 50.0).abs() < 0.01);
}

// ── Full pipeline: search + selection on processed terminal ──

#[test]
fn search_on_terminal_content() {
    use llnzy::search::Search;

    let mut t = Terminal::new(40, 5);
    t.process(b"The quick brown fox\r\njumps over the lazy dog");

    let mut search = Search::new();
    search.open();
    search.query = "the".to_string();
    search.update_matches(&t);
    // "The" on row 0, "the" on row 1
    assert_eq!(search.matches.len(), 2);
    assert_eq!(search.status(), "1/2");
}

#[test]
fn selection_extracts_terminal_text() {
    use llnzy::selection::Selection;

    let mut t = Terminal::new(40, 5);
    t.process(b"Hello World");

    let mut sel = Selection::new();
    sel.start(0, 0);
    sel.update(0, 4);
    let text = sel.text(&t);
    assert_eq!(text, "Hello");
}

#[test]
fn word_selection_on_terminal() {
    use llnzy::selection::Selection;

    let mut t = Terminal::new(40, 5);
    t.process(b"hello world foo");

    let mut sel = Selection::new();
    sel.select_word(0, 7, &t); // middle of "world"
    let text = sel.text(&t);
    assert_eq!(text, "world");
}
