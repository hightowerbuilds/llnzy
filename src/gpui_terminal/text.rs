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

pub(super) fn terminal_font(config: &Config, mut base_font: Font) -> Font {
    if let Some(font_family) = &config.font_family {
        base_font.family = font_family.clone().into();
    }
    base_font
}

pub(super) fn terminal_row_runs(
    session: &Session,
    config: &Config,
    row: usize,
    cols: usize,
    block_cursor: Option<(usize, usize)>,
    base_font: &Font,
) -> (String, Vec<TextRun>) {
    let mut text = String::new();
    let mut runs = Vec::new();
    let mut current_style: Option<TerminalTextStyle> = None;
    let mut current_len = 0;

    for col in 0..cols {
        let style = terminal_cell_text_style(session, config, row, col, block_cursor);
        let c = display_cell_char(session.terminal.cell_char(row, col));
        let byte_len = c.len_utf8();

        if current_style == Some(style) || current_style.is_none() {
            current_style = Some(style);
            current_len += byte_len;
        } else if let Some(previous_style) = current_style.replace(style) {
            runs.push(text_run(previous_style, current_len, base_font));
            current_len = byte_len;
        }

        text.push(c);
    }

    if let Some(style) = current_style {
        runs.push(text_run(style, current_len, base_font));
    }

    (text, runs)
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
