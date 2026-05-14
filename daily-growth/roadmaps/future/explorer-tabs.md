# Explorer Tabs

Status: research roadmap

Date: 2026-05-14

## Purpose

Today llnzy has exactly one file explorer — the tree rendered in the left
sidebar. Users navigating large repos (or comparing two parts of the same
repo) can only look at one slice of the filesystem at a time. The goal is to
let users open multiple, fully independent file explorers as workspace tabs,
in the same way they already open multiple terminal tabs.

The interaction model is unusual and worth stating up front, because it
drives most of the architecture work:

- The sidebar is the default explorer. As long as no explorer tab exists,
  the sidebar behaves exactly as it does today.
- The footer gains an **Explorer** button. Clicking it does two things in
  one motion: the sidebar explorer "pops out" of the sidebar and becomes
  the first explorer tab, and a brand new, independent second explorer tab
  is opened next to it. The active tab is the second one (the one the user
  just asked for) — same convention the Terminal button already uses.
- While any explorer tab is open, the sidebar no longer shows an explorer.
  It either hides or shows the project-controls header only (open question
  below).
- Each explorer tab carries its own state: workspace root view, expanded
  directories, selection, scroll position, status line.
- Closing explorer tabs one by one shrinks the count. Closing the **last**
  explorer tab returns the explorer to the sidebar — and the state of that
  last-closed tab becomes the sidebar's state, so nothing visually jumps.

The mental model is "the sidebar is one of the explorer tabs, parked." That
framing keeps the data model simple and matches how users will think about
it.

## Current state

The relevant code lives in:

- `src/gpui_workspace.rs` — top-level `WorkspacePrototype` struct and tab
  machinery.
- `src/gpui_workspace/sidebar.rs` — sidebar rendering, including the
  explorer tree.
- `src/gpui_workspace/footer.rs` — footer button strip.
- `src/gpui_workspace/tabs.rs`, `src/gpui_tabs.rs` — tab strip rendering and
  join/separate logic.
- `src/explorer.rs` — the `collect_explorer_entries` filesystem walker and
  recent-projects persistence (already standalone, shared today by the
  sidebar).

A few facts about how the sidebar works today that shape the design:

1. **Explorer state is stored directly on `WorkspacePrototype`**, not in a
   dedicated `Explorer` entity. The fields involved
   (`gpui_workspace.rs:313-321`) are:

       workspace_root: Option<PathBuf>,
       expanded_dirs: BTreeSet<PathBuf>,
       selected_path: Option<PathBuf>,
       recent_projects: Vec<PathBuf>,
       recent_projects_open: bool,
       sidebar_visible: bool,
       sidebar_width: f32,
       last_sidebar_width: f32,
       explorer_status: Option<String>,

   The sidebar render (`sidebar.rs:149-266`) reads these every frame and
   passes them into `WorkspaceSidebarContext` (`sidebar.rs:37-45`). There
   is no `Explorer` struct to clone or hand around — that's the first thing
   to fix.

2. **`WorkspaceSurface`** (`gpui_workspace.rs:120-129`) is the enum of tab
   kinds. Its current variants are `Home`, `Stacker`, `Editor`, `Terminal`,
   `Sketch`, `Appearances`, `Settings`. There is no `Explorer` variant.

3. **Per-tab state is held in `BTreeMap<u64, Entity<...>>`**:
   `terminals: BTreeMap<u64, Entity<TerminalSurface>>` and
   `file_editors: BTreeMap<u64, Entity<EditorPrototype>>`
   (`gpui_workspace.rs:300-301`). This is the established pattern for "one
   independent instance per tab id."

4. **Footer behaviour split**: `open_footer_surface`
   (`gpui_workspace.rs:836-847`) routes Terminal to `open_new_terminal_tab`
   (always create a fresh tab) and everything else to
   `open_or_activate_surface` (reuse existing tab if present). Explorer will
   want a third behaviour — see the Design section.

5. **`close_tab`** (`gpui_workspace.rs:849-908`) already has surface-specific
   cleanup branches (editor file paths, terminal map entries). It's the
   natural home for the "last explorer tab closed → rehydrate sidebar" logic.

6. **Footer items** (`footer.rs:9-63`): Home, Terminal, Stacker, Sketch,
   Appearances, Settings. No Explorer button today.

7. **Initial tabs on launch** are Home, Stacker, Terminal, Sketch
   (`gpui_workspace.rs:330-335`). Explorer is **not** a launch tab and
   shouldn't become one — the sidebar is its default home.

## Target behaviour

A precise specification of what the user should see and feel:

- **Cold start, no explorer tab.** Identical to today. Sidebar shows the
  explorer tree.
- **User clicks "Explorer" in the footer.** The sidebar's explorer
  collapses/empties (becomes the parked-projects header only, or hides
  entirely depending on the open question below). Two new tabs appear in
  the tab strip: the first inherits the sidebar's expanded dirs, selection,
  scroll, and status; the second is a fresh explorer rooted at the same
  `workspace_root` with default state. The second tab is activated.
- **User clicks "Explorer" again with two explorer tabs already open.** A
  third independent explorer tab is created and activated. (Same semantic
  as repeated Terminal clicks.)
- **Each explorer tab is fully independent.** Expanding `src/` in tab 1
  must not expand it in tab 2. Selecting a file in tab 1 does not move the
  selection in tab 2. Scrolling in one does not scroll the other.
- **Opening a file from any explorer tab** behaves the same as opening it
  from the sidebar today: it opens the file in an editor tab (or activates
  an existing one). The explorer tab itself stays put.
- **User closes an explorer tab.** Standard tab close. If other explorer
  tabs remain, nothing else changes.
- **User closes the last explorer tab.** That tab's state (root, expanded
  dirs, selection, scroll) is moved back into the workspace's sidebar
  fields, the sidebar explorer reappears, and the user is returned to
  whatever tab `close_tab` would normally activate.
- **Workspace root changes (Open Project) while explorer tabs exist.** All
  explorer tabs and the parked sidebar state retarget to the new root and
  reset their expanded/selection state, matching today's single-explorer
  behaviour. (We will not try to be clever about preserving paths across
  roots — that's a future enhancement.)

Out of scope for v1, but worth flagging:

- Per-tab roots (different projects in different explorer tabs).
- Drag-and-drop between explorer tabs.
- Persisting the open explorer-tab set across app launches.
- Joining two explorer tabs side-by-side via the existing
  `GpuiTabManager::join_pair` machinery. This **should work for free** once
  the surface plumbing is in place, but we should test it explicitly rather
  than assume.

## Design

### Extract `ExplorerState`

Create a new struct that owns everything currently scattered on
`WorkspacePrototype` for a single explorer:

    struct ExplorerState {
        workspace_root: Option<PathBuf>,
        expanded_dirs: BTreeSet<PathBuf>,
        selected_path: Option<PathBuf>,
        scroll_offset: f32,
        status: Option<String>,
    }

`recent_projects`, `recent_projects_open`, `sidebar_width`,
`last_sidebar_width`, and `sidebar_visible` stay on `WorkspacePrototype` —
they are workspace-level concerns, not per-explorer. `workspace_root` is
duplicated into each `ExplorerState` for v1 to keep the model uniform; in
practice it will always equal `WorkspacePrototype::workspace_root` until
per-tab roots ship.

### Two homes for explorer state

`WorkspacePrototype` gains:

    sidebar_explorer: Option<ExplorerState>,
    explorers: BTreeMap<u64, ExplorerState>,

Invariant: **either `sidebar_explorer` is `Some` and `explorers` is empty,
or `sidebar_explorer` is `None` and `explorers` is non-empty.** The
transition between the two states is what the popout / re-dock flow does.

The existing `expanded_dirs`, `selected_path`, `explorer_status`, and
`workspace_root` fields on `WorkspacePrototype` get removed. Anything that
reads them today reads through a helper:

    fn active_explorer_state(&self) -> &ExplorerState
    fn active_explorer_state_mut(&mut self) -> &mut ExplorerState

which returns the explorer for the active tab if it's an Explorer tab, or
the sidebar's parked state otherwise. The sidebar render reads through the
same helper (when in sidebar mode), so there is one and only one source of
truth for "the explorer the user is currently looking at."

### `WorkspaceSurface::Explorer`

Add the variant. Title: `"Explorer"`. Wire it through:

- `WorkspaceSurface::title` (`gpui_workspace.rs:131-143`)
- `workspace_tab_label` (wherever it lives in `gpui_workspace/tabs.rs`)
- `focus_surface` (so focus can land on an explorer tab)
- `tab_choices`, `tab_overflow_open`, `close_tab` cleanup branch

### Footer button + `open_footer_surface`

Add `footer_button("Explorer", WorkspaceSurface::Explorer, ...)` to
`workspace_footer` (`footer.rs:9-63`). In `open_footer_surface`
(`gpui_workspace.rs:836-847`) add a third branch:

- `WorkspaceSurface::Terminal` → `open_new_terminal_tab` (unchanged)
- `WorkspaceSurface::Explorer` → `open_new_explorer_tab` (new)
- everything else → `open_or_activate_surface` (unchanged)

### `open_new_explorer_tab`

This is the entry point for the popout behaviour. Pseudocode:

    fn open_new_explorer_tab(...):
        let was_sidebar_mode = self.sidebar_explorer.is_some();

        if was_sidebar_mode:
            // First explorer tab: pop the sidebar out as tab #1.
            let parked = self.sidebar_explorer.take().unwrap();
            let parked_id = self.allocate_tab_id();
            self.tabs.push(WorkspaceTab::new(parked_id, Explorer));
            self.explorers.insert(parked_id.0, parked);

        // Always: create a fresh second explorer.
        let fresh_id = self.allocate_tab_id();
        self.tabs.push(WorkspaceTab::new(fresh_id, Explorer));
        self.explorers.insert(
            fresh_id.0,
            ExplorerState::fresh(self.workspace_root.clone()),
        );

        self.active_tab_id = fresh_id;
        self.tab_manager.set_active_tab(fresh_id.0);
        self.focus_surface(Explorer, window, cx);
        cx.notify();

`ExplorerState::fresh(root)` returns a state with the given root, the
default initial expanded dirs (computed by today's `initial_expanded_dirs`
helper), no selection, zero scroll.

Note: the popout creates **two** tabs in one click only on the first
invocation. Subsequent clicks create only one tab (the fresh one). This
matches the spec: "if a user adds a second explorer tab we need the sidebar
explorer to popout."

### Render path

Currently `WorkspacePrototype::render` builds the central pane from the
active surface. Add an `Explorer` arm that renders the same explorer tree
component the sidebar uses today, but reading from
`self.explorers.get(&self.active_tab_id.0)` and routing
expand/select/scroll callbacks back to **that** entry, not to the workspace
fields.

The simplest path is to pull the tree-rendering code out of
`workspace_sidebar` (`sidebar.rs:149-266`) into a function
`render_explorer_tree(state: &ExplorerState, ctx: &mut RenderCtx) -> impl
IntoElement`, and call it from both:

- `workspace_sidebar` (when `sidebar_explorer` is `Some`), and
- the new `Explorer` arm of the central render.

The header chrome (project name, Open Project button, recent projects) only
appears in the sidebar — explorer tabs use the standard tab strip, so they
don't need their own header. They might want a small toolbar (e.g. a refresh
button or a "go to file" jump), but that's a polish item.

### Sidebar behaviour while explorer tabs exist

This is the open question. Two reasonable options:

**Option A: hide the sidebar entirely.** When `sidebar_explorer` is `None`,
collapse the sidebar to width 0 (or hide it). Re-expand it to
`last_sidebar_width` when the last explorer tab closes. This is the cleaner
visual story and what "popout" most naturally implies.

**Option B: keep the sidebar visible but show only the project header.**
Project name, Open Project, recent projects — but no tree. This preserves
muscle memory for project switching while explorer tabs are open.

I'd recommend Option A as v1 with a single line of code to switch to B if
it feels wrong in practice. We already have `sidebar_visible`,
`sidebar_width`, and `last_sidebar_width` — Option A is "set
`sidebar_visible = false` on first popout, restore on last close."

### `close_tab` rehydration

Extend the existing surface-specific cleanup (`gpui_workspace.rs:871-873`):

    if self.tabs[index].surface == WorkspaceSurface::Explorer {
        let state = self.explorers.remove(&tab_id.0);
        if self.explorers.is_empty() {
            // Last explorer tab — re-dock its state into the sidebar.
            self.sidebar_explorer = state;
            self.sidebar_visible = true;  // if Option A
        }
    }

The "next active tab" logic at the bottom of `close_tab` doesn't need to
know about explorers specifically — it already picks a sensible neighbour.

### Independence guarantees

The single source of footguns here is **shared state hiding behind moves
and clones**. Two things to be deliberate about:

1. `expanded_dirs: BTreeSet<PathBuf>` is owned by value inside
   `ExplorerState`. As long as we never `Arc<Mutex<...>>` it, two
   `ExplorerState` instances can't accidentally share expansion. The
   `collect_explorer_entries` walker reads `&BTreeSet<PathBuf>` and produces
   a fresh `Vec<ExplorerEntry>` per call (`sidebar.rs:28-34`), so it's
   already safe for parallel instances.

2. `recent_projects` and `workspace_root` (the workspace-level one) remain
   on `WorkspacePrototype`. When the user switches projects, every
   `ExplorerState` (parked + tabs) gets its `workspace_root` overwritten
   and its `expanded_dirs`/`selected_path` reset. Centralise that in one
   helper — `WorkspacePrototype::set_workspace_root` — so we never forget
   to reset a state.

## Phases

Five phases. Each ends with the app compiling and runnable; the user-visible
feature lights up at the end of phase 4.

### Phase 1 — Extract `ExplorerState` (refactor, no behaviour change)

- Define the struct.
- Move the five fields off `WorkspacePrototype` into a single
  `sidebar_explorer: ExplorerState` field (no `Option`, no map yet).
- Update every read/write site to go through the new field.
- Update `workspace_sidebar` and the helpers it calls (`toggle_explorer_dir`,
  `open_sidebar_file`, etc.) to take `&mut ExplorerState` instead of
  reaching into the workspace.
- Centralise project-root resets in `set_workspace_root`.

This is the largest mechanical change. After it lands, the app behaves
identically; we've just moved fields around.

### Phase 2 — Multi-explorer data model

- Change to `sidebar_explorer: Option<ExplorerState>` and add
  `explorers: BTreeMap<u64, ExplorerState>`.
- Add `WorkspaceSurface::Explorer` and the cosmetic plumbing (title, label,
  focus, tab choices, close cleanup) — but don't expose any way to create
  one yet.
- Add the `active_explorer_state` / `_mut` helpers and switch
  `workspace_sidebar` to read through them when `sidebar_explorer.is_some()`.
- Verify nothing else regresses (still single sidebar explorer, still
  works).

### Phase 3 — Explorer tab rendering

- Pull the tree-rendering body out of `workspace_sidebar` into a shared
  `render_explorer_tree(state, ctx)` function.
- Add an `Explorer` arm to the central pane render that calls it with the
  active tab's state.
- Wire expand/select/scroll callbacks to mutate the right entry in
  `self.explorers`.
- Add a temporary debug action (e.g. behind a hidden menu item) that
  manually inserts an Explorer tab, just so we can exercise the render path
  before the popout flow exists.

### Phase 4 — Popout and re-dock (the user-facing feature)

- Add the Explorer footer button.
- Implement `open_new_explorer_tab` with the popout-on-first-click logic.
- Route `WorkspaceSurface::Explorer` in `open_footer_surface` to it.
- Implement the close-last-tab → re-dock branch in `close_tab`.
- Decide and implement Option A vs B for the sidebar's appearance while
  explorer tabs exist.
- Remove the temporary debug action from Phase 3.

### Phase 5 — Polish and validation

- Test joining two explorer tabs side-by-side via existing tab-join
  machinery; fix anything that assumes joined tabs are heterogeneous.
- Test "Open Project" while explorer tabs are open: confirm all explorer
  states reset cleanly.
- Test cmd-W on the first explorer tab when it's the only one (must
  re-dock) and when it's one of several (must not re-dock).
- Test focus behaviour: clicking inside an explorer tab should focus it,
  and arrow-key tab navigation should land properly.
- Save/restore explorer-tab set across app launches — only if we already
  do this for terminal tabs; otherwise defer.
- Update keybindings? cmd-shift-E to open a new explorer tab is the
  obvious addition, mirroring how cmd-T opens a new terminal tab.

## Open questions

These should be answered before Phase 4 starts.

1. **Sidebar visibility while tabs are open: Option A (hide) or Option B
   (project-header only)?** My recommendation is A but the user's preference
   wins.
2. **Should clicking the Explorer footer button while explorer tabs exist
   create another tab, or focus the most recent existing one?** The Terminal
   button always creates new; the Stacker button activates existing. I'd
   match Terminal (always-new) since that's the closer analog, but it's a
   judgement call.
3. **When the user re-docks (closes the last tab), should the workspace
   focus go to whatever neighbour `close_tab` picks, or should the sidebar
   regain focus?** Today the sidebar isn't a focusable surface in the same
   sense as tabs — re-docking will probably mean "focus jumps to the
   neighbouring tab" by default, which is fine but worth confirming.
4. **Per-tab workspace roots**: explicitly out of scope for v1, but the
   `ExplorerState::workspace_root` field is structured to allow it later.
   Confirm we're okay deferring.
5. **Do explorer tabs need a title that distinguishes them?** "Explorer",
   "Explorer 2", "Explorer 3" — or always "Explorer" and trust the user to
   tell them apart by content? Terminals today are titled "Terminal" with
   no numbering and seem fine; the same default probably works here.
