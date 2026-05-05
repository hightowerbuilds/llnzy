use alacritty_terminal::vte::ansi::{Color as AnsiColor, NamedColor};

use crate::config::{indexed_color, Config};

pub(super) fn resolve_color(color: &AnsiColor, config: &Config, is_fg: bool) -> [u8; 3] {
    match color {
        AnsiColor::Named(named) => resolve_named(*named, config, is_fg),
        AnsiColor::Spec(rgb) => [rgb.r, rgb.g, rgb.b],
        AnsiColor::Indexed(idx) => indexed_color(*idx, &config.colors),
    }
}

fn resolve_named(named: NamedColor, config: &Config, is_fg: bool) -> [u8; 3] {
    let scheme = &config.colors;
    match named {
        NamedColor::Black => scheme.ansi[0],
        NamedColor::Red => scheme.ansi[1],
        NamedColor::Green => scheme.ansi[2],
        NamedColor::Yellow => scheme.ansi[3],
        NamedColor::Blue => scheme.ansi[4],
        NamedColor::Magenta => scheme.ansi[5],
        NamedColor::Cyan => scheme.ansi[6],
        NamedColor::White => scheme.ansi[7],
        NamedColor::BrightBlack => scheme.ansi[8],
        NamedColor::BrightRed => scheme.ansi[9],
        NamedColor::BrightGreen => scheme.ansi[10],
        NamedColor::BrightYellow => scheme.ansi[11],
        NamedColor::BrightBlue => scheme.ansi[12],
        NamedColor::BrightMagenta => scheme.ansi[13],
        NamedColor::BrightCyan => scheme.ansi[14],
        NamedColor::BrightWhite => scheme.ansi[15],
        NamedColor::Foreground => {
            if is_fg {
                scheme.foreground
            } else {
                scheme.background
            }
        }
        NamedColor::Background => {
            if is_fg {
                scheme.foreground
            } else {
                scheme.background
            }
        }
        _ => {
            if is_fg {
                scheme.foreground
            } else {
                scheme.background
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_named_ansi_colors() {
        let config = Config::default();
        assert_eq!(
            resolve_named(NamedColor::Red, &config, true),
            config.colors.ansi[1]
        );
        assert_eq!(
            resolve_named(NamedColor::Blue, &config, true),
            config.colors.ansi[4]
        );
        assert_eq!(
            resolve_named(NamedColor::BrightWhite, &config, true),
            config.colors.ansi[15]
        );
    }

    #[test]
    fn resolve_named_foreground() {
        let config = Config::default();
        assert_eq!(
            resolve_named(NamedColor::Foreground, &config, true),
            config.colors.foreground
        );
        assert_eq!(
            resolve_named(NamedColor::Foreground, &config, false),
            config.colors.background
        );
    }

    #[test]
    fn resolve_color_spec_rgb() {
        let config = Config::default();
        let rgb = alacritty_terminal::vte::ansi::Rgb {
            r: 100,
            g: 150,
            b: 200,
        };
        let color = AnsiColor::Spec(rgb);
        assert_eq!(resolve_color(&color, &config, true), [100, 150, 200]);
    }

    #[test]
    fn resolve_color_indexed() {
        let config = Config::default();
        let color = AnsiColor::Indexed(1);
        assert_eq!(resolve_color(&color, &config, true), config.colors.ansi[1]);
    }
}
