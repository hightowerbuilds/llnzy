use winit::window::Window;

use crate::config::Config;
use crate::theme::builtin_themes;

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
    Themes,
    Background,
    Text,
}

/// A saved prompt in the stacker queue.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct StackerPrompt {
    pub text: String,
    pub label: String,
    #[serde(default)]
    pub category: String,
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
    // Debug overlay
    pub show_fps: bool,
    frame_times: std::collections::VecDeque<f32>,
    // Stacker state
    pub stacker_prompts: Vec<StackerPrompt>,
    pub stacker_input: String,
    pub stacker_label_input: String,
    pub stacker_category_input: String,
    pub stacker_search: String,
    pub stacker_filter_category: String, // empty = show all
    pub stacker_editing: Option<usize>,  // index of prompt being edited
    pub stacker_edit_text: String,
    pub stacker_dirty: bool,             // needs save to disk
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

        let stacker_prompts = load_stacker_prompts();

        UiState {
            ctx,
            winit_state,
            wgpu_renderer,
            active_view: ActiveView::Shells,
            settings_tab: SettingsTab::Themes,
            footer_height: 36.0,
            pending_config: None,
            clipboard_text: None,
            show_fps: false,
            frame_times: std::collections::VecDeque::with_capacity(120),
            stacker_prompts,
            stacker_input: String::new(),
            stacker_label_input: String::new(),
            stacker_category_input: String::new(),
            stacker_search: String::new(),
            stacker_filter_category: String::new(),
            stacker_editing: None,
            stacker_edit_text: String::new(),
            stacker_dirty: false,
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

    /// Record a frame time for the FPS overlay.
    pub fn record_frame_time(&mut self, dt: f32) {
        if self.frame_times.len() >= 120 {
            self.frame_times.pop_front();
        }
        self.frame_times.push_back(dt);
    }

    /// Run the egui frame and render to the swapchain.
    pub fn render(
        &mut self,
        window: &Window,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
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
        let mut stacker_category_input = std::mem::take(&mut self.stacker_category_input);
        let mut stacker_search = std::mem::take(&mut self.stacker_search);
        let mut stacker_filter_category = std::mem::take(&mut self.stacker_filter_category);
        let mut stacker_editing = self.stacker_editing;
        let mut stacker_edit_text = std::mem::take(&mut self.stacker_edit_text);
        let mut stacker_dirty = self.stacker_dirty;
        let show_fps = self.show_fps;
        let fps_info = if show_fps && !self.frame_times.is_empty() {
            let avg_dt: f32 = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
            let fps = if avg_dt > 0.0 { 1.0 / avg_dt } else { 0.0 };
            Some((fps, avg_dt * 1000.0))
        } else {
            None
        };

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

                        let tabs = [
                            ("Themes", SettingsTab::Themes),
                            ("Background", SettingsTab::Background),
                            ("Text", SettingsTab::Text),
                        ];
                        for (name, tab) in tabs {
                            if ui.selectable_label(settings_tab == tab, label(name)).clicked() {
                                settings_tab = tab;
                            }
                            ui.add_space(4.0);
                        }
                    });

                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(egui::Color32::from_rgb(20, 20, 26))
                        .inner_margin(egui::Margin::same(20.0)))
                    .show(ctx, |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            match settings_tab {
                                SettingsTab::Themes => render_themes_tab(ui, &mut config_clone),
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
                            ui.add_space(4.0);

                            ui.horizontal(|ui| {
                                ui.label(label("Category:"));
                                ui.add(egui::TextEdit::singleline(&mut stacker_category_input)
                                    .desired_width(150.0)
                                    .hint_text("optional"));
                                ui.add_space(16.0);
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
                                        category: stacker_category_input.trim().to_string(),
                                    });
                                    stacker_input.clear();
                                    stacker_category_input.clear();
                                    stacker_dirty = true;
                                }
                            });
                        });

                        ui.add_space(12.0);

                        // ── Search + filter bar ──
                        ui.horizontal(|ui| {
                            ui.label(label("Search:"));
                            ui.add(egui::TextEdit::singleline(&mut stacker_search)
                                .desired_width(200.0)
                                .hint_text("filter prompts..."));
                            ui.add_space(16.0);

                            // Category filter dropdown
                            let categories: Vec<String> = {
                                let mut cats: Vec<String> = stacker_prompts.iter()
                                    .map(|p| p.category.clone())
                                    .filter(|c| !c.is_empty())
                                    .collect();
                                cats.sort();
                                cats.dedup();
                                cats
                            };
                            if !categories.is_empty() {
                                ui.label(label("Category:"));
                                let display = if stacker_filter_category.is_empty() { "All" } else { &stacker_filter_category };
                                egui::ComboBox::from_id_salt("stacker_cat_filter")
                                    .selected_text(display)
                                    .show_ui(ui, |ui| {
                                        if ui.selectable_label(stacker_filter_category.is_empty(), "All").clicked() {
                                            stacker_filter_category.clear();
                                        }
                                        for cat in &categories {
                                            if ui.selectable_label(stacker_filter_category == *cat, cat).clicked() {
                                                stacker_filter_category = cat.clone();
                                            }
                                        }
                                    });
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                // Import / Export buttons
                                if ui.small_button("Export").clicked() {
                                    if let Some(path) = stacker_path() {
                                        let export_path = path.with_extension("export.json");
                                        let _ = export_prompts(&stacker_prompts, &export_path);
                                    }
                                }
                                if ui.small_button("Import").clicked() {
                                    if let Some(path) = stacker_path() {
                                        let import_path = path.with_extension("export.json");
                                        if let Ok(imported) = import_prompts(&import_path) {
                                            for p in imported {
                                                if !stacker_prompts.iter().any(|e| e.text == p.text) {
                                                    stacker_prompts.push(p);
                                                }
                                            }
                                            stacker_dirty = true;
                                        }
                                    }
                                }
                            });
                        });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        // ── Filtered prompt list ──
                        let search_lower = stacker_search.to_lowercase();
                        let visible: Vec<usize> = (0..stacker_prompts.len())
                            .filter(|&i| {
                                let p = &stacker_prompts[i];
                                let cat_ok = stacker_filter_category.is_empty() || p.category == stacker_filter_category;
                                let search_ok = stacker_search.is_empty()
                                    || p.text.to_lowercase().contains(&search_lower)
                                    || p.label.to_lowercase().contains(&search_lower)
                                    || p.category.to_lowercase().contains(&search_lower);
                                cat_ok && search_ok
                            })
                            .collect();

                        ui.label(egui::RichText::new(format!("Queue ({}/{})", visible.len(), stacker_prompts.len()))
                            .size(18.0).color(egui::Color32::WHITE));
                        ui.add_space(8.0);

                        if stacker_prompts.is_empty() {
                            ui.label(label("No prompts saved yet. Add one above."));
                        } else if visible.is_empty() {
                            ui.label(label("No prompts match the current filter."));
                        }

                        let mut remove_idx: Option<usize> = None;
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for &i in &visible {
                                let prompt = &stacker_prompts[i];
                                let is_editing = stacker_editing == Some(i);

                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        // Title + category badge
                                        ui.label(egui::RichText::new(&prompt.label)
                                            .size(15.0).color(egui::Color32::WHITE).strong());
                                        if !prompt.category.is_empty() {
                                            ui.label(egui::RichText::new(format!("[{}]", prompt.category))
                                                .size(12.0).color(egui::Color32::from_rgb(120, 180, 255)));
                                        }

                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if ui.small_button("Delete").clicked() {
                                                remove_idx = Some(i);
                                            }
                                            if ui.button(label("Copy")).clicked() {
                                                clipboard_copy = Some(prompt.text.clone());
                                            }
                                            if !is_editing {
                                                if ui.small_button("Edit").clicked() {
                                                    stacker_editing = Some(i);
                                                    stacker_edit_text = prompt.text.clone();
                                                }
                                            }
                                        });
                                    });

                                    if is_editing {
                                        // Inline editor
                                        ui.add(
                                            egui::TextEdit::multiline(&mut stacker_edit_text)
                                                .desired_rows(4)
                                                .desired_width(f32::INFINITY)
                                                .font(egui::TextStyle::Monospace),
                                        );
                                        ui.horizontal(|ui| {
                                            if ui.button(label("Save")).clicked() {
                                                stacker_editing = None;
                                                stacker_dirty = true;
                                                // Applied after closure (can't mutate stacker_prompts here)
                                            }
                                            if ui.button(label("Cancel")).clicked() {
                                                stacker_editing = None;
                                                stacker_edit_text.clear();
                                            }
                                        });
                                    } else {
                                        // Preview
                                        let preview: String = prompt.text.lines().take(3).collect::<Vec<_>>().join("\n");
                                        ui.label(egui::RichText::new(preview)
                                            .size(13.0).color(egui::Color32::from_rgb(160, 160, 170))
                                            .monospace());
                                    }
                                });
                                ui.add_space(4.0);
                            }
                        });

                        if let Some(idx) = remove_idx {
                            stacker_prompts.remove(idx);
                            stacker_dirty = true;
                            if stacker_editing == Some(idx) {
                                stacker_editing = None;
                            }
                        }
                    });
            }

            // FPS overlay
            if let Some((fps, ms)) = fps_info {
                egui::Area::new(egui::Id::new("fps_overlay"))
                    .fixed_pos(egui::Pos2::new(8.0, 8.0))
                    .show(ctx, |ui| {
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgba_premultiplied(0, 0, 0, 180))
                            .rounding(egui::Rounding::same(4.0))
                            .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(format!("{:.0} FPS  {:.1}ms", fps, ms))
                                    .size(12.0).color(egui::Color32::from_rgb(150, 255, 150)).monospace());
                            });
                    });
            }
        });

        // Apply edit save (must happen before writeback)
        if stacker_dirty && stacker_editing.is_none() && !stacker_edit_text.is_empty() {
            // Find the prompt that was being edited and update it
            // (stacker_editing was cleared on Save click, but edit_text is still set)
        }
        // Apply inline edit if Save was clicked (editing was Some, now None, edit_text has content)
        if self.stacker_editing.is_some() && stacker_editing.is_none() && !stacker_edit_text.is_empty() {
            let idx = self.stacker_editing.unwrap();
            if idx < stacker_prompts.len() {
                stacker_prompts[idx].text = stacker_edit_text.clone();
                let words: String = stacker_prompts[idx].text.split_whitespace().take(6).collect::<Vec<_>>().join(" ");
                stacker_prompts[idx].label = if words.len() < stacker_prompts[idx].text.trim().len() {
                    format!("{}...", words)
                } else {
                    words
                };
                stacker_dirty = true;
            }
            stacker_edit_text.clear();
        }

        // Persist to disk when dirty
        if stacker_dirty {
            save_stacker_prompts(&stacker_prompts);
        }

        // Apply state changes
        self.settings_tab = settings_tab;
        self.stacker_prompts = stacker_prompts;
        self.stacker_input = stacker_input;
        self.stacker_category_input = stacker_category_input;
        self.stacker_search = stacker_search;
        self.stacker_filter_category = stacker_filter_category;
        self.stacker_editing = stacker_editing;
        self.stacker_edit_text = stacker_edit_text;
        self.stacker_dirty = false;

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
            let pass = render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
                    for name in &["none", "smoke", "aurora", "matrix", "nebula", "tron"] {
                        ui.selectable_value(&mut config.effects.background, name.to_string(), *name);
                    }
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

            ui.label(label("Time-of-Day Warmth"));
            ui.add(egui::Checkbox::without_text(&mut config.time_of_day_enabled));
            ui.end_row();
        });
}

fn render_themes_tab(ui: &mut egui::Ui, config: &mut Config) {
    ui.label(
        egui::RichText::new("Visual Themes")
            .size(18.0)
            .color(egui::Color32::WHITE),
    );
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new("Select a theme to apply its color scheme and effects.")
            .size(14.0)
            .color(egui::Color32::from_rgb(160, 160, 170)),
    );
    ui.add_space(16.0);

    let themes = builtin_themes();

    for theme in &themes {
        let is_frame = egui::Frame::none()
            .fill(egui::Color32::from_rgb(28, 28, 38))
            .rounding(egui::Rounding::same(6.0))
            .inner_margin(egui::Margin::same(14.0))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 65)));

        is_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(&theme.name)
                            .size(17.0)
                            .color(egui::Color32::WHITE)
                            .strong(),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(&theme.description)
                            .size(13.0)
                            .color(egui::Color32::from_rgb(150, 150, 165)),
                    );
                    ui.add_space(6.0);

                    // Color preview swatches
                    ui.horizontal(|ui| {
                        let colors = [
                            theme.colors.background,
                            theme.colors.foreground,
                            theme.colors.cursor,
                            theme.colors.ansi[1],  // red
                            theme.colors.ansi[2],  // green
                            theme.colors.ansi[4],  // blue
                            theme.colors.ansi[5],  // magenta
                            theme.colors.ansi[6],  // cyan
                        ];
                        for c in colors {
                            let (rect, _r) = ui.allocate_exact_size(
                                egui::Vec2::new(18.0, 18.0),
                                egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(
                                rect,
                                egui::Rounding::same(3.0),
                                egui::Color32::from_rgb(c[0], c[1], c[2]),
                            );
                        }
                    });
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(
                        egui::RichText::new("Apply").size(15.0),
                    ).clicked() {
                        theme.apply_to(config);
                    }
                });
            });
        });
        ui.add_space(8.0);
    }
}

// ── Stacker persistence ──

fn stacker_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join("llnzy").join("stacker.json"))
}

fn load_stacker_prompts() -> Vec<StackerPrompt> {
    let Some(path) = stacker_path() else { return Vec::new() };
    let Ok(data) = std::fs::read_to_string(&path) else { return Vec::new() };
    serde_json::from_str(&data).unwrap_or_default()
}

fn save_stacker_prompts(prompts: &[StackerPrompt]) {
    let Some(path) = stacker_path() else { return };
    let _ = std::fs::create_dir_all(path.parent().unwrap());
    if let Ok(json) = serde_json::to_string_pretty(prompts) {
        let _ = std::fs::write(path, json);
    }
}

/// Export prompts to a JSON file at the given path.
pub fn export_prompts(prompts: &[StackerPrompt], path: &std::path::Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(prompts).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

/// Import prompts from a JSON file, returning the loaded prompts.
pub fn import_prompts(path: &std::path::Path) -> Result<Vec<StackerPrompt>, String> {
    let data = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

