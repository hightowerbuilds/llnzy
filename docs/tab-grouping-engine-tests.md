# Tab Grouping Engine Tests

This test set covers the stable-ID tab grouping engine at two layers: the core
group model and the workspace layout projection that turns groups into visible
tab-bar entries.

## Test Layers

`src/tab_groups/model.rs` verifies the storage rules for grouped tabs. These
tests stay independent from rendering so group membership, active pane state,
split ratio, close behavior, and reorder behavior can be checked directly.

`src/workspace_layout.rs` verifies how stored groups are projected onto the
current tab list. These tests cover visual order, missing tab members, and
active joined-tab lookup after tabs have moved.

## Behaviors Covered

- Joining two standalone tabs creates one group with the expected primary,
  secondary, active tab, and default ratio.
- A tab cannot be joined to itself.
- Joining a tab that is already grouped removes only the old group containing
  the affected tab and creates the new group.
- Multiple groups can exist at the same time.
- Separating or closing a tab dissolves only that tab's group.
- Retaining the same stable tab IDs after a reorder preserves group
  membership.
- Active tab selection updates only the group that contains the selected tab.
- Split ratios clamp to the engine bounds and stay attached to the group.
- Swapping tabs flips left and right members while preserving active tab and
  split ratio.
- Tab-bar entries render multiple groups in visual order after reorder.
- Layout projection ignores groups with missing members instead of rendering an
  invalid joined entry.
- Active joined-tab lookup finds group members by stable ID instead of stale
  tab index.

## Commands

Run the focused tab grouping tests:

```bash
cargo test tab_groups --lib
cargo test tab_bar_entries --lib
```

Run the full compile check:

```bash
cargo check
```

## Manual Testing Boundary

These tests cover pure model behavior and layout projection. Manual testing is
still useful for pointer-driven UI behavior such as context menus, divider drag,
terminal scrolling inside joined panes, and editor word wrap toggling from the
View menu.
