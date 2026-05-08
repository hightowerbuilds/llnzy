use crate::editor::syntax::HighlightGroup;
use crate::keybindings::KeybindingPreset;

use super::colors::parse_hex;
use super::keybinding_mapping::apply_keybindings;
use super::model::{BackgroundImageFit, Config, CursorStyle};
use super::presets::preset_scheme;
use super::schema::ConfigFile;

impl Config {
    pub(super) fn apply(&mut self, file: ConfigFile) {
        if let Some(font) = file.font {
            if let Some(s) = font.size {
                self.font_size = s;
            }
            if font.family.is_some() {
                self.font_family = font.family;
            }
            if let Some(v) = font.ligatures {
                self.ligatures = v;
            }
            if let Some(w) = font.weight {
                self.font_weight = w;
            }
            if let Some(s) = font.style {
                self.font_style = s;
            }
            if let Some(lh) = font.line_height {
                self.line_height = lh;
            }
        }

        if let Some(colors) = file.colors {
            if let Some(scheme) = &colors.scheme {
                if let Some(preset) = preset_scheme(scheme) {
                    self.colors = preset;
                }
            }
            if let Some(time_of_day_enabled) = colors.time_of_day_enabled {
                self.time_of_day_enabled = time_of_day_enabled;
            }
            macro_rules! apply_color {
                ($field:ident, $cfg:expr) => {
                    if let Some(c) = $cfg.and_then(|s| parse_hex(&s)) {
                        self.colors.$field = c;
                    }
                };
            }
            apply_color!(foreground, colors.foreground);
            apply_color!(background, colors.background);
            apply_color!(cursor, colors.cursor);
            apply_color!(selection, colors.selection);
            if let Some(a) = colors.selection_alpha {
                self.colors.selection_alpha = a;
            }

            let ansi_keys: [Option<String>; 16] = [
                colors.black,
                colors.red,
                colors.green,
                colors.yellow,
                colors.blue,
                colors.magenta,
                colors.cyan,
                colors.white,
                colors.bright_black,
                colors.bright_red,
                colors.bright_green,
                colors.bright_yellow,
                colors.bright_blue,
                colors.bright_magenta,
                colors.bright_cyan,
                colors.bright_white,
            ];
            for (i, key) in ansi_keys.into_iter().enumerate() {
                if let Some(c) = key.and_then(|s| parse_hex(&s)) {
                    self.colors.ansi[i] = c;
                }
            }
        }

        if let Some(cursor) = file.cursor {
            if let Some(style) = cursor.style {
                self.cursor_style = match style.as_str() {
                    "beam" | "bar" => CursorStyle::Beam,
                    "underline" => CursorStyle::Underline,
                    _ => CursorStyle::Block,
                };
            }
            if let Some(rate) = cursor.blink_rate {
                self.cursor_blink_ms = rate;
            }
        }

        if let Some(window) = file.window {
            if let Some(px) = window.padding_x {
                self.padding_x = px;
            }
            if let Some(py) = window.padding_y {
                self.padding_y = py;
            }
            if let Some(o) = window.opacity {
                self.opacity = o.clamp(0.0, 1.0);
            }
        }

        if let Some(scrolling) = file.scrolling {
            if let Some(l) = scrolling.lines {
                self.scroll_lines = l;
            }
        }

        if let Some(terminal) = file.terminal {
            if let Some(copy_on_select) = terminal.copy_on_select {
                self.terminal.copy_on_select = copy_on_select;
            }
        }

        if let Some(shell) = file.shell {
            if let Some(p) = shell.program {
                self.shell = p;
            }
        }

        if let Some(effects) = file.effects {
            if let Some(e) = effects.enabled {
                self.effects.enabled = e;
            }
            if let Some(fps) = effects.fps_target {
                self.effects.fps_target = fps.clamp(15, 240);
            }
            if let Some(bg) = effects.background {
                self.effects.background = bg;
            }
            if let Some(i) = effects.background_intensity {
                self.effects.background_intensity = i.clamp(0.0, 1.0);
            }
            if let Some(s) = effects.background_speed {
                self.effects.background_speed = s.clamp(0.0, 10.0);
            }
            if let Some(c) = effects.background_color.and_then(|s| parse_hex(&s)) {
                self.effects.background_color = Some(c);
            }
            if let Some(c) = effects.background_color2.and_then(|s| parse_hex(&s)) {
                self.effects.background_color2 = Some(c);
            }
            if let Some(c) = effects.background_color3.and_then(|s| parse_hex(&s)) {
                self.effects.background_color3 = Some(c);
            }
            if let Some(p) = effects.background_image {
                self.effects.background_image = Some(p);
            }
            if let Some(fit) = effects
                .background_image_fit
                .as_deref()
                .and_then(BackgroundImageFit::from_str)
            {
                self.effects.background_image_fit = fit;
            }
            if let Some(b) = effects.bloom_enabled {
                self.effects.bloom_enabled = b;
            }
            if let Some(t) = effects.bloom_threshold {
                self.effects.bloom_threshold = t.clamp(0.0, 1.0);
            }
            if let Some(i) = effects.bloom_intensity {
                self.effects.bloom_intensity = i.clamp(0.0, 3.0);
            }
            if let Some(r) = effects.bloom_radius {
                self.effects.bloom_radius = r.clamp(0.5, 5.0);
            }
            if let Some(p) = effects.particles_enabled {
                self.effects.particles_enabled = p;
            }
            if let Some(c) = effects.particles_count {
                self.effects.particles_count = c.clamp(0, 4096);
            }
            if let Some(s) = effects.particles_speed {
                self.effects.particles_speed = s.clamp(0.0, 5.0);
            }
            if let Some(g) = effects.cursor_glow {
                self.effects.cursor_glow = g;
            }
            if let Some(t) = effects.cursor_trail {
                self.effects.cursor_trail = t;
            }
            if let Some(t) = effects.text_animation {
                self.effects.text_animation = t;
            }
            if let Some(c) = effects.crt_enabled {
                self.effects.crt_enabled = c;
            }
            if let Some(s) = effects.scanline_intensity {
                self.effects.scanline_intensity = s.clamp(0.0, 1.0);
            }
            if let Some(c) = effects.curvature {
                self.effects.curvature = c.clamp(0.0, 0.5);
            }
            if let Some(v) = effects.vignette_strength {
                self.effects.vignette_strength = v.clamp(0.0, 2.0);
            }
            if let Some(c) = effects.chromatic_aberration {
                self.effects.chromatic_aberration = c.clamp(0.0, 5.0);
            }
            if let Some(g) = effects.grain_intensity {
                self.effects.grain_intensity = g.clamp(0.0, 0.5);
            }
            if let Some(ui) = effects.effects_on_ui {
                self.effects.effects_on_ui = ui;
            }
        }

        if let Some(editor) = file.editor {
            if let Some(tab_size) = editor.tab_size {
                self.editor.tab_size = tab_size.clamp(1, 16);
            }
            if let Some(insert_spaces) = editor.insert_spaces {
                self.editor.insert_spaces = insert_spaces;
            }
            if let Some(rulers) = editor.rulers {
                self.editor.rulers = normalize_rulers(rulers);
            }
            if let Some(word_wrap) = editor.word_wrap {
                self.editor.word_wrap = word_wrap;
            }
            if let Some(visible_whitespace) = editor.visible_whitespace {
                self.editor.visible_whitespace = visible_whitespace;
            }
            if let Some(font_size) = editor.font_size {
                self.editor.font_size = Some(font_size.clamp(8.0, 40.0));
            }
            if let Some(line_height) = editor.line_height {
                self.editor.line_height = line_height.clamp(1.0, 2.2);
            }
            if let Some(sidebar_font_size) = editor.sidebar_font_size {
                self.editor.sidebar_font_size = sidebar_font_size.clamp(8.0, 24.0);
            }
            if let Some(show_line_numbers) = editor.show_line_numbers {
                self.editor.show_line_numbers = show_line_numbers;
            }
            if let Some(highlight_current_line) = editor.highlight_current_line {
                self.editor.highlight_current_line = highlight_current_line;
            }
            if let Some(preset) = editor.keybinding_preset {
                self.editor.keybinding_preset = KeybindingPreset::from_str(&preset);
            }
            if let Some(languages) = editor.languages {
                for (lang, lang_config) in languages {
                    let existing = self.editor.languages.entry(lang).or_default();
                    if let Some(tab_size) = lang_config.tab_size {
                        existing.tab_size = Some(tab_size.clamp(1, 16));
                    }
                    if let Some(insert_spaces) = lang_config.insert_spaces {
                        existing.insert_spaces = Some(insert_spaces);
                    }
                    if let Some(rulers) = lang_config.rulers {
                        existing.rulers = Some(normalize_rulers(rulers));
                    }
                    if let Some(word_wrap) = lang_config.word_wrap {
                        existing.word_wrap = Some(word_wrap);
                    }
                    if let Some(visible_whitespace) = lang_config.visible_whitespace {
                        existing.visible_whitespace = Some(visible_whitespace);
                    }
                }
            }

            if let Some(syntax_colors) = editor.syntax_colors {
                self.syntax_colors.clear();
                for (group_name, color) in syntax_colors {
                    let Some(group) = HighlightGroup::from_config_key(&group_name) else {
                        log::warn!("Unknown syntax highlight group: {}", group_name);
                        continue;
                    };
                    let Some(rgb) = parse_hex(&color) else {
                        log::warn!("Invalid syntax color for {}: {}", group_name, color);
                        continue;
                    };
                    self.syntax_colors.insert(group, rgb);
                }
            }
        }

        if let Some(kb) = file.keybindings {
            apply_keybindings(&mut self.keybindings, kb);
        }
    }
}

fn normalize_rulers(mut rulers: Vec<usize>) -> Vec<usize> {
    rulers.retain(|col| (1..=240).contains(col));
    rulers.sort_unstable();
    rulers.dedup();
    rulers
}
