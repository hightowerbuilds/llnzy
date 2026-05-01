use crate::layout::ScreenLayout;
use crate::session::Rect;
use crate::workspace::WorkspaceTab;

pub const JOINED_DIVIDER_GAP: f32 = 8.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JoinedTabs {
    pub primary: usize,
    pub secondary: usize,
    pub ratio: f32,
}

impl JoinedTabs {
    pub const MIN_RATIO: f32 = 0.18;
    pub const MAX_RATIO: f32 = 0.82;

    pub fn new(primary: usize, secondary: usize) -> Self {
        Self {
            primary,
            secondary,
            ratio: 0.5,
        }
    }

    pub fn contains(self, idx: usize) -> bool {
        self.primary == idx || self.secondary == idx
    }

    pub fn clamped(self) -> Self {
        Self {
            ratio: self.ratio.clamp(Self::MIN_RATIO, Self::MAX_RATIO),
            ..self
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabBarEntry {
    Single { tab_idx: usize },
    Joined { primary: usize, secondary: usize },
}

pub fn valid_joined_tabs(
    joined: Option<JoinedTabs>,
    active_tab: usize,
    tab_count: usize,
) -> Option<JoinedTabs> {
    let joined = joined?.clamped();
    (joined.primary < tab_count
        && joined.secondary < tab_count
        && joined.primary != joined.secondary
        && joined.contains(active_tab))
    .then_some(joined)
}

pub fn tab_bar_entries(tab_count: usize, joined: Option<JoinedTabs>) -> Vec<TabBarEntry> {
    let Some(joined) = joined.map(JoinedTabs::clamped) else {
        return (0..tab_count)
            .map(|tab_idx| TabBarEntry::Single { tab_idx })
            .collect();
    };

    if joined.primary >= tab_count
        || joined.secondary >= tab_count
        || joined.primary == joined.secondary
    {
        return (0..tab_count)
            .map(|tab_idx| TabBarEntry::Single { tab_idx })
            .collect();
    }

    let first = joined.primary.min(joined.secondary);
    let second = joined.primary.max(joined.secondary);
    let mut entries = Vec::with_capacity(tab_count.saturating_sub(1));
    for idx in 0..tab_count {
        if idx == first {
            entries.push(TabBarEntry::Joined {
                primary: joined.primary,
                secondary: joined.secondary,
            });
        } else if idx != second {
            entries.push(TabBarEntry::Single { tab_idx: idx });
        }
    }
    entries
}

pub fn joined_content_rects(layout: &ScreenLayout, ratio: f32) -> (Rect, Rect) {
    let content = &layout.content;
    let (left_w, right_w) = joined_split_widths(content.w, ratio);
    let left = Rect {
        x: content.x,
        y: content.y,
        w: left_w,
        h: content.h,
    };
    let right = Rect {
        x: content.x + left_w + JOINED_DIVIDER_GAP,
        y: content.y,
        w: right_w,
        h: content.h,
    };
    (left, right)
}

pub fn joined_split_widths(total_width: f32, ratio: f32) -> (f32, f32) {
    let usable_w = (total_width - JOINED_DIVIDER_GAP).max(2.0);
    let ratio = ratio.clamp(JoinedTabs::MIN_RATIO, JoinedTabs::MAX_RATIO);
    let left_w = (usable_w * ratio).max(1.0);
    let right_w = (usable_w - left_w).max(1.0);
    (left_w, right_w)
}

pub fn joined_terminal_content_rects(layout: &ScreenLayout, ratio: f32) -> (Rect, Rect) {
    let (left, mut right) = joined_content_rects(layout, ratio);
    let inset = layout.content_padding_x.min((right.w - 1.0).max(0.0));
    right.x += inset;
    right.w = (right.w - inset).max(1.0);
    (left, right)
}

pub fn terminal_effect_rect(
    tabs: &[WorkspaceTab],
    layout: &ScreenLayout,
    joined: Option<JoinedTabs>,
    active_tab: usize,
) -> Option<Rect> {
    if let Some(joined) = valid_joined_tabs(joined, active_tab, tabs.len()) {
        let (left, right) = joined_content_rects(layout, joined.ratio);
        let primary_terminal = tabs
            .get(joined.primary)
            .is_some_and(|tab| tab.content.as_terminal().is_some());
        let secondary_terminal = tabs
            .get(joined.secondary)
            .is_some_and(|tab| tab.content.as_terminal().is_some());

        return match (primary_terminal, secondary_terminal) {
            (true, true) => Some(Rect {
                x: layout.content.x,
                y: layout.content.y,
                w: layout.content.w,
                h: layout.content.h,
            }),
            (true, false) => Some(left),
            (false, true) => Some(right),
            (false, false) => None,
        };
    }

    tabs.get(active_tab)
        .filter(|tab| tab.content.as_terminal().is_some())
        .map(|_| Rect {
            x: layout.content.x,
            y: layout.content.y,
            w: layout.content.w,
            h: layout.content.h,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_bar_entries_group_joined_tabs_once() {
        let entries = tab_bar_entries(4, Some(JoinedTabs::new(2, 0)));
        assert_eq!(
            entries,
            vec![
                TabBarEntry::Joined {
                    primary: 2,
                    secondary: 0
                },
                TabBarEntry::Single { tab_idx: 1 },
                TabBarEntry::Single { tab_idx: 3 },
            ]
        );
    }
}
