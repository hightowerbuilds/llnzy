use crate::tab_groups::{PartitionAxis, TabGroupState, TabId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GpuiTabChoice {
    pub id: TabId,
    pub title: String,
    pub joined: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GpuiJoinedTabs {
    pub primary: TabId,
    pub secondary: TabId,
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

    pub fn joined_pair_for(
        &self,
        active_tab: TabId,
        valid_tabs: &[TabId],
    ) -> Option<GpuiJoinedTabs> {
        let group = self.groups.group_for_tab(active_tab)?.clamped();
        if !valid_tabs.contains(&group.primary) || !valid_tabs.contains(&group.secondary) {
            return None;
        }
        if group.primary == group.secondary {
            return None;
        }
        Some(GpuiJoinedTabs {
            primary: group.primary,
            secondary: group.secondary,
            ratio: group.ratio,
            axis: group.axis,
        })
    }

    pub fn join_choices(&self, tabs: &[GpuiTabChoice], source: TabId) -> Vec<GpuiTabChoice> {
        tabs.iter()
            .filter(|tab| tab.id != source)
            .filter(|tab| !self.is_joined(tab.id))
            .cloned()
            .collect()
    }

    pub fn join_pair(&mut self, primary: TabId, secondary: TabId) -> bool {
        self.join_pair_with_axis(primary, secondary, PartitionAxis::default())
    }

    pub fn join_pair_with_axis(
        &mut self,
        primary: TabId,
        secondary: TabId,
        axis: PartitionAxis,
    ) -> bool {
        let joined = self
            .groups
            .join_pair_with_axis(primary, secondary, axis)
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

    pub fn set_ratio_for_tab(&mut self, tab_id: TabId, ratio: f32) -> bool {
        self.groups.set_ratio_for_tab(tab_id, ratio)
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
