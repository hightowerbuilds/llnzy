use rustc_hash::FxHashMap;

use super::model::ColorScheme;
use crate::editor::syntax::HighlightGroup;

#[derive(Clone, Copy, Debug)]
pub(crate) struct EditorSyntaxPreset {
    pub(crate) name: &'static str,
    pub(crate) colors: &'static [(HighlightGroup, [u8; 3])],
}

impl EditorSyntaxPreset {
    pub(crate) fn colors_map(&self) -> FxHashMap<HighlightGroup, [u8; 3]> {
        self.colors.iter().copied().collect()
    }

    pub(crate) fn swatch(&self) -> [[u8; 3]; 6] {
        [
            syntax_color(self.colors, HighlightGroup::Keyword),
            syntax_color(self.colors, HighlightGroup::Function),
            syntax_color(self.colors, HighlightGroup::String),
            syntax_color(self.colors, HighlightGroup::Number),
            syntax_color(self.colors, HighlightGroup::Type),
            syntax_color(self.colors, HighlightGroup::Comment),
        ]
    }

    pub(crate) fn matches_colors(&self, colors: &FxHashMap<HighlightGroup, [u8; 3]>) -> bool {
        colors.len() == self.colors.len()
            && self
                .colors
                .iter()
                .all(|(group, color)| colors.get(group) == Some(color))
    }
}

pub(crate) fn editor_syntax_presets() -> &'static [EditorSyntaxPreset] {
    EDITOR_SYNTAX_PRESETS
}

pub(crate) fn editor_syntax_preset(name: &str) -> Option<EditorSyntaxPreset> {
    let normalized = normalize_preset_name(name);
    EDITOR_SYNTAX_PRESETS
        .iter()
        .copied()
        .find(|preset| normalize_preset_name(preset.name) == normalized)
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

const EDITOR_SYNTAX_PRESETS: &[EditorSyntaxPreset] = &[
    EditorSyntaxPreset {
        name: "One Dark",
        colors: &[
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
    EditorSyntaxPreset {
        name: "Dracula",
        colors: &[
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
    EditorSyntaxPreset {
        name: "Monokai",
        colors: &[
            (HighlightGroup::Keyword, [0xF9, 0x26, 0x72]),
            (HighlightGroup::Type, [0x66, 0xD9, 0xEF]),
            (HighlightGroup::Function, [0xA6, 0xE2, 0x2E]),
            (HighlightGroup::Variable, [0xF8, 0xF8, 0xF2]),
            (HighlightGroup::String, [0xE6, 0xDB, 0x74]),
            (HighlightGroup::Number, [0xAE, 0x81, 0xFF]),
            (HighlightGroup::Comment, [0x75, 0x71, 0x5E]),
            (HighlightGroup::Operator, [0xF9, 0x26, 0x72]),
            (HighlightGroup::Punctuation, [0xF8, 0xF8, 0xF2]),
            (HighlightGroup::Constant, [0xAE, 0x81, 0xFF]),
            (HighlightGroup::Attribute, [0xA6, 0xE2, 0x2E]),
            (HighlightGroup::Tag, [0xF9, 0x26, 0x72]),
            (HighlightGroup::Property, [0x66, 0xD9, 0xEF]),
            (HighlightGroup::Escape, [0xAE, 0x81, 0xFF]),
            (HighlightGroup::Label, [0xE6, 0xDB, 0x74]),
            (HighlightGroup::Module, [0x66, 0xD9, 0xEF]),
        ],
    },
    EditorSyntaxPreset {
        name: "Nord",
        colors: &[
            (HighlightGroup::Keyword, [0x81, 0xA1, 0xC1]),
            (HighlightGroup::Type, [0x8F, 0xBC, 0xBB]),
            (HighlightGroup::Function, [0x88, 0xC0, 0xD0]),
            (HighlightGroup::Variable, [0xD8, 0xDE, 0xE9]),
            (HighlightGroup::String, [0xA3, 0xBE, 0x8C]),
            (HighlightGroup::Number, [0xB4, 0x8E, 0xAD]),
            (HighlightGroup::Comment, [0x61, 0x6E, 0x88]),
            (HighlightGroup::Operator, [0x81, 0xA1, 0xC1]),
            (HighlightGroup::Punctuation, [0xD8, 0xDE, 0xE9]),
            (HighlightGroup::Constant, [0xB4, 0x8E, 0xAD]),
            (HighlightGroup::Attribute, [0xEB, 0xCB, 0x8B]),
            (HighlightGroup::Tag, [0x81, 0xA1, 0xC1]),
            (HighlightGroup::Property, [0xD8, 0xDE, 0xE9]),
            (HighlightGroup::Escape, [0x88, 0xC0, 0xD0]),
            (HighlightGroup::Label, [0xEB, 0xCB, 0x8B]),
            (HighlightGroup::Module, [0x8F, 0xBC, 0xBB]),
        ],
    },
    EditorSyntaxPreset {
        name: "Solarized Dark",
        colors: &[
            (HighlightGroup::Keyword, [0x85, 0x99, 0x00]),
            (HighlightGroup::Type, [0xB5, 0x89, 0x00]),
            (HighlightGroup::Function, [0x26, 0x8B, 0xD2]),
            (HighlightGroup::Variable, [0x83, 0x94, 0x96]),
            (HighlightGroup::String, [0x2A, 0xA1, 0x98]),
            (HighlightGroup::Number, [0xD3, 0x36, 0x82]),
            (HighlightGroup::Comment, [0x58, 0x6E, 0x75]),
            (HighlightGroup::Operator, [0x93, 0xA1, 0xA1]),
            (HighlightGroup::Punctuation, [0x83, 0x94, 0x96]),
            (HighlightGroup::Constant, [0xD3, 0x36, 0x82]),
            (HighlightGroup::Attribute, [0xB5, 0x89, 0x00]),
            (HighlightGroup::Tag, [0xCB, 0x4B, 0x16]),
            (HighlightGroup::Property, [0x26, 0x8B, 0xD2]),
            (HighlightGroup::Escape, [0x2A, 0xA1, 0x98]),
            (HighlightGroup::Label, [0xB5, 0x89, 0x00]),
            (HighlightGroup::Module, [0x26, 0x8B, 0xD2]),
        ],
    },
    EditorSyntaxPreset {
        name: "GitHub Dark",
        colors: &[
            (HighlightGroup::Keyword, [0xFF, 0x7B, 0x72]),
            (HighlightGroup::Type, [0xFF, 0xD8, 0x66]),
            (HighlightGroup::Function, [0xD2, 0xA8, 0xFF]),
            (HighlightGroup::Variable, [0xC9, 0xD1, 0xD9]),
            (HighlightGroup::String, [0xA5, 0xD6, 0xFF]),
            (HighlightGroup::Number, [0x79, 0xC0, 0xFF]),
            (HighlightGroup::Comment, [0x8B, 0x94, 0x9E]),
            (HighlightGroup::Operator, [0xFF, 0x7B, 0x72]),
            (HighlightGroup::Punctuation, [0xC9, 0xD1, 0xD9]),
            (HighlightGroup::Constant, [0x79, 0xC0, 0xFF]),
            (HighlightGroup::Attribute, [0x7E, 0xC7, 0xFF]),
            (HighlightGroup::Tag, [0x7E, 0xC7, 0xFF]),
            (HighlightGroup::Property, [0x79, 0xC0, 0xFF]),
            (HighlightGroup::Escape, [0xA5, 0xD6, 0xFF]),
            (HighlightGroup::Label, [0xFF, 0xD8, 0x66]),
            (HighlightGroup::Module, [0x7E, 0xC7, 0xFF]),
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
