# Roadmap: Final Stretch

## Current Read

The GPUI migration has crossed out of abstract feasibility and into product
integration. We have enough proof to keep going, but the current GPUI workspace
is not yet the app. It is a credible shell with mounted prototype surfaces.
Some things work, some things are cosmetic, and some major workflows are still
missing.

The important conclusion is not "GPUI is done." The important conclusion is
"GPUI is now real enough that the next work should be vertical integration,
not more isolated experiments."

Approximate status:

- GPUI feasibility: complete.
- GPUI Stacker prototype: strong but not production parity.
- GPUI editor prototype: promising but not production parity.
- GPUI workspace shell: real first-pass tab model with project open/close and
  recent-project launcher flow.
- Terminal in GPUI workspace: live Alacritty-backed surface, with mouse
  reporting, true shader parity, and polish still pending.
- Explorer/sidebar in GPUI workspace: first real project-tree slice is live;
  project close/open/recent handling is live; file operations, watcher updates,
  and keyboard navigation still pending.
- Sketch Pad in GPUI workspace: first real canvas surface is live; marker,
  rectangle, selection/move, grid toggles, save, clear, undo/redo, persisted
  scratch loading, and rendering of existing text/image/symbol records are in
  place; GPUI text editing, imports, symbols, and full appearance controls are
  still pending.
- Appearances in GPUI workspace: first GPUI tab entry point exists for terminal,
  editor, and sketch appearance controls; terminal effects controls now feed the
  GPUI terminal surface.
- Release readiness: optimized binaries build, but the migrated workspace is
  not yet a replacement for the current app.

## What We Have Proven

Completed and worth preserving:

- GPUI builds on this machine without a full Xcode install.
- The GPUI dependency is pinned and isolated behind Cargo features.
- The current app remains available while GPUI work proceeds.
- Standalone GPUI windows open correctly when launched with desktop graphics
  access.
- GPUI text input, selection, clipboard, focus, scrolling, and redraw behavior
  are good enough to keep building on.
- Wispr Flow works well in the GPUI text-entry path.
- A standalone GPUI Stacker prototype can load real saved prompts and support
  real text editing behavior.
- A standalone GPUI editor prototype can open real project files, edit text,
  save, show line numbers, render selections, and display syntax highlighting.
- A shared `gpui-workspace` binary exists and mounts Stacker plus editor in one
  GPUI process.
- The GPUI workspace shell now has clickable top tabs, sidebar rows, footer
  controls, active-surface switching, and release builds.
- The GPUI workspace now restores a native desktop menu bar with LLNZY, File,
  Edit, Tab, and View menus.
- The GPUI workspace can close the active repo from the sidebar, leaving Open
  Project and Open Recent controls visible.
- Home now shows the active project plus the top five recent projects.
- The GPUI terminal has first-pass visible effects for smoke/aurora background
  layers, bloom-style cursor glow, cursor trail, particles, CRT overlays, and
  text shimmer.
- The old Sketch model, persistence, appearance settings, hit testing, and tool
  state survived the migration and are now mounted through a first-pass GPUI
  Sketch Pad surface.
- The main app and GPUI binaries build in optimized release mode:
  `llnzy`, `gpui-workspace`, `gpui-editor`, and `gpui-stacker`.

This is a good base. It is also still a base.

## What The Desktop Test Taught Us

The live inspection clarified the gap:

- The visual shell is roughly shaped like the LLNZY workbench again.
- Some chrome interactions work now, but the shell still feels partial.
- The terminal tab is now a real shell surface.
- The sidebar looks like a file explorer but is not yet backed by the real
  project tree, file operations, or watcher updates.
- The workspace now has real first-pass tab identity, open/activate semantics,
  close buttons, and footer navigation.
- The footer has an Appearances entry point, but the editor and sketch
  appearance controls still need deeper surface integration.
- Project open/close now belongs to the shell instead of being implicit in the
  process cwd.
- Terminal effects can be toggled from Appearances, but these are GPUI paint
  layers, not yet the full old `wgpu` shader pipeline.
- Sketch was missing from the GPUI shell and has been restored as a real
  workspace tab/footer surface, but text editing, image import, symbols, and
  appearance-control parity still need follow-up.
- Stacker and editor are embedded, but still carry prototype assumptions.
- We need to restore production-grade styling and interaction polish, not just
  compile a new framework.

The next phase should therefore be strict: every slice should turn one visible
placeholder into real behavior.

## Product Target

The final migrated workbench should open as a coherent desktop app with:

- Real terminal tab.
- Real code editor tab and buffers.
- Real Stacker tab.
- Real file explorer/sidebar.
- Real Sketch Pad tab.
- Real workspace/tab model.
- Restored LLNZY visual quality.
- Release builds and repeatable smoke checks.
- A fallback path until the GPUI shell reaches daily-driver quality.

The terminal cannot stay as "not set up" if the GPUI workspace is meant to
become the real workbench. It can start simple, but it needs to be real.

## Commit And Branch Discipline

Use a short branch for each risky vertical slice, then merge back to `main`
only after that slice builds and has been manually opened.

Recommended pattern:

```sh
git switch -c final-stretch/terminal-surface
cargo check --features gpui-workspace --bin gpui-workspace
cargo build --release --features gpui-workspace --bin gpui-workspace
git add <changed files>
git commit -m "Add GPUI terminal surface foundation"
git push -u origin final-stretch/terminal-surface
```

Merge or fast-forward to `main` only when:

- The branch has one coherent behavioral improvement.
- `cargo check` passes for the affected target.
- The relevant release build passes.
- The desktop app has been opened at least once.
- Any placeholder that remains is named honestly in the UI or roadmap.

For low-risk documentation or tiny shell polish, committing directly to `main`
is acceptable. For terminal, explorer, editor model ownership, or event-loop
changes, use a branch.

## Phase 1: Make The Workspace Shell Honest

Goal: make the shell behave like a real host instead of a visual mock.

Tasks:

- [x] Keep `gpui-workspace` feature-gated.
- [x] Keep standalone `gpui-editor` and `gpui-stacker` binaries available.
- [x] Add clickable surface switching for top tabs, sidebar rows, and footer.
- [x] Restore native desktop menu bar entries for File, Edit, Tab, and View.
- [x] Show the terminal as an explicit placeholder instead of pretending it is
  complete.
- [ ] Rename prototype types into production-facing surface names.
- [ ] Move prototype-only labels and demo copy out of child surfaces.
- [ ] Define a single workspace state object for active surface, selected file,
  selected prompt, selected terminal, and footer status.
- [ ] Make focus policy deterministic when switching between Stacker, editor,
  terminal, and explorer.
- [ ] Restore the workbench visual standard: sidebar density, tab sizing,
  active states, footer styling, spacing, typography, and empty states.

Exit criteria:

- Clicking visible chrome always does something understandable.
- The active surface is reflected consistently in tabs, sidebar, footer, and
  content.
- The shell has no fake controls that look production-ready but do nothing.

## Phase 2: Bring Up A Real GPUI Terminal Surface

Status: live terminal surface implemented in the GPUI workspace; visual parity
pass started, with manual verification, terminal mouse reporting, and full
shader-pipeline parity still pending.

Goal: replace the terminal placeholder with a minimal real terminal.

This should be a deliberately small first version. Do not try to port every
terminal effect on day one.

Tasks:

- [x] Identify the smallest reusable terminal backend boundary from the current
  app: PTY/session ownership, terminal grid, input route, resize, scrollback,
  kill/restart, and cwd tracking.
- [x] Create a `TerminalSurface` module for GPUI.
- [x] Spawn one real terminal session inside `gpui-workspace`.
- [x] Render plain terminal grid text in GPUI first.
- [x] Route keyboard text, Enter, Backspace, arrows, paste, and common modifier
  keys into the PTY.
- [x] Handle terminal resize from GPUI layout dimensions.
- [x] Add basic scrollback and selection.
- [x] Add copy/paste behavior.
- [x] Show process exit and restart controls.
- [x] Add color/style spans, cell backgrounds, and cursor styling parity.
- [x] Add first-pass GPUI terminal effects toggles and paint-layer rendering:
  smoke, aurora, bloom/glow, particles, CRT overlay, cursor trail, and shimmer.
- [ ] Preserve the existing terminal implementation until GPUI terminal
  behavior has been manually verified.
- [ ] Manually verify the first GPUI terminal slice in the running release
  workspace.
- [ ] Add terminal mouse reporting for full-screen terminal apps.
- [ ] Add URL/file opening and copy-on-select parity.
- [ ] Bridge or replace the old `wgpu` shader effects pipeline for true shader
  parity inside the GPUI terminal.
- [ ] Restore pixel-level shader control for smoke/aurora/background effects by
  bridging the existing renderer path back under GPUI instead of expanding the
  current rectangle-based paint approximation.

Deferred until after the terminal is alive:

- Background images.
- Advanced shader effects beyond the current GPUI paint-layer approximation,
  unless they are part of the dedicated shader bridge slice above.
- Terminal theme import/export polish.
- Pixel-perfect parity with the current `wgpu` renderer.

Exit criteria:

- Opening Terminal in `gpui-workspace` gives a real shell prompt.
- Typing, pasting, command execution, resize, scrollback, and restart work.
- A failed terminal spawn produces a visible, useful error.

## Phase 3: Turn The Sidebar Into A Real Explorer

Goal: replace the static sidebar with the actual project tree workflow.

Tasks:

- [x] Reuse the current workspace root as the first project boundary.
- [x] Load the real current project tree.
- [x] Render folders and files with expandable state.
- [x] Open files into the GPUI editor surface.
- [x] Preserve selected file and expanded folders during the running session.
- [x] Add an explicit project close button that clears the project without
  hiding Open Project and Open Recent.
- [x] Add sidebar Open Project and Open Recent controls.
- [x] Track recent projects and cap visible recent entries to the top five.
- [ ] Wire file watcher updates into the tree.
- [ ] Add rename, delete, new file, new folder, and reveal-in-finder actions.
- [ ] Add keyboard navigation.
- [x] Add sidebar collapse/open via the bumper.
- [x] Add sidebar drag resize and sensible width bounds.
- [x] Keep large repositories responsive with skipped heavy directories and an
  entry cap.

Exit criteria:

- The sidebar is no longer decorative.
- Opening files from the explorer becomes the normal path into the GPUI editor.
- File changes on disk are reflected without restarting the workspace.

## Phase 4: Bring The Editor To Workbench Parity

Goal: make the GPUI editor usable as a real daily coding surface.

Detailed editor roadmap:
`daily-growth/roadmaps/gpui-editor-professional-grade.md`

Already working in prototype form:

- Real file open.
- Typing and deletion.
- Selection and mouse placement.
- Clipboard.
- Undo/redo.
- Save.
- Line numbers.
- Syntax highlighting.

Still needed:

- [x] Add a single GPUI editor command dispatcher.
- [x] Expand keyboard traversal to command, option, shift-command, and
  shift-option movement.
- [x] Keep cursor, selection, scroll reveal, line, and column state exact after
  every editor command in the current fixed-width renderer.
- [ ] Connect editor tabs and open-buffer state to workspace state.
- [ ] Add find-in-file.
- [ ] Add project search handoff or a clear placeholder.
- [ ] Add dirty-buffer close protection.
- [ ] Validate external file changes and reload prompts.
- [ ] Replace approximate hit testing with measured layout where needed.
- [ ] Restore diagnostics, hover, completion, rename, references, code actions,
  symbols, and formatting.
- [ ] Add large-file guardrails around syntax refresh and layout.
- [ ] Verify keyboard shortcuts do not leak between editor, Stacker, and
  terminal.

Exit criteria:

- The GPUI editor can handle normal edit/save/reopen workflows without falling
  back to the old UI.
- LSP behavior has at least a credible first slice, even if not full parity.
- Dirty files are protected.

## Phase 4.5: Restore The Sketch Pad

Status: first GPUI Sketch Pad surface is restored.

Goal: keep Sketch as a real workbench surface, not an old-app-only feature.

Already working in first GPUI form:

- Scratch sketch loading from the preserved sketch document path.
- Marker drawing with GPUI path rendering.
- Rectangle drawing.
- Select, move, delete, undo, redo, clear, and save.
- Grid cycling between hidden, lines, and dots.
- Rendering of existing strokes, rectangles, text records, imported-image
  placeholders, symbols, selection outlines, and resize handles.
- Persisted sketch scratch save behavior.
- Sketch Pad tab and footer entry in the GPUI workspace.

Still needed:

- [ ] Add GPUI-native text editing for sketch text boxes.
- [ ] Restore image import and real image rendering.
- [ ] Restore symbol palette insertion.
- [ ] Expose full sketch appearance controls in the Appearances tab: canvas
  color, grid spacing, grid opacity, selection outline, handle size, toolbar
  position, and canvas border/shadow.
- [ ] Add named sketch browser/save-as/delete flows.
- [ ] Add export behavior from the GPUI surface.
- [ ] Verify mouse hit testing and resize handles across dense sketches.

Exit criteria:

- Sketch is usable for quick diagrams without falling back to the old UI.
- Existing saved sketches render faithfully enough for daily use.
- Text, image, symbol, and export workflows are no longer placeholders.

## Phase 5: Bring Stacker To Workbench Parity

Goal: preserve the wins from the Stacker work while moving it into the GPUI
workspace cleanly.

Already working in prototype form:

- Saved prompt loading.
- Native-feeling text input.
- Selection, clipboard, undo/redo, scrolling, and multiline rendering.
- Wispr Flow path validated.

Still needed:

- [ ] Define the production Stacker surface boundary.
- [ ] Use the real Stacker session/state model instead of prototype-only state.
- [ ] Restore saved prompt management.
- [ ] Restore queue controls.
- [ ] Restore search/filter behavior if still part of the product.
- [ ] Restore formatting controls that still matter.
- [ ] Add copy/send/open-in-editor affordances.
- [ ] Verify Stacker alongside terminal and editor focus routing.

Exit criteria:

- Stacker is not just text input. It supports the actual prompt workflow.
- Wispr Flow remains reliable after embedding in the full workspace.
- Stacker commands do not interfere with editor or terminal commands.

## Phase 6: Unify The Workspace Model

Goal: stop treating the GPUI workspace as four disconnected demos.

Tasks:

- [x] Define first-pass singleton workspace tabs: Terminal, Editor, Stacker,
  Sketch, Home, Settings, and Appearances, with Explorer owned by the sidebar.
- [x] Decide what opens as a singleton tab versus a multi-instance tab for the
  first GPUI shell pass.
- [x] Rebuild the top tab bar around real tab identity.
- [x] Keep the footer as command/navigation chrome, not fake state.
- [x] Add an Appearances footer command that opens/focuses an Appearances tab.
- [x] Add Home project launcher content: active project plus top-five recent
  projects.
- [ ] Restore session save/restore for open tabs and selected project.
- [ ] Upgrade editor file tabs from singleton surface tabs to real open-buffer
  tabs.
- [ ] Define split/join behavior after single-surface tabs are stable.
- [ ] Route app commands through a shared command dispatcher instead of
  per-surface ad hoc handlers.

Exit criteria:

- The workspace can be closed and reopened without losing obvious state.
- Tabs correspond to real work, not just demo surface switches.
- Keyboard and mouse focus are predictable across all surfaces.

## Phase 7: Release Hardening

Goal: make the migrated workbench buildable, launchable, and testable as a
release target.

Tasks:

- [x] Build optimized current app binary.
- [x] Build optimized `gpui-workspace`.
- [x] Build optimized `gpui-editor`.
- [x] Build optimized `gpui-stacker`.
- [ ] Add a repeatable release smoke checklist.
- [ ] Add CI or local script coverage for the GPUI feature builds.
- [ ] Package the GPUI workspace path as an app bundle when it becomes useful
  enough for repeated manual testing.
- [ ] Verify bundled assets, fonts, themes, and background images.
- [ ] Add crash/error reporting hooks appropriate for GPUI launch failures.
- [ ] Track binary size and dependency overlap between current and GPUI paths.

Smoke checklist:

- [ ] Launch optimized build from desktop context.
- [ ] Open terminal and run a command.
- [ ] Open project sidebar.
- [ ] Open and edit a source file.
- [ ] Save and reopen file.
- [ ] Use Stacker text input with normal typing.
- [ ] Use Stacker with Wispr Flow.
- [ ] Switch between terminal, editor, Stacker, and explorer with mouse and
  keyboard.
- [ ] Resize the window.
- [ ] Close and reopen the app.

## Phase 8: Cutover Decision

Do not delete the current app path just because the GPUI shell builds.

The GPUI workspace can become the default only when:

- Terminal is real.
- Explorer is real.
- Editor is safe for normal code work.
- Stacker preserves the external-input win.
- Session restore works.
- Release build launches reliably from the desktop.
- Manual smoke testing passes without major caveats.

Until then, the current app remains the stable path and GPUI remains the
migration path.

## Recommended Next Slice

The terminal is alive, the first real workspace tab shell is in place, the
footer has an Appearances tab entry point, and Sketch is back as a real GPUI
surface. Keep the old app path preserved until GPUI surfaces have been manually
verified, but move the main migration pressure to the workbench surfaces around
the shell.

Next objective:

- Restore pixel-level terminal shader control by bridging the existing `wgpu`
  shader pipeline back under the GPUI terminal, instead of treating paint-layer
  effects as final.
- Reorganize Stacker into a production workflow surface instead of a prompt
  editor prototype.
- Finish explorer production actions: create, rename, delete, reveal, watcher
  updates, and keyboard navigation.
- Wire editor appearance settings into the GPUI editor renderer instead of only
  storing the config.
- Move into editor workspace tabs and dirty-file safety.
- Preserve terminal mouse reporting as a focused terminal follow-up.
- Finish Sketch parity work: text editing, image import, symbols, named
  sketches, export, and full appearance controls.

Order:

1. Terminal shader bridge for real pixel-level smoke/background control.
2. Stacker production workflow parity.
3. Explorer production actions and watcher updates.
4. Editor workspace tabs and dirty-file safety.
5. Wire editor and sketch appearance controls into their GPUI renderers.
6. Terminal mouse reporting for full-screen terminal apps.
7. Sketch text/image/symbol/export parity.
8. Styling restoration pass.
9. Release packaging and smoke automation.

## Guiding Rule

Every next commit should remove one lie from the interface.

If a control looks clickable, make it work or make it visibly unavailable. If a
surface has a tab, make it real or label it honestly. If a feature is still on
the old app path, preserve that path until the GPUI version earns the right to
replace it.
