use rustc_hash::FxHashMap;

use super::model::ColorScheme;
use crate::editor::syntax::HighlightGroup;

use super::model::EditorColors;
use crate::ui_theme::UiMode;

/// One editor theme: the surface colors the code area paints plus the
/// syntax palette drawn on top of them. Independent of the terminal scheme.
#[derive(Clone, Copy, Debug)]
pub(crate) struct EditorTheme {
    pub(crate) name: &'static str,
    pub(crate) mode: UiMode,
    pub(crate) colors: EditorColors,
    pub(crate) syntax: &'static [(HighlightGroup, [u8; 3])],
}

impl EditorTheme {
    pub(crate) fn colors_map(&self) -> FxHashMap<HighlightGroup, [u8; 3]> {
        self.syntax.iter().copied().collect()
    }

    pub(crate) fn swatch(&self) -> [[u8; 3]; 6] {
        [
            self.colors.background,
            syntax_color(self.syntax, HighlightGroup::Keyword),
            syntax_color(self.syntax, HighlightGroup::Function),
            syntax_color(self.syntax, HighlightGroup::String),
            syntax_color(self.syntax, HighlightGroup::Type),
            syntax_color(self.syntax, HighlightGroup::Comment),
        ]
    }

    pub(crate) fn matches_colors(&self, colors: &FxHashMap<HighlightGroup, [u8; 3]>) -> bool {
        colors.len() == self.syntax.len()
            && self
                .syntax
                .iter()
                .all(|(group, color)| colors.get(group) == Some(color))
    }
}

pub(crate) fn editor_themes() -> &'static [EditorTheme] {
    EDITOR_THEMES
}

pub(crate) fn editor_theme(name: &str) -> Option<EditorTheme> {
    let normalized = normalize_preset_name(name);
    EDITOR_THEMES
        .iter()
        .copied()
        .find(|theme| normalize_preset_name(theme.name) == normalized)
}

fn normalize_preset_name(name: &str) -> String {
    name.trim().to_ascii_lowercase().replace([' ', '_'], "-")
}

fn syntax_color(colors: &[(HighlightGroup, [u8; 3])], group: HighlightGroup) -> [u8; 3] {
    colors
        .iter()
        .find_map(|(candidate, color)| (*candidate == group).then_some(*color))
        .unwrap_or([0xAB, 0xB2, 0xBF])
}

const fn editor_colors(
    background: [u8; 3],
    foreground: [u8; 3],
    cursor: [u8; 3],
    selection: [u8; 3],
) -> EditorColors {
    EditorColors {
        background,
        foreground,
        cursor,
        selection,
        selection_alpha: 0.35,
    }
}

/// Six editor themes: three dark, then three light. Deliberately short — a
/// code palette is picked once and then lived in, so a long list is a menu
/// to scroll past rather than a choice worth making.
const EDITOR_THEMES: &[EditorTheme] = &[
    EditorTheme {
        name: "One Dark",
        mode: UiMode::Dark,
        colors: editor_colors(
            [0x28, 0x2C, 0x34],
            [0xAB, 0xB2, 0xBF],
            [0x52, 0x8B, 0xFF],
            [0x3E, 0x44, 0x51],
        ),
        syntax: &[
            (HighlightGroup::Keyword, [198, 120, 221]),
            (HighlightGroup::Type, [86, 182, 194]),
            (HighlightGroup::Function, [97, 175, 239]),
            (HighlightGroup::Variable, [224, 108, 117]),
            (HighlightGroup::String, [152, 195, 121]),
            (HighlightGroup::Number, [209, 154, 102]),
            (HighlightGroup::Comment, [92, 99, 112]),
            (HighlightGroup::Operator, [171, 178, 191]),
            (HighlightGroup::Punctuation, [130, 137, 151]),
            (HighlightGroup::Constant, [209, 154, 102]),
            (HighlightGroup::Attribute, [229, 192, 123]),
            (HighlightGroup::Tag, [224, 108, 117]),
            (HighlightGroup::Property, [224, 108, 117]),
            (HighlightGroup::Escape, [86, 182, 194]),
            (HighlightGroup::Label, [229, 192, 123]),
            (HighlightGroup::Module, [86, 182, 194]),
        ],
    },
    EditorTheme {
        name: "Dracula",
        mode: UiMode::Dark,
        colors: editor_colors(
            [0x28, 0x2A, 0x36],
            [0xF8, 0xF8, 0xF2],
            [0xF8, 0xF8, 0xF2],
            [0x44, 0x47, 0x5A],
        ),
        syntax: &[
            (HighlightGroup::Keyword, [0xBD, 0x93, 0xF9]),
            (HighlightGroup::Type, [0x8B, 0xE9, 0xFD]),
            (HighlightGroup::Function, [0x50, 0xFA, 0x7B]),
            (HighlightGroup::Variable, [0xF8, 0xF8, 0xF2]),
            (HighlightGroup::String, [0xF1, 0xFA, 0x8C]),
            (HighlightGroup::Number, [0xBD, 0x93, 0xF9]),
            (HighlightGroup::Comment, [0x62, 0x72, 0xA4]),
            (HighlightGroup::Operator, [0xFF, 0x79, 0xC6]),
            (HighlightGroup::Punctuation, [0xF8, 0xF8, 0xF2]),
            (HighlightGroup::Constant, [0xBD, 0x93, 0xF9]),
            (HighlightGroup::Attribute, [0x50, 0xFA, 0x7B]),
            (HighlightGroup::Tag, [0xFF, 0x79, 0xC6]),
            (HighlightGroup::Property, [0x8B, 0xE9, 0xFD]),
            (HighlightGroup::Escape, [0xFF, 0x79, 0xC6]),
            (HighlightGroup::Label, [0xF1, 0xFA, 0x8C]),
            (HighlightGroup::Module, [0x8B, 0xE9, 0xFD]),
        ],
    },
    EditorTheme {
        name: "Nord",
        mode: UiMode::Dark,
        colors: editor_colors(
            [0x2E, 0x34, 0x40],
            [0xD8, 0xDE, 0xE9],
            [0xD8, 0xDE, 0xE9],
            [0x43, 0x4C, 0x5E],
        ),
        syntax: &[
            (HighlightGroup::Keyword, [0x81, 0xA1, 0xC1]),
            (HighlightGroup::Type, [0x8F, 0xBC, 0xBB]),
            (HighlightGroup::Function, [0x88, 0xC0, 0xD0]),
            (HighlightGroup::Variable, [0xD8, 0xDE, 0xE9]),
            (HighlightGroup::String, [0xA3, 0xBE, 0x8C]),
            (HighlightGroup::Number, [0xB4, 0x8E, 0xAD]),
            (HighlightGroup::Comment, [0x61, 0x6E, 0x88]),
            (HighlightGroup::Operator, [0x81, 0xA1, 0xC1]),
            (HighlightGroup::Punctuation, [0xEC, 0xEF, 0xF4]),
            (HighlightGroup::Constant, [0xB4, 0x8E, 0xAD]),
            (HighlightGroup::Attribute, [0xD0, 0x87, 0x70]),
            (HighlightGroup::Tag, [0x81, 0xA1, 0xC1]),
            (HighlightGroup::Property, [0x8F, 0xBC, 0xBB]),
            (HighlightGroup::Escape, [0xEB, 0xCB, 0x8B]),
            (HighlightGroup::Label, [0xEB, 0xCB, 0x8B]),
            (HighlightGroup::Module, [0x8F, 0xBC, 0xBB]),
        ],
    },
    // Mirrors One Dark group for group, in the Atom One Light hues, so
    // switching between them moves the background without moving where each
    // kind of token sits in the palette.
    EditorTheme {
        name: "One Light",
        mode: UiMode::Light,
        colors: editor_colors(
            [0xFA, 0xFA, 0xFA],
            [0x38, 0x3A, 0x42],
            [0x52, 0x6F, 0xFF],
            [0xD7, 0xDA, 0xE0],
        ),
        syntax: &[
            (HighlightGroup::Keyword, [0xA6, 0x26, 0xA4]),
            (HighlightGroup::Type, [0x01, 0x84, 0xBC]),
            (HighlightGroup::Function, [0x40, 0x78, 0xF2]),
            (HighlightGroup::Variable, [0xE4, 0x56, 0x49]),
            (HighlightGroup::String, [0x50, 0xA1, 0x4F]),
            (HighlightGroup::Number, [0x98, 0x68, 0x01]),
            (HighlightGroup::Comment, [0xA0, 0xA1, 0xA7]),
            (HighlightGroup::Operator, [0x38, 0x3A, 0x42]),
            (HighlightGroup::Punctuation, [0x69, 0x6C, 0x77]),
            (HighlightGroup::Constant, [0x98, 0x68, 0x01]),
            (HighlightGroup::Attribute, [0xC1, 0x84, 0x01]),
            (HighlightGroup::Tag, [0xE4, 0x56, 0x49]),
            (HighlightGroup::Property, [0xE4, 0x56, 0x49]),
            (HighlightGroup::Escape, [0x01, 0x84, 0xBC]),
            (HighlightGroup::Label, [0xC1, 0x84, 0x01]),
            (HighlightGroup::Module, [0x01, 0x84, 0xBC]),
        ],
    },
    EditorTheme {
        name: "Solarized Light",
        mode: UiMode::Light,
        colors: editor_colors(
            [0xFD, 0xF6, 0xE3],
            [0x58, 0x6E, 0x75],
            [0x58, 0x6E, 0x75],
            [0xE3, 0xDC, 0xC6],
        ),
        syntax: &[
            (HighlightGroup::Keyword, [0x85, 0x99, 0x00]),
            (HighlightGroup::Type, [0xB5, 0x89, 0x00]),
            (HighlightGroup::Function, [0x26, 0x8B, 0xD2]),
            (HighlightGroup::Variable, [0x26, 0x8B, 0xD2]),
            (HighlightGroup::String, [0x2A, 0xA1, 0x98]),
            (HighlightGroup::Number, [0xD3, 0x36, 0x82]),
            (HighlightGroup::Comment, [0x93, 0xA1, 0xA1]),
            (HighlightGroup::Operator, [0x85, 0x99, 0x00]),
            (HighlightGroup::Punctuation, [0x65, 0x7B, 0x83]),
            (HighlightGroup::Constant, [0xCB, 0x4B, 0x16]),
            (HighlightGroup::Attribute, [0xB5, 0x89, 0x00]),
            (HighlightGroup::Tag, [0x26, 0x8B, 0xD2]),
            (HighlightGroup::Property, [0x26, 0x8B, 0xD2]),
            (HighlightGroup::Escape, [0xDC, 0x32, 0x2F]),
            (HighlightGroup::Label, [0xB5, 0x89, 0x00]),
            (HighlightGroup::Module, [0xB5, 0x89, 0x00]),
        ],
    },
    EditorTheme {
        name: "GitHub Light",
        mode: UiMode::Light,
        colors: editor_colors(
            [0xFF, 0xFF, 0xFF],
            [0x24, 0x29, 0x2F],
            [0x09, 0x69, 0xDA],
            [0xB6, 0xD6, 0xFC],
        ),
        syntax: &[
            (HighlightGroup::Keyword, [0xCF, 0x22, 0x2E]),
            (HighlightGroup::Type, [0x95, 0x38, 0x00]),
            (HighlightGroup::Function, [0x82, 0x50, 0xDF]),
            (HighlightGroup::Variable, [0x24, 0x29, 0x2F]),
            (HighlightGroup::String, [0x0A, 0x30, 0x69]),
            (HighlightGroup::Number, [0x05, 0x50, 0xAE]),
            (HighlightGroup::Comment, [0x6E, 0x77, 0x81]),
            (HighlightGroup::Operator, [0xCF, 0x22, 0x2E]),
            (HighlightGroup::Punctuation, [0x24, 0x29, 0x2F]),
            (HighlightGroup::Constant, [0x05, 0x50, 0xAE]),
            (HighlightGroup::Attribute, [0x11, 0x63, 0x29]),
            (HighlightGroup::Tag, [0x11, 0x63, 0x29]),
            (HighlightGroup::Property, [0x05, 0x50, 0xAE]),
            (HighlightGroup::Escape, [0x0A, 0x30, 0x69]),
            (HighlightGroup::Label, [0x95, 0x38, 0x00]),
            (HighlightGroup::Module, [0x95, 0x38, 0x00]),
        ],
    },
];

pub(super) fn preset_scheme(name: &str) -> Option<ColorScheme> {
    let (ansi, fg, bg, cur, sel) = match name.to_lowercase().as_str() {
        "dracula" => (
            [
                [0x21, 0x22, 0x2C],
                [0xFF, 0x55, 0x55],
                [0x50, 0xFA, 0x7B],
                [0xF1, 0xFA, 0x8C],
                [0xBD, 0x93, 0xF9],
                [0xFF, 0x79, 0xC6],
                [0x8B, 0xE9, 0xFD],
                [0xF8, 0xF8, 0xF2],
                [0x62, 0x72, 0xA4],
                [0xFF, 0x6E, 0x6E],
                [0x69, 0xFF, 0x94],
                [0xFF, 0xFF, 0xA5],
                [0xD6, 0xAC, 0xFF],
                [0xFF, 0x92, 0xDF],
                [0xA4, 0xFF, 0xFF],
                [0xFF, 0xFF, 0xFF],
            ],
            [0xF8, 0xF8, 0xF2],
            [0x28, 0x2A, 0x36],
            [0xF8, 0xF8, 0xF2],
            [0x44, 0x47, 0x5A],
        ),
        "nord" => (
            [
                [0x3B, 0x42, 0x52],
                [0xBF, 0x61, 0x6A],
                [0xA3, 0xBE, 0x8C],
                [0xEB, 0xCB, 0x8B],
                [0x81, 0xA1, 0xC1],
                [0xB4, 0x8E, 0xAD],
                [0x88, 0xC0, 0xD0],
                [0xE5, 0xE9, 0xF0],
                [0x4C, 0x56, 0x6A],
                [0xBF, 0x61, 0x6A],
                [0xA3, 0xBE, 0x8C],
                [0xEB, 0xCB, 0x8B],
                [0x81, 0xA1, 0xC1],
                [0xB4, 0x8E, 0xAD],
                [0x8F, 0xBC, 0xBB],
                [0xEC, 0xEF, 0xF4],
            ],
            [0xD8, 0xDE, 0xE9],
            [0x2E, 0x34, 0x40],
            [0xD8, 0xDE, 0xE9],
            [0x43, 0x4C, 0x5E],
        ),
        "one-dark" | "onedark" => (
            [
                [0x28, 0x2C, 0x34],
                [0xE0, 0x6C, 0x75],
                [0x98, 0xC3, 0x79],
                [0xE5, 0xC0, 0x7B],
                [0x61, 0xAF, 0xEF],
                [0xC6, 0x78, 0xDD],
                [0x56, 0xB6, 0xC2],
                [0xAB, 0xB2, 0xBF],
                [0x54, 0x58, 0x62],
                [0xE0, 0x6C, 0x75],
                [0x98, 0xC3, 0x79],
                [0xE5, 0xC0, 0x7B],
                [0x61, 0xAF, 0xEF],
                [0xC6, 0x78, 0xDD],
                [0x56, 0xB6, 0xC2],
                [0xFF, 0xFF, 0xFF],
            ],
            [0xAB, 0xB2, 0xBF],
            [0x28, 0x2C, 0x34],
            [0x52, 0x8B, 0xFF],
            [0x3E, 0x44, 0x51],
        ),
        "solarized-dark" | "solarized" => (
            [
                [0x07, 0x36, 0x42],
                [0xDC, 0x32, 0x2F],
                [0x85, 0x99, 0x00],
                [0xB5, 0x89, 0x00],
                [0x26, 0x8B, 0xD2],
                [0xD3, 0x36, 0x82],
                [0x2A, 0xA1, 0x98],
                [0xEE, 0xE8, 0xD5],
                [0x00, 0x2B, 0x36],
                [0xCB, 0x4B, 0x16],
                [0x58, 0x6E, 0x75],
                [0x65, 0x7B, 0x83],
                [0x83, 0x94, 0x96],
                [0x6C, 0x71, 0xC4],
                [0x93, 0xA1, 0xA1],
                [0xFD, 0xF6, 0xE3],
            ],
            [0x83, 0x94, 0x96],
            [0x00, 0x2B, 0x36],
            [0x83, 0x94, 0x96],
            [0x07, 0x36, 0x42],
        ),
        "monokai" => (
            [
                [0x27, 0x28, 0x22],
                [0xF9, 0x26, 0x72],
                [0xA6, 0xE2, 0x2E],
                [0xF4, 0xBF, 0x75],
                [0x66, 0xD9, 0xEF],
                [0xAE, 0x81, 0xFF],
                [0xA1, 0xEF, 0xE4],
                [0xF8, 0xF8, 0xF2],
                [0x75, 0x71, 0x5E],
                [0xF9, 0x26, 0x72],
                [0xA6, 0xE2, 0x2E],
                [0xF4, 0xBF, 0x75],
                [0x66, 0xD9, 0xEF],
                [0xAE, 0x81, 0xFF],
                [0xA1, 0xEF, 0xE4],
                [0xF9, 0xF8, 0xF5],
            ],
            [0xF8, 0xF8, 0xF2],
            [0x27, 0x28, 0x22],
            [0xF8, 0xF8, 0xF2],
            [0x49, 0x48, 0x3E],
        ),
        _ => return None,
    };
    Some(ColorScheme {
        ansi,
        foreground: fg,
        background: bg,
        cursor: cur,
        selection: sel,
        selection_alpha: 0.4,
    })
}
