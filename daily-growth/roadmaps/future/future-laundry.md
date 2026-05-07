# Future Laundry

Status: future backlog

This document collects future-facing roadmap items from the archived roadmap folder, excluding crossover compatibility work. Items are intentionally rough. Some may already be partially implemented and should be verified before being scheduled.

Source notes:
- `daily-growth/roadmaps/old/laundry-list.md`
- `daily-growth/roadmaps/old/cold-storage/future-roadmap.md`
- `daily-growth/roadmaps/old/cold-storage/tweaks.md`
- `daily-growth/roadmaps/old/cold-storage/tweaks-2.md`
- `daily-growth/roadmaps/old/cold-storage/roadmap-to-maintenance.md`
- `daily-growth/roadmaps/old/code-quality-review-2026-05-04.md`
- `daily-growth/roadmaps/old/cold-storage/leftovers-roadmap-3.md`
- `daily-growth/roadmaps/old/cold-storage/sidebar-file-explorer-hardening-05-03-2026.md`
- `daily-growth/roadmaps/old/cold-storage/llnzy-sketch-pad-roadmap.md`
- `daily-growth/roadmaps/old/cold-storage/llnzy-local-git-tab-roadmap-05-01-2026.md`
- `daily-growth/roadmaps/old/cold-storage/llnzy-extension-system-roadmap-05-02-2026.md`
- `daily-growth/roadmaps/old/cold-storage/llnzy-3d-code-editor-research-04-29-2026.md`

Related future roadmap:
- Crossover compatibility now lives in `daily-growth/roadmaps/future/crossover-compatibility.md`.

## Reliability And Code Quality

- [ ] Keep reducing large UI and runtime files so new work can land without expanding oversized modules.
- [ ] Split `src/lsp/mod.rs` into types, lifecycle, diagnostics, requests, workspace edit, symbols, and manager modules.
- [ ] Split `src/config.rs` into model, schema, loading, apply, colors, presets, and keybinding modules.
- [ ] Continue splitting `src/main.rs` into focused app modules for host state, winit handling, window lifecycle, frame prep, terminal input, Stacker bridge, platform menu, and shortcuts.
- [ ] Split `src/editor/buffer.rs` into model, IO, editing, indent, history, and recovery modules.
- [ ] Split `src/terminal.rs` into event, grid, selection, OSC, links, and color modules.
- [ ] Split UI-heavy files such as editor commands, settings tabs, Stacker view, and sidebar modals into component families.
- [ ] Finish or intentionally suppress current warnings with specific justification.
- [ ] Run periodic manual smoke checks for tabs, splits, search, settings, Stacker, paste, terminal routing, and effects.
- [ ] Define a "full day of normal use" durability expectation and use it as a release gate.

## Workspaces, Settings, Themes, And UX

- [ ] Verify workspace auto-save and restore, including active theme persistence.
- [ ] Keep Settings and Appearances as separate product surfaces.
- [ ] Verify Settings and Stacker UI after structural changes.
- [ ] Verify optional asset lists have clear empty and unavailable states.
- [ ] Verify background image switching in the built app.
- [ ] Finish Appearance setup if the page still has unclear or incomplete controls.
- [ ] Decide whether per-view theme application belongs in the product model.
- [ ] Add or polish welcome/onboarding for first launch.
- [ ] Add a keyboard shortcut cheat sheet overlay.
- [ ] Add non-blocking notification toasts.
- [ ] Keep README, config docs, known limitations, and product claims aligned with actual behavior.

## Source Editor

- [ ] Finish find-and-replace behavior and tests if replacement remains deferred.
- [ ] Add or verify indentation guides.
- [ ] Add multi-cursor commands such as next occurrence and select all occurrences.
- [ ] Add minimap click-to-scroll.
- [ ] Add smooth scrolling and smooth caret animation only after measuring interaction quality.
- [ ] Add configurable editor keybinding presets, including VS Code-style defaults, basic Vim mode, and optional Emacs bindings if they still fit the product.
- [ ] Add custom key mappings in config.
- [ ] Keep editor-specific settings discoverable in Settings.
- [ ] Add or verify snippet engine behavior.
- [ ] Audit editor documentation against actual behavior.
- [ ] Add parser-backed Markdown preview for code files if rendered inspection remains important.
- [ ] Add local image rendering in Markdown preview if code editor preview supports local assets.
- [ ] Add Markdown preview scroll sync only for the code editor preview flow.

## LSP And Code Intelligence

- [ ] Verify incremental text sync sends changed ranges instead of full documents.
- [ ] Verify language server progress and status messaging.
- [ ] Verify multiple workspace folder behavior.
- [ ] Verify automatic restart behavior for crashed language servers.
- [ ] Verify diagnostic quick fixes and related code actions.
- [ ] Add platform-aware PATH initialization only where it belongs outside crossover compatibility.
- [ ] Add per-language server command overrides in config.
- [ ] Document that language features depend on installed language servers.

## Stacker And External Input

- [ ] Re-test Wispr Flow and comparable dictation/input software after event loop, menu, clipboard, paste, or text-input changes.
- [ ] Verify whether external text tools deliver text through IME commit, pasteboard paste, accessibility insertion, synthetic key events, or a mix.
- [ ] Keep Stacker text mutation centralized through the document/editor engine.
- [ ] Keep normal paste, typing, select all, undo, redo, formatting, saved prompt edit/delete, and queue actions covered by manual checks.
- [ ] Keep a narrow debug trace available for external input debugging without noisy normal logs.
- [ ] Revisit any future case where dictated text is queued and delivered to the wrong surface.
- [ ] Maintain a Stacker command registry shared by shortcuts, toolbar buttons, command palette, native menus, and future external tools.
- [ ] Keep public local IPC deferred until there is a security, permission, and settings design.
- [ ] Treat future shell automation as a separate terminal execution contract, not as editor insertion.
- [ ] Consider direct insert/replace-selection support for the code editor as a later external command handoff.
- [ ] Decide whether a prompt queue bar still belongs in the product.

## Terminal

- [ ] Continue terminal lifecycle hardening around startup, restart, resize, close, and long-running sessions.
- [ ] Recheck terminal right-click/menu behavior after interaction changes.
- [ ] Keep terminal selection, paste, control keys, alternate screen, ANSI color, and scrollback behavior covered.
- [ ] Profile and re-check terminal selection drag latency when mouse-reporting TUI apps are active.
- [ ] Verify copy correctness after forward drag, backward drag, word selection, line selection, and select all.
- [ ] Keep terminal title, URL detection, copy-on-select, and scrollback search verified after renderer or input changes.

## Sidebar, Files, Drag And Drop, And Tabs

- [ ] Consider inline rename/create instead of modal flows.
- [ ] Replace first-pass drag geometry checks with per-surface drop target registration.
- [ ] Aggregate multi-file hover/drop state because native window events may arrive one file at a time.
- [ ] Add modifier/context-menu alternate drag operations later.
- [ ] Decide whether file operations should become undoable, and ensure one undo step can restore a whole drag move/copy operation if so.
- [ ] Consider native outbound drag and drop to Finder, Explorer, and other apps as a future platform bridge.
- [ ] Keep drag and drop as a command-producing interaction layer, not feature business logic.
- [ ] Support internal app payloads for editor text movement, tab reorder/split/detach, Stacker prompt/queue movement, Sketch object movement, and cross-surface workflows.
- [ ] Continue joined-tab polish for mixed groups such as Terminal+CodeFile, Git+Stacker, and CodeFile+Sketch.
- [ ] Move joined-tab internals toward stable group IDs if index-based state becomes fragile again.
- [ ] Verify singleton tabs focus existing tabs, keep stable titles, and preserve persisted state after close/reopen.
- [ ] Verify joined panes keep divider bounds, close/remap behavior, tab switching, context menus, and narrow-pane degradation correct.
- [ ] Polish tab right-click menu presentation and perceived latency.

## Sketch And Visual Workspace

- [ ] Add optional background and outline controls for Sketch text boxes.
- [ ] Decide whether Sketch should remain one scratchpad or support multiple named sketches as a first-class model.
- [ ] Verify sketch save/recall and saved sketch browser behavior.
- [ ] Decide whether Sketch should use an infinite canvas or a fixed page-like canvas.
- [ ] Decide whether Stacker prompts should be draggable into Sketch as text boxes.
- [ ] Decide whether selected Sketch content should copy to clipboard as text, image, or both.
- [ ] Continue Sketch text tool polish if the current interaction model still feels incomplete.
- [ ] Keep Sketch singleton behavior consistent with Home, Stacker, Git, Appearances, and Settings.

## Git

- [ ] Keep mutating Git actions deferred until explicit safety UX exists.
- [ ] Consider a `git2` backend later, behind the existing Git model boundary.
- [ ] Add file history for the active editor file.
- [ ] Add local blame view.
- [ ] Add stage/unstage selected files with clear previews.
- [ ] Add discard selected worktree changes with confirmation.
- [ ] Add commit UI only after mutation safety is designed.
- [ ] Add stash create/apply/drop with confirmation.
- [ ] Add branch checkout/create/delete with confirmation.
- [ ] Add cherry-pick/revert with confirmation.
- [ ] Add conflict panel.
- [ ] Add local bisect helper.
- [ ] Add compare-two-commits and compare-working-tree-against-selected-commit flows.

## Rendering, Performance, And Graphics

- [ ] Validate stale text artifacts after scroll, split resize, tab switch, and theme change.
- [ ] Verify terminal text correctness across resize, scrollback, splits, tabs, and themes.
- [ ] Keep performance budgets for terminal, editor, effects, LSP, and Git scenarios current.
- [ ] Keep repeatable performance harnesses available for terminal, editor, effects, LSP, and Git workflows.
- [ ] Keep adaptive quality and power-aware rendering behavior verified.
- [ ] Migrate editor text rendering to GPU only after profiling shows a real bottleneck.
- [ ] Add lazy rendering for long lines only if profiling identifies long-line cost.
- [ ] Continue moving renderer work toward frame, layer, primitive, effect, and hit-region boundaries.

## Security, Governance, And Enterprise Readiness

- [ ] Keep the documented threat model current as extension, Stacker, terminal, Git, and external tool features evolve.
- [ ] Define permission checks and user-visible settings before opening command execution to local clients.
- [ ] Keep dependency review, vulnerability handling, security response, and secrets-handling policies current.
- [ ] Keep secure update chain requirements defined before auto-update ships.
- [ ] Keep narrowing the product promise around core reliability before adding broad enterprise features.
- [ ] Revisit managed settings, disabled features, enforced defaults, approved language servers/extensions, telemetry policy, and profile locking only if enterprise deployment becomes a real target.

## Extension System

- [ ] Design extension boundaries before exposing plugin-like behavior.
- [ ] Decide which APIs are stable enough for extensions: editor commands, Stacker commands, workspace data, terminal automation, Git data, Sketch objects, or theme assets.
- [ ] Start with a declarative extension loader for manifests, commands, snippets, themes, language contributions, and terminal profiles.
- [ ] Avoid arbitrary code execution, network install, marketplace behavior, and custom UI panels in the first extension phase.
- [ ] Add local folder/archive installation, disable/enable, uninstall, reload extensions, and safe mode startup.
- [ ] Add an extension manager UI with installed extensions, permissions, enable/disable controls, load errors, and update status.
- [ ] Add Git URL installation only after local installation is stable.
- [ ] Consider a curated extension index after the manager and manifest model prove useful.
- [ ] Research programmable extensions later through process protocols or WASM sandboxing with resource limits and permission enforcement.
- [ ] Consider first-party extensions for Rust tools, Markdown tools, GitHub local helpers, accessible theme packs, and project health checks.

## Experimental And Long-Range Ideas

- [ ] Explore a 3D code map only as an optional workspace visualization, not as a replacement for the editor.
- [ ] Build a constrained top-down or 2.5D software city before considering any free-flight 3D interaction.
- [ ] Render project folders and files as scene objects with click-to-open behavior.
- [ ] Highlight the active editor file in the code map.
- [ ] Color code-map objects by file type, diagnostics, search hits, git recency, or churn.
- [ ] Scale object height by lines of code or symbol count only if it remains readable.
- [ ] Add hover details, search focus, and jump-to-current-file controls.
- [ ] Consider dependency/reference arcs, tree-sitter symbol clusters, and LSP document symbol clusters.
- [ ] Consider git churn, author color mode, timeline scrubber, and files-changed-together relationship mode.
- [ ] Add minimap, object-id picking, saved camera/bookmark views, and editor-plus-code-map split view only after the first code map is useful.
- [ ] Avoid free-flying controls and tiny hit targets in any future 3D workspace feature.
