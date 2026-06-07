use gpui::prelude::*;
use gpui::{div, px, rgb, Context};

use crate::config::Config;
use crate::gpui_workspace::{WorkspacePrototype, BORDER};

use super::widgets::{appearance_button, color_strip, control_label, metric_row, palette_band};

// Curated palettes for shader-driven background effects. Each preset sets
// the three palette stops the active shader reads from
// `Config.effects.background_color{1,2,3}`. The list shown to the user is
// filtered per effect (`shader_palettes_for_mode`) so each effect surfaces
// palettes that suit it.
type ColorStop = [u8; 3];
pub(super) type ShaderPalette = (&'static str, ColorStop, ColorStop, ColorStop);

pub(super) const SMOKE_PALETTES: &[ShaderPalette] = &[
    (
        "Mauve",
        [0x10, 0x09, 0x14],
        [0x4d, 0x1f, 0x4f],
        [0xc5, 0x7a, 0xc8],
    ),
    (
        "Aqua",
        [0x05, 0x10, 0x14],
        [0x1f, 0x44, 0x53],
        [0x6a, 0xd2, 0xe5],
    ),
    (
        "Ember",
        [0x0e, 0x06, 0x04],
        [0x4c, 0x1c, 0x09],
        [0xe6, 0x84, 0x3c],
    ),
    (
        "Forest",
        [0x05, 0x0c, 0x07],
        [0x1c, 0x3a, 0x22],
        [0x7a, 0xc8, 0x82],
    ),
    (
        "Slate",
        [0x09, 0x0c, 0x10],
        [0x24, 0x2a, 0x33],
        [0x9a, 0xa6, 0xb8],
    ),
];

pub(super) const FIRE_PALETTES: &[ShaderPalette] = &[
    (
        "Hearth",
        [0x12, 0x04, 0x02],
        [0xff, 0x55, 0x18],
        [0xff, 0xd6, 0x6b],
    ),
    (
        "Bonfire",
        [0x08, 0x02, 0x01],
        [0xd6, 0x3a, 0x0e],
        [0xff, 0xb0, 0x42],
    ),
    (
        "Forge",
        [0x10, 0x05, 0x02],
        [0xff, 0x84, 0x1a],
        [0xff, 0xf0, 0xb0],
    ),
    (
        "Ember",
        [0x0e, 0x06, 0x04],
        [0x4c, 0x1c, 0x09],
        [0xe6, 0x84, 0x3c],
    ),
    (
        "Blackfire",
        [0x09, 0x0c, 0x10],
        [0x24, 0x2a, 0x33],
        [0x9a, 0xa6, 0xb8],
    ),
];

pub(super) const AURORA_PALETTES: &[ShaderPalette] = &[
    (
        "Aurora",
        [0x08, 0x0e, 0x26],
        [0x2e, 0xdc, 0x96],
        [0xc8, 0x5a, 0xe6],
    ),
    (
        "Boreal",
        [0x04, 0x0a, 0x18],
        [0x18, 0xc0, 0xb4],
        [0x6a, 0x84, 0xff],
    ),
    (
        "Solar",
        [0x12, 0x06, 0x20],
        [0xff, 0xa0, 0x4a],
        [0xff, 0x5a, 0xc8],
    ),
    (
        "Glacial",
        [0x05, 0x0c, 0x14],
        [0x4a, 0xa0, 0xff],
        [0xc8, 0xf0, 0xff],
    ),
];

pub(super) const TREES_PALETTES: &[ShaderPalette] = &[
    (
        "Canopy",
        [0x0a, 0x16, 0x0e],
        [0x3a, 0x78, 0x34],
        [0xd0, 0xe8, 0x96],
    ),
    (
        "Mossy",
        [0x08, 0x12, 0x0e],
        [0x2e, 0x60, 0x46],
        [0xa8, 0xc8, 0x8c],
    ),
    (
        "Autumnal",
        [0x14, 0x0c, 0x08],
        [0x96, 0x46, 0x1c],
        [0xf0, 0xbe, 0x5a],
    ),
    (
        "Twilight",
        [0x06, 0x0a, 0x12],
        [0x28, 0x3c, 0x5a],
        [0xaa, 0xb4, 0xdc],
    ),
];

pub(super) fn shader_palettes_for_mode(mode: &str) -> &'static [ShaderPalette] {
    match mode {
        "fire" => FIRE_PALETTES,
        "aurora" => AURORA_PALETTES,
        "trees" => TREES_PALETTES,
        _ => SMOKE_PALETTES,
    }
}

pub(super) fn visible_shader_mode(mode: &str) -> bool {
    matches!(mode, "smoke" | "fire" | "aurora" | "trees")
}

pub(super) fn shader_palette_label(mode: &str) -> &'static str {
    match mode {
        "fire" => "Fire Palette",
        "aurora" => "Aurora Palette",
        "trees" => "Canopy Palette",
        _ => "Smoke Palette",
    }
}

pub(super) fn shader_intensity_label(mode: &str) -> &'static str {
    match mode {
        "fire" => "Fire Intensity",
        "aurora" => "Aurora Intensity",
        "trees" => "Canopy Intensity",
        _ => "Smoke Intensity",
    }
}

/// Per-effect controls: palette presets + intensity slider + live preview.
/// Shown for any visible shader-driven background mode (smoke / fire /
/// aurora / trees).
/// renders an empty div otherwise so the Terminal tab's row layout doesn't
/// shift when None or Image is selected.
pub(super) fn terminal_smoke_controls(
    config: &Config,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let mode = config.effects.background.as_str();
    if !visible_shader_mode(mode) {
        return div();
    }
    let kind = match crate::effects::EffectKind::from_background_mode(mode) {
        Some(k) => k,
        None => return div(),
    };

    let palettes = shader_palettes_for_mode(mode);

    let mut palette_row = div()
        .flex()
        .items_center()
        .gap_2()
        .child(control_label(shader_palette_label(mode)));
    for (label, c1, c2, c3) in palettes {
        let label = (*label).to_string();
        let active = config.effects.background_color == Some(*c1)
            && config.effects.background_color2 == Some(*c2)
            && config.effects.background_color3 == Some(*c3);
        let c1 = *c1;
        let c2 = *c2;
        let c3 = *c3;
        palette_row = palette_row.child(appearance_button(label, active, cx, move |this, cx| {
            this.set_effect_palette(c1, c2, c3, cx)
        }));
    }

    let (default_c1, default_c2, default_c3) = crate::gpui_terminal::default_palette_for(kind);
    let c1 = config.effects.background_color.unwrap_or(default_c1);
    let c2 = config.effects.background_color2.unwrap_or(default_c2);
    let c3 = config.effects.background_color3.unwrap_or(default_c3);
    let intensity_pct = (config.effects.background_intensity * 100.0).round() as i32;

    let preview = shader_preview(kind, config.effects.background_intensity, c1, c2, c3);

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(palette_row)
        .child(metric_row(
            shader_intensity_label(mode),
            format!("{intensity_pct}%"),
            cx,
            |this, cx| this.adjust_effect_intensity(-0.05, cx),
            |this, cx| this.adjust_effect_intensity(0.05, cx),
        ))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(control_label("Active"))
                .child(color_strip([c1, c2, c3])),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(control_label("Preview"))
                .child(preview),
        )
}

pub(super) fn shader_preview(
    kind: crate::effects::EffectKind,
    intensity: f32,
    c1: ColorStop,
    c2: ColorStop,
    c3: ColorStop,
) -> impl IntoElement {
    div()
        .relative()
        .w_full()
        .h(px(160.0))
        .border_1()
        .border_color(rgb(BORDER))
        .overflow_hidden()
        .bg(rgb(0x08090d))
        .child(
            div()
                .absolute()
                .size_full()
                .flex()
                .flex_col()
                .child(palette_band(c1))
                .child(palette_band(c2))
                .child(palette_band(c3)),
        )
        .child(
            div().absolute().size_full().child(
                crate::effects::EffectsElement::new()
                    .with_kind(kind)
                    .with_intensity(intensity)
                    .with_palette(c1, c2, c3),
            ),
        )
}

#[cfg(test)]
mod tests {
    use super::{shader_palettes_for_mode, visible_shader_mode, FIRE_PALETTES, SMOKE_PALETTES};

    #[test]
    fn fire_palettes_include_blackfire_with_smoke_slate_colors() {
        let (_, slate_c1, slate_c2, slate_c3) = SMOKE_PALETTES
            .iter()
            .find(|(label, _, _, _)| *label == "Slate")
            .copied()
            .expect("smoke Slate palette should exist");
        let (_, blackfire_c1, blackfire_c2, blackfire_c3) = FIRE_PALETTES
            .iter()
            .find(|(label, _, _, _)| *label == "Blackfire")
            .copied()
            .expect("fire Blackfire palette should exist");

        assert_eq!(
            (blackfire_c1, blackfire_c2, blackfire_c3),
            (slate_c1, slate_c2, slate_c3)
        );
    }

    #[test]
    fn shader_palette_options_do_not_expose_rain_palettes() {
        for mode in ["smoke", "fire", "aurora", "trees"] {
            assert!(
                shader_palettes_for_mode(mode)
                    .iter()
                    .all(|(label, _, _, _)| !label.contains("Rain")),
                "{mode} should not expose rain palette labels"
            );
        }
    }

    #[test]
    fn rain_is_not_a_visible_shader_mode() {
        assert!(visible_shader_mode("smoke"));
        assert!(visible_shader_mode("fire"));
        assert!(!visible_shader_mode("rain"));
    }
}
