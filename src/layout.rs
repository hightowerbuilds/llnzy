use taffy::prelude::*;

/// All computed screen regions, updated once per frame on resize.
#[derive(Clone, Debug)]
pub struct ScreenLayout {
    pub window_w: f32,
    pub window_h: f32,
    pub sidebar_w: f32,
    pub tab_bar: Zone,
    pub content: Zone,
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

impl Zone {
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

// ── Layout constants ──

pub const TAB_BAR_HEIGHT: f32 = 28.0;
pub const FOOTER_HEIGHT: f32 = 36.0;

impl ScreenLayout {
    /// Compute the full screen layout using Taffy flexbox.
    pub fn compute(
        window_w: f32,
        window_h: f32,
        cell_w: f32,
        cell_h: f32,
        _tab_count: usize,
        padding_x: f32,
        padding_y: f32,
        glyph_offset_x: f32,
        sidebar_w: f32,
    ) -> Self {
        let padding_x = padding_x + glyph_offset_x;
        let mut tree: TaffyTree<()> = TaffyTree::new();
        let show_tab_bar = true; // always show tab bar

        // Effective width excluding sidebar (sidebar is rendered by egui)
        let effective_w = (window_w - sidebar_w).max(1.0);

        // ── Build flexbox tree ──
        // Root: vertical column filling the effective area
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
                        width: Dimension::Length(effective_w),
                        height: Dimension::Length(window_h),
                    },
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
                &[tab_bar_node, content_node, footer_node],
            )
            .unwrap();

        tree.compute_layout(root, Size::MAX_CONTENT).unwrap();

        // ── Extract computed positions (offset by sidebar_w) ──

        let tab_bar_layout = tree.layout(tab_bar_node).unwrap();
        let content_layout = tree.layout(content_node).unwrap();

        let tab_bar = Zone {
            x: tab_bar_layout.location.x + sidebar_w,
            y: tab_bar_layout.location.y,
            w: tab_bar_layout.size.width,
            h: tab_bar_layout.size.height,
        };

        // Content zone: the padding is handled by Taffy, so content_box gives us
        // the inner area. But Taffy's layout gives the outer box — we need inner.
        let content = Zone {
            x: content_layout.location.x + padding_x + sidebar_w,
            y: content_layout.location.y + padding_y,
            w: content_layout.size.width - padding_x * 2.0,
            h: content_layout.size.height - padding_y * 2.0,
        };

        // ── Grid dimensions from content zone ──
        let grid_cols = (content.w / cell_w).max(1.0) as u16;
        let grid_rows = (content.h / cell_h).max(1.0) as u16;

        ScreenLayout {
            window_w,
            window_h,
            sidebar_w,
            tab_bar,
            content,
            cell_w,
            cell_h,
            grid_cols,
            grid_rows,
            show_tab_bar,
        }
    }

    /// Convert pixel position to grid (row, col) within the content zone.
    pub fn pixel_to_grid(&self, px: f32, py: f32) -> (usize, usize) {
        let col = ((px - self.content.x) / self.cell_w).max(0.0) as usize;
        let row = ((py - self.content.y) / self.cell_h).max(0.0) as usize;
        let col = col.min(self.grid_cols.saturating_sub(1) as usize);
        let row = row.min(self.grid_rows.saturating_sub(1) as usize);
        (row, col)
    }
}
