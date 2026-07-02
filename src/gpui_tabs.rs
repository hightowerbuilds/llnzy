use crate::tab_groups::{PartitionAxis, TabGroupState, TabId};

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

    pub fn joined_member_index(&self, tab_id: TabId) -> Option<usize> {
        self.groups
            .group_for_tab(tab_id)?
            .members()
            .iter()
            .position(|member| *member == tab_id)
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

    pub fn can_join(&self, source: TabId, target: TabId, max_members: usize) -> bool {
        if source == target {
            return false;
        }
        let Some(source_group) = self.groups.group_for_tab(source) else {
            return self.groups.group_member_count(target) < max_members.clamp(2, 4);
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
        combined.len() <= max_members.clamp(2, 4)
    }

    pub fn join_choices(
        &self,
        tabs: &[GpuiTabChoice],
        source: TabId,
        max_members: usize,
    ) -> Vec<GpuiTabChoice> {
        tabs.iter()
            .filter(|tab| tab.id != source)
            .filter(|tab| self.can_join(source, tab.id, max_members))
            .cloned()
            .collect()
    }

    pub fn join_tabs(&mut self, primary: TabId, secondary: TabId, max_members: usize) -> bool {
        self.join_tabs_with_axis(primary, secondary, max_members, PartitionAxis::default())
    }

    pub fn join_tabs_with_axis(
        &mut self,
        primary: TabId,
        secondary: TabId,
        max_members: usize,
        axis: PartitionAxis,
    ) -> bool {
        let joined = self
            .groups
            .join_tabs_with_axis(primary, secondary, max_members, axis)
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

    pub fn enforce_join_limit(&mut self, max_members: usize) {
        self.groups.enforce_max_members(max_members);
    }
}
