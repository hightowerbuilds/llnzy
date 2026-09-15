use crate::tab_groups::{PartitionAxis, TabGroupState, TabId, MAX_JOINED_TABS};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GpuiTabChoice {
    pub id: TabId,
    pub title: String,
    pub joined: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GpuiJoinedTabs {
    pub members: Vec<TabId>,
    pub shares: Vec<f32>,
    pub ratio: f32,
    pub axis: PartitionAxis,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GpuiTabContextMenuView {
    Main,
    JoinTargets,
    Rename,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GpuiTabContextMenu {
    pub tab_id: TabId,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub view: GpuiTabContextMenuView,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct GpuiTabManager {
    groups: TabGroupState,
    context_menu: Option<GpuiTabContextMenu>,
}

impl GpuiTabManager {
    pub fn context_menu(&self) -> Option<GpuiTabContextMenu> {
        self.context_menu
    }

    pub fn open_context_menu(&mut self, tab_id: TabId, x: f32, y: f32, width: f32) {
        self.context_menu = Some(GpuiTabContextMenu {
            tab_id,
            x,
            y,
            width,
            view: GpuiTabContextMenuView::Main,
        });
    }

    pub fn close_context_menu(&mut self) {
        self.context_menu = None;
    }

    pub fn show_join_targets(&mut self) {
        if let Some(menu) = &mut self.context_menu {
            menu.view = GpuiTabContextMenuView::JoinTargets;
        }
    }

    pub fn show_rename(&mut self) {
        if let Some(menu) = &mut self.context_menu {
            menu.view = GpuiTabContextMenuView::Rename;
        }
    }

    pub fn is_joined(&self, tab_id: TabId) -> bool {
        self.groups.group_for_tab(tab_id).is_some()
    }

    /// Which joined group this tab belongs to, as an ordinal among the live
    /// groups. Drives tab border color: every member of a group shares one
    /// color, and a separate group takes the next one.
    pub fn joined_group_ordinal(&self, tab_id: TabId) -> Option<usize> {
        self.groups.group_ordinal_for_tab(tab_id)
    }

    pub fn joined_group_for(
        &self,
        active_tab: TabId,
        valid_tabs: &[TabId],
    ) -> Option<GpuiJoinedTabs> {
        let group = self.groups.group_for_tab(active_tab)?.clamped();
        if group.member_count() < 2
            || !group
                .members()
                .iter()
                .all(|member| valid_tabs.contains(member))
        {
            return None;
        }
        Some(GpuiJoinedTabs {
            members: group.members().to_vec(),
            shares: group.shares().to_vec(),
            ratio: group.ratio,
            axis: group.axis,
        })
    }

    pub fn joined_member_count(&self, tab_id: TabId) -> usize {
        self.groups.group_member_count(tab_id)
    }

    pub fn can_join(&self, source: TabId, target: TabId) -> bool {
        if source == target {
            return false;
        }
        let Some(source_group) = self.groups.group_for_tab(source) else {
            return self.groups.group_member_count(target) < MAX_JOINED_TABS;
        };
        if source_group.contains(target) {
            return false;
        }
        let source_members = source_group.members();
        let target_members = self
            .groups
            .group_for_tab(target)
            .map(|group| group.members().to_vec())
            .unwrap_or_else(|| vec![target]);
        let mut combined = source_members.to_vec();
        for member in target_members {
            if !combined.contains(&member) {
                combined.push(member);
            }
        }
        combined.len() <= MAX_JOINED_TABS
    }

    pub fn join_choices(&self, tabs: &[GpuiTabChoice], source: TabId) -> Vec<GpuiTabChoice> {
        tabs.iter()
            .filter(|tab| tab.id != source)
            .filter(|tab| self.can_join(source, tab.id))
            .cloned()
            .collect()
    }

    pub fn join_tabs(&mut self, primary: TabId, secondary: TabId) -> bool {
        self.join_tabs_with_axis(primary, secondary, PartitionAxis::default())
    }

    pub fn join_tabs_with_axis(
        &mut self,
        primary: TabId,
        secondary: TabId,
        axis: PartitionAxis,
    ) -> bool {
        let joined = self
            .groups
            .join_tabs_with_axis(primary, secondary, MAX_JOINED_TABS, axis)
            .is_some();
        if joined {
            self.groups.set_active_tab(primary);
            self.close_context_menu();
        }
        joined
    }

    pub fn separate_tab(&mut self, tab_id: TabId) -> bool {
        let separated = self.groups.separate_tab(tab_id);
        if separated {
            self.close_context_menu();
        }
        separated
    }

    pub fn swap_tabs_for_tab(&mut self, tab_id: TabId) -> bool {
        let swapped = self.groups.swap_tabs_for_tab(tab_id);
        if swapped {
            self.close_context_menu();
        }
        swapped
    }

    pub fn set_active_tab(&mut self, tab_id: TabId) {
        self.groups.set_active_tab(tab_id);
    }

    pub fn set_split_for_tab(
        &mut self,
        tab_id: TabId,
        divider_index: usize,
        boundary: f32,
    ) -> bool {
        self.groups
            .set_split_for_tab(tab_id, divider_index, boundary)
    }

    pub fn restore_shares_for_tab(&mut self, tab_id: TabId, shares: &[f32]) -> bool {
        self.groups.restore_shares_for_tab(tab_id, shares)
    }

    pub fn retain_tabs(&mut self, valid_tabs: &[TabId]) {
        self.groups
            .retain_tabs(|tab_id| valid_tabs.contains(&tab_id));
        if self
            .context_menu
            .is_some_and(|menu| !valid_tabs.contains(&menu.tab_id))
        {
            self.context_menu = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_four_tabs_and_rejects_a_fifth_without_changing_the_group() {
        for axis in [PartitionAxis::Vertical, PartitionAxis::Horizontal] {
            let mut manager = GpuiTabManager::default();
            for target in 2..=4 {
                assert!(manager.can_join(1, target));
                assert!(manager.can_join(target, 1));
                assert!(manager.join_tabs_with_axis(1, target, axis));
            }
            let before = manager.joined_group_for(4, &[1, 2, 3, 4, 5]).unwrap();
            assert_eq!(before.members, vec![1, 2, 3, 4]);
            assert_eq!(before.axis, axis);
            assert!(!manager.can_join(1, 5));
            assert!(!manager.can_join(5, 1));
            assert!(!manager.join_tabs(1, 5));
            assert_eq!(manager.joined_group_for(1, &[1, 2, 3, 4, 5]), Some(before));
            let choices = (1..=5)
                .map(|id| GpuiTabChoice {
                    id,
                    title: id.to_string(),
                    joined: id <= 4,
                })
                .collect::<Vec<_>>();
            assert!(manager.join_choices(&choices, 1).is_empty());
            assert!(manager.join_choices(&choices, 5).is_empty());
        }
    }

    #[test]
    fn two_pairs_merge_and_four_panes_can_resize_separate_and_close() {
        let mut manager = GpuiTabManager::default();
        assert!(manager.join_tabs(1, 2));
        assert!(manager.join_tabs(3, 4));
        assert!(manager.can_join(1, 3));
        assert!(manager.join_tabs(1, 3));
        for (divider, boundary) in [0.18, 0.48, 0.8].into_iter().enumerate() {
            assert!(manager.set_split_for_tab(1, divider, boundary));
        }
        let group = manager.joined_group_for(4, &[1, 2, 3, 4]).unwrap();
        for (actual, expected) in group.shares.iter().zip([0.18, 0.30, 0.32, 0.20]) {
            assert!((actual - expected).abs() < 0.0001);
        }
        manager.set_active_tab(4);
        assert!(manager.separate_tab(4));
        assert_eq!(manager.joined_member_count(1), 3);
        assert!(!manager.is_joined(4));
        assert!(manager.join_tabs(1, 5));
        manager.retain_tabs(&[1, 3, 4, 5]);
        assert_eq!(
            manager.joined_group_for(5, &[1, 3, 4, 5]).unwrap().members,
            vec![1, 3, 5]
        );
    }
}
