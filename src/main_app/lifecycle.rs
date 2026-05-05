use crate::*;

impl App {
    pub(super) fn handle_resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        // Try to restore saved window size
        let saved_size = window_state::load_window_size();
        let inner_size = saved_size
            .map(|(w, h)| winit::dpi::PhysicalSize::new(w, h))
            .unwrap_or(winit::dpi::PhysicalSize::new(900, 600));

        let mut attrs = WindowAttributes::default()
            .with_title("LLNZY")
            .with_inner_size(inner_size);

        if let Ok(image) = image::load_from_memory(include_bytes!("../../llnzy.jpg")) {
            let image = image.to_rgba8();
            let (width, height) = image.dimensions();
            if let Ok(icon) = winit::window::Icon::from_rgba(image.into_raw(), width, height) {
                attrs = attrs.with_window_icon(Some(icon));
            }
        }

        if self.config.opacity < 1.0 {
            attrs = attrs.with_transparent(true);
        }

        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        // Input methods can deliver text through AppKit's IME/text-input path.
        // winit disables that by default, so opt in for terminal and Stacker text entry.
        window.set_ime_allowed(true);
        let power_source = llnzy::platform::power::current_power_source();
        self.current_power_source = power_source;
        self.last_power_check = std::time::Instant::now();
        let mut renderer = pollster::block_on(Renderer::new(window.clone(), self.config.clone()));
        renderer.set_power_source(power_source);

        let (cw, ch) = renderer.cell_dimensions();
        let gox = renderer.glyph_offset_x();
        let size = window.inner_size();
        let layout = ScreenLayout::compute(LayoutInputs {
            window_w: size.width as f32,
            window_h: size.height as f32,
            cell_w: cw,
            cell_h: ch,
            padding_x: self.config.padding_x,
            padding_y: self.config.padding_y,
            glyph_offset_x: gox,
            sidebar_w: logical_to_physical_width(BUMPER_WIDTH, window.scale_factor()),
        });
        self.screen_layout = Some(layout);

        let ui_state = UiState::new(
            &window,
            renderer.gpu_device(),
            renderer.gpu_surface_format(),
        );

        self.window = Some(window);
        self.renderer = Some(renderer);
        self.ui = Some(ui_state);
        self.create_stacker_webview();
        self.restore_last_session();
        if self.tabs.is_empty() {
            self.open_singleton_tab(llnzy::workspace::TabKind::Home);
        }

        #[cfg(target_os = "macos")]
        llnzy::menu::setup_menu_bar(self.proxy.clone());
        #[cfg(target_os = "macos")]
        {
            llnzy::macos_text_bridge::setup(self.proxy.clone());
            if self.stacker_webview.is_none() {
                if let Some(window) = &self.window {
                    llnzy::macos_text_bridge::install(window);
                }
            }
        }
    }
}
