use winit::window::Window;

use crate::config::Config;

/// Which view is active.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveView {
    Shells,
    Stacker,
    Settings,
}

/// Which settings tab is active.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsTab {
    Background,
    Text,
}

/// A saved prompt in the stacker queue.
#[derive(Clone, Debug)]
pub struct StackerPrompt {
    pub text: String,
    pub label: String,
}

/// State for the egui-driven UI overlay.
pub struct UiState {
    pub ctx: egui::Context,
    pub winit_state: egui_winit::State,
    pub wgpu_renderer: egui_wgpu::Renderer,
    pub active_view: ActiveView,
    pub settings_tab: SettingsTab,
    /// How much vertical space the footer occupies (for terminal content layout)
    pub footer_height: f32,
    /// Config changes from the settings panel, to be applied by main loop
    pub pending_config: Option<Config>,
    /// Text copied to clipboard by Stacker (main loop applies it)
    pub clipboard_text: Option<String>,
    // Stacker state
    pub stacker_prompts: Vec<StackerPrompt>,
    pub stacker_input: String,
    pub stacker_label_input: String,
}

impl UiState {
    pub fn new(
        window: &Window,
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let ctx = egui::Context::default();

        // Style: dark theme with our terminal aesthetic
        let mut style = egui::Style::default();
        style.visuals = egui::Visuals::dark();
        style.visuals.window_rounding = egui::Rounding::same(4.0);
        style.visuals.button_frame = true;
        ctx.set_style(style);

        let viewport_id = ctx.viewport_id();
        let winit_state = egui_winit::State::new(
            ctx.clone(),
            viewport_id,
            window,
            None, // native_pixels_per_point — auto-detect
            None, // max_texture_side
            None, // max_image_side
        );

        let wgpu_renderer = egui_wgpu::Renderer::new(
            device,
            surface_format,
            None, // depth format
            1,    // sample count
            false,
        );

        UiState {
            ctx,
            winit_state,
            wgpu_renderer,
            active_view: ActiveView::Shells,
            settings_tab: SettingsTab::Background,
            footer_height: 36.0,
            pending_config: None,
            clipboard_text: None,
            stacker_prompts: Vec::new(),
            stacker_input: String::new(),
            stacker_label_input: String::new(),
        }
    }

    /// Pass a winit event to egui. Returns true if egui consumed it.
    pub fn handle_event(&mut self, window: &Window, event: &winit::event::WindowEvent) -> bool {
        let response = self.winit_state.on_window_event(window, event);
        response.consumed
    }

    /// Whether the terminal is covered by a full-screen view.
    pub fn settings_open(&self) -> bool {
        self.active_view != ActiveView::Shells
    }

    /// Run the egui frame and render. Returns the clipping info for the footer.
    /// Call this AFTER rendering the terminal content to the swapchain.
    /// Take pending config changes, if any.
    pub fn take_config(&mut self) -> Option<Config> {
        self.pending_config.take()
    }

    /// Run the egui frame and render to the swapchain.
    /// Creates its own command encoder and submits it.
    pub fn render(
        &mut self,
        window: &Window,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        screen_desc: egui_wgpu::ScreenDescriptor,
        config: &Config,
    ) {
        let raw_input = self.winit_state.take_egui_input(window);

        // Extract state to avoid borrowing self inside the closure
        let current_view = self.active_view;
        let footer_height = self.footer_height;
        let mut settings_tab = self.settings_tab;
        let mut nav_target: Option<ActiveView> = None;
        let mut config_clone = config.clone();
        let mut clipboard_copy: Option<String> = None;

        // Stacker state — extract for closure
        let mut stacker_prompts = std::mem::take(&mut self.stacker_prompts);
        let mut stacker_input = std::mem::take(&mut self.stacker_input);

        let full_output = self.ctx.run(raw_input, |ctx| {
            // ── Footer nav bar (ALWAYS visible) ──
            egui::TopBottomPanel::bottom("footer")
                .exact_height(footer_height)
                .frame(egui::Frame::none().fill(egui::Color32::from_rgb(28, 28, 36)))
                .show(ctx, |ui| {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);

                        let views = [
                            ("Shells", ActiveView::Shells),
                            ("Stacker", ActiveView::Stacker),
                            ("Settings", ActiveView::Settings),
                        ];

                        for (name, view) in views {
                            let text = egui::RichText::new(name).size(16.0);
                            let btn = if current_view == view {
                                ui.add(egui::Button::new(text.color(egui::Color32::WHITE))
                                    .fill(egui::Color32::from_rgb(50, 50, 65)))
                            } else {
                                ui.button(text)
                            };
                            if btn.clicked() && current_view != view {
                                nav_target = Some(view);
                            }
                        }
                    });
                });

            // ── Settings view ──
            if current_view == ActiveView::Settings {
                egui::SidePanel::left("settings_sidebar")
                    .exact_width(170.0)
                    .frame(egui::Frame::none().fill(egui::Color32::from_rgb(24, 24, 32))
                        .inner_margin(egui::Margin::same(12.0)))
                    .show(ctx, |ui| {
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("Settings").size(22.0).color(egui::Color32::WHITE));
                        ui.add_space(16.0);

                        if ui.selectable_label(settings_tab == SettingsTab::Background, label("Background")).clicked() {
                            settings_tab = SettingsTab::Background;
                        }
                        ui.add_space(4.0);
                        if ui.selectable_label(settings_tab == SettingsTab::Text, label("Text")).clicked() {
                            settings_tab = SettingsTab::Text;
                        }
                    });

                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(egui::Color32::from_rgb(20, 20, 26))
                        .inner_margin(egui::Margin::same(20.0)))
                    .show(ctx, |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            match settings_tab {
                                SettingsTab::Background => render_background_tab(ui, &mut config_clone),
                                SettingsTab::Text => render_text_tab(ui, &mut config_clone),
                            }
                        });
                    });
            }

            // ── Stacker view ──
            if current_view == ActiveView::Stacker {
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(egui::Color32::from_rgb(20, 20, 26))
                        .inner_margin(egui::Margin::same(20.0)))
                    .show(ctx, |ui| {
                        ui.label(egui::RichText::new("Stacker — Prompt Queue")
                            .size(22.0).color(egui::Color32::WHITE));
                        ui.add_space(12.0);

                        // ── Input area ──
                        ui.group(|ui| {
                            ui.label(label("New Prompt"));
                            ui.add_space(4.0);

                            ui.add(
                                egui::TextEdit::multiline(&mut stacker_input)
                                    .desired_rows(4)
                                    .desired_width(f32::INFINITY)
                                    .hint_text("Type or paste your prompt here...")
                                    .font(egui::TextStyle::Monospace),
                            );
                            ui.add_space(8.0);

                            if ui.add_enabled(
                                !stacker_input.trim().is_empty(),
                                egui::Button::new(label("Save to Queue")),
                            ).clicked() {
                                let words: String = stacker_input.split_whitespace().take(6).collect::<Vec<_>>().join(" ");
                                let prompt_label = if words.len() < stacker_input.trim().len() {
                                    format!("{}...", words)
                                } else {
                                    words
                                };
                                stacker_prompts.push(StackerPrompt {
                                    text: stacker_input.trim().to_string(),
                                    label: prompt_label,
                                });
                                stacker_input.clear();
                            }
                        });

                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(8.0);

                        // ── Prompt queue ──
                        ui.label(egui::RichText::new(format!("Queue ({})", stacker_prompts.len()))
                            .size(18.0).color(egui::Color32::WHITE));
                        ui.add_space(8.0);

                        if stacker_prompts.is_empty() {
                            ui.label(label("No prompts saved yet. Add one above."));
                        }

                        let mut remove_idx: Option<usize> = None;
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for (i, prompt) in stacker_prompts.iter().enumerate() {
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(&prompt.label)
                                            .size(15.0).color(egui::Color32::WHITE).strong());

                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if ui.small_button("Delete").clicked() {
                                                remove_idx = Some(i);
                                            }
                                            if ui.button(label("Copy")).clicked() {
                                                clipboard_copy = Some(prompt.text.clone());
                                            }
                                        });
                                    });

                                    // Show preview of prompt text
                                    let preview: String = prompt.text.lines().take(3).collect::<Vec<_>>().join("\n");
                                    ui.label(egui::RichText::new(preview)
                                        .size(13.0).color(egui::Color32::from_rgb(160, 160, 170))
                                        .monospace());
                                });
                                ui.add_space(4.0);
                            }
                        });

                        if let Some(idx) = remove_idx {
                            stacker_prompts.remove(idx);
                        }
                    });
            }
        });

        // Apply state changes
        self.settings_tab = settings_tab;
        self.stacker_prompts = stacker_prompts;
        self.stacker_input = stacker_input;

        if let Some(view) = nav_target {
            self.active_view = view;
        }
        if let Some(text) = clipboard_copy {
            self.clipboard_text = Some(text);
        }

        // Push config changes when on settings view
        if current_view == ActiveView::Settings {
            self.pending_config = Some(config_clone);
        }

        self.winit_state
            .handle_platform_output(window, full_output.platform_output);

        let tris = self
            .ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        for (id, image_delta) in &full_output.textures_delta.set {
            self.wgpu_renderer
                .update_texture(device, queue, *id, image_delta);
        }

        // egui needs its own encoder due to wgpu 22 RenderPass lifetime requirements
        let mut egui_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("egui_encoder"),
        });

        self.wgpu_renderer
            .update_buffers(device, queue, &mut egui_encoder, &tris, &screen_desc);

        // Submit the update buffers encoder first
        queue.submit(std::iter::once(egui_encoder.finish()));

        // Render egui using a raw render pass.
        // wgpu 22's RenderPass has 'static lifetime, so we create+finish the encoder
        // in a way that satisfies the borrow checker.
        {
            let mut render_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("egui_render"),
            });
            let mut pass = render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            let mut static_pass = pass.forget_lifetime();
            self.wgpu_renderer.render(&mut static_pass, &tris, &screen_desc);
            drop(static_pass);
            queue.submit(std::iter::once(render_encoder.finish()));
        }

        for id in &full_output.textures_delta.free {
            self.wgpu_renderer.free_texture(id);
        }
    }

}

const S: f32 = 16.0; // settings panel font size

fn label(text: &str) -> egui::RichText {
    egui::RichText::new(text).size(S)
}

fn render_background_tab(ui: &mut egui::Ui, config: &mut Config) {
    ui.label(
        egui::RichText::new("Background Effects")
            .size(18.0)
            .color(egui::Color32::WHITE),
    );
    ui.add_space(12.0);

    egui::Grid::new("bg_settings")
        .num_columns(2)
        .spacing([24.0, 10.0])
        .show(ui, |ui| {
            // Background type
            ui.label(label("Background"));
            egui::ComboBox::from_id_salt("bg_type")
                .selected_text(label(&config.effects.background))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut config.effects.background, "smoke".to_string(), "smoke");
                    ui.selectable_value(&mut config.effects.background, "none".to_string(), "none");
                });
            ui.end_row();

            // Intensity slider
            ui.label(label("Intensity"));
            ui.add(egui::Slider::new(&mut config.effects.background_intensity, 0.0..=1.0).text(""));
            ui.end_row();

            // Speed slider
            ui.label(label("Speed"));
            ui.add(egui::Slider::new(&mut config.effects.background_speed, 0.1..=5.0).text(""));
            ui.end_row();
        });

    ui.add_space(16.0);
    ui.separator();
    ui.add_space(8.0);

    egui::CollapsingHeader::new(
        egui::RichText::new("Bloom / Glow").size(18.0).color(egui::Color32::WHITE),
    )
    .default_open(false)
    .show(ui, |ui| {
        ui.add_space(8.0);
        egui::Grid::new("bloom_settings")
            .num_columns(2)
            .spacing([24.0, 10.0])
            .show(ui, |ui| {
                ui.label(label("Enabled"));
                ui.add(egui::Checkbox::without_text(&mut config.effects.bloom_enabled));
                ui.end_row();

                ui.label(label("Threshold"));
                ui.add(egui::Slider::new(&mut config.effects.bloom_threshold, 0.1..=0.9).text(""));
                ui.end_row();

                ui.label(label("Intensity"));
                ui.add(egui::Slider::new(&mut config.effects.bloom_intensity, 0.0..=2.0).text(""));
                ui.end_row();

                ui.label(label("Radius"));
                ui.add(egui::Slider::new(&mut config.effects.bloom_radius, 0.5..=5.0).text(""));
                ui.end_row();
            });
    });

    ui.add_space(16.0);
    ui.separator();
    ui.add_space(8.0);

    egui::CollapsingHeader::new(
        egui::RichText::new("Particles").size(18.0).color(egui::Color32::WHITE),
    )
    .default_open(false)
    .show(ui, |ui| {
        ui.add_space(8.0);
        egui::Grid::new("particle_settings")
            .num_columns(2)
            .spacing([24.0, 10.0])
            .show(ui, |ui| {
                ui.label(label("Enabled"));
                ui.add(egui::Checkbox::without_text(&mut config.effects.particles_enabled));
                ui.end_row();

                let mut count = config.effects.particles_count as f32;
                ui.label(label("Count"));
                if ui.add(egui::Slider::new(&mut count, 0.0..=4096.0).text("")).changed() {
                    config.effects.particles_count = count as u32;
                }
                ui.end_row();

                ui.label(label("Speed"));
                ui.add(egui::Slider::new(&mut config.effects.particles_speed, 0.0..=5.0).text(""));
                ui.end_row();
            });
    });

    ui.add_space(16.0);
    ui.separator();
    ui.add_space(8.0);

    egui::CollapsingHeader::new(
        egui::RichText::new("CRT / Retro").size(18.0).color(egui::Color32::WHITE),
    )
    .default_open(false)
    .show(ui, |ui| {
        ui.add_space(8.0);
        egui::Grid::new("crt_settings")
            .num_columns(2)
            .spacing([24.0, 10.0])
            .show(ui, |ui| {
                ui.label(label("Enabled"));
                ui.add(egui::Checkbox::without_text(&mut config.effects.crt_enabled));
                ui.end_row();

                ui.label(label("Scanlines"));
                ui.add(egui::Slider::new(&mut config.effects.scanline_intensity, 0.0..=1.0).text(""));
                ui.end_row();

                ui.label(label("Curvature"));
                ui.add(egui::Slider::new(&mut config.effects.curvature, 0.0..=0.5).text(""));
                ui.end_row();

                ui.label(label("Vignette"));
                ui.add(egui::Slider::new(&mut config.effects.vignette_strength, 0.0..=2.0).text(""));
                ui.end_row();

                ui.label(label("Chromatic Aberration"));
                ui.add(egui::Slider::new(&mut config.effects.chromatic_aberration, 0.0..=5.0).text(""));
                ui.end_row();

                ui.label(label("Film Grain"));
                ui.add(egui::Slider::new(&mut config.effects.grain_intensity, 0.0..=0.5).text(""));
                ui.end_row();
            });
    });
}

fn render_text_tab(ui: &mut egui::Ui, config: &mut Config) {
    ui.label(
        egui::RichText::new("Cursor & Animation")
            .size(18.0)
            .color(egui::Color32::WHITE),
    );
    ui.add_space(12.0);

    egui::Grid::new("cursor_settings")
        .num_columns(2)
        .spacing([24.0, 10.0])
        .show(ui, |ui| {
            ui.label(label("Text Animation"));
            ui.add(egui::Checkbox::without_text(&mut config.effects.text_animation));
            ui.end_row();

            ui.label(label("Cursor Glow"));
            ui.add(egui::Checkbox::without_text(&mut config.effects.cursor_glow));
            ui.end_row();

            ui.label(label("Cursor Trail"));
            ui.add(egui::Checkbox::without_text(&mut config.effects.cursor_trail));
            ui.end_row();

            // Cursor style
            ui.label(label("Cursor Style"));
            egui::ComboBox::from_id_salt("cursor_style")
                .selected_text(label(match config.cursor_style {
                    crate::config::CursorStyle::Block => "Block",
                    crate::config::CursorStyle::Beam => "Beam",
                    crate::config::CursorStyle::Underline => "Underline",
                }))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut config.cursor_style, crate::config::CursorStyle::Block, "Block");
                    ui.selectable_value(&mut config.cursor_style, crate::config::CursorStyle::Beam, "Beam");
                    ui.selectable_value(&mut config.cursor_style, crate::config::CursorStyle::Underline, "Underline");
                });
            ui.end_row();

            // Cursor blink rate
            let mut blink = config.cursor_blink_ms as f32;
            ui.label(label("Blink Rate"));
            if ui.add(egui::Slider::new(&mut blink, 0.0..=1500.0).text("ms")).changed() {
                config.cursor_blink_ms = blink as u64;
            }
            ui.end_row();
        });
}

