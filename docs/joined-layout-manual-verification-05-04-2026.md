# Joined Layout Manual Verification - 05-04-2026

## Scope

Future Roadmap item: `Tab Grouping Manual Verification`.

Verify mixed joined tab groups whose panes use different rendering and input
paths:

- Terminal + CodeFile
- Git + Stacker
- CodeFile + Sketch

This record is based on the current stable-ID tab grouping behavior documented
in `docs/tab-grouping-engine-tests.md` and the roadmap requirements in
`daily-growth/roadmaps/future-roadmap.md` and
`daily-growth/roadmaps/old/leftovers-roadmap-3.md`.

## Current Expected Behavior

- Joining two standalone tabs creates one joined group with a 50/50 divider.
- Joined groups are tracked by stable tab identity, not tab index, so close and
  reorder operations should not leave stale group membership.
- Multiple joined groups can exist at the same time.
- The joined tab-bar entry shows both tab titles separated by `/`.
- Clicking either joined tab segment makes that member active.
- Right-clicking the joined tab opens one group-level context menu.
- Joined groups expose close, rename, swap, and separate actions.
- Dragging the vertical divider updates the group-local split ratio and clamps
  it between the documented min and max.
- Separating a joined group returns both tabs to normal tab-bar rendering
  without changing unrelated groups.
- Singleton tabs such as Git, Stacker, and Sketch remain singleton surfaces even
  while joined.

## Setup

- Use a normal project workspace with a visible file tree and at least one
  editable source file.
- Prefer a Git repository with at least one tracked file so the Git tab has
  meaningful repository content.
- Open Terminal, one CodeFile tab, Git, Stacker, and Sketch before starting.
- If possible, keep a second unrelated tab open to confirm unrelated tabs are
  not affected by joins, closes, separates, or reorders.

## General Checklist

Run these checks for each mixed pair listed below.

- [ ] Join the first tab with the second tab from the tab context menu.
  - Pass: one joined tab-bar entry appears, both titles are visible or cleanly
    truncated, and no duplicate singleton tabs are created.
- [ ] Click the left and right joined tab segments.
  - Pass: the clicked pane becomes the active member, active styling moves to
    the clicked segment, and keyboard/pointer input goes to that pane.
- [ ] Drag the divider left, right, and near both extremes.
  - Pass: both panes resize without overlap, the divider remains draggable, and
    each pane remains usable at narrow widths.
- [ ] Right-click the joined tab entry.
  - Pass: one group-level menu opens at the joined tab width with rename,
    close, swap, and separate behavior available as appropriate.
- [ ] Use Swap Tabs from the joined context menu.
  - Pass: left/right pane order flips, active member remains coherent, and pane
    content does not reset.
- [ ] Rename the left and right joined tab names.
  - Pass: each rename targets the requested member only and the joined label
    updates without changing tab identity.
- [ ] Close one member from the joined tab entry.
  - Pass: the joined group dissolves, the remaining tab stays open, and
    unrelated joined groups or standalone tabs are unchanged.
- [ ] Recreate the pair and use Separate Tabs.
  - Pass: both tabs return to standalone tab rendering in predictable order with
    their content and singleton identity preserved.
- [ ] Recreate the pair and drag/reorder the joined tab entry.
  - Pass: the joined group remains intact after reorder and no stale
    primary/secondary index behavior appears.

## Pair-Specific Checks

### Terminal + CodeFile

- [ ] Run a command that produces enough terminal output to scroll.
  - Pass: terminal output stays clipped to the terminal pane.
- [ ] Scroll the terminal while CodeFile is the other joined member.
  - Pass: terminal scrollback routes to the terminal pane and does not scroll or
    edit the CodeFile pane.
- [ ] Click into CodeFile and type a small edit.
  - Pass: the editor switches to the CodeFile buffer before rendering and the
    typed edit lands in that buffer.
- [ ] Toggle editor word wrap from the View menu while the CodeFile pane is
  active.
  - Pass: CodeFile responds normally inside the joined pane.
- [ ] Resize the divider after terminal output and editor edits.
  - Pass: terminal PTY sizing and CodeFile layout update without visual
    artifacts or lost editor state.

### Git + Stacker

- [ ] Join Git with Stacker, then click each side.
  - Pass: Git selection/refresh controls and Stacker text entry each receive
    input only when their pane is active.
- [ ] Refresh the Git tab or select Git content while Stacker is visible.
  - Pass: Git repository content stays inside the Git pane and Stacker state is
    unchanged.
- [ ] Type, paste, undo, and redo in Stacker.
  - Pass: Stacker's editor/WebView remains clipped to its pane, cursor and text
    state stay synchronized, and Git does not receive the input.
- [ ] Queue or save a Stacker prompt.
  - Pass: normal Stacker toolbar and draft state behavior works in the joined
    pane.
- [ ] Close and reopen Git or Stacker after separation.
  - Pass: each singleton reuses its existing singleton tab behavior rather than
    duplicating unexpectedly.

### CodeFile + Sketch

- [ ] Edit the CodeFile pane, then click and draw in Sketch.
  - Pass: text input and drawing input route to the active pane only.
- [ ] Use Sketch select, marker, rectangle, and text tools.
  - Pass: the Sketch canvas rect is correct after joining and after divider
    resize; drawing does not appear outside the Sketch pane.
- [ ] Save or browse sketches while CodeFile remains visible.
  - Pass: Sketch controls behave normally and do not obscure or mutate the
    CodeFile pane.
- [ ] Resize the divider to make Sketch narrow, then draw again.
  - Pass: Sketch remains usable or degrades gracefully without overlapped
    toolbar/canvas controls.
- [ ] Switch back to CodeFile after Sketch interactions.
  - Pass: the editor buffer is still selected correctly and prior edits remain.

## Pass Criteria

The roadmap item can be treated as manually verified when all three mixed pairs
meet these criteria:

- Join, separate, swap, close, rename, click-to-activate, and reorder behavior
  works for each pair.
- Pane-specific rendering stays clipped to each split pane.
- Pane-specific input routes to the active pane and does not leak into the
  other pane.
- Divider resizing preserves usable layout and clamps at the documented bounds.
- Singleton tabs keep singleton identity before, during, and after grouping.
- Closing or reordering one pair does not affect unrelated tabs or groups.
- No crash, panic, obvious flicker loop, stale tab label, stale pane content, or
  lost user edits occurs during the pass.

## Risks To Watch

- Terminal rendering is WGPU-backed while CodeFile, Git, Stacker, and Sketch are
  egui/WebView-driven, so clipping and input routing can fail differently per
  pane type.
- Stacker's WebView-backed textarea must stay bounded inside its joined pane.
- Sketch tracks a canvas pixel rect; divider changes can expose stale canvas
  bounds or pointer-coordinate mapping bugs.
- Git refreshes can return asynchronously, so stale repository results should
  not overwrite current UI state after tab switches or separation.
- Joined group actions must continue to use stable tab identity; stale index
  bugs usually appear after reorder or close.
- Very narrow panes may reveal toolbar overlap, truncation, or unusable controls
  before functional failures appear.

## Follow-Up

- Add visual smoke coverage for Terminal + CodeFile on each supported OS, as
  already called out in the cross-platform roadmap.
- Consider a lightweight automated screenshot or interaction smoke for
  CodeFile + Sketch because canvas bounds regressions are easy to miss in model
  tests.
- Keep Git + Stacker as a manual smoke candidate until Git refresh timing and
  Stacker WebView clipping have automated coverage.
