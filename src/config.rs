use serde::Deserialize;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CursorStyle {
    Block,
    Beam,
    Underline,
}

#[derive(Clone)]
pub struct Config {
    pub font_size: f32,
    pub cols: u16,
    pub rows: u16,
    pub shell: String,
    pub bg: [f32; 4],
    pub fg: [u8; 3],
    pub cursor_color: [u8; 3],
    pub cursor_style: CursorStyle,
}

impl Default for Config {
    fn default() -> Self {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        Self {
            font_size: 16.0,
            cols: 80,
            rows: 24,
            shell,
            bg: [0.12, 0.12, 0.14, 1.0],
            fg: [204, 204, 204],
            cursor_color: [204, 204, 204],
            cursor_style: CursorStyle::Block,
        }
    }
}

impl Config {
    /// Load config from ~/.config/llnzy/config.toml, falling back to defaults.
    pub fn load() -> Self {
        let mut config = Self::default();

        let Some(config_dir) = dirs::config_dir() else {
            return config;
        };

        let path = config_dir.join("llnzy").join("config.toml");
        let Ok(content) = std::fs::read_to_string(&path) else {
            return config;
        };

        let Ok(file) = toml::from_str::<ConfigFile>(&content) else {
            log::warn!("Failed to parse {}", path.display());
            return config;
        };

        // Apply overrides
        if let Some(font) = file.font {
            if let Some(size) = font.size {
                config.font_size = size;
            }
        }

        if let Some(colors) = file.colors {
            if let Some(fg) = colors.foreground.and_then(|s| parse_hex(&s)) {
                config.fg = fg;
            }
            if let Some(bg) = colors.background.and_then(|s| parse_hex(&s)) {
                config.bg = [bg[0] as f32 / 255.0, bg[1] as f32 / 255.0, bg[2] as f32 / 255.0, 1.0];
            }
            if let Some(c) = colors.cursor.and_then(|s| parse_hex(&s)) {
                config.cursor_color = c;
            }
        }

        if let Some(cursor) = file.cursor {
            if let Some(style) = cursor.style {
                config.cursor_style = match style.as_str() {
                    "beam" | "bar" => CursorStyle::Beam,
                    "underline" => CursorStyle::Underline,
                    _ => CursorStyle::Block,
                };
            }
        }

        if let Some(shell) = file.shell {
            if let Some(program) = shell.program {
                config.shell = program;
            }
        }

        config
    }
}

// --- TOML schema ---

#[derive(Deserialize)]
struct ConfigFile {
    font: Option<FontConfig>,
    colors: Option<ColorConfig>,
    cursor: Option<CursorConfig>,
    shell: Option<ShellConfig>,
}

#[derive(Deserialize)]
struct FontConfig {
    size: Option<f32>,
}

#[derive(Deserialize)]
struct ColorConfig {
    foreground: Option<String>,
    background: Option<String>,
    cursor: Option<String>,
}

#[derive(Deserialize)]
struct CursorConfig {
    style: Option<String>,
}

#[derive(Deserialize)]
struct ShellConfig {
    program: Option<String>,
}

// --- Helpers ---

fn parse_hex(hex: &str) -> Option<[u8; 3]> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([r, g, b])
}

/// Resolve a 256-color palette index to RGB.
pub fn indexed_color(idx: u8) -> [u8; 3] {
    match idx {
        0 => [0, 0, 0],
        1 => [170, 0, 0],
        2 => [0, 170, 0],
        3 => [170, 85, 0],
        4 => [0, 0, 170],
        5 => [170, 0, 170],
        6 => [0, 170, 170],
        7 => [170, 170, 170],
        8 => [85, 85, 85],
        9 => [255, 85, 85],
        10 => [85, 255, 85],
        11 => [255, 255, 85],
        12 => [85, 85, 255],
        13 => [255, 85, 255],
        14 => [85, 255, 255],
        15 => [255, 255, 255],
        16..=231 => {
            let idx = idx - 16;
            let r = idx / 36;
            let g = (idx % 36) / 6;
            let b = idx % 6;
            let to_val = |v: u8| if v == 0 { 0 } else { 55 + 40 * v };
            [to_val(r), to_val(g), to_val(b)]
        }
        232..=255 => {
            let v = 8 + 10 * (idx - 232);
            [v, v, v]
        }
    }
}
