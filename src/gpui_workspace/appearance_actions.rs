use gpui::Context;

use crate::{
    config::{editor_syntax_preset, BackgroundImageFit, CursorStyle, TerminalLayoutMode},
    sketch::SketchToolbarPosition,
    theme::builtin_themes,
};

use super::{
    appearances::{gpui_terminal_background_reference, is_display_font},
    AppearancePage, WorkspacePalette, WorkspacePrototype,
};

impl WorkspacePrototype {
    pub(super) fn apply_appearance_config(&mut self, cx: &mut Context<Self>) {
        let config = self.appearance_config.clone();
        let light_mode = WorkspacePalette::from_config(&config).is_light;
        let shared_config = std::sync::Arc::new(config);
        self.stacker
            .update(cx, |stacker, cx| stacker.set_light_mode(light_mode, cx));
        self.sketch
            .update(cx, |sketch, cx| sketch.set_light_mode(light_mode, cx));
        for editor in self.editor_entities() {
            let config = (*shared_config).clone();
            editor.update(cx, |editor, cx| editor.set_appearance_config(config, cx));
        }
        for terminal in self.terminals.values() {
            let config = std::sync::Arc::clone(&shared_config);
            terminal.update(cx, |terminal, cx| terminal.set_config(config, cx));
        }
        cx.notify();
    }

    pub(super) fn set_appearance_page(&mut self, page: AppearancePage, cx: &mut Context<Self>) {
        self.appearance_page = page;
        cx.notify();
    }

    pub(super) fn set_sketch_toolbar_position(
        &mut self,
        position: SketchToolbarPosition,
        cx: &mut Context<Self>,
    ) {
        self.sketch
            .update(cx, |sketch, cx| sketch.set_toolbar_position(position, cx));
        cx.notify();
    }

    pub(super) fn apply_builtin_theme(&mut self, theme_name: &str, cx: &mut Context<Self>) {
        if let Some(theme) = builtin_themes()
            .into_iter()
            .find(|theme| theme.name == theme_name)
        {
            let theme_name = theme.name.clone();
            let terminal_effects = self.appearance_config.effects.clone();
            let preserve_terminal_effects = theme.preserve_terminal_effects;
            theme.apply_to(&mut self.appearance_config);
            if preserve_terminal_effects {
                self.appearance_config.effects = terminal_effects;
            }
            self.preferences.app_theme = Some(theme_name);
            self.preferences.save();
            self.apply_appearance_config(cx);
        }
    }

    pub(super) fn adjust_font_size(&mut self, delta: f32, cx: &mut Context<Self>) {
        self.appearance_config.font_size =
            (self.appearance_config.font_size + delta).clamp(8.0, 40.0);
        self.apply_appearance_config(cx);
    }

    pub(super) fn adjust_line_height(&mut self, delta: f32, cx: &mut Context<Self>) {
        self.appearance_config.line_height =
            (self.appearance_config.line_height + delta).clamp(0.9, 2.2);
        self.apply_appearance_config(cx);
    }

    pub(super) fn set_terminal_font_family(
        &mut self,
        family: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.appearance_config.font_family = family.clone();
        self.preferences.terminal_font_family = family;
        self.preferences.save();
        self.apply_appearance_config(cx);
    }

    pub(super) fn set_terminal_layout_mode(
        &mut self,
        mode: TerminalLayoutMode,
        cx: &mut Context<Self>,
    ) {
        self.appearance_config.terminal_layout = mode;
        self.preferences.terminal_layout = mode.as_str().to_string();
        // If the active font doesn't belong to the new mode's font list,
        // clear it so the picker below isn't showing a selection from a
        // hidden row. Display fonts are explicit; anything else is treated
        // as monospace.
        let belongs = match (mode, self.appearance_config.font_family.as_deref()) {
            (_, None) => true,
            (TerminalLayoutMode::Display, Some(family)) => is_display_font(family),
            (TerminalLayoutMode::Monospace, Some(family)) => !is_display_font(family),
        };
        if !belongs {
            self.appearance_config.font_family = None;
            self.preferences.terminal_font_family = None;
        }
        self.preferences.save();
        self.apply_appearance_config(cx);
    }

    pub(super) fn set_cursor_style(&mut self, style: CursorStyle, cx: &mut Context<Self>) {
        self.appearance_config.cursor_style = style;
        self.apply_appearance_config(cx);
    }

    pub(super) fn set_background_mode(&mut self, mode: &'static str, cx: &mut Context<Self>) {
        self.appearance_config.effects.background = mode.to_string();
        self.terminal_background_import_error = None;
        self.apply_appearance_config(cx);
    }

    pub(super) fn import_terminal_background(&mut self, cx: &mut Context<Self>) {
        cx.spawn(
            |workspace: gpui::WeakEntity<WorkspacePrototype>, cx: &mut gpui::AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    let Some(file) = rfd::AsyncFileDialog::new()
                        .add_filter("Images", &["png", "jpg", "jpeg", "bmp", "webp", "gif"])
                        .pick_file()
                        .await
                    else {
                        return;
                    };

                    let import_result = crate::theme_store::import_background(file.path())
                        .and_then(|saved_path| gpui_terminal_background_reference(&saved_path));

                    let _ = workspace.update(&mut cx, |workspace, cx| match import_result {
                        Ok(reference) => {
                            workspace.terminal_background_import_error = None;
                            workspace.appearance_config.effects.enabled = true;
                            workspace.appearance_config.effects.background = "image".to_string();
                            workspace.appearance_config.effects.background_image =
                                Some(reference.clone());
                            workspace.preferences.terminal_background_image = Some(reference);
                            workspace.preferences.save();
                            workspace.apply_appearance_config(cx);
                        }
                        Err(error) => {
                            workspace.terminal_background_import_error = Some(error);
                            cx.notify();
                        }
                    });
                }
            },
        )
        .detach();
    }

    pub(super) fn clear_terminal_background_image(&mut self, cx: &mut Context<Self>) {
        self.terminal_background_import_error = None;
        self.appearance_config.effects.background_image = None;
        if self.appearance_config.effects.background == "image" {
            self.appearance_config.effects.background = "none".to_string();
        }
        self.preferences.terminal_background_image = None;
        self.preferences.save();
        self.apply_appearance_config(cx);
    }

    pub(super) fn set_background_image_fit(
        &mut self,
        fit: BackgroundImageFit,
        cx: &mut Context<Self>,
    ) {
        self.appearance_config.effects.background_image_fit = fit;
        self.preferences.terminal_background_image_fit = fit.as_str().to_string();
        self.preferences.save();
        self.apply_appearance_config(cx);
    }

    /// Apply an image from the saved background library as the active
    /// terminal background. Used by the per-row "Apply" button in the
    /// Appearances library list.
    pub(super) fn apply_library_background(
        &mut self,
        image_path: std::path::PathBuf,
        cx: &mut Context<Self>,
    ) {
        let reference = match super::appearances::gpui_terminal_background_reference(&image_path) {
            Ok(reference) => reference,
            Err(error) => {
                self.terminal_background_import_error = Some(error);
                cx.notify();
                return;
            }
        };
        self.terminal_background_import_error = None;
        self.appearance_config.effects.enabled = true;
        self.appearance_config.effects.background = "image".to_string();
        self.appearance_config.effects.background_image = Some(reference.clone());
        self.preferences.terminal_background_image = Some(reference);
        self.preferences.save();
        self.apply_appearance_config(cx);
    }

    /// Delete a saved background image from the library. If it was the
    /// currently active image, also clear the active selection so the
    /// terminal stops trying to render it.
    pub(super) fn delete_library_background(
        &mut self,
        image_path: std::path::PathBuf,
        cx: &mut Context<Self>,
    ) {
        let was_active = match (
            super::appearances::gpui_terminal_background_reference(&image_path).ok(),
            self.appearance_config.effects.background_image.as_deref(),
        ) {
            (Some(reference), Some(active)) => reference == active,
            _ => false,
        };

        if let Err(error) = crate::theme_store::delete_background(&image_path) {
            self.terminal_background_import_error = Some(error);
            cx.notify();
            return;
        }

        self.terminal_background_import_error = None;
        if was_active {
            self.appearance_config.effects.background_image = None;
            if self.appearance_config.effects.background == "image" {
                self.appearance_config.effects.background = "none".to_string();
            }
            self.preferences.terminal_background_image = None;
            self.preferences.save();
        }
        self.apply_appearance_config(cx);
    }

    pub(super) fn toggle_bloom(&mut self, cx: &mut Context<Self>) {
        self.appearance_config.effects.bloom_enabled =
            !self.appearance_config.effects.bloom_enabled;
        self.apply_appearance_config(cx);
    }

    pub(super) fn toggle_crt(&mut self, cx: &mut Context<Self>) {
        self.appearance_config.effects.crt_enabled = !self.appearance_config.effects.crt_enabled;
        self.apply_appearance_config(cx);
    }

    pub(super) fn toggle_particles(&mut self, cx: &mut Context<Self>) {
        self.appearance_config.effects.particles_enabled =
            !self.appearance_config.effects.particles_enabled;
        self.apply_appearance_config(cx);
    }

    pub(super) fn toggle_cursor_glow(&mut self, cx: &mut Context<Self>) {
        self.appearance_config.effects.cursor_glow = !self.appearance_config.effects.cursor_glow;
        self.apply_appearance_config(cx);
    }

    pub(super) fn toggle_cursor_trail(&mut self, cx: &mut Context<Self>) {
        self.appearance_config.effects.cursor_trail = !self.appearance_config.effects.cursor_trail;
        self.apply_appearance_config(cx);
    }

    pub(super) fn toggle_text_animation(&mut self, cx: &mut Context<Self>) {
        self.appearance_config.effects.text_animation =
            !self.appearance_config.effects.text_animation;
        self.apply_appearance_config(cx);
    }

    pub(super) fn toggle_effects_enabled(&mut self, cx: &mut Context<Self>) {
        self.appearance_config.effects.enabled = !self.appearance_config.effects.enabled;
        self.apply_appearance_config(cx);
    }

    pub(super) fn adjust_editor_font_size(&mut self, delta: f32, cx: &mut Context<Self>) {
        let current = self
            .appearance_config
            .editor
            .font_size
            .unwrap_or((self.appearance_config.font_size - 2.0).max(10.0));
        self.appearance_config.editor.font_size = Some((current + delta).clamp(8.0, 28.0));
        self.apply_appearance_config(cx);
    }

    pub(super) fn adjust_editor_line_height(&mut self, delta: f32, cx: &mut Context<Self>) {
        self.appearance_config.editor.line_height =
            (self.appearance_config.editor.line_height + delta).clamp(1.0, 2.2);
        self.apply_appearance_config(cx);
    }

    pub(super) fn adjust_sidebar_font_size(&mut self, delta: f32, cx: &mut Context<Self>) {
        self.appearance_config.editor.sidebar_font_size =
            (self.appearance_config.editor.sidebar_font_size + delta).clamp(8.0, 24.0);
        cx.notify();
    }

    pub(super) fn apply_editor_syntax_theme(&mut self, theme_name: &str, cx: &mut Context<Self>) {
        if let Some(theme) = editor_syntax_preset(theme_name) {
            self.appearance_config.syntax_colors = theme.colors_map();
            self.preferences.editor_syntax_theme = Some(theme.name.to_string());
            self.preferences.save();
            self.apply_appearance_config(cx);
        }
    }

    pub(super) fn adjust_selection_alpha(&mut self, delta: f32, cx: &mut Context<Self>) {
        self.appearance_config.colors.selection_alpha =
            (self.appearance_config.colors.selection_alpha + delta).clamp(0.05, 1.0);
        self.apply_appearance_config(cx);
    }

    pub(super) fn adjust_effect_intensity(&mut self, delta: f32, cx: &mut Context<Self>) {
        let next = (self.appearance_config.effects.background_intensity + delta).clamp(0.05, 1.0);
        self.appearance_config.effects.background_intensity = next;
        self.preferences.terminal_background_intensity = Some(next);
        self.preferences.save();
        self.apply_appearance_config(cx);
    }

    pub(super) fn set_effect_palette(
        &mut self,
        c1: [u8; 3],
        c2: [u8; 3],
        c3: [u8; 3],
        cx: &mut Context<Self>,
    ) {
        self.appearance_config.effects.background_color = Some(c1);
        self.appearance_config.effects.background_color2 = Some(c2);
        self.appearance_config.effects.background_color3 = Some(c3);
        self.preferences.terminal_palette = Some([c1, c2, c3]);
        self.preferences.save();
        self.apply_appearance_config(cx);
    }

    pub(super) fn toggle_time_of_day(&mut self, cx: &mut Context<Self>) {
        self.appearance_config.time_of_day_enabled = !self.appearance_config.time_of_day_enabled;
        cx.notify();
    }
}
