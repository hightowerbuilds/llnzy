use taffy::prelude::*;

/// All computed screen regions, updated once per frame on resize.
#[derive(Clone, Debug)]
pub struct ScreenLayout {
    pub window_w: f32,
    pub window_h: f32,
    pub tab_bar: Zone,
    pub content: Zone,
    pub footer: Zone,
    pub footer_buttons: Vec<ButtonZone>,
    // Cell metrics
    pub cell_w: f32,
    pub cell_h: f32,
    pub grid_cols: u16,
    pub grid_rows: u16,
    // Whether zones are visible
    pub show_tab_bar: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Zone {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Debug)]
pub struct ButtonZone {
    pub label: String,
    pub zone: Zone,
}

impl Zone {
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

// ── Layout constants ──

pub const TAB_BAR_HEIGHT: f32 = 28.0;
pub const FOOTER_HEIGHT: f32 = 36.0;
const FOOTER_BTN_W: f32 = 90.0;
const FOOTER_BTN_H: f32 = 26.0;
const FOOTER_BTN_GAP: f32 = 8.0;
const FOOTER_BTN_PAD: f32 = 10.0;

// ── Settings panel layout ──

pub const SETTINGS_SIDEBAR_W: f32 = 180.0;
pub const SETTINGS_HEADER_H: f32 = 44.0;
pub const SETTINGS_TAB_H: f32 = 38.0;
pub const SETTINGS_TAB_PAD: f32 = 6.0;

#[derive(Clone, Debug)]
pub struct SettingsLayout {
    pub header: Zone,
    pub back_button: Zone,
    pub sidebar: Zone,
    pub tab_zones: Vec<(String, Zone)>,
    pub content: Zone,
}

impl ScreenLayout {
    /// Compute the full screen layout using Taffy flexbox.
    pub fn compute(
        window_w: f32,
        window_h: f32,
        cell_w: f32,
        cell_h: f32,
        tab_count: usize,
        padding_x: f32,
        padding_y: f32,
        glyph_offset_x: f32,
    ) -> Self {
        let padding_x = padding_x + glyph_offset_x;
        let mut tree: TaffyTree<()> = TaffyTree::new();
        let show_tab_bar = true; // always show tab bar

        // ── Build flexbox tree ──
        // Root: vertical column filling the window
        // Children: [tab_bar (optional)] [content (flex:1)] [footer]

        let tab_bar_node = tree
            .new_leaf(Style {
                size: Size {
                    width: Dimension::Percent(1.0),
                    height: if show_tab_bar {
                        Dimension::Length(TAB_BAR_HEIGHT)
                    } else {
                        Dimension::Length(0.0)
                    },
                },
                ..Default::default()
            })
            .unwrap();

        let content_node = tree
            .new_leaf(Style {
                flex_grow: 1.0,
                size: Size {
                    width: Dimension::Percent(1.0),
                    height: Dimension::Auto,
                },
                padding: Rect {
                    left: LengthPercentage::Length(padding_x),
                    right: LengthPercentage::Length(padding_x),
                    top: LengthPercentage::Length(padding_y),
                    bottom: LengthPercentage::Length(padding_y),
                },
                ..Default::default()
            })
            .unwrap();

        let footer_node = tree
            .new_leaf(Style {
                size: Size {
                    width: Dimension::Percent(1.0),
                    height: Dimension::Length(FOOTER_HEIGHT),
                },
                ..Default::default()
            })
            .unwrap();

        let root = tree
            .new_with_children(
                Style {
                    size: Size {
                        width: Dimension::Length(window_w),
                        height: Dimension::Length(window_h),
                    },
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
                &[tab_bar_node, content_node, footer_node],
            )
            .unwrap();

        tree.compute_layout(root, Size::MAX_CONTENT).unwrap();

        // ── Extract computed positions ──

        let tab_bar_layout = tree.layout(tab_bar_node).unwrap();
        let content_layout = tree.layout(content_node).unwrap();
        let footer_layout = tree.layout(footer_node).unwrap();

        let tab_bar = Zone {
            x: tab_bar_layout.location.x,
            y: tab_bar_layout.location.y,
            w: tab_bar_layout.size.width,
            h: tab_bar_layout.size.height,
        };

        // Content zone: the padding is handled by Taffy, so content_box gives us
        // the inner area. But Taffy's layout gives the outer box — we need inner.
        let content = Zone {
            x: content_layout.location.x + padding_x,
            y: content_layout.location.y + padding_y,
            w: content_layout.size.width - padding_x * 2.0,
            h: content_layout.size.height - padding_y * 2.0,
        };

        let footer = Zone {
            x: footer_layout.location.x,
            y: footer_layout.location.y,
            w: footer_layout.size.width,
            h: footer_layout.size.height,
        };

        // ── Footer buttons (right-aligned) ──
        let btn_y = footer.y + (FOOTER_HEIGHT - FOOTER_BTN_H) / 2.0;

        let settings_x = footer.x + footer.w - FOOTER_BTN_W - FOOTER_BTN_PAD;
        let stacker_x = settings_x - FOOTER_BTN_W - FOOTER_BTN_GAP;

        let footer_buttons = vec![
            ButtonZone {
                label: "Stacker".to_string(),
                zone: Zone {
                    x: stacker_x,
                    y: btn_y,
                    w: FOOTER_BTN_W,
                    h: FOOTER_BTN_H,
                },
            },
            ButtonZone {
                label: "Settings".to_string(),
                zone: Zone {
                    x: settings_x,
                    y: btn_y,
                    w: FOOTER_BTN_W,
                    h: FOOTER_BTN_H,
                },
            },
        ];

        // ── Grid dimensions from content zone ──
        let grid_cols = (content.w / cell_w).max(1.0) as u16;
        let grid_rows = (content.h / cell_h).max(1.0) as u16;

        ScreenLayout {
            window_w,
            window_h,
            tab_bar,
            content,
            footer,
            footer_buttons,
            cell_w,
            cell_h,
            grid_cols,
            grid_rows,
            show_tab_bar,
        }
    }

    /// Find which footer button (if any) was clicked.
    pub fn footer_button_at(&self, px: f32, py: f32) -> Option<&str> {
        for btn in &self.footer_buttons {
            if btn.zone.contains(px, py) {
                return Some(&btn.label);
            }
        }
        None
    }

    /// Convert pixel position to grid (row, col) within the content zone.
    pub fn pixel_to_grid(&self, px: f32, py: f32) -> (usize, usize) {
        let col = ((px - self.content.x) / self.cell_w)
            .max(0.0) as usize;
        let row = ((py - self.content.y) / self.cell_h)
            .max(0.0) as usize;
        let col = col.min(self.grid_cols.saturating_sub(1) as usize);
        let row = row.min(self.grid_rows.saturating_sub(1) as usize);
        (row, col)
    }

    /// Get cursor pixel position for a given grid (row, col).
    pub fn grid_to_pixel(&self, row: usize, col: usize) -> (f32, f32) {
        (
            col as f32 * self.cell_w + self.content.x,
            row as f32 * self.cell_h + self.content.y,
        )
    }

    /// Compute the settings panel layout for a given window size.
    pub fn settings_layout(window_w: f32, window_h: f32) -> SettingsLayout {
        let header = Zone {
            x: 0.0,
            y: 0.0,
            w: window_w,
            h: SETTINGS_HEADER_H,
        };

        let back_button = Zone {
            x: window_w - 90.0,
            y: 6.0,
            w: 80.0,
            h: SETTINGS_HEADER_H - 12.0,
        };

        let sidebar = Zone {
            x: 0.0,
            y: SETTINGS_HEADER_H,
            w: SETTINGS_SIDEBAR_W,
            h: window_h - SETTINGS_HEADER_H,
        };

        let tab_y_start = SETTINGS_HEADER_H + SETTINGS_TAB_PAD;
        let tab_zones = vec![
            (
                "Background".to_string(),
                Zone {
                    x: SETTINGS_TAB_PAD,
                    y: tab_y_start,
                    w: SETTINGS_SIDEBAR_W - SETTINGS_TAB_PAD * 2.0,
                    h: SETTINGS_TAB_H,
                },
            ),
            (
                "Text".to_string(),
                Zone {
                    x: SETTINGS_TAB_PAD,
                    y: tab_y_start + SETTINGS_TAB_H + SETTINGS_TAB_PAD,
                    w: SETTINGS_SIDEBAR_W - SETTINGS_TAB_PAD * 2.0,
                    h: SETTINGS_TAB_H,
                },
            ),
        ];

        let content = Zone {
            x: SETTINGS_SIDEBAR_W + 20.0,
            y: SETTINGS_HEADER_H + 20.0,
            w: window_w - SETTINGS_SIDEBAR_W - 40.0,
            h: window_h - SETTINGS_HEADER_H - 40.0,
        };

        SettingsLayout {
            header,
            back_button,
            sidebar,
            tab_zones,
            content,
        }
    }
}
