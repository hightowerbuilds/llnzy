use alacritty_terminal::term::cell::Flags;
use gpui::{rgb, Font, FontStyle, FontWeight, TextRun};

use super::effects::rgb_u32;
use crate::config::Config;
use crate::session::Session;

#[derive(Clone, Copy, PartialEq, Eq)]
struct TerminalTextStyle {
    fg: [u8; 3],
    bold: bool,
    italic: bool,
}

/// One glyph + its grid column. The terminal renders these per-cell instead
/// of handing whole rows to `shape_text`, because alacritty's grid is
/// strictly column-aligned and any per-glyph drift inside a multi-character
/// `shape_text` call (ligatures, kerning, fallback metrics, subpixel
/// rounding) compounds across the row and breaks cursor placement, selection
/// rectangles, click hit-testing, and ASCII/TUI box drawing.
pub(super) struct TerminalCellGlyph {
    pub(super) col: usize,
    pub(super) ch: char,
    pub(super) run: TextRun,
}

pub(super) fn terminal_font(config: &Config, mut base_font: Font) -> Font {
    if let Some(font_family) = &config.font_family {
        base_font.family = font_family.clone().into();
    }
    // Programming ligatures (`=>`, `->`, `!=`, `==`, etc.) collapse multiple
    // characters into a single glyph whose advance does not equal the sum of
    // per-cell advances. Terminal grids rely on every column being the same
    // width, so ligatures break cursor positioning, selection rectangles,
    // and click-to-grid hit testing the moment any ligaturable sequence
    // appears in the output. Disable `calt` for both rendering and metric
    // measurement.
    base_font.features = gpui::FontFeatures::disable_ligatures();
    base_font
}

/// Collect every visible glyph in `row` along with its grid column. Empty
/// (space) cells with the default style are omitted: backgrounds are painted
/// separately via `Terminal::background_rects`, so an empty cell does not
/// need a glyph at all. Wide-character spacer cells are also skipped because
/// the preceding cell holds the wide glyph and its natural advance carries
/// it across the spacer column.
pub(super) fn terminal_row_glyphs(
    session: &Session,
    config: &Config,
    row: usize,
    cols: usize,
    block_cursor: Option<(usize, usize)>,
    base_font: &Font,
) -> Vec<TerminalCellGlyph> {
    let mut glyphs = Vec::new();

    for col in 0..cols {
        let flags = session.terminal.cell_flags(row, col);
        if flags.contains(Flags::WIDE_CHAR_SPACER)
            || flags.contains(Flags::LEADING_WIDE_CHAR_SPACER)
        {
            continue;
        }

        let raw = session.terminal.cell_char(row, col);
        let is_cursor_cell = block_cursor == Some((row, col));
        let style = terminal_cell_text_style(session, config, row, col, block_cursor);

        // Skip invisible cells: blank glyph, default foreground, no decoration.
        // The block cursor still needs a rendered glyph (it inverts the fg) so
        // we keep cells underneath the cursor even when they hold a space.
        if !is_cursor_cell && (raw == ' ' || raw == '\0') {
            continue;
        }

        let ch = display_cell_char(raw);
        let run = text_run(style, ch.len_utf8(), base_font);
        glyphs.push(TerminalCellGlyph { col, ch, run });
    }

    glyphs
}

fn terminal_cell_text_style(
    session: &Session,
    config: &Config,
    row: usize,
    col: usize,
    block_cursor: Option<(usize, usize)>,
) -> TerminalTextStyle {
    let flags = session.terminal.cell_flags(row, col);
    let is_block_cursor = block_cursor == Some((row, col));
    let mut fg = if is_block_cursor {
        config.colors.background
    } else {
        session.terminal.resolve_fg_with_attrs(row, col, config)
    };

    if flags.contains(Flags::DIM) && !is_block_cursor {
        fg = [
            (fg[0] as u16 * 2 / 3) as u8,
            (fg[1] as u16 * 2 / 3) as u8,
            (fg[2] as u16 * 2 / 3) as u8,
        ];
    }
    if flags.contains(Flags::HIDDEN) && !is_block_cursor {
        fg = session.terminal.resolve_bg_with_attrs(row, col, config);
    }

    TerminalTextStyle {
        fg,
        bold: flags.contains(Flags::BOLD),
        italic: flags.contains(Flags::ITALIC),
    }
}

fn text_run(style: TerminalTextStyle, len: usize, base_font: &Font) -> TextRun {
    let mut font = base_font.clone();
    if style.bold {
        font.weight = FontWeight::BOLD;
    }
    if style.italic {
        font.style = FontStyle::Italic;
    }

    TextRun {
        len,
        font,
        color: rgb(rgb_u32(style.fg)).into(),
        background_color: None,
        underline: None,
        strikethrough: None,
    }
}

fn display_cell_char(c: char) -> char {
    if c == '\0' {
        ' '
    } else {
        c
    }
}

pub(super) fn terminal_paste_payload(text: &str, bracketed: bool) -> Vec<u8> {
    if !bracketed {
        return text.as_bytes().to_vec();
    }

    let mut bytes = Vec::with_capacity(text.len() + 12);
    bytes.extend_from_slice(b"\x1b[200~");
    bytes.extend_from_slice(text.as_bytes());
    bytes.extend_from_slice(b"\x1b[201~");
    bytes
}
