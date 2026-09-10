use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use crate::config::{BackgroundImageFit, ColorScheme, Config, CursorStyle, EffectsConfig};
use crate::path_utils::{
    path_extension_is, path_extension_matches, safe_config_stem, BACKGROUND_IMAGE_EXTS, TOML_EXT,
};
use crate::theme::VisualTheme;
use sha2::{Digest, Sha256};

// ── Background Image Library ──

pub const MAX_BACKGROUND_IMAGE_BYTES: u64 = 50 * 1024 * 1024;
pub const MAX_BACKGROUND_IMAGE_DIMENSION: u32 = 8192;
pub const MAX_BACKGROUND_IMAGE_PIXELS: u64 = 24_000_000;
/// Maximum number of saved background images in the library. Imports past
/// this count are rejected with an error so the user has to make room
/// before adding more — files are only ever removed by an explicit delete.
pub const MAX_BACKGROUND_IMAGES: usize = 10;

/// Directory where saved background images live.
pub fn backgrounds_dir() -> Option<PathBuf> {
    crate::platform::paths::current_paths().map(|paths| paths.backgrounds_dir())
}

/// Import an image file into the backgrounds library. Returns the new path.
pub fn import_background(source: &Path) -> Result<PathBuf, String> {
    let dir = backgrounds_dir().ok_or("No config directory")?;
    import_background_into_dir(source, &dir)
}

pub fn validate_background_image(path: &Path) -> Result<(u32, u32), String> {
    if !path_extension_matches(path, BACKGROUND_IMAGE_EXTS) {
        return Err("Background image must be PNG, JPEG, BMP, WEBP, or GIF.".to_string());
    }

    let metadata = std::fs::metadata(path).map_err(|e| format!("Image is unavailable: {e}"))?;
    if !metadata.is_file() {
        return Err("Background image must be a file.".to_string());
    }
    if metadata.len() > MAX_BACKGROUND_IMAGE_BYTES {
        return Err(format!(
            "Background image is too large on disk: {}. Maximum is {}.",
            format_bytes(metadata.len()),
            format_bytes(MAX_BACKGROUND_IMAGE_BYTES)
        ));
    }

    let (width, height) =
        image::image_dimensions(path).map_err(|e| format!("Could not read image size: {e}"))?;
    let pixels = u64::from(width) * u64::from(height);
    if width > MAX_BACKGROUND_IMAGE_DIMENSION || height > MAX_BACKGROUND_IMAGE_DIMENSION {
        return Err(format!(
            "Background image is too large: {width}x{height}. Maximum edge is {MAX_BACKGROUND_IMAGE_DIMENSION}px."
        ));
    }
    if pixels > MAX_BACKGROUND_IMAGE_PIXELS {
        return Err(format!(
            "Background image is too large: {width}x{height} ({:.1} MP). Maximum is {:.1} MP.",
            pixels as f64 / 1_000_000.0,
            MAX_BACKGROUND_IMAGE_PIXELS as f64 / 1_000_000.0
        ));
    }

    Ok((width, height))
}

fn format_bytes(bytes: u64) -> String {
    const MIB: u64 = 1024 * 1024;
    if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else {
        format!("{bytes} bytes")
    }
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
    validate_background_image(source)?;
    std::fs::create_dir_all(dir).map_err(|e| format!("Failed to create backgrounds dir: {e}"))?;

    let source_digest = background_file_digest(source)?;
    if let Some(existing) = find_matching_background(&source_digest, dir) {
        return Ok(existing);
    }

    if list_backgrounds_in_dir(dir).len() >= MAX_BACKGROUND_IMAGES {
        return Err(format!(
            "Background library is full ({MAX_BACKGROUND_IMAGES} max). Delete one before adding more."
        ));
    }

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

fn find_matching_background(source_digest: &[u8; 32], dir: &Path) -> Option<PathBuf> {
    list_backgrounds_in_dir(dir)
        .into_iter()
        .find(|path| match background_file_digest(path) {
            Ok(digest) => &digest == source_digest,
            Err(err) => {
                log::warn!(
                    "failed to fingerprint background image at {}: {err}",
                    path.display()
                );
                false
            }
        })
}

fn background_file_digest(path: &Path) -> Result<[u8; 32], String> {
    let mut file = File::open(path).map_err(|e| format!("Failed to read background image: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| format!("Failed to fingerprint background image: {e}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().into())
}

// ── Blurred Background Cache ──

/// Width the blur actually runs at. Downscaling first is what makes this
/// affordable: a Gaussian over a 2048px image costs seconds, while the
/// same visual result comes from blurring a small copy and letting the
/// renderer scale it back up. The detail a blur destroys is discarded
/// before the expensive part rather than after it.
const BLUR_WORK_WIDTH: u32 = 320;

type BlurMemo = Mutex<HashMap<(PathBuf, u32), Option<PathBuf>>>;

/// In-process memo over `build_blurred_background`. Without it a cache
/// miss — or an unreadable image — would be retried on every frame, which
/// turns one slow decode into a permanent stall.
fn blur_memo() -> &'static BlurMemo {
    static MEMO: OnceLock<BlurMemo> = OnceLock::new();
    MEMO.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Path to a blurred copy of `source`, built and cached on first use.
///
/// Returns `None` when the image cannot be read or no cache directory is
/// available. Callers should fall back to the sharp image: a crisp
/// background is a much smaller failure than no background at all.
pub fn blurred_background_path(source: &Path, sigma: f32) -> Option<PathBuf> {
    // Key on tenths of a sigma so the memo and the on-disk name agree, and
    // so a float that differs in the last bit doesn't miss the cache.
    let sigma_key = (sigma.max(0.0) * 10.0).round() as u32;
    let key = (source.to_path_buf(), sigma_key);

    if let Ok(memo) = blur_memo().lock() {
        if let Some(cached) = memo.get(&key) {
            return cached.clone();
        }
    }

    let built = build_blurred_background(source, sigma, sigma_key);
    if let Ok(mut memo) = blur_memo().lock() {
        memo.insert(key, built.clone());
    }
    built
}

fn build_blurred_background(source: &Path, sigma: f32, sigma_key: u32) -> Option<PathBuf> {
    let dir = crate::platform::paths::current_paths()?
        .cache_dir
        .join("backgrounds-blurred");
    build_blurred_background_in_dir(source, sigma, sigma_key, &dir)
}

fn build_blurred_background_in_dir(
    source: &Path,
    sigma: f32,
    sigma_key: u32,
    dir: &Path,
) -> Option<PathBuf> {
    let digest = background_file_digest(source).ok()?;

    // Name by content digest, not by source path: re-importing the same
    // picture under a new filename reuses the blur instead of rebuilding
    // it, and editing a file in place misses the stale entry.
    let mut name = String::with_capacity(32);
    for byte in &digest[..16] {
        use std::fmt::Write as _;
        let _ = write!(name, "{byte:02x}");
    }
    let target = dir.join(format!("{name}-{sigma_key}.png"));
    if target.is_file() {
        return Some(target);
    }

    std::fs::create_dir_all(dir).ok()?;
    let image = image::open(source).ok()?;
    let width = image.width().max(1);
    let height = image.height().max(1);
    let work_width = BLUR_WORK_WIDTH.min(width);
    let work_height = ((work_width as u64 * height as u64) / width as u64).max(1) as u32;
    let small = image.resize_exact(
        work_width,
        work_height,
        image::imageops::FilterType::Triangle,
    );
    let blurred = image::imageops::blur(&small.to_rgba8(), sigma);
    blurred.save(&target).ok()?;
    Some(target)
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
                background: crate::config::normalize_background_mode(&tf.effects_background)
                    .to_string(),
                background_intensity: tf.effects_background_intensity,
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
            preserve_terminal_effects: false,
        };

        let flags = ThemeViewFlags {
            terminal: tf.apply_to_terminal,
            editor: tf.apply_to_editor,
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
    use std::path::Path;
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

    fn write_test_png(path: &Path) {
        write_test_png_with_pixel(path, [10, 20, 30, 255]);
    }

    fn write_test_png_with_pixel(path: &Path, pixel: [u8; 4]) {
        let image = image::RgbaImage::from_pixel(2, 2, image::Rgba(pixel));
        image.save(path).unwrap();
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
        write_test_png(&source.join("sky.png"));
        std::fs::write(source.join("notes.txt"), b"ignore").unwrap();

        let first = import_background_into_dir(&source.join("sky.png"), &library).unwrap();
        let second = import_background_into_dir(&source.join("sky.png"), &library).unwrap();

        assert_eq!(
            first.file_name().and_then(|name| name.to_str()),
            Some("sky.png")
        );
        assert_eq!(
            second.file_name().and_then(|name| name.to_str()),
            Some("sky.png")
        );
        assert_eq!(first, second);
        assert_eq!(list_backgrounds_in_dir(&library), vec![first.clone()]);

        delete_background(&first).unwrap();
        assert!(!first.exists());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn background_import_suffixes_same_name_with_different_content() {
        let root = test_dir("backgrounds-same-name-different-content");
        let source_a = root.join("source-a");
        let source_b = root.join("source-b");
        let library = root.join("library");
        std::fs::create_dir_all(&source_a).unwrap();
        std::fs::create_dir_all(&source_b).unwrap();
        write_test_png_with_pixel(&source_a.join("sky.png"), [10, 20, 30, 255]);
        write_test_png_with_pixel(&source_b.join("sky.png"), [30, 20, 10, 255]);

        let first = import_background_into_dir(&source_a.join("sky.png"), &library).unwrap();
        let second = import_background_into_dir(&source_b.join("sky.png"), &library).unwrap();

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
            vec![first.clone(), second.clone()]
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn background_import_rejects_when_library_is_full() {
        let root = test_dir("background-library-full");
        let source = root.join("source");
        let library = root.join("library");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&library).unwrap();
        write_test_png(&source.join("img.png"));

        // Pre-seed the library with the maximum number of placeholder
        // image files. Their contents don't need to validate since the
        // cap check happens before we try to copy `source`.
        for i in 0..MAX_BACKGROUND_IMAGES {
            std::fs::write(library.join(format!("seed_{i}.png")), b"placeholder").unwrap();
        }

        let err = import_background_into_dir(&source.join("img.png"), &library).unwrap_err();

        assert!(err.contains("Background library is full"));
        assert_eq!(
            list_backgrounds_in_dir(&library).len(),
            MAX_BACKGROUND_IMAGES
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn background_import_rejects_oversized_files_before_copying() {
        let root = test_dir("background-too-large");
        let source = root.join("huge.png");
        let library = root.join("library");
        let file = std::fs::File::create(&source).unwrap();
        file.set_len(MAX_BACKGROUND_IMAGE_BYTES + 1).unwrap();

        let err = import_background_into_dir(&source, &library).unwrap_err();

        assert!(err.contains("too large on disk"));
        assert!(list_backgrounds_in_dir(&library).is_empty());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn background_validation_reports_dimensions() {
        let root = test_dir("background-dimensions");
        let path = root.join("tiny.png");
        write_test_png(&path);

        assert_eq!(validate_background_image(&path).unwrap(), (2, 2));

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
    fn user_theme_ignores_removed_view_flags() {
        // Themes saved before the Stacker and Sketch surfaces were removed
        // still carry their per-view flags; they must load without error.
        let root = test_dir("themes-stale-flags");
        let config = Config::default();
        let flags = ThemeViewFlags {
            terminal: true,
            editor: false,
        };
        let path = save_theme_to_dir("Old Theme", "desc", &config, &flags, &root).unwrap();
        let mut text = std::fs::read_to_string(&path).unwrap();
        text.push_str("apply_to_sketch = true\napply_to_stacker = true\n");
        std::fs::write(&path, text).unwrap();

        let themes = load_user_themes_from_dir(&root);
        assert_eq!(themes.len(), 1);
        let (theme, loaded_flags) = &themes[0];
        assert_eq!(theme.name, "Old Theme");
        assert!(loaded_flags.terminal);
        assert!(!loaded_flags.editor);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn user_theme_roundtrips_flags_and_deletes() {
        let root = test_dir("themes");
        let mut config = Config::default();
        config.colors.foreground = [1, 2, 3];
        config.effects.background = "image".to_string();
        config.effects.background_image = Some("/tmp/background.png".to_string());
        config.cursor_style = CursorStyle::Beam;
        let flags = ThemeViewFlags {
            terminal: true,
            editor: true,
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
        assert_eq!(theme.effects.background, "image");
        assert_eq!(
            theme.effects.background_image.as_deref(),
            Some("/tmp/background.png")
        );
        assert_eq!(theme.cursor_style, CursorStyle::Beam);
        assert!(loaded_flags.terminal);
        assert!(loaded_flags.editor);

        delete_user_theme_from_dir("My Theme", &root).unwrap();
        assert!(load_user_themes_from_dir(&root).is_empty());

        let _ = std::fs::remove_dir_all(root);
    }

    /// Themes saved before the shader patterns were retired still name one
    /// as their background mode. Loading one must not hand the renderer a
    /// mode it can no longer draw.
    #[test]
    fn user_theme_with_retired_shader_background_loads_as_none() {
        let root = test_dir("themes-retired-shader");
        let mut config = Config::default();
        config.effects.background = "image".to_string();
        save_theme_to_dir("Legacy", "desc", &config, &ThemeViewFlags::default(), &root).unwrap();

        let path = root.join("Legacy.toml");
        let saved = std::fs::read_to_string(&path).unwrap();
        std::fs::write(
            &path,
            saved.replace(
                r#"effects_background = "image""#,
                r#"effects_background = "aurora""#,
            ),
        )
        .unwrap();

        let themes = load_user_themes_from_dir(&root);
        assert_eq!(themes.len(), 1);
        assert_eq!(themes[0].0.effects.background, "none");

        let _ = std::fs::remove_dir_all(root);
    }

    /// Write a real PNG so the blur path exercises actual decoding rather
    /// than a stub the image crate would reject.
    fn write_checkerboard_png(path: &Path, width: u32, height: u32) {
        let mut buffer = image::RgbaImage::new(width, height);
        for (x, y, pixel) in buffer.enumerate_pixels_mut() {
            // A hard checkerboard: high-frequency detail a blur must visibly
            // destroy, which is what the variance assertion below measures.
            let on = ((x / 4) + (y / 4)) % 2 == 0;
            *pixel = image::Rgba(if on {
                [255, 255, 255, 255]
            } else {
                [0, 0, 0, 255]
            });
        }
        buffer.save(path).expect("write test png");
    }

    #[test]
    fn blurred_background_downscales_and_smooths() {
        let root = std::env::temp_dir().join(format!("llnzy-blur-build-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create root");
        let source = root.join("source.png");
        write_checkerboard_png(&source, 1024, 512);

        let cache = root.join("cache");
        let blurred = build_blurred_background_in_dir(&source, 8.0, 80, &cache)
            .expect("blurred image is built");
        assert!(blurred.is_file());

        let output = image::open(&blurred).expect("blurred image decodes");
        assert_eq!(output.width(), BLUR_WORK_WIDTH);
        assert_eq!(output.height(), BLUR_WORK_WIDTH / 2, "aspect ratio is kept");

        // The checkerboard is pure black and white; a real blur has to pull
        // interior pixels toward mid grey. Sample away from the edges, where
        // the blur kernel clamps and can preserve extremes.
        let rgba = output.to_rgba8();
        let mut extremes = 0usize;
        let mut sampled = 0usize;
        for y in (rgba.height() / 4)..(rgba.height() * 3 / 4) {
            for x in (rgba.width() / 4)..(rgba.width() * 3 / 4) {
                let value = rgba.get_pixel(x, y).0[0];
                if !(16..=239).contains(&value) {
                    extremes += 1;
                }
                sampled += 1;
            }
        }
        assert!(sampled > 0);
        assert!(
            extremes * 10 < sampled,
            "expected the blur to remove the checkerboard's extremes, \
             got {extremes} of {sampled} still black or white"
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn blurred_background_reuses_the_cached_file() {
        let root = std::env::temp_dir().join(format!("llnzy-blur-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create root");
        let source = root.join("source.png");
        write_checkerboard_png(&source, 64, 64);
        let cache = root.join("cache");

        let first = build_blurred_background_in_dir(&source, 8.0, 80, &cache).expect("first build");
        let marker = b"cached, not rebuilt";
        std::fs::write(&first, marker).expect("overwrite cache entry");

        let second =
            build_blurred_background_in_dir(&source, 8.0, 80, &cache).expect("second build");
        assert_eq!(first, second);
        assert_eq!(
            std::fs::read(&second).expect("read cache entry"),
            marker,
            "a cache hit must not re-encode the image"
        );

        // A different sigma is a different entry.
        let other = build_blurred_background_in_dir(&source, 2.0, 20, &cache).expect("other sigma");
        assert_ne!(first, other);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn blurred_background_reports_unreadable_sources() {
        let root = std::env::temp_dir().join(format!("llnzy-blur-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create root");
        let source = root.join("not-an-image.png");
        std::fs::write(&source, b"definitely not a png").expect("write junk");

        assert!(
            build_blurred_background_in_dir(&source, 8.0, 80, &root.join("cache")).is_none(),
            "an undecodable source must fall back rather than panic"
        );

        let _ = std::fs::remove_dir_all(root);
    }
}
