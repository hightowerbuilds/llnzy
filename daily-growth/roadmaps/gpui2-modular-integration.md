# Roadmap: GPUI Modular Integration

## Current Read

We have moved beyond the original GPUI feasibility spike. GPUI is now proven enough to keep investing in it for Stacker and the code editor, but the work is still prototype-stage rather than production-integrated.

Approximate status:

- Prototype/proof work: 65-75% complete.
- Production migration: 10-20% complete.
- Current app shell replacement: not started.

The practical next milestone is not more abstract feasibility. It is turning the Stacker and editor prototypes into reusable GPUI surfaces that can be mounted by a future app shell without dragging in terminal migration.

## Migration Scope

In scope for this wave:

- Stacker
- Code editor
- Explorer/sidebar after Stacker and editor are credible
- Minimal shared GPUI workspace shell needed to host those surfaces

Out of scope for this wave:

- Terminal rendering
- PTY/session ownership
- Custom terminal `wgpu` effects
- Full event-loop replacement
- Removing the current app before GPUI surfaces are verified

## Branch and Commit Discipline

We already completed the initial branch/spike discipline and merged the useful work back to `main`. From here:

- Keep `main` green after each meaningful migration step.
- Use short-lived topic branches only when a slice is risky or likely to need parallel work.
- Commit each completed capability with a message that names the behavior proven or integrated.
- Push after each coherent milestone.
- Keep prototype binaries available until production mounting makes them redundant.
- Do not delete old UI paths until the GPUI replacement is mounted, verified, and easy to disable.

For parallel work, use separate branches or clearly disjoint file ownership. Good branch names:

```sh
git switch -c gpui/editor-shell-surface
git switch -c gpui/stacker-surface-boundary
git switch -c gpui/explorer-survey
```

## Phase 0: Standalone GPUI Spike

**Status: effectively complete.**

Completed:

- [x] Confirmed GPUI can build on this machine with Command Line Tools using `macos-blade`.
- [x] Added isolated feature-gated GPUI binaries.
- [x] Pinned GPUI through Cargo dependency configuration.
- [x] Opened standalone GPUI windows.
- [x] Proved basic layout, redraw, resize, scrolling, focus, keyboard input, clipboard, mouse text selection, and native text entry.
- [x] Manually confirmed Wispr Flow/dictation-style input works well enough to continue.
- [x] Deferred terminal texture/surface bridging.

Remaining notes:

- [ ] Record exact API constraints discovered during prototype work.
- [ ] Decide later whether deeper IME marked-text behavior needs a focused test.

Decision:

GPUI is viable enough for Stacker and editor prototypes. Terminal migration remains deferred.

## Phase 1: GPUI Stacker Prototype

**Status: substantially prototyped, not production-integrated.**

Completed:

- [x] Built a standalone GPUI Stacker prototype.
- [x] Loaded real saved prompts.
- [x] Implemented prompt selection/loading.
- [x] Implemented GPUI-native text input behavior.
- [x] Implemented selection, clipboard, undo/redo, mouse selection, scrolling, and keyboard movement.
- [x] Implemented multiline prompt rendering/wrapping behavior in the prototype.

Still needed:

- [ ] Extract a production-ready Stacker surface boundary.
- [ ] Decide what state is owned by shared Stacker model code vs GPUI view code.
- [ ] Add queue controls, search, copy affordances, and production Stacker workflow parity.
- [ ] Validate against current Stacker behavior with real workflows.
- [ ] Mount the GPUI Stacker surface in a shared GPUI workspace shell.

Exit criteria:

- Stacker can run as a reusable GPUI surface, not just a standalone prototype.
- Core workflows match or beat the current app.
- Old Stacker UI can remain available behind a clear fallback path.

## Phase 2: GPUI Code Editor Prototype

**Status: active and promising, still prototype-stage.**

Completed:

- [x] Mapped enough of the editor model to reuse `EditorState`, `BufferView`, buffers, cursor state, history, save behavior, and syntax parsing.
- [x] Built a standalone GPUI editor prototype.
- [x] Opened real project files.
- [x] Implemented typing, deletion, enter, tab, shift-tab, arrow movement, home/end, page up/down, selection, mouse placement, drag selection, clipboard, undo/redo, and save.
- [x] Rendered line numbers, caret, selection, dirty status, scroll position, and syntax highlighting.
- [x] Kept the terminal untouched.

Still needed:

- [ ] Convert the prototype into a reusable GPUI editor surface.
- [ ] Replace approximate fixed-width hit testing with measured text layout where GPUI APIs allow it.
- [ ] Add find-in-file UI and commands.
- [ ] Validate file watching and external file changes.
- [ ] Add diagnostics and LSP overlays: diagnostics, completions, hover, rename, code actions, inlay hints, and symbols.
- [ ] Add large-file guardrails around synchronous syntax refresh.
- [ ] Decide how tabs/open buffers are represented in the GPUI shell.
- [ ] Mount the editor beside Stacker in a shared GPUI workspace shell.

Exit criteria:

- Editor can run as a reusable GPUI surface against real buffers.
- Typing, scrolling, selection, save, and syntax rendering are reliable.
- LSP behavior has a clear migration path.
- Existing editor remains available until feature parity is verified.

## Phase 3: Shared GPUI Workspace Shell

**Status: first slice implemented.**

Objective:

Create a minimal GPUI workspace shell that hosts the migrated surfaces without touching terminal ownership.

Target behavior:

- [x] One GPUI window with workspace chrome.
- [x] Stacker and editor regions mounted as separate surfaces.
- [x] No terminal region.
- [x] Simple split layout.
- [x] Shared title/status area.
- [x] Clear feature flag and binary entrypoint: `gpui-workspace`.
- [x] Reuse existing prototype code rather than duplicating logic.

Still needed:

- [ ] Rename prototype types into production-facing surface names.
- [ ] Split editor construction so embedded use does not depend on `env::args()`.
- [ ] Decide focus policy between Stacker and editor.
- [ ] Decide whether Stacker/editor prototype headers should remain inside child surfaces or move into shared workspace chrome.
- [ ] Add a richer workspace switching model if split layout is not enough.

Why this is next:

The prototypes have already proved enough in isolation. A shared shell will reveal the real integration problems: focus routing, shared app state, layout ownership, buffer selection, and how much refactoring is needed before production mounting.

## Phase 4: Explorer/Sidebar Prototype

**Status: planned after Stacker/editor shell integration.**

Objectives:

- [ ] Rebuild Explorer with GPUI list/tree primitives.
- [ ] Preserve file operations, rename flows, modals, drag/drop, context menus, and watcher updates.
- [ ] Validate large repositories, keyboard navigation, selection, sidebar resizing, and file watcher updates.
- [ ] Decide whether file tabs and footer chrome belong to the GPUI shell in this migration wave.

Exit criteria:

- Explorer performs well on large project trees.
- Existing file operations remain intact.
- Sidebar migration does not force terminal migration.

## Phase 5: Production Mounting

**Status: not started.**

For each migrated surface:

- [ ] Define the model/view boundary.
- [ ] Mount the GPUI surface behind a feature flag or alternate entrypoint.
- [ ] Preserve the old surface until the new one is verified.
- [ ] Add focused tests when model behavior changes.
- [ ] Manually verify keyboard, mouse, focus, resize, persistence, and fallback behavior.

Recommended order:

1. Shared GPUI workspace shell
2. Stacker surface boundary
3. Editor surface boundary
4. Editor search/status additions
5. Explorer/sidebar
6. Production app mounting

## Phase 6: Event Loop and Terminal Reassessment

**Status: intentionally deferred.**

Only revisit this after Stacker, editor, and Explorer prove GPUI is worth expanding.

- [ ] Decide whether GPUI should ever own the main event loop.
- [ ] Decide whether terminal bridging deserves a separate spike.
- [ ] Keep existing terminal visuals and behavior unless there is a clear win.
- [ ] Remove obsolete egui paths only for surfaces with verified GPUI replacements.
- [ ] Audit binary size and dependency overlap after production surfaces land.

## Completed One-Hour Slice

Built a minimal shared GPUI workspace shell.

Done:

- [x] New feature-gated binary and entrypoint: `gpui-workspace`.
- [x] Window opens with LLNZY-like chrome.
- [x] Stacker and editor are represented in the same GPUI process.
- [x] Terminal is explicitly absent.
- [x] Existing `gpui-stacker` and `gpui-editor` checks still pass.
- [x] `cargo check` still passes.

Recommended next one-hour slice:

- Rename/refine the Stacker and editor prototype roots into reusable surface APIs.
- Add embedded editor constructors that do not read `env::args()`.
- Move prototype-specific labels toward workspace-owned chrome.
- Keep standalone prototype binaries working.

## Risks

| Risk | Why It Matters | Mitigation |
| :--- | :--- | :--- |
| GPUI API instability | The dependency may change under us. | Keep GPUI isolated behind local modules and feature flags. |
| Prototype code hardens accidentally | Early shortcuts may leak into production. | Promote prototypes into reusable surfaces intentionally, with clear boundaries. |
| Terminal migration creep | Terminal rendering is core and risky. | Keep terminal out of this wave. |
| Input regressions | Text quality is the reason to migrate. | Verify typing, dictation, clipboard, selection, and focus after each step. |
| Editor complexity | LSP, diagnostics, search, and large files hide edge cases. | Move editor in layers and keep current editor available. |
| Scope creep | Full shell rewrites can stall the app. | Keep the next milestone to a minimal GPUI workspace shell. |

## Success Criteria

- Stacker gains reliable native-feeling text input without losing core workflows.
- Code editor gains GPUI-backed text interaction while preserving LLNZY's editor model.
- Explorer/sidebar can move later without forcing terminal migration.
- Terminal remains on its existing path with no regressions.
- The app remains usable throughout the migration.
- GPUI only expands beyond Stacker/editor/Explorer after those surfaces prove it should.
