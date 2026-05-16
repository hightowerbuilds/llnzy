pub type TabId = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TabGroupId(u64);

impl TabGroupId {
    pub fn raw(self) -> u64 {
        self.0
    }
}

/// Orientation of a partition (split) between joined tabs.
///
/// `Vertical` means vertical dividers between side-by-side shelves
/// (left / right). `Horizontal` means horizontal dividers between stacked
/// shelves (top / bottom). The naming matches the visible divider, which is
/// the user-facing reference point.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PartitionAxis {
    #[default]
    Vertical,
    Horizontal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TabGroup {
    pub id: TabGroupId,
    members: Vec<TabId>,
    shares: Vec<f32>,
    pub active: TabId,
    pub ratio: f32,
    pub axis: PartitionAxis,
}

impl TabGroup {
    pub const MIN_RATIO: f32 = 0.18;
    pub const MAX_RATIO: f32 = 0.82;
    pub const MIN_MULTI_SHARE: f32 = 0.12;

    pub fn new(id: TabGroupId, primary: TabId, secondary: TabId, axis: PartitionAxis) -> Self {
        Self::from_members(
            id,
            vec![primary, secondary],
            primary,
            axis,
            0.5,
            pair_shares(0.5),
        )
    }

    fn from_members(
        id: TabGroupId,
        members: Vec<TabId>,
        active: TabId,
        axis: PartitionAxis,
        ratio: f32,
        shares: Vec<f32>,
    ) -> Self {
        let members = dedupe_members(members);
        let shares = normalize_shares(members.len(), shares, ratio);
        let ratio = shares.first().copied().unwrap_or(0.5);
        let active = if members.contains(&active) {
            active
        } else {
            members.first().copied().unwrap_or(active)
        };
        Self {
            id,
            members,
            shares,
            active,
            ratio,
            axis,
        }
    }

    pub fn contains(&self, tab_id: TabId) -> bool {
        self.members.contains(&tab_id)
    }

    pub fn members(&self) -> &[TabId] {
        &self.members
    }

    pub fn shares(&self) -> &[f32] {
        &self.shares
    }

    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    pub fn primary(&self) -> Option<TabId> {
        self.members.first().copied()
    }

    pub fn secondary(&self) -> Option<TabId> {
        self.members.get(1).copied()
    }

    pub fn clamped(&self) -> Self {
        let mut group = self.clone();
        group.normalize_layout();
        group
    }

    fn normalize_layout(&mut self) {
        self.shares = normalize_shares(self.members.len(), self.shares.clone(), self.ratio);
        self.ratio = self.shares.first().copied().unwrap_or(0.5);
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

    pub fn group_member_count(&self, tab_id: TabId) -> usize {
        self.group_for_tab(tab_id)
            .map(TabGroup::member_count)
            .unwrap_or(1)
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
        self.join_tabs_with_axis(primary, secondary, 2, axis)
    }

    pub fn join_tabs_with_axis(
        &mut self,
        primary: TabId,
        secondary: TabId,
        max_members: usize,
        axis: PartitionAxis,
    ) -> Option<TabGroupId> {
        if primary == secondary {
            return None;
        }

        let max_members = max_members.clamp(2, 4);
        let primary_group = self.group_for_tab(primary).cloned();
        let secondary_group = self.group_for_tab(secondary).cloned();

        if primary_group
            .as_ref()
            .is_some_and(|group| group.contains(secondary))
        {
            return None;
        }

        if max_members == 2 {
            let preserved_axis = primary_group
                .as_ref()
                .or(secondary_group.as_ref())
                .map(|group| group.axis)
                .unwrap_or(axis);
            let preserved_ratio = primary_group
                .as_ref()
                .or(secondary_group.as_ref())
                .map(|group| group.ratio)
                .unwrap_or(0.5);
            let removed_ids = [
                primary_group.as_ref().map(|group| group.id),
                secondary_group.as_ref().map(|group| group.id),
            ];
            self.groups
                .retain(|group| !removed_ids.into_iter().flatten().any(|id| id == group.id));

            let id = self.alloc_group_id();
            self.groups.push(TabGroup::from_members(
                id,
                vec![primary, secondary],
                primary,
                preserved_axis,
                preserved_ratio,
                pair_shares(preserved_ratio),
            ));
            return Some(id);
        }

        let (primary_members, primary_shares) = members_and_shares(primary_group.as_ref(), primary);
        let (secondary_members, secondary_shares) =
            members_and_shares(secondary_group.as_ref(), secondary);

        let mut members = primary_members.clone();
        members.extend(secondary_members.iter().copied());
        let members = dedupe_members(members);

        if members.len() < 2 || members.len() > max_members {
            return None;
        }

        let preserved_axis = primary_group
            .as_ref()
            .or(secondary_group.as_ref())
            .map(|group| group.axis)
            .unwrap_or(axis);
        let preserved_ratio = primary_group
            .as_ref()
            .or(secondary_group.as_ref())
            .map(|group| group.ratio)
            .unwrap_or(0.5);

        let removed_ids = [
            primary_group.as_ref().map(|group| group.id),
            secondary_group.as_ref().map(|group| group.id),
        ];
        self.groups
            .retain(|group| !removed_ids.into_iter().flatten().any(|id| id == group.id));

        let id = self.alloc_group_id();
        self.groups.push(TabGroup::from_members(
            id,
            members,
            primary,
            preserved_axis,
            preserved_ratio,
            merge_shares(
                &primary_shares,
                primary_members.len(),
                &secondary_shares,
                secondary_members.len(),
            ),
        ));
        Some(id)
    }

    pub fn separate_tab(&mut self, tab_id: TabId) -> bool {
        let Some(index) = self.groups.iter().position(|group| group.contains(tab_id)) else {
            return false;
        };

        if self.groups[index].member_count() <= 2 {
            self.groups.remove(index);
            return true;
        }

        let group = &mut self.groups[index];
        if let Some(member_index) = group.members.iter().position(|member| *member == tab_id) {
            group.members.remove(member_index);
            if member_index < group.shares.len() {
                group.shares.remove(member_index);
            }
            group.normalize_layout();
        }
        if !group.contains(group.active) {
            if let Some(first) = group.primary() {
                group.active = first;
            }
        }
        true
    }

    pub fn remove_tab(&mut self, tab_id: TabId) -> bool {
        self.separate_tab(tab_id)
    }

    pub fn retain_tabs(&mut self, valid: impl Fn(TabId) -> bool) {
        for group in &mut self.groups {
            let mut members = Vec::with_capacity(group.members.len());
            let mut shares = Vec::with_capacity(group.shares.len());
            for (index, member) in group.members.iter().copied().enumerate() {
                if valid(member) {
                    members.push(member);
                    shares.push(group.shares.get(index).copied().unwrap_or(1.0));
                }
            }
            group.members = members;
            group.shares = shares;
            group.normalize_layout();
            if !group.contains(group.active) {
                if let Some(first) = group.primary() {
                    group.active = first;
                }
            }
        }
        self.groups.retain(|group| group.member_count() >= 2);
    }

    pub fn set_active_tab(&mut self, tab_id: TabId) {
        if let Some(group) = self.group_for_tab_mut(tab_id) {
            group.active = tab_id;
        }
    }

    pub fn set_ratio_for_tab(&mut self, tab_id: TabId, ratio: f32) -> bool {
        self.set_split_for_tab(tab_id, 0, ratio)
    }

    pub fn set_split_for_tab(
        &mut self,
        tab_id: TabId,
        divider_index: usize,
        boundary: f32,
    ) -> bool {
        let Some(group) = self.group_for_tab_mut(tab_id) else {
            return false;
        };
        if divider_index + 1 >= group.member_count() {
            return false;
        }

        group.normalize_layout();
        let count = group.member_count();
        let min_share = min_share_for_count(count);
        let before_fixed: f32 = group.shares[..divider_index].iter().sum();
        let after_fixed: f32 = group.shares[divider_index + 2..].iter().sum();
        let min_boundary = before_fixed + min_share;
        let max_boundary = 1.0 - after_fixed - min_share;
        if min_boundary > max_boundary {
            return false;
        }
        let boundary = boundary.clamp(min_boundary, max_boundary);
        let left = boundary - before_fixed;
        let right = 1.0 - boundary - after_fixed;

        if (group.shares[divider_index] - left).abs() <= f32::EPSILON
            && (group.shares[divider_index + 1] - right).abs() <= f32::EPSILON
        {
            return false;
        }

        group.shares[divider_index] = left;
        group.shares[divider_index + 1] = right;
        group.normalize_layout();
        true
    }

    pub fn swap_tabs_for_tab(&mut self, tab_id: TabId) -> bool {
        let Some(group) = self.group_for_tab_mut(tab_id) else {
            return false;
        };
        if group.member_count() != 2 {
            return false;
        }
        group.members.swap(0, 1);
        group.normalize_layout();
        true
    }

    pub fn enforce_max_members(&mut self, max_members: usize) {
        let max_members = max_members.clamp(2, 4);
        for group in &mut self.groups {
            group.members.truncate(max_members);
            group.shares.truncate(max_members);
            group.normalize_layout();
            if !group.contains(group.active) {
                if let Some(first) = group.primary() {
                    group.active = first;
                }
            }
        }
        self.groups.retain(|group| group.member_count() >= 2);
    }

    fn alloc_group_id(&mut self) -> TabGroupId {
        self.next_group_id += 1;
        TabGroupId(self.next_group_id)
    }
}

fn dedupe_members(members: Vec<TabId>) -> Vec<TabId> {
    let mut deduped = Vec::with_capacity(members.len());
    for member in members {
        if !deduped.contains(&member) {
            deduped.push(member);
        }
    }
    deduped
}

fn pair_shares(ratio: f32) -> Vec<f32> {
    let ratio = ratio.clamp(TabGroup::MIN_RATIO, TabGroup::MAX_RATIO);
    vec![ratio, 1.0 - ratio]
}

fn min_share_for_count(count: usize) -> f32 {
    if count <= 2 {
        TabGroup::MIN_RATIO
    } else {
        TabGroup::MIN_MULTI_SHARE
    }
}

fn normalize_shares(count: usize, shares: Vec<f32>, fallback_ratio: f32) -> Vec<f32> {
    if count == 0 {
        return Vec::new();
    }
    let min_share = min_share_for_count(count);
    let mut shares = if shares.len() == count {
        shares
    } else if count == 2 {
        pair_shares(fallback_ratio)
    } else {
        vec![1.0 / count as f32; count]
    };

    if shares.iter().any(|share| !share.is_finite()) {
        shares = vec![1.0 / count as f32; count];
    }

    for share in &mut shares {
        *share = share.max(min_share);
    }
    let total: f32 = shares.iter().sum();
    if total <= f32::EPSILON {
        return vec![1.0 / count as f32; count];
    }
    for share in &mut shares {
        *share /= total;
    }
    shares
}

fn members_and_shares(group: Option<&TabGroup>, fallback: TabId) -> (Vec<TabId>, Vec<f32>) {
    match group {
        Some(group) => {
            let group = group.clamped();
            (group.members().to_vec(), group.shares().to_vec())
        }
        None => (vec![fallback], vec![1.0]),
    }
}

fn merge_shares(
    primary: &[f32],
    primary_count: usize,
    secondary: &[f32],
    secondary_count: usize,
) -> Vec<f32> {
    let total_count = primary_count + secondary_count;
    if total_count == 0 {
        return Vec::new();
    }
    let primary_weight = primary_count as f32 / total_count as f32;
    let secondary_weight = secondary_count as f32 / total_count as f32;
    let mut merged = primary
        .iter()
        .map(|share| share * primary_weight)
        .collect::<Vec<_>>();
    merged.extend(secondary.iter().map(|share| share * secondary_weight));
    merged
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
        assert_eq!(group.members(), &[1, 2]);
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

        assert_eq!(group_members(&groups, 2), vec![2, 5]);
        assert_eq!(group_members(&groups, 3), vec![3, 4]);
        assert!(groups.group_for_tab(1).is_none());
        assert_eq!(groups.groups().len(), 2);
    }

    #[test]
    fn join_tabs_can_extend_group_to_preference_limit() {
        let mut groups = TabGroupState::default();
        groups.join_pair(1, 2);

        assert!(groups
            .join_tabs_with_axis(1, 3, 3, PartitionAxis::Vertical)
            .is_some());
        assert_eq!(group_members(&groups, 1), vec![1, 2, 3]);
    }

    #[test]
    fn join_tabs_rejects_group_over_preference_limit() {
        let mut groups = TabGroupState::default();
        groups.join_pair(1, 2);
        groups.join_tabs_with_axis(1, 3, 3, PartitionAxis::Vertical);

        assert!(groups
            .join_tabs_with_axis(1, 4, 3, PartitionAxis::Vertical)
            .is_none());
        assert_eq!(group_members(&groups, 1), vec![1, 2, 3]);
    }

    #[test]
    fn join_tabs_can_merge_groups_when_they_fit() {
        let mut groups = TabGroupState::default();
        groups.join_pair(1, 2);
        groups.join_pair(3, 4);

        assert!(groups
            .join_tabs_with_axis(1, 3, 4, PartitionAxis::Vertical)
            .is_some());

        assert_eq!(groups.groups().len(), 1);
        assert_eq!(group_members(&groups, 1), vec![1, 2, 3, 4]);
    }

    #[test]
    fn separate_pair_removes_whole_group_without_affecting_others() {
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
    fn separate_multi_group_removes_only_selected_tab() {
        let mut groups = TabGroupState::default();
        groups.join_pair(1, 2);
        groups.join_tabs_with_axis(1, 3, 3, PartitionAxis::Vertical);

        assert!(groups.separate_tab(2));

        assert_eq!(group_members(&groups, 1), vec![1, 3]);
        assert!(groups.group_for_tab(2).is_none());
    }

    #[test]
    fn remove_tab_dissolves_pair_for_closed_tab() {
        let mut groups = TabGroupState::default();
        groups.join_pair(1, 2);
        groups.join_pair(3, 4);

        assert!(groups.remove_tab(2));

        assert!(groups.group_for_tab(1).is_none());
        assert!(groups.group_for_tab(2).is_none());
        assert_eq!(group_members(&groups, 3), vec![3, 4]);
        assert_eq!(groups.groups().len(), 1);
    }

    #[test]
    fn retain_tabs_drops_groups_with_too_few_members() {
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

        assert_eq!(group_members(&groups, 1), vec![1, 2]);
        assert_eq!(group_members(&groups, 3), vec![3, 4]);
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
        assert_eq!(
            groups.group_for_tab(1).unwrap().shares(),
            &[TabGroup::MIN_RATIO, 1.0 - TabGroup::MIN_RATIO]
        );

        assert!(groups.set_ratio_for_tab(1, 0.95));
        assert_eq!(groups.group_for_tab(2).unwrap().ratio, TabGroup::MAX_RATIO);
    }

    #[test]
    fn set_split_for_tab_resizes_adjacent_multi_panes() {
        let mut groups = TabGroupState::default();
        groups.join_pair(1, 2);
        groups.join_tabs_with_axis(1, 3, 3, PartitionAxis::Vertical);

        assert!(groups.set_split_for_tab(1, 1, 0.8));

        let group = groups.group_for_tab(1).unwrap();
        assert!((group.shares()[0] - (1.0 / 3.0)).abs() < 0.001);
        assert!((group.shares()[1] - 0.466).abs() < 0.01);
        assert!((group.shares()[2] - 0.2).abs() < 0.01);
    }

    #[test]
    fn set_split_for_tab_clamps_multi_pane_minimums() {
        let mut groups = TabGroupState::default();
        groups.join_pair(1, 2);
        groups.join_tabs_with_axis(1, 3, 3, PartitionAxis::Vertical);

        assert!(groups.set_split_for_tab(1, 0, 0.98));

        let group = groups.group_for_tab(1).unwrap();
        assert!(group.shares()[1] >= TabGroup::MIN_MULTI_SHARE);
        assert!(group.shares()[2] >= TabGroup::MIN_MULTI_SHARE);
        assert!((group.shares().iter().sum::<f32>() - 1.0).abs() < 0.001);
    }

    #[test]
    fn swap_tabs_for_tab_flips_pair_members() {
        let mut groups = TabGroupState::default();
        groups.join_pair(1, 2);

        assert!(groups.swap_tabs_for_tab(1));

        assert_eq!(group_members(&groups, 1), vec![2, 1]);
    }

    #[test]
    fn swap_tabs_for_tab_rejects_multi_group() {
        let mut groups = TabGroupState::default();
        groups.join_pair(1, 2);
        groups.join_tabs_with_axis(1, 3, 3, PartitionAxis::Vertical);

        assert!(!groups.swap_tabs_for_tab(1));
        assert_eq!(group_members(&groups, 1), vec![1, 2, 3]);
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
    fn adding_to_group_preserves_axis() {
        let mut groups = TabGroupState::default();
        groups
            .join_pair_with_axis(1, 2, PartitionAxis::Horizontal)
            .unwrap();
        groups.join_tabs_with_axis(1, 3, 3, PartitionAxis::Vertical);
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
        assert_eq!(group.members(), &[2, 1]);
        assert_eq!(group.active, 2);
        assert!((group.ratio - 0.72).abs() <= f32::EPSILON);
    }

    #[test]
    fn enforce_max_members_truncates_large_groups() {
        let mut groups = TabGroupState::default();
        groups.join_pair(1, 2);
        groups.join_tabs_with_axis(1, 3, 4, PartitionAxis::Vertical);
        groups.join_tabs_with_axis(1, 4, 4, PartitionAxis::Vertical);

        groups.enforce_max_members(3);

        assert_eq!(group_members(&groups, 1), vec![1, 2, 3]);
        assert!(groups.group_for_tab(4).is_none());
    }

    fn group_members(groups: &TabGroupState, tab_id: TabId) -> Vec<TabId> {
        groups.group_for_tab(tab_id).unwrap().members().to_vec()
    }
}
