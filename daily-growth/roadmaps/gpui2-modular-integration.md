# Roadmap: GPUI Integration Spike and Incremental Migration

## Purpose

Evaluate GPUI as a replacement for LLNZY's productivity UI layer without risking the current app. The desired end state is still a split architecture:

- **GPUI shell:** native-feeling app chrome, Explorer, Stacker, settings, and eventually editor surfaces.
- **LLNZY core:** existing terminal emulation, PTY/session model, workspace state, and custom `wgpu` visual effects.

The first goal is not migration. The first goal is proof.

## Guiding Principles

- Do not replace the `winit` event loop until GPUI proves it can host LLNZY's hardest requirements.
- Keep the current app runnable throughout the migration.
- Move one surface at a time, starting with the smallest surface that benefits most from native text behavior.
- Preserve LLNZY's terminal visuals unless a deliberate product decision changes them.
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
- [ ] Investigate whether GPUI can display a dynamic texture or host a custom-rendered surface suitable for the terminal.
- [ ] Document concrete API constraints, unstable areas, and missing primitives.

**Exit Criteria**

- We know whether GPUI can host a high-frequency custom-rendered terminal pane.
- We know whether native text input is materially better than the current Stacker path.
- We have a small reproducible example, not just notes.
- We can name the exact GPUI commit and dependency setup to use.

## Phase 1: Stacker Prototype

**Objective:** Build the first real LLNZY surface in GPUI using actual Stacker data and behavior.

- [ ] Extract or adapt Stacker state so it can be used by both the current app and the GPUI prototype.
- [ ] Build a GPUI Stacker window/surface with prompt editing, saved prompts, queue controls, search, and copy behavior.
- [ ] Replace custom text-input handling in the prototype with GPUI-native text components where possible.
- [ ] Validate dictation, IME, multiline editing, selection, undo/redo, and clipboard workflows.
- [ ] Compare typing latency and interaction behavior against the current egui/AppKit bridge implementation.

**Exit Criteria**

- GPUI Stacker is clearly better for text input and does not regress core Stacker workflows.
- The shared Stacker model boundary is clean enough to keep.
- We can decide whether Stacker should be the first production migration target.

## Phase 2: Terminal Bridge Feasibility

**Objective:** Prove the existing terminal/effects pipeline can be embedded in or coordinated with GPUI.

- [ ] Refactor a minimal renderer path that can draw to an offscreen target or externally hosted surface.
- [ ] Feed a dynamic terminal-like frame into a GPUI custom element or equivalent host.
- [ ] Route GPUI keyboard and mouse events into LLNZY's terminal input path.
- [ ] Validate resize, DPI scaling, mouse hit testing, selection, scrollback, cursor blinking, and redraw scheduling.
- [ ] Measure input latency and frame pacing under load.
- [ ] Identify whether GPUI's graphics stack conflicts with the current `wgpu` device/surface ownership.

**Exit Criteria**

- The terminal bridge is technically viable with acceptable latency.
- The CRT, bloom, particles, and background effects have a credible preservation path.
- If the bridge is not viable, we stop before rewriting the main app around it.

## Phase 3: GPUI Shell Prototype

**Objective:** Recreate LLNZY's outer workspace shell in GPUI while keeping production behavior intact.

- [ ] Implement workspace layout regions: tab bar, sidebar, main pane, footer/status area.
- [ ] Represent LLNZY tab kinds in GPUI: Home, Terminal, CodeFile, ImageFile, Stacker, Sketch, Git, Appearances, Settings.
- [ ] Prototype tab switching, closing, singleton tabs, joined panes, and sidebar visibility.
- [ ] Rebuild Explorer with GPUI list/tree primitives and test large projects.
- [ ] Validate drag-and-drop, modal flows, context menus, keyboard shortcuts, and command palette viability.

**Exit Criteria**

- GPUI can express the shell without excessive custom layout code.
- Explorer performance is measurably better or simpler to maintain.
- Existing workspace/tab semantics can survive the migration.

## Phase 4: Production Migration, Surface by Surface

**Objective:** Start replacing egui surfaces only after the spike work proves the foundation.

Recommended order:

1. Stacker
2. Explorer/sidebar
3. Settings/Appearances
4. Git dashboard
5. Sketch, if GPUI canvas is a better fit
6. Code editor
7. Terminal host and final event-loop ownership

For each surface:

- [ ] Define the state boundary between core model and GPUI view.
- [ ] Port the view with feature parity for the most-used workflows.
- [ ] Keep the old surface available until the new one is verified.
- [ ] Add focused tests where model behavior changes.
- [ ] Manually verify keyboard, mouse, focus, resize, and persistence behavior.

## Phase 5: Code Editor Migration

**Objective:** Move the editor experience only after GPUI has already proven itself in smaller surfaces.

- [ ] Map the current editor modules and rendering path before implementation.
- [ ] Keep LLNZY's buffer model, history, cursor logic, tree-sitter parsing, project search, recovery, git gutter, and LSP manager unless there is a specific reason to replace them.
- [ ] Prototype GPUI-backed text layout and editing with real `ropey` buffers.
- [ ] Port diagnostics, completions, hover, rename, code actions, inlay hints, and symbols as overlays/popovers.
- [ ] Validate large-file behavior, many diagnostics, multi-cursor editing, Vim/Emacs/VS Code key presets, and file watching.

**Exit Criteria**

- Editor typing and scrolling are faster or more correct than the current implementation.
- LSP behavior remains intact.
- No major editor workflow is lost.

## Phase 6: Event Loop Cutover and Cleanup

**Objective:** Let GPUI own the app only after production surfaces and terminal bridging are proven.

- [ ] Move main window ownership from `winit` to GPUI.
- [ ] Remove obsolete egui paths once replacement surfaces are verified.
- [ ] Remove obsolete `winit` ownership code only after the terminal bridge is stable.
- [ ] Revisit packaging, menu integration, app lifecycle, session restore, config reload, and power/performance checks.
- [ ] Audit binary size and dependency overlap.

## Risks

| Risk | Why It Matters | Mitigation |
| :--- | :--- | :--- |
| GPUI API instability | The dependency may change under us. | Pin a commit and isolate GPUI behind local modules. |
| Terminal embedding failure | The terminal/effects pipeline is core to LLNZY's identity. | Prove the bridge before app migration. |
| Input latency | Bridged input can feel worse than native paths. | Measure early with terminal and Stacker prototypes. |
| Graphics ownership conflicts | GPUI may not coexist cleanly with the existing `wgpu` pipeline. | Test texture/surface ownership in Phase 0 and Phase 2. |
| Scope creep | A full shell rewrite can stall the app. | Migrate one surface at a time with exit criteria. |
| Editor complexity | The editor has many hidden behaviors and integrations. | Leave editor migration until after smaller GPUI wins. |

## Success Criteria

- Stacker gains reliable native-feeling text input, including dictation and IME behavior.
- Explorer handles large trees smoothly with simpler UI code.
- Terminal visuals remain recognizably LLNZY: CRT, bloom, particles, cursor effects, and backgrounds.
- The app remains usable throughout the migration.
- GPUI only becomes the primary app shell after the spike and bridge work prove it should.
