use std::path::{Path, PathBuf};

use crate::config::{ColorScheme, Config, CursorStyle, EffectsConfig};
use crate::theme::VisualTheme;

/// Get the llnzy config directory (~/.config/llnzy/).
fn config_base() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("llnzy"))
}

// ── Background Image Library ──

/// Directory where saved background images live.
pub fn backgrounds_dir() -> Option<PathBuf> {
    config_base().map(|d| d.join("backgrounds"))
}

/// Import an image file into the backgrounds library. Returns the new path.
pub fn import_background(source: &Path) -> Result<PathBuf, String> {
    let dir = backgrounds_dir().ok_or("No config directory")?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create backgrounds dir: {e}"))?;

    let file_name = source.file_name().ok_or("No file name")?;
    let dest = dir.join(file_name);

    // If a file with the same name exists, add a suffix
    let dest = if dest.exists() {
        let stem = source.file_stem().and_then(|s| s.to_str()).unwrap_or("bg");
        let ext = source.extension().and_then(|s| s.to_str()).unwrap_or("png");
        let mut i = 1;
        loop {
            let candidate = dir.join(format!("{stem}_{i}.{ext}"));
            if !candidate.exists() {
                break candidate;
            }
            i += 1;
        }
    } else {
        dest
    };

    std::fs::copy(source, &dest).map_err(|e| format!("Failed to copy image: {e}"))?;
    Ok(dest)
}

/// List all saved background images.
pub fn list_backgrounds() -> Vec<PathBuf> {
    let Some(dir) = backgrounds_dir() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut images: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| {
            let ext = e.path().extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
            matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "bmp" | "webp" | "gif")
        })
        .map(|e| e.path())
        .collect();
    images.sort();
    images
}

/// Delete a background image from the library.
pub fn delete_background(path: &Path) -> Result<(), String> {
    std::fs::remove_file(path).map_err(|e| format!("Failed to delete: {e}"))
}

// ── Custom Theme Storage ──

/// Directory where saved user themes live.
fn themes_dir() -> Option<PathBuf> {
    config_base().map(|d| d.join("themes"))
}

/// A serializable theme definition.
#[derive(serde::Serialize, serde::Deserialize)]
struct ThemeFile {
    name: String,
    description: String,
    // Colors
    foreground: String,
    background: String,
    cursor: String,
    selection: String,
    selection_alpha: f32,
    ansi: Vec<String>,
    // Effects
    effects_background: String,
    effects_background_intensity: f32,
    effects_background_speed: f32,
    effects_background_color: Option<String>,
    effects_background_image: Option<String>,
    effects_bloom_enabled: bool,
    effects_bloom_threshold: f32,
    effects_bloom_intensity: f32,
    effects_bloom_radius: f32,
    effects_particles_enabled: bool,
    effects_particles_count: u32,
    effects_particles_speed: f32,
    effects_cursor_glow: bool,
    effects_cursor_trail: bool,
    effects_text_animation: bool,
    effects_crt_enabled: bool,
    effects_scanline_intensity: f32,
    effects_curvature: f32,
    effects_vignette_strength: f32,
    effects_chromatic_aberration: f32,
    effects_grain_intensity: f32,
    effects_on_ui: bool,
    // Cursor
    cursor_style: String,
    // Per-view toggles
    #[serde(default)]
    apply_to_terminal: bool,
    #[serde(default)]
    apply_to_editor: bool,
    #[serde(default)]
    apply_to_sketch: bool,
    #[serde(default)]
    apply_to_stacker: bool,
}

fn rgb_to_hex(c: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

fn hex_to_rgb(s: &str) -> [u8; 3] {
    let s = s.trim_start_matches('#');
    if s.len() >= 6 {
        let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0);
        [r, g, b]
    } else {
        [0, 0, 0]
    }
}

fn cursor_style_to_str(style: CursorStyle) -> &'static str {
    match style {
        CursorStyle::Block => "block",
        CursorStyle::Beam => "beam",
        CursorStyle::Underline => "underline",
    }
}

fn str_to_cursor_style(s: &str) -> CursorStyle {
    match s {
        "beam" | "bar" => CursorStyle::Beam,
        "underline" => CursorStyle::Underline,
        _ => CursorStyle::Block,
    }
}

/// Per-view application flags stored alongside a theme.
#[derive(Clone, Debug, Default)]
pub struct ThemeViewFlags {
    pub terminal: bool,
    pub editor: bool,
    pub sketch: bool,
    pub stacker: bool,
}

/// Save the current config as a named theme.
pub fn save_theme(name: &str, description: &str, config: &Config, view_flags: &ThemeViewFlags) -> Result<PathBuf, String> {
    let dir = themes_dir().ok_or("No config directory")?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create themes dir: {e}"))?;

    let safe_name: String = name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let path = dir.join(format!("{safe_name}.toml"));

    let theme_file = ThemeFile {
        name: name.to_string(),
        description: description.to_string(),
        foreground: rgb_to_hex(config.colors.foreground),
        background: rgb_to_hex(config.colors.background),
        cursor: rgb_to_hex(config.colors.cursor),
        selection: rgb_to_hex(config.colors.selection),
        selection_alpha: config.colors.selection_alpha,
        ansi: config.colors.ansi.iter().map(|c| rgb_to_hex(*c)).collect(),
        effects_background: config.effects.background.clone(),
        effects_background_intensity: config.effects.background_intensity,
        effects_background_speed: config.effects.background_speed,
        effects_background_color: config.effects.background_color.map(|c| rgb_to_hex(c)),
        effects_background_image: config.effects.background_image.clone(),
        effects_bloom_enabled: config.effects.bloom_enabled,
        effects_bloom_threshold: config.effects.bloom_threshold,
        effects_bloom_intensity: config.effects.bloom_intensity,
        effects_bloom_radius: config.effects.bloom_radius,
        effects_particles_enabled: config.effects.particles_enabled,
        effects_particles_count: config.effects.particles_count,
        effects_particles_speed: config.effects.particles_speed,
        effects_cursor_glow: config.effects.cursor_glow,
        effects_cursor_trail: config.effects.cursor_trail,
        effects_text_animation: config.effects.text_animation,
        effects_crt_enabled: config.effects.crt_enabled,
        effects_scanline_intensity: config.effects.scanline_intensity,
        effects_curvature: config.effects.curvature,
        effects_vignette_strength: config.effects.vignette_strength,
        effects_chromatic_aberration: config.effects.chromatic_aberration,
        effects_grain_intensity: config.effects.grain_intensity,
        effects_on_ui: config.effects.effects_on_ui,
        cursor_style: cursor_style_to_str(config.cursor_style).to_string(),
        apply_to_terminal: view_flags.terminal,
        apply_to_editor: view_flags.editor,
        apply_to_sketch: view_flags.sketch,
        apply_to_stacker: view_flags.stacker,
    };

    let toml_str = toml::to_string_pretty(&theme_file).map_err(|e| format!("Serialize failed: {e}"))?;
    std::fs::write(&path, toml_str).map_err(|e| format!("Write failed: {e}"))?;
    Ok(path)
}

/// Load all user-saved themes.
pub fn load_user_themes() -> Vec<(VisualTheme, ThemeViewFlags)> {
    let Some(dir) = themes_dir() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };

    let mut themes = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("toml") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let Ok(tf) = toml::from_str::<ThemeFile>(&text) else { continue };

        let mut ansi = [[0u8; 3]; 16];
        for (i, hex) in tf.ansi.iter().enumerate().take(16) {
            ansi[i] = hex_to_rgb(hex);
        }

        let theme = VisualTheme {
            name: tf.name,
            description: tf.description,
            colors: ColorScheme {
                foreground: hex_to_rgb(&tf.foreground),
                background: hex_to_rgb(&tf.background),
                cursor: hex_to_rgb(&tf.cursor),
                selection: hex_to_rgb(&tf.selection),
                selection_alpha: tf.selection_alpha,
                ansi,
            },
            effects: EffectsConfig {
                enabled: true,
                fps_target: 60,
                background: tf.effects_background,
                background_intensity: tf.effects_background_intensity,
                background_speed: tf.effects_background_speed,
                background_color: tf.effects_background_color.map(|s| hex_to_rgb(&s)),
                background_image: tf.effects_background_image,
                bloom_enabled: tf.effects_bloom_enabled,
                bloom_threshold: tf.effects_bloom_threshold,
                bloom_intensity: tf.effects_bloom_intensity,
                bloom_radius: tf.effects_bloom_radius,
                particles_enabled: tf.effects_particles_enabled,
                particles_count: tf.effects_particles_count,
                particles_speed: tf.effects_particles_speed,
                cursor_glow: tf.effects_cursor_glow,
                cursor_trail: tf.effects_cursor_trail,
                text_animation: tf.effects_text_animation,
                crt_enabled: tf.effects_crt_enabled,
                scanline_intensity: tf.effects_scanline_intensity,
                curvature: tf.effects_curvature,
                vignette_strength: tf.effects_vignette_strength,
                chromatic_aberration: tf.effects_chromatic_aberration,
                grain_intensity: tf.effects_grain_intensity,
                effects_on_ui: tf.effects_on_ui,
            },
            cursor_style: str_to_cursor_style(&tf.cursor_style),
        };

        let flags = ThemeViewFlags {
            terminal: tf.apply_to_terminal,
            editor: tf.apply_to_editor,
            sketch: tf.apply_to_sketch,
            stacker: tf.apply_to_stacker,
        };

        themes.push((theme, flags));
    }

    themes
}

/// Delete a user-saved theme by name.
pub fn delete_user_theme(name: &str) -> Result<(), String> {
    let dir = themes_dir().ok_or("No config directory")?;
    let safe_name: String = name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let path = dir.join(format!("{safe_name}.toml"));
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("Delete failed: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_hex_roundtrip() {
        let color = [255, 128, 0];
        assert_eq!(hex_to_rgb(&rgb_to_hex(color)), color);
    }

    #[test]
    fn cursor_style_roundtrip() {
        assert_eq!(str_to_cursor_style(cursor_style_to_str(CursorStyle::Beam)), CursorStyle::Beam);
        assert_eq!(str_to_cursor_style(cursor_style_to_str(CursorStyle::Block)), CursorStyle::Block);
    }
}
