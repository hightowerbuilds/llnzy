use crate::*;

impl App {
    pub(super) fn handle_redraw_requested(&mut self, event_loop: &ActiveEventLoop) {
        self.process_all_output();
        self.update_ime_cursor_area();
        #[cfg(target_os = "macos")]
        self.sync_macos_text_bridge();

        if let (Some(renderer), Some(ui)) = (&self.renderer, &mut self.ui) {
            ui.record_frame_time(renderer.gpu_delta_time());
        }

        let bell_active = self.visual_bell_until.is_some_and(|t| Instant::now() < t);
        if bell_active {
            self.request_redraw();
        } else {
            self.visual_bell_until = None;
        }

        let tab_info = self.tab_titles();
        let tab_pane_info = self.tab_panes();
        let render_titles: Vec<(String, bool)> = tab_info
            .iter()
            .enumerate()
            .map(|(i, tab)| (tab.title.clone(), i == self.active_tab))
            .collect();
        let (cw, ch) = self
            .renderer
            .as_ref()
            .map(|r| r.cell_dimensions())
            .unwrap_or((1.0, 1.0));
        let sel_info = self.active_selection_rects(cw, ch);
        let search_rects = self
            .active_session()
            .map(|session| self.search.highlight_rects(&session.terminal, cw, ch))
            .unwrap_or_default();
        let search_bar = if self.search.active {
            Some((self.search.query.as_str(), self.search.status()))
        } else {
            None
        };
        let search_bar_ref = search_bar.as_ref().map(|(q, s)| (*q, s.as_str()));

        let err_panel = if self.error_panel.visible {
            Some((&self.error_panel, &self.error_log))
        } else {
            None
        };

        let sidebar_w_before = self.sidebar_width_px();
        let mut project_tree_changed = false;

        if let Some(renderer) = &mut self.renderer {
            if let Some(layout) = &self.screen_layout {
                if let Some(ui) = self.ui.as_mut() {
                    if let Ok(text) = self.clipboard.get_text() {
                        ui.editor_view.clipboard_in = Some(text);
                    }
                    ui.editor_view.init_lsp(self.proxy.clone());
                    ui.explorer.ensure_project_watcher(self.proxy.clone());
                    if ui.explorer.poll_project_watcher() {
                        project_tree_changed = true;
                    }
                }
                if let Some(ui) = self.ui.as_mut() {
                    ui.set_tab_context(self.tabs.len(), self.active_tab);
                    ui.active_tab_kind = self.tabs.get(self.active_tab).map(|t| t.content.kind());
                    #[cfg(target_os = "macos")]
                    llnzy::menu::set_save_enabled(matches!(
                        ui.active_tab_kind,
                        Some(TabKind::CodeFile)
                    ));
                    ui.tab_names = tab_info.clone();
                    ui.tab_panes = tab_pane_info.clone();
                }

                let active_tab = self.tabs.get(self.active_tab);
                let tab_id = active_tab.map(|t| t.id).unwrap_or(0);
                let joined_tabs = self
                    .ui
                    .as_ref()
                    .and_then(|ui| active_joined_tabs(&self.tabs, self.active_tab, &ui.tab_groups));
                let terminal_panes = joined_tabs
                    .map(|joined| {
                        joined_terminal_panes(&self.tabs, self.active_tab, layout, joined)
                    })
                    .unwrap_or_default();
                let terminal_session = if terminal_panes.is_empty() {
                    active_tab.and_then(|t| t.content.as_terminal())
                } else {
                    None
                };
                let terminal_effect_rect = self.ui.as_ref().and_then(|ui| {
                    terminal_effect_rect(&self.tabs, layout, &ui.tab_groups, self.active_tab)
                });
                let terminal_effects_enabled = terminal_effect_rect.is_some();
                let effects_mask = terminal_effect_rect.and_then(|rect| {
                    self.window
                        .as_ref()
                        .map(|window| rect_to_uv(rect, window.inner_size()))
                });

                let ui_state = &mut self.ui;
                let window_ref = &self.window;
                let config_ref = &self.config;
                let mut ui_frame_output = UiFrameOutput::default();
                let mut egui_cb =
                    |device: &wgpu::Device,
                     queue: &wgpu::Queue,
                     view: &wgpu::TextureView,
                     desc: egui_wgpu::ScreenDescriptor| {
                        if let (Some(ui), Some(window)) = (ui_state.as_mut(), window_ref.as_ref()) {
                            ui_frame_output =
                                ui.render(window, device, queue, view, desc, config_ref);
                        }
                    };
                renderer.render(RenderRequest {
                    terminal: terminal_session,
                    tab_id,
                    terminal_panes: &terminal_panes,
                    tab_titles: &render_titles,
                    selection_rects: &sel_info,
                    search_rects: &search_rects,
                    search_bar: search_bar_ref,
                    error_panel: err_panel,
                    visual_bell: bell_active,
                    screen_layout: layout,
                    egui_render: Some(&mut egui_cb),
                    effects_enabled: terminal_effects_enabled,
                    apply_effects_to_ui: false,
                    effects_mask,
                });
                self.handle_ui_frame_output(ui_frame_output, event_loop);
                self.sync_stacker_webview();
            }
        }
        if project_tree_changed {
            self.request_redraw();
        }

        let sidebar_w_after = self.sidebar_width_px();
        if (sidebar_w_after - sidebar_w_before).abs() > 0.1 {
            self.recompute_layout();
            self.resize_terminal_tabs();
            self.clear_terminal_selection();
            self.invalidate_and_redraw();
        }
    }
}
