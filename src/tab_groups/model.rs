pub type TabId = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TabGroupId(u64);

impl TabGroupId {
    pub fn raw(self) -> u64 {
        self.0
    }
}

/// Orientation of a partition (split) between two tabs.
///
/// `Vertical` means a vertical divider line between two side-by-side shelves
/// (left / right). `Horizontal` means a horizontal divider line between two
/// stacked shelves (top / bottom). The naming matches the visible divider,
/// which is the user-facing reference point.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PartitionAxis {
    #[default]
    Vertical,
    Horizontal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TabGroup {
    pub id: TabGroupId,
    pub primary: TabId,
    pub secondary: TabId,
    pub active: TabId,
    pub ratio: f32,
    pub axis: PartitionAxis,
}

impl TabGroup {
    pub const MIN_RATIO: f32 = 0.18;
    pub const MAX_RATIO: f32 = 0.82;

    pub fn new(id: TabGroupId, primary: TabId, secondary: TabId, axis: PartitionAxis) -> Self {
        Self {
            id,
            primary,
            secondary,
            active: primary,
            ratio: 0.5,
            axis,
        }
    }

    pub fn contains(&self, tab_id: TabId) -> bool {
        self.primary == tab_id || self.secondary == tab_id
    }

    pub fn members(&self) -> [TabId; 2] {
        [self.primary, self.secondary]
    }

    pub fn clamped(&self) -> Self {
        Self {
            ratio: self.ratio.clamp(Self::MIN_RATIO, Self::MAX_RATIO),
            ..self.clone()
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TabGroupState {
    groups: Vec<TabGroup>,
    next_group_id: u64,
}

impl TabGroupState {
    pub fn groups(&self) -> &[TabGroup] {
        &self.groups
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    pub fn group_for_tab(&self, tab_id: TabId) -> Option<&TabGroup> {
        self.groups.iter().find(|group| group.contains(tab_id))
    }

    pub fn group_for_tab_mut(&mut self, tab_id: TabId) -> Option<&mut TabGroup> {
        self.groups.iter_mut().find(|group| group.contains(tab_id))
    }

    pub fn join_pair(&mut self, primary: TabId, secondary: TabId) -> Option<TabGroupId> {
        self.join_pair_with_axis(primary, secondary, PartitionAxis::default())
    }

    pub fn join_pair_with_axis(
        &mut self,
        primary: TabId,
        secondary: TabId,
        axis: PartitionAxis,
    ) -> Option<TabGroupId> {
        if primary == secondary {
            return None;
        }
        self.remove_tab(primary);
        self.remove_tab(secondary);
        let id = self.alloc_group_id();
        self.groups
            .push(TabGroup::new(id, primary, secondary, axis));
        Some(id)
    }

    pub fn separate_tab(&mut self, tab_id: TabId) -> bool {
        let before = self.groups.len();
        self.groups.retain(|group| !group.contains(tab_id));
        before != self.groups.len()
    }

    pub fn remove_tab(&mut self, tab_id: TabId) -> bool {
        self.separate_tab(tab_id)
    }

    pub fn retain_tabs(&mut self, valid: impl Fn(TabId) -> bool) {
        self.groups
            .retain(|group| group.members().into_iter().all(&valid));
    }

    pub fn set_active_tab(&mut self, tab_id: TabId) {
        if let Some(group) = self.group_for_tab_mut(tab_id) {
            group.active = tab_id;
        }
    }

    pub fn set_ratio_for_tab(&mut self, tab_id: TabId, ratio: f32) -> bool {
        let Some(group) = self.group_for_tab_mut(tab_id) else {
            return false;
        };
        let ratio = ratio.clamp(TabGroup::MIN_RATIO, TabGroup::MAX_RATIO);
        if (group.ratio - ratio).abs() <= f32::EPSILON {
            return false;
        }
        group.ratio = ratio;
        true
    }

    pub fn swap_tabs_for_tab(&mut self, tab_id: TabId) -> bool {
        let Some(group) = self.group_for_tab_mut(tab_id) else {
            return false;
        };
        std::mem::swap(&mut group.primary, &mut group.secondary);
        true
    }

    fn alloc_group_id(&mut self) -> TabGroupId {
        self.next_group_id += 1;
        TabGroupId(self.next_group_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_pair_rejects_same_tab() {
        let mut groups = TabGroupState::default();

        assert_eq!(groups.join_pair(1, 1), None);
        assert!(groups.is_empty());
    }

    #[test]
    fn join_pair_creates_one_group_for_standalone_tabs() {
        let mut groups = TabGroupState::default();

        let id = groups.join_pair(1, 2).unwrap();

        assert_eq!(id.raw(), 1);
        assert_eq!(groups.groups().len(), 1);

        let group = groups.group_for_tab(1).unwrap();
        assert_eq!(group.id, id);
        assert_eq!(group.primary, 1);
        assert_eq!(group.secondary, 2);
        assert_eq!(group.active, 1);
        assert_eq!(group.ratio, 0.5);
    }

    #[test]
    fn join_pair_allows_multiple_groups() {
        let mut groups = TabGroupState::default();

        groups.join_pair(1, 2);
        groups.join_pair(3, 4);

        assert_eq!(groups.groups().len(), 2);
        assert!(groups.group_for_tab(1).is_some());
        assert!(groups.group_for_tab(4).is_some());
    }

    #[test]
    fn join_pair_removes_only_involved_existing_groups() {
        let mut groups = TabGroupState::default();
        groups.join_pair(1, 2);
        groups.join_pair(3, 4);

        groups.join_pair(2, 5);

        assert!(groups.group_for_tab(1).is_none());
        assert!(groups.group_for_tab(2).is_some());
        assert!(groups.group_for_tab(3).is_some());
        assert!(groups.group_for_tab(4).is_some());
        assert_eq!(groups.groups().len(), 2);
    }

    #[test]
    fn join_pair_rehomes_only_tabs_in_the_new_pair() {
        let mut groups = TabGroupState::default();
        groups.join_pair(1, 2);
        groups.join_pair(3, 4);

        groups.join_pair(2, 5);

        assert_eq!(group_members(&groups, 2), (2, 5));
        assert_eq!(group_members(&groups, 3), (3, 4));
        assert!(groups.group_for_tab(1).is_none());
        assert_eq!(groups.groups().len(), 2);
    }

    #[test]
    fn separate_tab_removes_one_group_without_affecting_others() {
        let mut groups = TabGroupState::default();
        groups.join_pair(1, 2);
        groups.join_pair(3, 4);

        assert!(groups.separate_tab(1));

        assert!(groups.group_for_tab(1).is_none());
        assert!(groups.group_for_tab(2).is_none());
        assert!(groups.group_for_tab(3).is_some());
        assert!(groups.group_for_tab(4).is_some());
    }

    #[test]
    fn remove_tab_dissolves_only_the_closed_tabs_group() {
        let mut groups = TabGroupState::default();
        groups.join_pair(1, 2);
        groups.join_pair(3, 4);

        assert!(groups.remove_tab(2));

        assert!(groups.group_for_tab(1).is_none());
        assert!(groups.group_for_tab(2).is_none());
        assert_eq!(group_members(&groups, 3), (3, 4));
        assert_eq!(groups.groups().len(), 1);
    }

    #[test]
    fn retain_tabs_drops_groups_with_closed_members() {
        let mut groups = TabGroupState::default();
        groups.join_pair(1, 2);
        groups.join_pair(3, 4);

        groups.retain_tabs(|tab_id| tab_id != 2);

        assert!(groups.group_for_tab(1).is_none());
        assert!(groups.group_for_tab(3).is_some());
    }

    #[test]
    fn retain_tabs_preserves_groups_across_reorder() {
        let mut groups = TabGroupState::default();
        groups.join_pair(1, 2);
        groups.join_pair(3, 4);

        let reordered = [4, 1, 3, 2];
        groups.retain_tabs(|tab_id| reordered.contains(&tab_id));

        assert_eq!(group_members(&groups, 1), (1, 2));
        assert_eq!(group_members(&groups, 3), (3, 4));
        assert_eq!(groups.groups().len(), 2);
    }

    #[test]
    fn set_active_tab_updates_only_containing_group() {
        let mut groups = TabGroupState::default();
        groups.join_pair(1, 2);
        groups.join_pair(3, 4);

        groups.set_active_tab(4);

        assert_eq!(groups.group_for_tab(1).unwrap().active, 1);
        assert_eq!(groups.group_for_tab(3).unwrap().active, 4);
    }

    #[test]
    fn set_ratio_for_tab_clamps_to_group_bounds() {
        let mut groups = TabGroupState::default();
        groups.join_pair(1, 2);

        assert!(groups.set_ratio_for_tab(2, 0.03));
        assert_eq!(groups.group_for_tab(1).unwrap().ratio, TabGroup::MIN_RATIO);

        assert!(groups.set_ratio_for_tab(1, 0.95));
        assert_eq!(groups.group_for_tab(2).unwrap().ratio, TabGroup::MAX_RATIO);
    }

    #[test]
    fn swap_tabs_for_tab_flips_left_and_right_members() {
        let mut groups = TabGroupState::default();
        groups.join_pair(1, 2);

        assert!(groups.swap_tabs_for_tab(1));

        let group = groups.group_for_tab(1).unwrap();
        assert_eq!(group.primary, 2);
        assert_eq!(group.secondary, 1);
    }

    #[test]
    fn join_pair_defaults_to_vertical_axis() {
        let mut groups = TabGroupState::default();
        groups.join_pair(1, 2).unwrap();
        assert_eq!(
            groups.group_for_tab(1).unwrap().axis,
            PartitionAxis::Vertical
        );
    }

    #[test]
    fn join_pair_with_axis_records_the_axis() {
        let mut groups = TabGroupState::default();
        groups
            .join_pair_with_axis(1, 2, PartitionAxis::Horizontal)
            .unwrap();
        assert_eq!(
            groups.group_for_tab(1).unwrap().axis,
            PartitionAxis::Horizontal
        );
    }

    #[test]
    fn swap_tabs_preserves_axis() {
        let mut groups = TabGroupState::default();
        groups
            .join_pair_with_axis(1, 2, PartitionAxis::Horizontal)
            .unwrap();
        assert!(groups.swap_tabs_for_tab(1));
        assert_eq!(
            groups.group_for_tab(1).unwrap().axis,
            PartitionAxis::Horizontal
        );
    }

    #[test]
    fn swap_tabs_preserves_active_tab_and_ratio() {
        let mut groups = TabGroupState::default();
        groups.join_pair(1, 2);
        groups.set_active_tab(2);
        groups.set_ratio_for_tab(1, 0.72);

        assert!(groups.swap_tabs_for_tab(2));

        let group = groups.group_for_tab(1).unwrap();
        assert_eq!(group.primary, 2);
        assert_eq!(group.secondary, 1);
        assert_eq!(group.active, 2);
        assert!((group.ratio - 0.72).abs() <= f32::EPSILON);
    }

    fn group_members(groups: &TabGroupState, tab_id: TabId) -> (TabId, TabId) {
        let group = groups.group_for_tab(tab_id).unwrap();
        (group.primary, group.secondary)
    }
}
