use std::path::Path;

use llnzy::app::drag_drop::{tab_drop_zone_at_x, tab_index_at_x, DropTarget, TerminalDropMode};
use llnzy::layout::{LayoutInputs, ScreenLayout};
use llnzy::session::Rect as PaneRect;
use llnzy::ui::ActiveView;
use llnzy::workspace::TabContent;
use llnzy::workspace_layout::joined_terminal_content_rects;

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

    pub(crate) fn joined_terminal_grid_sizes(&self, ratio: f32) -> ((u16, u16), (u16, u16)) {
        if let Some(layout) = &self.screen_layout {
            let (left, right) = joined_terminal_content_rects(layout, ratio);
            (
                ((left.w / layout.cell_w).max(1.0) as u16, layout.grid_rows),
                ((right.w / layout.cell_w).max(1.0) as u16, layout.grid_rows),
            )
        } else {
            ((40, 24), (40, 24))
        }
    }

    pub(crate) fn pixel_to_grid(&self, pos: winit::dpi::PhysicalPosition<f64>) -> (usize, usize) {
        if let Some(layout) = &self.screen_layout {
            let rect = self
                .active_joined_terminal_rect(layout)
                .unwrap_or(PaneRect {
                    x: layout.content.x,
                    y: layout.content.y,
                    w: layout.content.w,
                    h: layout.content.h,
                });
            let (cols, rows) = self
                .active_session()
                .map(|session| session.terminal.size())
                .unwrap_or((layout.grid_cols as usize, layout.grid_rows as usize));
            let col = ((pos.x as f32 - rect.x) / layout.cell_w).max(0.0) as usize;
            let row = ((pos.y as f32 - rect.y) / layout.cell_h).max(0.0) as usize;
            let col = col.min(cols.saturating_sub(1));
            let row = row.min(rows.saturating_sub(1));
            (row, col)
        } else {
            (0, 0)
        }
    }

    fn active_joined_terminal_rect(&self, layout: &ScreenLayout) -> Option<PaneRect> {
        let joined = self
            .ui
            .as_ref()
            .and_then(|ui| ui.joined_tabs)
            .filter(|joined| joined.contains(self.active_tab))?;
        let (left, right) = joined_terminal_content_rects(layout, joined.ratio);

        if joined.primary == self.active_tab {
            Some(left)
        } else if joined.secondary == self.active_tab {
            Some(right)
        } else {
            None
        }
    }

    pub(crate) fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    pub(crate) fn update_ime_cursor_area(&self) {
        let (Some(window), Some(layout), Some(session)) =
            (&self.window, &self.screen_layout, self.active_session())
        else {
            return;
        };
        let Some((row, col)) = session.terminal.cursor_point() else {
            return;
        };
        let x = layout.content.x + col as f32 * layout.cell_w;
        let y = layout.content.y + row as f32 * layout.cell_h;
        window.set_ime_cursor_area(
            winit::dpi::PhysicalPosition::new(x as f64, y as f64),
            winit::dpi::PhysicalSize::new(
                layout.cell_w.max(1.0) as u32,
                layout.cell_h.max(1.0) as u32,
            ),
        );
    }

    pub(crate) fn native_file_drop_target(&self, dropped_path: &Path) -> Option<DropTarget> {
        let pos = self.cursor_pos;
        let x = pos.x as f32;
        let y = pos.y as f32;

        if self
            .ui
            .as_ref()
            .is_some_and(|ui| x < ui.sidebar_width() && ui.sidebar.open)
        {
            return Some(DropTarget::Home);
        }

        let Some(layout) = &self.screen_layout else {
            return if dropped_path.is_dir() {
                Some(DropTarget::Home)
            } else {
                None
            };
        };

        if layout.tab_bar.contains(x, y) {
            let index = tab_index_at_x(x, layout.tab_bar.x, layout.tab_bar.w, self.tabs.len())?;
            let zone = tab_drop_zone_at_x(x, layout.tab_bar.x, layout.tab_bar.w, self.tabs.len())?;
            return Some(DropTarget::TabBar { index, zone });
        }

        if layout.content.contains(x, y) {
            if self.active_session().is_some() {
                return Some(DropTarget::Terminal {
                    tab_idx: self.active_tab,
                    mode: TerminalDropMode::InsertEscapedPath,
                });
            }

            if let Some(TabContent::CodeFile { buffer_id, .. }) =
                self.tabs.get(self.active_tab).map(|tab| &tab.content)
            {
                let Some(buffer_idx) = self
                    .ui
                    .as_ref()
                    .and_then(|ui| ui.editor_view.editor.index_for_id(*buffer_id))
                else {
                    return None;
                };
                return Some(DropTarget::Editor {
                    buffer_idx,
                    position: llnzy::editor::buffer::Position::new(0, 0),
                });
            }
        }

        if self
            .ui
            .as_ref()
            .is_some_and(|ui| ui.active_view == ActiveView::Home)
            || dropped_path.is_dir()
        {
            return Some(DropTarget::Home);
        }

        None
    }

    pub(crate) fn resize_terminal_tabs(&mut self) {
        if let Some(layout) = &self.screen_layout {
            let (cols, rows) = (layout.grid_cols, layout.grid_rows);
            let joined = self.ui.as_ref().and_then(|ui| ui.joined_tabs);
            let joined_sizes = joined.map(|joined| self.joined_terminal_grid_sizes(joined.ratio));
            for (idx, tab) in self.tabs.iter_mut().enumerate() {
                if let TabContent::Terminal(ref mut session) = tab.content {
                    if let Some(joined) = joined {
                        if idx == joined.primary {
                            let ((joined_cols, joined_rows), _) =
                                joined_sizes.unwrap_or(((cols, rows), (cols, rows)));
                            session.resize(joined_cols, joined_rows);
                            continue;
                        }
                        if idx == joined.secondary {
                            let (_, (joined_cols, joined_rows)) =
                                joined_sizes.unwrap_or(((cols, rows), (cols, rows)));
                            session.resize(joined_cols, joined_rows);
                            continue;
                        }
                    }
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
