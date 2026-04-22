use std::sync::Arc;
use winit::window::Window;

use crate::config::Config;

/// Which settings tab is active.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsTab {
    Background,
    Text,
}

/// State for the egui-driven UI overlay.
pub struct UiState {
    pub ctx: egui::Context,
    pub winit_state: egui_winit::State,
    pub wgpu_renderer: egui_wgpu::Renderer,
    pub settings_open: bool,
    pub settings_tab: SettingsTab,
    /// Flip animation: 0.0 = terminal, 1.0 = settings
    pub flip_t: f32,
    pub flip_target_open: bool,
    /// How much vertical space the footer occupies (for terminal content layout)
    pub footer_height: f32,
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
            settings_open: false,
            settings_tab: SettingsTab::Background,
            flip_t: 0.0,
            flip_target_open: false,
            footer_height: 36.0,
        }
    }

    /// Pass a winit event to egui. Returns true if egui consumed it.
    pub fn handle_event(&mut self, window: &Window, event: &winit::event::WindowEvent) -> bool {
        let response = self.winit_state.on_window_event(window, event);
        response.consumed
    }

    /// Toggle settings open/closed.
    pub fn toggle_settings(&mut self) {
        self.settings_open = !self.settings_open;
    }

    /// Run the egui frame and render. Returns the clipping info for the footer.
    /// Call this AFTER rendering the terminal content to the swapchain.
    /// Run the egui frame and render to the swapchain.
    /// Creates its own command encoder and submits it.
    pub fn render(
        &mut self,
        window: &Window,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _encoder: &mut wgpu::CommandEncoder, // unused — we create our own for lifetime reasons
        view: &wgpu::TextureView,
        screen_desc: egui_wgpu::ScreenDescriptor,
        config: &Config,
    ) {
        let raw_input = self.winit_state.take_egui_input(window);

        // Extract state to avoid borrowing self inside the closure
        let settings_open = self.settings_open;
        let footer_height = self.footer_height;
        let mut settings_tab = self.settings_tab;
        let mut nav_target: Option<bool> = None; // Some(true) = open settings, Some(false) = close
        let config_clone = config.clone();

        let full_output = self.ctx.run(raw_input, |ctx| {
            // ── Footer nav bar (ALWAYS visible) ──
            egui::TopBottomPanel::bottom("footer")
                .exact_height(footer_height)
                .frame(egui::Frame::none().fill(egui::Color32::from_rgb(28, 28, 36)))
                .show(ctx, |ui| {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        // Shells button — navigates to terminal
                        let shells_text = egui::RichText::new("Shells").size(16.0);
                        let shells_btn = if !settings_open {
                            ui.add(egui::Button::new(shells_text.color(egui::Color32::WHITE))
                                .fill(egui::Color32::from_rgb(50, 50, 65)))
                        } else {
                            ui.button(shells_text)
                        };
                        if shells_btn.clicked() && settings_open {
                            nav_target = Some(false);
                        }

                        // Stacker button — placeholder
                        if ui.button(egui::RichText::new("Stacker").size(16.0)).clicked() {
                            // Placeholder
                        }

                        // Settings button — navigates to settings
                        let settings_text = egui::RichText::new("Settings").size(16.0);
                        let settings_btn = if settings_open {
                            ui.add(egui::Button::new(settings_text.color(egui::Color32::WHITE))
                                .fill(egui::Color32::from_rgb(50, 50, 65)))
                        } else {
                            ui.button(settings_text)
                        };
                        if settings_btn.clicked() && !settings_open {
                            nav_target = Some(true);
                        }
                    });
                });

            // ── Settings panel (when open) ──
            if settings_open {
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(egui::Color32::from_rgb(20, 20, 26)))
                    .show(ctx, |ui| {
                        render_settings_panel_static(ui, &config_clone, &mut settings_tab, &mut false);
                    });
            }
        });

        // Apply navigation
        self.settings_tab = settings_tab;
        if let Some(open) = nav_target {
            self.settings_open = open;
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

fn render_settings_panel_static(
    ui: &mut egui::Ui,
    config: &Config,
    settings_tab: &mut SettingsTab,
    toggle_requested: &mut bool,
) {
        // Header
        ui.heading(
            egui::RichText::new("Settings")
                .size(20.0)
                .color(egui::Color32::WHITE),
        );
        ui.separator();

        ui.horizontal(|ui| {
            // Sidebar
            ui.vertical(|ui| {
                ui.set_width(160.0);
                ui.add_space(8.0);

                let bg_btn = ui.selectable_label(
                    *settings_tab == SettingsTab::Background,
                    egui::RichText::new("Background").size(14.0),
                );
                if bg_btn.clicked() {
                    *settings_tab = SettingsTab::Background;
                }

                let text_btn = ui.selectable_label(
                    *settings_tab == SettingsTab::Text,
                    egui::RichText::new("Text").size(14.0),
                );
                if text_btn.clicked() {
                    *settings_tab = SettingsTab::Text;
                }
            });

            ui.separator();

            // Content
            ui.vertical(|ui| {
                ui.add_space(8.0);
                match *settings_tab {
                    SettingsTab::Background => {
                        ui.label(
                            egui::RichText::new("Background Effects")
                                .size(16.0)
                                .color(egui::Color32::WHITE),
                        );
                        ui.add_space(12.0);

                        egui::Grid::new("bg_settings")
                            .num_columns(2)
                            .spacing([20.0, 8.0])
                            .show(ui, |ui| {
                                ui.label("Background:");
                                ui.label(&config.effects.background);
                                ui.end_row();

                                ui.label("Intensity:");
                                ui.label(format!("{:.1}", config.effects.background_intensity));
                                ui.end_row();

                                ui.label("Speed:");
                                ui.label(format!("{:.1}", config.effects.background_speed));
                                ui.end_row();

                                ui.label("Bloom:");
                                ui.label(if config.effects.bloom_enabled {
                                    "ON"
                                } else {
                                    "OFF"
                                });
                                ui.end_row();

                                ui.label("Bloom Threshold:");
                                ui.label(format!("{:.2}", config.effects.bloom_threshold));
                                ui.end_row();

                                ui.label("Bloom Intensity:");
                                ui.label(format!("{:.1}", config.effects.bloom_intensity));
                                ui.end_row();

                                ui.label("Particles:");
                                ui.label(if config.effects.particles_enabled {
                                    "ON"
                                } else {
                                    "OFF"
                                });
                                ui.end_row();

                                ui.label("Particle Count:");
                                ui.label(format!("{}", config.effects.particles_count));
                                ui.end_row();
                            });
                    }
                    SettingsTab::Text => {
                        ui.label(
                            egui::RichText::new("Text & Font")
                                .size(16.0)
                                .color(egui::Color32::WHITE),
                        );
                        ui.add_space(12.0);

                        egui::Grid::new("text_settings")
                            .num_columns(2)
                            .spacing([20.0, 8.0])
                            .show(ui, |ui| {
                                ui.label("Font Size:");
                                ui.label(format!("{:.0}", config.font_size));
                                ui.end_row();

                                ui.label("Font Family:");
                                ui.label(
                                    config
                                        .font_family
                                        .as_deref()
                                        .unwrap_or("JetBrains Mono"),
                                );
                                ui.end_row();

                                ui.label("Line Height:");
                                ui.label(format!("{:.1}", config.line_height));
                                ui.end_row();

                                ui.label("Ligatures:");
                                ui.label(if config.ligatures { "ON" } else { "OFF" });
                                ui.end_row();

                                ui.label("Text Animation:");
                                ui.label(if config.effects.text_animation {
                                    "ON"
                                } else {
                                    "OFF"
                                });
                                ui.end_row();

                                ui.label("Cursor Glow:");
                                ui.label(if config.effects.cursor_glow {
                                    "ON"
                                } else {
                                    "OFF"
                                });
                                ui.end_row();

                                ui.label("Cursor Trail:");
                                ui.label(if config.effects.cursor_trail {
                                    "ON"
                                } else {
                                    "OFF"
                                });
                                ui.end_row();
                            });
                    }
                }
            });
        });
    }

