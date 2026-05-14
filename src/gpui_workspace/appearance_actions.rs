use gpui::Context;

use crate::{
    config::{BackgroundImageFit, CursorStyle, TerminalLayoutMode},
    theme::builtin_themes,
};

use super::{
    appearances::{gpui_terminal_background_reference, is_display_font},
    AppearancePage, WorkspacePrototype,
};

impl WorkspacePrototype {
    pub(super) fn apply_appearance_config(&mut self, cx: &mut Context<Self>) {
        let config = self.appearance_config.clone();
        self.editor.update(cx, |editor, cx| {
            editor.set_appearance_config(config.clone(), cx)
        });
        for editor in self.file_editors.values() {
            let config = config.clone();
            editor.update(cx, |editor, cx| editor.set_appearance_config(config, cx));
        }
        for terminal in self.terminals.values() {
            let config = config.clone();
            terminal.update(cx, |terminal, cx| terminal.set_config(config, cx));
        }
        cx.notify();
    }

    pub(super) fn set_appearance_page(&mut self, page: AppearancePage, cx: &mut Context<Self>) {
        self.appearance_page = page;
        cx.notify();
    }

    pub(super) fn apply_builtin_theme(&mut self, theme_name: &str, cx: &mut Context<Self>) {
        if let Some(theme) = builtin_themes()
            .into_iter()
            .find(|theme| theme.name == theme_name)
        {
            theme.apply_to(&mut self.appearance_config);
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
        self.appearance_config.font_family = family;
        self.apply_appearance_config(cx);
    }

    pub(super) fn set_terminal_layout_mode(
        &mut self,
        mode: TerminalLayoutMode,
        cx: &mut Context<Self>,
    ) {
        self.appearance_config.terminal_layout = mode;
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
        }
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
                            workspace.appearance_config.effects.background_image = Some(reference);
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
        self.apply_appearance_config(cx);
    }

    pub(super) fn set_background_image_fit(
        &mut self,
        fit: BackgroundImageFit,
        cx: &mut Context<Self>,
    ) {
        self.appearance_config.effects.background_image_fit = fit;
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

    pub(super) fn adjust_sidebar_font_size(&mut self, delta: f32, cx: &mut Context<Self>) {
        self.appearance_config.editor.sidebar_font_size =
            (self.appearance_config.editor.sidebar_font_size + delta).clamp(8.0, 24.0);
        cx.notify();
    }

    pub(super) fn adjust_selection_alpha(&mut self, delta: f32, cx: &mut Context<Self>) {
        self.appearance_config.colors.selection_alpha =
            (self.appearance_config.colors.selection_alpha + delta).clamp(0.05, 1.0);
        self.apply_appearance_config(cx);
    }

    pub(super) fn adjust_effect_intensity(&mut self, delta: f32, cx: &mut Context<Self>) {
        self.appearance_config.effects.background_intensity =
            (self.appearance_config.effects.background_intensity + delta).clamp(0.05, 1.0);
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
        self.apply_appearance_config(cx);
    }

    pub(super) fn toggle_time_of_day(&mut self, cx: &mut Context<Self>) {
        self.appearance_config.time_of_day_enabled = !self.appearance_config.time_of_day_enabled;
        cx.notify();
    }
}
