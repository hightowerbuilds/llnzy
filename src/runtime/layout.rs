use llnzy::layout::{LayoutInputs, ScreenLayout};
use llnzy::session::Rect as PaneRect;
use llnzy::workspace::TabContent;

use crate::App;

impl App {
    pub(crate) fn recompute_layout(&mut self) {
        if let Some(renderer) = &self.renderer {
            let (cw, ch) = renderer.cell_dimensions();
            let gox = renderer.glyph_offset_x();
            let w = self
                .window
                .as_ref()
                .map(|w| w.inner_size().width as f32)
                .unwrap_or(900.0);
            let h = self
                .window
                .as_ref()
                .map(|w| w.inner_size().height as f32)
                .unwrap_or(600.0);
            let sidebar_w = self.ui.as_ref().map(|u| u.sidebar_width()).unwrap_or(0.0);
            self.screen_layout = Some(ScreenLayout::compute(LayoutInputs {
                window_w: w,
                window_h: h,
                cell_w: cw,
                cell_h: ch,
                padding_x: self.config.padding_x,
                padding_y: self.config.padding_y,
                glyph_offset_x: gox,
                sidebar_w,
            }));
        }
    }

    pub(crate) fn grid_size(&self) -> (u16, u16) {
        if let Some(layout) = &self.screen_layout {
            (layout.grid_cols, layout.grid_rows)
        } else {
            (80, 24)
        }
    }

    pub(crate) fn pixel_to_grid(&self, pos: winit::dpi::PhysicalPosition<f64>) -> (usize, usize) {
        if let Some(layout) = &self.screen_layout {
            layout.pixel_to_grid(pos.x as f32, pos.y as f32)
        } else {
            (0, 0)
        }
    }

    pub(crate) fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    pub(crate) fn resize_terminal_tabs(&mut self) {
        if let Some(layout) = &self.screen_layout {
            let (cols, rows) = (layout.grid_cols, layout.grid_rows);
            for tab in &mut self.tabs {
                if let TabContent::Terminal(ref mut session) = tab.content {
                    session.resize(cols, rows);
                }
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) fn content_rect(&self) -> Option<PaneRect> {
        self.screen_layout.as_ref().map(|l| PaneRect {
            x: l.content.x,
            y: l.content.y,
            w: l.content.w,
            h: l.content.h,
        })
    }

    pub(crate) fn invalidate_and_redraw(&mut self) {
        if let Some(r) = &mut self.renderer {
            r.invalidate_text_cache();
        }
        self.request_redraw();
    }
}
