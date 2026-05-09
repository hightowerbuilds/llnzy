use std::path::{Path, PathBuf};

use crate::config::{BackgroundImageFit, ColorScheme, Config, CursorStyle, EffectsConfig};
use crate::path_utils::{
    path_extension_is, path_extension_matches, safe_config_stem, BACKGROUND_IMAGE_EXTS, TOML_EXT,
};
use crate::theme::VisualTheme;

// ── Background Image Library ──

/// Directory where saved background images live.
pub fn backgrounds_dir() -> Option<PathBuf> {
    crate::platform::paths::current_paths().map(|paths| paths.backgrounds_dir())
}

/// Import an image file into the backgrounds library. Returns the new path.
pub fn import_background(source: &Path) -> Result<PathBuf, String> {
    let dir = backgrounds_dir().ok_or("No config directory")?;
    import_background_into_dir(source, &dir)
}

/// Resolve a stored background image reference to an image in the background library.
///
/// Older configs may contain absolute paths. Newer UI writes can use just the library
/// file name, which is more portable across packaged app installs and data moves.
pub fn resolve_background_path(reference: &str) -> Option<PathBuf> {
    let dir = backgrounds_dir()?;
    resolve_background_path_in_dir(reference, &dir)
}

fn resolve_background_path_in_dir(reference: &str, dir: &Path) -> Option<PathBuf> {
    let reference = reference.trim();
    if reference.is_empty() {
        return None;
    }

    let path = PathBuf::from(reference);
    if path.is_file() {
        return Some(path);
    }

    if !path.is_absolute() {
        let library_path = dir.join(&path);
        if library_path.is_file() {
            return Some(library_path);
        }
    }

    let file_name = path.file_name()?;
    let library_path = dir.join(file_name);
    if library_path.is_file() {
        return Some(library_path);
    }

    None
}

fn import_background_into_dir(source: &Path, dir: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("Failed to create backgrounds dir: {e}"))?;

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
    list_backgrounds_in_dir(&dir)
}

fn list_backgrounds_in_dir(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut images: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| path_extension_matches(&e.path(), BACKGROUND_IMAGE_EXTS))
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
    crate::platform::paths::current_paths().map(|paths| paths.themes_dir)
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
    effects_background_color2: Option<String>,
    effects_background_color3: Option<String>,
    effects_background_image: Option<String>,
    #[serde(default)]
    effects_background_image_fit: Option<String>,
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
pub fn save_theme(
    name: &str,
    description: &str,
    config: &Config,
    view_flags: &ThemeViewFlags,
) -> Result<PathBuf, String> {
    let dir = themes_dir().ok_or("No config directory")?;
    save_theme_to_dir(name, description, config, view_flags, &dir)
}

fn save_theme_to_dir(
    name: &str,
    description: &str,
    config: &Config,
    view_flags: &ThemeViewFlags,
    dir: &Path,
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("Failed to create themes dir: {e}"))?;

    let safe_name = safe_config_stem(name);
    let path = dir.join(format!("{safe_name}.toml"));

    let theme_file = ThemeFile {
        name: name.to_string(),
        description: description.to_string(),
        foreground: rgb_to_hex(config.colors.foreground),
        background: rgb_to_hex(config.colors.background),
        cursor: rgb_to_hex(config.colors.cursor),
        selection: rgb_to_hex(config.colors.selection),
        selection_alpha: config.colors.selection_alpha,
        ansi: config.colors.ansi.iter().copied().map(rgb_to_hex).collect(),
        effects_background: config.effects.background.clone(),
        effects_background_intensity: config.effects.background_intensity,
        effects_background_speed: config.effects.background_speed,
        effects_background_color: config.effects.background_color.map(rgb_to_hex),
        effects_background_color2: config.effects.background_color2.map(rgb_to_hex),
        effects_background_color3: config.effects.background_color3.map(rgb_to_hex),
        effects_background_image: config.effects.background_image.clone(),
        effects_background_image_fit: Some(
            config.effects.background_image_fit.as_str().to_string(),
        ),
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

    let toml_str =
        toml::to_string_pretty(&theme_file).map_err(|e| format!("Serialize failed: {e}"))?;
    std::fs::write(&path, toml_str).map_err(|e| format!("Write failed: {e}"))?;
    Ok(path)
}

/// Load all user-saved themes.
pub fn load_user_themes() -> Vec<(VisualTheme, ThemeViewFlags)> {
    let Some(dir) = themes_dir() else {
        return Vec::new();
    };
    load_user_themes_from_dir(&dir)
}

fn load_user_themes_from_dir(dir: &Path) -> Vec<(VisualTheme, ThemeViewFlags)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut themes = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path_extension_is(&path, TOML_EXT) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(tf) = toml::from_str::<ThemeFile>(&text) else {
            continue;
        };

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
                background_color: tf.effects_background_color.as_deref().map(hex_to_rgb),
                background_color2: tf.effects_background_color2.as_deref().map(hex_to_rgb),
                background_color3: tf.effects_background_color3.as_deref().map(hex_to_rgb),
                background_image: tf.effects_background_image,
                background_image_fit: tf
                    .effects_background_image_fit
                    .as_deref()
                    .and_then(BackgroundImageFit::parse)
                    .unwrap_or_default(),
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
    delete_user_theme_from_dir(name, &dir)
}

fn delete_user_theme_from_dir(name: &str, dir: &Path) -> Result<(), String> {
    let safe_name = safe_config_stem(name);
    let path = dir.join(format!("{safe_name}.toml"));
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("Delete failed: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_dir(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("llnzy-theme-store-{name}-{stamp}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn rgb_hex_roundtrip() {
        let color = [255, 128, 0];
        assert_eq!(hex_to_rgb(&rgb_to_hex(color)), color);
    }

    #[test]
    fn cursor_style_roundtrip() {
        assert_eq!(
            str_to_cursor_style(cursor_style_to_str(CursorStyle::Beam)),
            CursorStyle::Beam
        );
        assert_eq!(
            str_to_cursor_style(cursor_style_to_str(CursorStyle::Block)),
            CursorStyle::Block
        );
    }

    #[test]
    fn background_library_imports_sorts_and_deletes_images() {
        let root = test_dir("backgrounds");
        let source = root.join("source");
        let library = root.join("library");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("sky.png"), b"first").unwrap();
        std::fs::write(source.join("notes.txt"), b"ignore").unwrap();

        let first = import_background_into_dir(&source.join("sky.png"), &library).unwrap();
        let second = import_background_into_dir(&source.join("sky.png"), &library).unwrap();

        assert_eq!(
            first.file_name().and_then(|name| name.to_str()),
            Some("sky.png")
        );
        assert_eq!(
            second.file_name().and_then(|name| name.to_str()),
            Some("sky_1.png")
        );
        assert_eq!(
            list_backgrounds_in_dir(&library),
            vec![first.clone(), second]
        );

        delete_background(&first).unwrap();
        assert!(!first.exists());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn background_references_resolve_from_library_names_and_stale_absolute_paths() {
        let root = test_dir("background-resolve");
        let library = root.join("library");
        std::fs::create_dir_all(&library).unwrap();
        let image = library.join("sky.png");
        std::fs::write(&image, b"not actually decoded here").unwrap();

        assert_eq!(
            resolve_background_path_in_dir("sky.png", &library),
            Some(image.clone())
        );
        assert_eq!(
            resolve_background_path_in_dir("/missing/old/location/sky.png", &library),
            Some(image.clone())
        );
        assert_eq!(
            resolve_background_path_in_dir(image.to_str().unwrap(), &library),
            Some(image.clone())
        );
        assert_eq!(
            resolve_background_path_in_dir("missing.png", &library),
            None
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn user_theme_roundtrips_flags_and_deletes() {
        let root = test_dir("themes");
        let mut config = Config::default();
        config.colors.foreground = [1, 2, 3];
        config.effects.background = "aurora".to_string();
        config.effects.background_image = Some("/tmp/background.png".to_string());
        config.cursor_style = CursorStyle::Beam;
        let flags = ThemeViewFlags {
            terminal: true,
            editor: true,
            sketch: false,
            stacker: true,
        };

        let path = save_theme_to_dir("My Theme", "desc", &config, &flags, &root).unwrap();
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("My_Theme.toml")
        );

        let themes = load_user_themes_from_dir(&root);
        assert_eq!(themes.len(), 1);
        let (theme, loaded_flags) = &themes[0];
        assert_eq!(theme.name, "My Theme");
        assert_eq!(theme.description, "desc");
        assert_eq!(theme.colors.foreground, [1, 2, 3]);
        assert_eq!(theme.effects.background, "aurora");
        assert_eq!(
            theme.effects.background_image.as_deref(),
            Some("/tmp/background.png")
        );
        assert_eq!(theme.cursor_style, CursorStyle::Beam);
        assert!(loaded_flags.terminal);
        assert!(loaded_flags.editor);
        assert!(!loaded_flags.sketch);
        assert!(loaded_flags.stacker);

        delete_user_theme_from_dir("My Theme", &root).unwrap();
        assert!(load_user_themes_from_dir(&root).is_empty());

        let _ = std::fs::remove_dir_all(root);
    }
}
