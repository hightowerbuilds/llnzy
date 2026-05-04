use crate::layout::ScreenLayout;
use crate::session::Rect;
use crate::tab_groups::TabGroupState;
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TabBarEntry {
    Single {
        tab_idx: usize,
    },
    Joined {
        primary: usize,
        secondary: usize,
        ratio: f32,
    },
}

pub fn active_joined_tabs(
    tabs: &[WorkspaceTab],
    active_tab: usize,
    groups: &TabGroupState,
) -> Option<JoinedTabs> {
    let active_id = tabs.get(active_tab)?.id;
    let group = groups.group_for_tab(active_id)?.clamped();
    let primary = tab_index_for_id(tabs, group.primary)?;
    let secondary = tab_index_for_id(tabs, group.secondary)?;
    (primary != secondary).then_some(JoinedTabs {
        primary,
        secondary,
        ratio: group.ratio,
    })
}

pub fn tab_bar_entries(tabs: &[WorkspaceTab], groups: &TabGroupState) -> Vec<TabBarEntry> {
    let tab_count = tabs.len();
    let mut grouped = std::collections::HashSet::new();
    let mut group_starts = std::collections::HashMap::new();

    for group in groups.groups() {
        let group = group.clamped();
        let Some(primary) = tab_index_for_id(tabs, group.primary) else {
            continue;
        };
        let Some(secondary) = tab_index_for_id(tabs, group.secondary) else {
            continue;
        };
        if primary == secondary {
            continue;
        }
        grouped.insert(primary);
        grouped.insert(secondary);
        group_starts.insert(
            primary.min(secondary),
            JoinedTabs {
                primary,
                secondary,
                ratio: group.ratio,
            },
        );
    }

    let mut entries = Vec::with_capacity(tab_count);
    for idx in 0..tab_count {
        if let Some(joined) = group_starts.get(&idx).copied() {
            entries.push(TabBarEntry::Joined {
                primary: joined.primary,
                secondary: joined.secondary,
                ratio: joined.ratio,
            });
        } else if !grouped.contains(&idx) {
            entries.push(TabBarEntry::Single { tab_idx: idx });
        }
    }
    entries
}

pub fn tab_index_for_id(tabs: &[WorkspaceTab], tab_id: u64) -> Option<usize> {
    tabs.iter().position(|tab| tab.id == tab_id)
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
    groups: &TabGroupState,
    active_tab: usize,
) -> Option<Rect> {
    if let Some(joined) = active_joined_tabs(tabs, active_tab, groups) {
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
        let tabs = tabs_with_ids(&[10, 11, 12, 13]);
        let mut groups = TabGroupState::default();
        groups.join_pair(12, 10);
        let entries = tab_bar_entries(&tabs, &groups);
        assert_eq!(
            entries,
            vec![
                TabBarEntry::Joined {
                    primary: 2,
                    secondary: 0,
                    ratio: 0.5
                },
                TabBarEntry::Single { tab_idx: 1 },
                TabBarEntry::Single { tab_idx: 3 },
            ]
        );
    }

    #[test]
    fn tab_bar_entries_allow_multiple_groups() {
        let tabs = tabs_with_ids(&[10, 11, 12, 13, 14]);
        let mut groups = TabGroupState::default();
        groups.join_pair(10, 11);
        groups.join_pair(13, 14);

        let entries = tab_bar_entries(&tabs, &groups);

        assert_eq!(
            entries,
            vec![
                TabBarEntry::Joined {
                    primary: 0,
                    secondary: 1,
                    ratio: 0.5
                },
                TabBarEntry::Single { tab_idx: 2 },
                TabBarEntry::Joined {
                    primary: 3,
                    secondary: 4,
                    ratio: 0.5
                },
            ]
        );
    }

    fn tabs_with_ids(ids: &[u64]) -> Vec<WorkspaceTab> {
        ids.iter()
            .copied()
            .map(|id| WorkspaceTab {
                content: crate::workspace::TabContent::Home,
                name: None,
                id,
            })
            .collect()
    }
}
