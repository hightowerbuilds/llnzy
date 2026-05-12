# Roadmap: GPUI Integration Spike and Incremental Migration

## Purpose

Evaluate GPUI as a replacement for LLNZY's productivity UI layer without risking the current app. The near-term migration scope is intentionally limited:

- **In scope first:** Stacker and the code editor.
- **In scope later:** Explorer/sidebar and related file-navigation chrome.
- **Out of scope for this migration wave:** terminal rendering, PTY/session ownership, and custom `wgpu` terminal effects.

The first goal is not migration. The first goal is proof.

## Guiding Principles

- Do not replace the `winit` event loop until GPUI proves it can host LLNZY's hardest requirements.
- Keep the current app runnable throughout the migration.
- Move one surface at a time, starting with the smallest surface that benefits most from native text behavior.
- Leave the terminal on the existing `winit`/`wgpu` path for now.
- Treat GPUI as an unstable dependency until pinned, tested, and isolated behind local boundaries.

## Branch and Commit Discipline

**Objective:** Keep the spike isolated, understandable, and easy to roll back.

- Create a dedicated branch before starting the spike, for example `spike/gpui-foundation`.
- Keep production app changes separate from throwaway spike code.
- Prefer small commits that each prove one thing:
  1. Add roadmap and spike notes.
  2. Add isolated GPUI spike crate or example.
  3. Prove basic window/layout.
  4. Prove text input and native macOS behavior.
  5. Prove custom drawing or dynamic surface behavior.
  6. Record findings and next decision.
- Push the branch early, then push after each meaningful milestone.
- Use commit messages that state the proof or decision, not just the files changed.
- Do not merge the spike branch into the main development branch until Phase 0 exit criteria are met.
- If the spike fails, keep the branch as research history and write down why it failed.

Suggested commands:

```sh
git switch -c spike/gpui-foundation
git add daily-growth/roadmaps/gpui2-modular-integration.md
git commit -m "Document GPUI spike-first migration plan"
git push -u origin spike/gpui-foundation
```

## Phase 0: Standalone GPUI Spike

**Objective:** Prove GPUI can support the core interaction and rendering requirements outside the app.

- [x] Confirm macOS build prerequisites. Prefer GPUI's `macos-blade` feature for the spike so Command Line Tools can build it without full Xcode's `metal` compiler.
- [x] Create a separate spike crate or isolated example directory outside the production runtime.
- [x] Pin GPUI to a specific known-good commit.
- [x] Open a basic GPUI window with app-like layout regions: sidebar, tab bar, main pane, footer/status area.
- [ ] Prove keyboard input, text input, focus movement, clipboard, mouse selection, scrolling, resize, and redraw behavior.
- [ ] Test macOS-native text features: IME composition, dictation/Wispr-style input, selection gestures, and command-key editing behavior.
- [x] Create a custom drawing area that updates at interactive frame rates.
- [ ] Document whether terminal texture/surface bridging should remain deferred.
- [ ] Document concrete API constraints, unstable areas, and missing primitives.

**Exit Criteria**

- We know whether native text input is materially better than the current Stacker path.
- We have a small reproducible example, not just notes.
- We can name the exact GPUI commit and dependency setup to use.

## Phase 1: Stacker Prototype

**Objective:** Build the first real LLNZY surface in GPUI using actual Stacker data and behavior.

- [ ] Extract or adapt Stacker state so it can be used by both the current app and the GPUI prototype.
- [ ] Build a GPUI Stacker window/surface with prompt editing, saved prompts, queue controls, search, and copy behavior.
- [ ] Replace custom text-input handling in the prototype with GPUI-native text components where possible.
- [ ] Implement multiline prompt layout; the first GPUI prototype renders newlines as spaces because `shape_line` is single-line only.
- [ ] Validate dictation, IME, multiline editing, selection, undo/redo, and clipboard workflows.
- [ ] Compare typing latency and interaction behavior against the current egui/AppKit bridge implementation.

**Exit Criteria**

- GPUI Stacker is clearly better for text input and does not regress core Stacker workflows.
- The shared Stacker model boundary is clean enough to keep.
- We can decide whether Stacker should be the first production migration target.

## Phase 2: Code Editor Prototype

**Objective:** Prove the editor can use GPUI for text layout/input while preserving LLNZY's editor model and language features.

- [ ] Map the current editor modules and rendering path before implementation.
- [ ] Keep LLNZY's buffer model, history, cursor logic, tree-sitter parsing, project search, recovery, git gutter, and LSP manager unless there is a specific reason to replace them.
- [ ] Prototype GPUI-backed text layout and editing with real `ropey` buffers.
- [ ] Validate typing, selection, scrolling, find-in-file, save, dirty tracking, and file watching.
- [ ] Prototype diagnostics, completions, hover, rename, code actions, inlay hints, and symbols as GPUI overlays/popovers.
- [ ] Validate large-file behavior, many diagnostics, multi-cursor editing, Vim/Emacs/VS Code key presets, and file watching.

**Exit Criteria**

- Editor typing and scrolling are faster or more correct than the current implementation.
- LSP behavior remains intact.
- No major editor workflow is lost.
- The terminal remains unaffected.

## Phase 3: Explorer/Sidebar Prototype

**Objective:** Move file navigation to GPUI after Stacker and editor are credible.

- [ ] Rebuild Explorer with GPUI list/tree primitives and test large projects.
- [ ] Preserve existing file operations, modals, rename flows, drag/drop behavior, and context menus.
- [ ] Validate large repositories, keyboard navigation, selection, sidebar resizing, and file watcher updates.
- [ ] Decide whether tab bar/footer chrome should also move to GPUI or remain in the existing app shell during this wave.

**Exit Criteria**

- Explorer performance is measurably better or simpler to maintain.
- Existing file operations remain intact.
- Sidebar migration does not force terminal migration.

## Phase 4: Production Migration, Surface by Surface

**Objective:** Start replacing egui surfaces only after the spike work proves the foundation.

Recommended order:

1. Stacker
2. Code editor
3. Explorer/sidebar
4. Settings/Appearances, only if needed
5. Git dashboard, only if needed

Explicitly excluded from this wave:

- Terminal host
- PTY/session ownership
- Terminal `wgpu` effects bridge
- Final event-loop ownership

For each surface:

- [ ] Define the state boundary between core model and GPUI view.
- [ ] Port the view with feature parity for the most-used workflows.
- [ ] Keep the old surface available until the new one is verified.
- [ ] Add focused tests where model behavior changes.
- [ ] Manually verify keyboard, mouse, focus, resize, and persistence behavior.

## Phase 5: Event Loop and Terminal Reassessment

**Objective:** Revisit full shell ownership only after Stacker, editor, and Explorer prove GPUI is worth expanding.

- [ ] Decide whether GPUI should ever own the main event loop.
- [ ] Decide whether terminal bridging is worth a separate spike.
- [ ] Keep existing terminal visuals and behavior unless there is a clear win.
- [ ] Remove obsolete egui paths only for surfaces that have verified GPUI replacements.
- [ ] Audit binary size and dependency overlap after the production surfaces land.

## Risks

| Risk | Why It Matters | Mitigation |
| :--- | :--- | :--- |
| GPUI API instability | The dependency may change under us. | Pin a commit and isolate GPUI behind local modules. |
| Terminal migration creep | The terminal/effects pipeline is core to LLNZY's identity. | Keep terminal out of scope for this wave. |
| Input latency | Bridged input can feel worse than native paths. | Measure early with terminal and Stacker prototypes. |
| Graphics ownership conflicts | GPUI may not coexist cleanly with the existing `wgpu` pipeline. | Avoid terminal bridging until Stacker/editor/Explorer justify more work. |
| Scope creep | A full shell rewrite can stall the app. | Migrate one surface at a time with exit criteria. |
| Editor complexity | The editor has many hidden behaviors and integrations. | Leave editor migration until after smaller GPUI wins. |

## Success Criteria

- Stacker gains reliable native-feeling text input, including dictation and IME behavior.
- Code editor gains GPUI-backed text interaction without losing LLNZY's existing editor/LSP behavior.
- Explorer handles large trees smoothly with simpler UI code.
- Terminal remains on its existing path with no regressions.
- The app remains usable throughout the migration.
- GPUI only expands beyond Stacker/editor/Explorer after those surfaces prove it should.
