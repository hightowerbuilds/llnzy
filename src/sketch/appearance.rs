use std::path::{Path, PathBuf};

const DEFAULT_CANVAS_BACKGROUND_COLOR: [u8; 4] = [28, 30, 36, 255];
const DEFAULT_SELECTION_OUTLINE_COLOR: [u8; 4] = [60, 130, 255, 255];
const DEFAULT_GRID_SPACING: f32 = 24.0;
const MIN_GRID_SPACING: f32 = 4.0;
const MAX_GRID_SPACING: f32 = 256.0;
const DEFAULT_GRID_OPACITY: f32 = 0.18;
const DEFAULT_HANDLE_SIZE: f32 = 6.0;
const MIN_HANDLE_SIZE: f32 = 2.0;
const MAX_HANDLE_SIZE: f32 = 24.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SketchCanvasBackgroundMode {
    #[default]
    Theme,
    Solid,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SketchGridMode {
    #[default]
    Hidden,
    Lines,
    Dots,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SketchToolbarPosition {
    #[default]
    Top,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SketchAppearanceSettings {
    /// Theme preserves the existing canvas background path; Solid uses canvas_background_color.
    pub canvas_background_mode: SketchCanvasBackgroundMode,
    pub canvas_background_color: [u8; 4],
    pub grid_mode: SketchGridMode,
    pub grid_spacing: f32,
    pub grid_opacity: f32,
    pub selection_outline_color: [u8; 4],
    pub handle_size: f32,
    pub canvas_border_visible: bool,
    pub canvas_shadow_visible: bool,
    pub toolbar_position: SketchToolbarPosition,
}

impl Default for SketchAppearanceSettings {
    fn default() -> Self {
        Self {
            canvas_background_mode: SketchCanvasBackgroundMode::Theme,
            canvas_background_color: DEFAULT_CANVAS_BACKGROUND_COLOR,
            grid_mode: SketchGridMode::Hidden,
            grid_spacing: DEFAULT_GRID_SPACING,
            grid_opacity: DEFAULT_GRID_OPACITY,
            selection_outline_color: DEFAULT_SELECTION_OUTLINE_COLOR,
            handle_size: DEFAULT_HANDLE_SIZE,
            canvas_border_visible: true,
            canvas_shadow_visible: false,
            toolbar_position: SketchToolbarPosition::Top,
        }
    }
}

impl SketchAppearanceSettings {
    pub fn normalized(mut self) -> Self {
        self.grid_spacing = self.grid_spacing.clamp(MIN_GRID_SPACING, MAX_GRID_SPACING);
        self.grid_opacity = self.grid_opacity.clamp(0.0, 1.0);
        self.handle_size = self.handle_size.clamp(MIN_HANDLE_SIZE, MAX_HANDLE_SIZE);
        self
    }

    pub fn grid_visible(&self) -> bool {
        self.grid_mode != SketchGridMode::Hidden && self.effective_grid_opacity() > 0.0
    }

    pub fn effective_grid_spacing(&self) -> f32 {
        self.grid_spacing.clamp(MIN_GRID_SPACING, MAX_GRID_SPACING)
    }

    pub fn effective_grid_opacity(&self) -> f32 {
        self.grid_opacity.clamp(0.0, 1.0)
    }

    pub fn effective_handle_size(&self) -> f32 {
        self.handle_size.clamp(MIN_HANDLE_SIZE, MAX_HANDLE_SIZE)
    }
}

impl SketchGridMode {
    pub fn toggled_visibility(self) -> Self {
        match self {
            Self::Hidden => Self::Lines,
            Self::Lines | Self::Dots => Self::Hidden,
        }
    }
}

pub fn appearance_settings_path() -> Option<PathBuf> {
    crate::platform::paths::current_paths()
        .map(|paths| paths.sketches_dir().join("appearance.json"))
}

pub fn load_appearance_settings() -> SketchAppearanceSettings {
    appearance_settings_path()
        .and_then(|path| load_appearance_settings_from_path(&path).ok())
        .unwrap_or_default()
}

pub fn load_appearance_settings_from_path(path: &Path) -> Result<SketchAppearanceSettings, String> {
    let data = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let settings: SketchAppearanceSettings =
        serde_json::from_str(&data).map_err(|e| e.to_string())?;
    Ok(settings.normalized())
}

pub fn save_appearance_settings(settings: &SketchAppearanceSettings) -> Result<(), String> {
    let Some(path) = appearance_settings_path() else {
        return Ok(());
    };
    save_appearance_settings_to_path(settings, &path)
}

pub fn save_appearance_settings_to_path(
    settings: &SketchAppearanceSettings,
    path: &Path,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&settings.normalized()).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}
