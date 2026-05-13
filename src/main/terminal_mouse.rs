use super::*;

impl App {
    pub(crate) fn joined_terminal_tab_at_cursor(&self) -> Option<usize> {
        self.terminal_pane_hit_at_cursor().map(|hit| hit.tab_idx)
    }

    pub(crate) fn joined_tab_at_cursor(&self) -> Option<usize> {
        let layout = self.screen_layout.as_ref()?;
        let x = self.cursor_pos.x as f32;
        let y = self.cursor_pos.y as f32;
        let joined = self
            .ui
            .as_ref()
            .and_then(|ui| active_joined_tabs(&self.tabs, self.active_tab, &ui.tab_groups))?;
        let (left_rect, right_rect) = joined_pane_rects(layout, joined.ratio);

        if rect_contains(left_rect, x, y) {
            Some(joined.primary)
        } else if rect_contains(right_rect, x, y) {
            Some(joined.secondary)
        } else {
            None
        }
    }

    pub(crate) fn terminal_pane_hit_at_cursor(&self) -> Option<TerminalPaneHit> {
        let layout = self.screen_layout.as_ref()?;
        let x = self.cursor_pos.x as f32;
        let y = self.cursor_pos.y as f32;
        let joined = self
            .ui
            .as_ref()
            .and_then(|ui| active_joined_tabs(&self.tabs, self.active_tab, &ui.tab_groups));

        if let Some(joined) = joined {
            let (left_rect, right_rect) = joined_terminal_content_rects(layout, joined.ratio);
            for (idx, rect) in [(joined.primary, left_rect), (joined.secondary, right_rect)] {
                if let Some(hit) = self.terminal_pane_hit(idx, rect, x, y) {
                    return Some(hit);
                }
            }
            return None;
        }

        let rect = llnzy::session::Rect {
            x: layout.content.x,
            y: layout.content.y,
            w: layout.content.w,
            h: layout.content.h,
        };
        self.terminal_pane_hit(self.active_tab, rect, x, y)
    }

    pub(crate) fn terminal_pane_hit(
        &self,
        tab_idx: usize,
        rect: llnzy::session::Rect,
        x: f32,
        y: f32,
    ) -> Option<TerminalPaneHit> {
        if !rect_contains(rect, x, y) {
            return None;
        }
        let session = self.session_for_tab(tab_idx)?;
        let layout = self.screen_layout.as_ref()?;
        let (cols, rows) = session.terminal.size();
        let col = ((x - rect.x) / layout.cell_w).max(0.0) as usize;
        let row = ((y - rect.y) / layout.cell_h).max(0.0) as usize;
        Some(TerminalPaneHit {
            tab_idx,
            row: row.min(rows.saturating_sub(1)),
            col: col.min(cols.saturating_sub(1)),
        })
    }

    pub(crate) fn active_selection_rects(
        &mut self,
        cell_w: f32,
        cell_h: f32,
    ) -> Arc<[SelectionRect]> {
        let empty: Arc<[SelectionRect]> = Arc::from(Vec::<SelectionRect>::new().into_boxed_slice());
        let Some(tab) = self.tabs.get(self.active_tab) else {
            self.selection_rect_cache = None;
            return empty;
        };
        let Some(session) = tab.content.as_terminal() else {
            self.selection_rect_cache = None;
            return empty;
        };

        let tab_id = tab.id;
        let revision = session.terminal.selection_revision();
        let cell_w_bits = cell_w.to_bits();
        let cell_h_bits = cell_h.to_bits();
        let color = self.config.colors.selection;
        let alpha = self.config.colors.selection_alpha;
        let alpha_bits = alpha.to_bits();

        if let Some(cache) = &self.selection_rect_cache {
            if cache.tab_id == tab_id
                && cache.revision == revision
                && cache.cell_w_bits == cell_w_bits
                && cache.cell_h_bits == cell_h_bits
                && cache.color == color
                && cache.alpha_bits == alpha_bits
            {
                return Arc::clone(&cache.rects);
            }
        }

        let rects: Arc<[SelectionRect]> = session
            .terminal
            .selection_rects(cell_w, cell_h, color, alpha)
            .into();
        self.selection_rect_cache = Some(SelectionRectCache {
            tab_id,
            revision,
            cell_w_bits,
            cell_h_bits,
            color,
            alpha_bits,
            rects: Arc::clone(&rects),
        });
        rects
    }

    pub(crate) fn update_active_terminal_selection(&mut self, row: usize, col: usize) -> bool {
        self.active_session_mut()
            .is_some_and(|session| session.terminal.update_selection(row, col))
    }

    pub(crate) fn route_terminal_mouse_wheel(&mut self, delta: &MouseScrollDelta) -> bool {
        if self.cursor_over_non_terminal_chrome() {
            return false;
        }
        let Some(hit) = self.terminal_pane_hit_at_cursor() else {
            return false;
        };

        let Some(session) = self.session_for_tab(hit.tab_idx) else {
            return false;
        };
        if session.terminal.mouse_mode() {
            let sgr = session.terminal.sgr_mouse();
            let lines = self.wheel_lines(delta, 1.0);
            for _ in 0..lines.unsigned_abs() {
                let button = if lines > 0 { 64 } else { 65 };
                let intent = llnzy::platform::input::mouse_report_intent(
                    button,
                    hit.col,
                    hit.row,
                    true,
                    sgr,
                    &self.modifiers,
                );
                if let llnzy::platform::input::PlatformInputIntent::MouseReport(bytes) = intent {
                    self.write_to_terminal_tab(hit.tab_idx, &bytes);
                }
            }
            self.request_redraw();
            return true;
        }

        let lines = self.wheel_lines(delta, self.config.scroll_lines as f32);
        if lines != 0 {
            if let Some(session) = self.session_for_tab_mut(hit.tab_idx) {
                session.terminal.scroll(lines);
            }
            self.invalidate_and_redraw();
        } else {
            self.request_redraw();
        }
        true
    }

    pub(crate) fn wheel_lines(&self, delta: &MouseScrollDelta, line_multiplier: f32) -> i32 {
        match delta {
            MouseScrollDelta::LineDelta(_, y) => (y * line_multiplier) as i32,
            MouseScrollDelta::PixelDelta(pos) => {
                let (_, ch) = self
                    .renderer
                    .as_ref()
                    .map(|r| r.cell_dimensions())
                    .unwrap_or((1.0, 1.0));
                let lines = (pos.y / ch as f64) as i32;
                if lines == 0 && pos.y != 0.0 {
                    pos.y.signum() as i32
                } else {
                    lines
                }
            }
        }
    }
}
