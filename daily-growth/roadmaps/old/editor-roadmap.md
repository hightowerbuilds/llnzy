# Code Editor Stabilization Roadmap

Date: 2026-05-16

## Purpose

Stabilize the LLNZY code editor without changing the behavior of Terminal, Stacker, Sketch, project browsing, or other working app surfaces. The work should stay inside the editor boundary and the workspace routing needed to host editor tabs correctly.

## Guardrails

- Preserve existing non-editor behavior.
- Keep changes staged and reviewable.
- Prefer fixing editor state routing over broad UI rewrites.
- Treat the pure editor model as the stable core unless a specific model bug is proven.
- Add regression coverage for any state, scroll, or appearance behavior we change.

## Current Architecture

The editor has three main layers:

- `src/editor/*`: pure editor model, buffers, views, cursor state, syntax, search, git gutter, and per-buffer scroll state.
- `src/gpui_editor*`: GPUI adapter that renders editor state, handles input, LSP, Markdown preview, image previews, status bar, and editor appearance.
- `src/gpui_workspace*`: workspace shell that owns tabs and decides which editor entity is displayed.

The pure model is mostly coherent: `EditorState` owns buffers, and `BufferView` owns cursor, scroll offsets, syntax state, and Markdown mode. The highest-risk area is the workspace/editor host boundary, where the app currently mixes a singleton editor entity with per-file editor entities.

## Findings To Address

1. Source-file wheel scrolling appears directionally inconsistent.
   - `src/gpui_editor/input.rs` applies `ScrollWheelEvent` vertical delta directly.
   - Other custom scrollers in the app normalize the same event differently.
   - Markdown Preview uses native GPUI scrolling, so it can behave correctly while source files feel stuck.

2. Workspace editor hosting is split across two models.
   - `WorkspacePrototype` owns one singleton `editor`.
   - File tabs also create separate `file_editors`.
   - Generic Editor tabs and file-backed Editor tabs can therefore render different editor state, footer data, load state, and LSP state.

3. File-tab editors open hidden fallback buffers.
   - `EditorPrototype::new` calls the same constructor path that auto-opens `initial_path`.
   - A file tab then opens the selected file on top of that hidden initial buffer.
   - This can leak extra status, LSP, buffer, and navigation state into what should be a single-file editor tab.

4. Markdown behavior is editor-like in source mode but document-like in preview mode.
   - Markdown files default to Preview.
   - Preview scroll state is separate from source scroll state.
   - The status bar still reports source-buffer cursor and line-window data, which can look stale or incorrect in Preview.

5. Editor appearance is only partially wired.
   - Code source rendering flows through `EditorAppearance`.
   - Markdown preview still has hard-coded layout and typography.
   - The Markdown appearance page currently exposes readouts and placeholder copy rather than real controls.

## Roadmap

## Resume Status

Status updated after resuming the interrupted session on 2026-05-16.

- Phase 0 is partially satisfied by the investigation in this roadmap. A true
  pre-change manual baseline was not captured before implementation began, so
  the remaining value is a final manual smoke pass against the current build.
- Phase 1 is implemented. Source editor wheel deltas are normalized in
  `src/gpui_editor/input.rs`, with fractional scroll accumulation and direction
  tests.
- Phase 2 is implemented. Workspace file tabs now use
  `EditorPrototype::file_tab`, which skips fallback initial-file loading. A
  regression test covers the startup path that keeps file-tab editor state
  empty before the requested file is opened.
- Phase 3 is implemented for the current per-file editor model. Menu actions
  route through the visible editor entity, and cross-editor operations route
  through the shared editor-entity iterator for appearance, delete cleanup, and
  move remaps.
- Phase 4 is implemented with Option A. Markdown files open in Source mode by
  default, while Preview and Split remain available from controls and menu
  actions.
- Phase 5 is implemented for the current scope. Source syntax highlighting
  consumes configured syntax colors, editor syntax presets are exposed in
  Appearances, the selected syntax theme persists in workspace preferences, and
  Markdown preview now uses editor font size, line height, and theme colors.
- Phase 6 has focused automated coverage for scroll deltas, file-tab startup,
  Markdown default mode, syntax presets, syntax-color propagation, and Markdown
  preview typography. Manual smoke on the rebuilt `target/llnzy.app` confirmed
  source scrolling, Markdown modes, file-tab routing, live appearance updates,
  and basic edit/save behavior.

### Manual Smoke Status

- [x] Open a long `.rs` file and verify wheel direction, page keys, cursor
  reveal, footer line window, save, and LSP status.
- [x] Open a long `.json` or plain text file and repeat the source-scrolling
  checks.
- [x] Open a `.md` file and confirm it starts in Source, then switch to Preview
  and Split without losing source cursor/scroll state.
- [x] Open an image preview and confirm the footer reports read-only image
  details.
- [x] Open multiple file tabs and verify visible-editor routing for normal
  editor actions.
- [x] Join editor panes with mixed file types and verify each joined file
  scrolls correctly.
- [x] Change editor font size, Markdown preview line height, and syntax theme
  while multiple file tabs are open; confirm every visible editor updates.
- [ ] Confirm Terminal, Stacker, Sketch, and project browsing behavior is
  unchanged after the editor fixes.

### Phase 0: Baseline The Current Behavior

Capture a short manual smoke matrix before changing code:

- Open a long `.rs` file and verify wheel direction, page keys, cursor reveal, footer line window, and LSP status.
- Open a long `.json` or plain text file and repeat the same checks.
- Open a `.md` file in Preview, Source, and Split.
- Open an image preview and confirm read-only footer behavior.
- Switch editor syntax theme and editor font size while multiple file tabs are open.

Output: a short before/after checklist in `docs/manual-smoke-tests.md` or a linked section in this roadmap.

### Phase 1: Fix Source Editor Scrolling

Scope:

- Normalize source-editor wheel delta so natural down-scroll moves further into the file.
- Keep source scrolling custom and line-based for now.
- Do not modify terminal or Stacker scrolling.

Implementation targets:

- `src/gpui_editor/input.rs`
- `src/gpui_editor/render.rs` only if the event capture area needs to be made more explicit.

Acceptance criteria:

- Long source files scroll with the mouse wheel.
- Scroll direction matches the rest of the app.
- Page Up, Page Down, cursor movement, and drag selection still work.
- Markdown Preview still uses native preview scrolling.

### Phase 2: Clean Up Editor Entity Construction

Scope:

- Add an embedded/file-tab constructor that does not auto-open `initial_path`.
- Use that constructor for file-backed workspace tabs.
- Leave the standalone editor constructor available for direct editor launches.

Implementation targets:

- `src/gpui_editor.rs`
- `src/gpui_workspace/project.rs`

Acceptance criteria:

- Opening a file tab creates an editor entity with only the requested file or image preview.
- No hidden `src/main.rs` or `Cargo.toml` buffer appears inside file tabs.
- Standalone editor startup still opens the same fallback file it does today.

### Phase 3: Make Workspace Editor Routing Explicit

Scope:

- Keep the current per-file editor model initially, but centralize all workspace-to-editor access through helper methods.
- Add a helper for iterating all editor entities when applying appearance, delete/move remaps, or cleanup.
- Audit menu actions and focus routing so they always target the active file editor when a file tab is active.

Implementation targets:

- `src/gpui_workspace.rs`
- `src/gpui_workspace/menu_actions.rs`
- `src/gpui_workspace/appearance_actions.rs`
- `src/gpui_workspace/project.rs`
- `src/gpui_workspace/panes.rs`

Acceptance criteria:

- Save, undo, redo, copy, paste, find, Markdown mode actions, and LSP actions always affect the visible editor.
- Appearance changes apply consistently to every open file editor.
- Closing, moving, deleting, and remapping project files cannot leave stale editor entities.

### Phase 4: Normalize Markdown Editor Behavior

Decision to make before implementation:

- Option A: Markdown opens in Source by default because this is the code editor.
- Option B: Markdown opens in Preview by default, but the footer and mode state become preview-aware.

Preferred starting point: Option A. It is the least surprising behavior for a code editor and keeps source, cursor, footer, and scroll state aligned.

Implementation targets:

- `src/gpui_editor/files.rs`
- `src/gpui_editor/render.rs`
- `src/gpui_editor.rs`

Acceptance criteria:

- Markdown Source behaves like other source files.
- Markdown Preview and Split remain available through existing controls and menu actions.
- Footer text accurately reflects the visible mode.
- Switching modes does not corrupt source scroll position or cursor state.

### Phase 5: Wire Editor Appearance Completely

Scope:

- Make source and Markdown preview consume editor appearance through a single config path.
- Replace hard-coded Markdown preview typography with config-backed values.
- Turn the Markdown appearance page from readouts/placeholders into functional controls only after the rendering path supports them.

Implementation targets:

- `src/config/model.rs`
- `src/config/apply.rs`
- `src/config/schema.rs`
- `src/gpui_editor.rs`
- `src/gpui_editor/render.rs`
- `src/gpui_workspace/appearances.rs`
- `src/gpui_workspace/appearance_actions.rs`

Acceptance criteria:

- Editor font size changes affect source editor predictably.
- Markdown preview uses editor theme colors and configured typography.
- Open file tabs update consistently when appearance changes.
- Terminal appearance behavior is unchanged.

### Phase 6: Regression Coverage

Add focused tests around the corrected behavior:

- Scroll delta maps to expected `scroll_line` changes.
- File-tab editor construction starts empty and opens only the requested path.
- Markdown open mode matches the chosen policy.
- Appearance updates propagate to all open editor entities through one workspace helper.

Manual smoke should cover:

- [x] `.rs`, `.json`, `.md`.
- [x] Image preview.
- [x] Multiple file tabs.
- [x] Joined panes if editor tabs can participate.
- [x] Theme/font changes while files are open.
- [x] Save and close flows for modified and unmodified files.

## Completion Criteria

The editor stabilization is complete when:

- All source file types scroll consistently.
- Every visible editor has the same footer structure and accurate status data.
- File tabs do not carry hidden fallback buffers.
- Markdown modes are intentional and predictable.
- Appearance changes are fully wired for the editor and do not affect unrelated app surfaces.
- Manual smoke checks pass without regressions in Terminal, Stacker, Sketch, or project browsing.
