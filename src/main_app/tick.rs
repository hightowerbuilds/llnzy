use crate::*;

impl App {
    pub(super) fn handle_about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.process_all_output();

        let now = Instant::now();

        let editor_recovery_dirty = self
            .ui
            .as_ref()
            .is_some_and(|ui| ui.editor_view.recovery_dirty);
        if editor_recovery_dirty
            || now.duration_since(self.last_editor_recovery_save).as_secs() >= 5
        {
            self.save_editor_recovery_snapshots();
            if let Some(ui) = &mut self.ui {
                ui.editor_view.recovery_dirty = false;
            }
            self.last_editor_recovery_save = now;
        }

        // Cursor blink
        let blink_ms = self.config.cursor_blink_ms;
        if blink_ms > 0 {
            let since_key = now.duration_since(self.last_keypress).as_millis() as u64;
            if since_key < blink_ms {
                if let Some(r) = &mut self.renderer {
                    if !r.cursor_visible {
                        r.cursor_visible = true;
                        self.request_redraw();
                    }
                }
                self.last_blink_toggle = now;
            } else {
                let since_blink = now.duration_since(self.last_blink_toggle).as_millis() as u64;
                if since_blink >= blink_ms {
                    if let Some(r) = &mut self.renderer {
                        r.cursor_visible = !r.cursor_visible;
                        self.request_redraw();
                    }
                    self.last_blink_toggle = now;
                }
            }
            let next = self.last_blink_toggle + std::time::Duration::from_millis(blink_ms);
            event_loop.set_control_flow(ControlFlow::WaitUntil(next));
        }

        // Advance theme color transition
        if let Some(ref mut trans) = self.config.transition {
            let dt = self
                .renderer
                .as_ref()
                .map(|r| r.gpu_delta_time())
                .unwrap_or(1.0 / 60.0);
            let done = trans.advance(dt);
            let blended = trans.current();
            // Apply blended colors to renderer without overwriting the target config
            if let Some(renderer) = &mut self.renderer {
                let mut render_config = self.config.clone();
                render_config.colors = blended;
                renderer.update_config(render_config);
            }
            if done {
                self.config.transition = None;
            }
            self.request_redraw();
        }

        // Config hot-reload from disk (skip when settings UI is open or recently changed)
        let settings_active = self.ui.as_ref().is_some_and(|u| u.settings_open());
        let recently_changed = now.duration_since(self.last_ui_config_change).as_secs() < 10;
        if !settings_active
            && !recently_changed
            && now.duration_since(self.last_config_check).as_secs() >= 2
        {
            self.last_config_check = now;
            if self.config.check_reload() {
                self.error_log.info("Config reloaded from disk");
                if let Some(renderer) = &mut self.renderer {
                    renderer.update_config(self.config.clone());
                }
                self.request_redraw();
            }
        }

        if now.duration_since(self.last_power_check) >= std::time::Duration::from_secs(30) {
            self.last_power_check = now;
            let power_source = llnzy::platform::power::current_power_source();
            if power_source != self.current_power_source {
                self.current_power_source = power_source;
                if let Some(renderer) = &mut self.renderer {
                    renderer.set_power_source(power_source);
                }
                self.request_redraw();
            }
        }

        // Continuous animation mode — only when effects actually need it
        let terminal_active = self.screen_layout.as_ref().is_some_and(|layout| {
            self.ui.as_ref().is_some_and(|ui| {
                terminal_effect_rect(&self.tabs, layout, &ui.tab_groups, self.active_tab).is_some()
            })
        });
        let ui_active = self.ui.as_ref().is_some_and(|u| u.settings_open());
        if (terminal_active && self.config.effects.any_active()) || ui_active {
            event_loop.set_control_flow(ControlFlow::Poll);
            self.request_redraw();
        }
    }
}
