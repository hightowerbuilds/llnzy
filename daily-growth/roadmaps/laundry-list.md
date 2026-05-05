# LLNZY Laundry List

Created: 2026-05-04

This document collects deferred, optional, or still-relevant work found across the archived roadmap documents in `daily-growth/roadmaps/old/`.

Treat this as a parking lot, not a current sprint plan. Many entries came from older documents and should be verified against the current app before implementation. Items that may already be partly complete are phrased as verification or polish work instead of assumed missing features.

## Completed Slices

- [x] Durable editor foundation: hardened dirty-buffer close prompts around stable `tab_id` and `BufferId`, added corrupt session restore handling, ran the focused smoke/regression pass, and archived the completed slice. Source: `old/durable-editor-foundation.md`.
- [x] File lifecycle foundation: verified clean sidebar moves, dirty open-file blocking, folder rename policy, selection/focus restoration, clean tab path remaps, and dirty-tab safety around move/rename/delete behavior. Sources: `manual-smoke-checklist.md`, `reliability-manual-testing-05-02-2026.md`, `sidebar-file-explorer-hardening-05-03-2026.md`.
- [x] Editor command routing foundation: smoke-tested command palette editor actions and verified save, undo, redo, copy, paste, select all, and find behavior route through the active editor command path. Source: manual smoke test on 2026-05-04.
- [x] External file change foundation: verified clean external modifications, dirty modification prompts, deleted-file handling, moved-file handling, stable `BufferId` prompt targets, and prompt responses; added focused handler tests for modified, deleted, and moved file events. Source: `llnzy-source-editor-roadmap.md`.
- [x] Find command smoke check: verified the Find command opens and closes the editor find UI consistently. Replacement behavior remains deferred. Source: manual smoke test on 2026-05-04.
- [x] Per-language comment toggle foundation: routed comment toggles through `EditorCommand`, added command palette entries for line and block comments, expanded language-aware comment styles, and covered Rust, Python, SQL, Ruby, block-comment, missing-style, shortcut, and undo behavior with focused tests. Source: `llnzy-source-editor-roadmap.md`.
- [x] Bracket matching foundation: verified pair detection, active-pair highlighting, and jump-to-match behavior; routed the jump command through `EditorCommand`, exposed it in the command palette, and added focused command dispatch tests. Source: `llnzy-source-editor-roadmap.md`.
- [x] Code folding foundation: verified tree-sitter fold range detection, gutter folding, visible-line snapping, placeholder rendering, fold shortcuts, and command palette folding actions; added focused dispatch coverage for fold current, unfold current, fold all, and unfold all. Source: `llnzy-source-editor-roadmap.md`.
- [x] Word wrap foundation: verified editor wrap row computation, wrapped cursor hit testing, wrapped terminal output behavior, sidebar filename wrapping, Settings and native-menu toggles, and added a command palette toggle that updates editor config. Sources: `tweaks.md`, `tweaks-2.md`.
- [x] Multi-file search foundation: verified project search opens from shortcuts and command palette, searches asynchronously across searchable text files, supports regex mode, skips heavy dependency/build folders, includes searchable dotfiles, and opens result matches through the normal file-tab handoff. Source: `llnzy-source-editor-roadmap.md`.
- [x] Sidebar/file UX polish: newly created files now open as normal code tabs, sidebar right-click menus cover file/folder actions, Close Folder and sidebar font-size behavior are wired, file icons remain available for known types, and expanded folders use green connector lines instead of disclosure glyphs. Sources: `sidebar-file-explorer-hardening-05-03-2026.md`, `tweaks.md`.
- [x] External sidebar import drop foundation: native files and folders dropped onto the sidebar root or folders copy into the workspace without overwriting existing items, reuse sidebar drop targeting, and refresh the destination folder after import. Source: later sidebar workflow decision.

## Cross-Platform Compatibility

- [ ] Publish a clear compatibility promise for macOS, Windows, and Linux. Define what is shared across all platforms and what is intentionally platform-specific. Source: `future-roadmap.md`.
- [ ] Keep the workspace, tabs, editor, Stacker, Sketch, Git, and settings model shared across operating systems. Source: `future-roadmap.md`.
- [ ] Document macOS/Linux as Unix PTY platforms and Windows as a ConPTY platform. Source: `future-roadmap.md`.
- [ ] Document shell differences by OS, including default shell discovery, login shell behavior, control sequences, signal behavior, path syntax, encoding, and terminal profile behavior. Source: `future-roadmap.md`.
- [ ] Build a first-class shell profile model with platform, profile name, executable path, args, cwd, environment, PTY host, and task policy. Source: `future-roadmap.md`.
- [ ] Validate macOS shell startup across zsh, bash, and fish. Source: `future-roadmap.md`.
- [ ] Validate Linux shell startup across bash, zsh, and fish. Source: `future-roadmap.md`.
- [ ] Validate Windows shell startup across PowerShell 7, Windows PowerShell, cmd, Git Bash, WSL, and custom profiles. Source: `future-roadmap.md`.
- [ ] Decide and document Windows command execution policy for `.exe`, `.cmd`, `.bat`, PowerShell scripts, and structured tasks. Source: `future-roadmap.md`.
- [ ] Normalize path handling for drive-letter paths, slash-normalized paths, UNC paths, and `file:line:column` references. Source: `future-roadmap.md`.
- [ ] Treat WSL as an explicit terminal profile instead of an invisible fallback. Source: `future-roadmap.md`.
- [ ] Add renderer diagnostics that show backend, adapter, OS, driver hints, and fallback state. Source: `future-roadmap.md`.
- [ ] Define rendering backend policy: Metal on macOS, DX12 on Windows, Vulkan on Linux, and fallback behavior where needed. Source: `future-roadmap.md`.
- [ ] Build visual smoke coverage for startup, Home, ANSI terminal colors, syntax highlighting, joined panes, Stacker/editor layouts, settings, appearances, resize, high DPI, and Linux Wayland/X11. Source: `future-roadmap.md`.
- [ ] Decide app identity and install locations for config, data, and cache on each OS. Source: `future-roadmap.md`.
- [ ] Add macOS signing and notarization. Source: `future-roadmap.md`.
- [ ] Decide Apple Silicon and Intel support policy. Source: `future-roadmap.md`.
- [ ] Add Windows code signing and choose installer/portable distribution format. Source: `future-roadmap.md`.
- [ ] Decide Linux distribution formats such as AppImage, deb, rpm, or Flatpak. Source: `future-roadmap.md`.
- [ ] Add Linux `.desktop` integration and verify dialogs, clipboard, drag and drop, and IME behavior under GNOME/KDE. Source: `future-roadmap.md`.
- [ ] Add release binaries through GitHub Releases, starting with macOS universal binaries if supported. Source: `llnzy-roadmap.md`.
- [ ] Create a Homebrew formula when macOS distribution is stable enough. Source: `llnzy-roadmap.md`.
- [ ] Plan an auto-update mechanism after signing and packaging decisions are settled. Source: `llnzy-roadmap.md`.

## Reliability And Data Safety

- [ ] Add or verify autosave and crash recovery behavior for dirty editor buffers. Source: `enterprise-editor-readiness-review-05-02-2026.md`.
- [x] Verify corrupt state handling for workspace/session restore files. Source: `enterprise-editor-readiness-review-05-02-2026.md`.
- [ ] Add higher-level UI/close-flow tests for tab close, window close, and quit behavior with dirty buffers. Sources: `llnzy-critical-review-04-28-2026.md`, `old/durable-editor-foundation.md`.
- [x] Add tests for save failure paths and failed writes. Source: `llnzy-critical-review-04-28-2026.md`.
- [x] Restore last sessions safely without reopening stale or missing files as valid buffers. Source: `llnzy-reliability-durability-roadmap-05-01-2026.md`.
- [ ] Add stale async response guards for LSP, Git detail loading, and any future background work that can target an old buffer or tab. Source: `llnzy-reliability-durability-roadmap-05-01-2026.md`.
- [ ] Add large-file fixtures and stress fixtures. Source: `llnzy-reliability-durability-roadmap-05-01-2026.md`.
- [ ] Define graceful degradation behavior for large files. Source: `llnzy-reliability-durability-roadmap-05-01-2026.md`.
- [ ] Keep reducing very large UI files and feature ownership until new work can land without expanding oversized modules. Source: `llnzy-reliability-durability-roadmap-05-01-2026.md`.
- [ ] Finish or intentionally suppress current warnings with specific justification. Source: `llnzy-reliability-durability-roadmap-05-01-2026.md`.
- [ ] Run periodic manual smoke checks for tabs, splits, search, settings, Stacker, paste, terminal routing, and effects. Source: `llnzy-code-quality-cleanup-roadmap.md`.
- [ ] Re-test Wispr Flow and comparable dictation software after event loop, menu, clipboard, paste, or text-input changes. Source: `llnzy-code-quality-cleanup-roadmap.md`.
- [ ] Define "full day of normal use" durability expectations and use them as a release gate. Source: `llnzy-reliability-durability-roadmap-05-01-2026.md`.

## Source Editor

- [x] Introduce or verify a stable `BufferId` model for editor buffers. Source: `llnzy-reliability-durability-roadmap-05-01-2026.md`.
- [x] Introduce or verify `EditorCommand` as the command boundary for editor actions. Source: `llnzy-reliability-durability-roadmap-05-01-2026.md`.
- [x] Wire command palette actions into real editor commands instead of ad hoc handlers. Source: `llnzy-reliability-durability-roadmap-05-01-2026.md`.
- [x] Add prompt-before-close and prompt-before-quit handling for modified buffers. Source: `llnzy-source-editor-roadmap.md`.
- [x] Add file watching and external file reload prompts. Source: `llnzy-source-editor-roadmap.md`.
- [ ] Add find and replace. Find opens/closes consistently; replacement behavior and tests remain deferred. Source: `llnzy-source-editor-roadmap.md`.
- [x] Add per-language comment toggle behavior. Source: `llnzy-source-editor-roadmap.md`.
- [ ] Add indentation guides. Source: `llnzy-source-editor-roadmap.md`.
- [x] Add bracket matching. Source: `llnzy-source-editor-roadmap.md`.
- [x] Add code folding. Source: `llnzy-source-editor-roadmap.md`.
- [x] Add multi-file search. Source: `llnzy-source-editor-roadmap.md`.
- [ ] Add multi-cursor commands such as Cmd+D and Cmd+Shift+L. Source: `tweaks.md`.
- [x] Add word wrap rendering and verify sidebar word wrap separately. Sources: `tweaks.md`, `tweaks-2.md`.
- [ ] Add minimap click-to-scroll. Source: `tweaks.md`.
- [ ] Add smooth scrolling and caret animation only after measuring interaction quality. Source: `tweaks.md`.
- [ ] Add configurable editor keybinding presets. Sources: `llnzy-source-editor-roadmap.md`, `leftovers-code-editor.md`, `tweaks.md`.
- [ ] Add custom key mappings in `config.toml`. Source: `llnzy-source-editor-roadmap.md`.
- [ ] Add editor settings surface for editor-specific behavior. Source: `llnzy-source-editor-roadmap.md`.
- [ ] Add snippet engine. Source: `llnzy-source-editor-roadmap.md`.
- [ ] Audit editor documentation against actual behavior. Source: `llnzy-code-quality-cleanup-roadmap.md`.
- [ ] Add parser-backed Markdown preview for Markdown code files if rendered inspection is needed. Do not reintroduce Stacker source/split preview. Sources: `llnzy-reliability-durability-roadmap-05-01-2026.md`, later Stacker decision.
- [ ] Add local image rendering in Markdown preview if the code editor preview supports local assets. Source: `llnzy-reliability-durability-roadmap-05-01-2026.md`.
- [ ] Add Markdown preview scroll sync only for the code editor preview flow. Source: `llnzy-reliability-durability-roadmap-05-01-2026.md`.

## LSP And Code Intelligence

- [ ] Build a fake-LSP test harness for deterministic editor/LSP tests. Source: `llnzy-reliability-durability-roadmap-05-01-2026.md`.
- [ ] Add find references. Source: `llnzy-source-editor-roadmap.md`.
- [ ] Add signature help. Source: `llnzy-source-editor-roadmap.md`.
- [ ] Add workspace symbol search. Source: `llnzy-source-editor-roadmap.md`.
- [ ] Add inlay hints. Source: `llnzy-source-editor-roadmap.md`.
- [ ] Add code lens. Source: `llnzy-source-editor-roadmap.md`.
- [ ] Add diagnostic quick-fix integration. Source: `tweaks.md`.
- [ ] Add LSP progress notifications and status bar server status. Source: `tweaks.md`.
- [ ] Support multiple workspace folders in LSP sessions. Source: `tweaks.md`.
- [ ] Auto-restart crashed language servers with visible status. Source: `tweaks.md`.
- [ ] Add tree-sitter incremental edit support if profiling shows full reparse costs are meaningful. Source: `tweaks.md`.

## Stacker And External Input

- [ ] Keep the Stacker text engine as the owner of text mutation and avoid bypassing it for formatting or queue actions. Source: `leftovers-roadmap-3.md`.
- [ ] Preserve cursor and selection through formatting toolbar actions. Source: `leftovers-roadmap-3.md`.
- [ ] Continue manual verification for typing, copy, paste, select all, undo, redo, formatting, saved prompt edit/delete, and queue actions. Source: `leftovers-roadmap-3.md`.
- [ ] Keep normal Cmd+V paste and normal typing working after any external input changes. Source: `leftovers-roadmap-3.md`.
- [ ] Keep a debug/instrumentation flag available for future external input debugging. Source: `leftovers-roadmap-3.md`.
- [ ] Revisit remaining external input delay if future AppKit/WebView text ingress changes reintroduce latency. Source: `leftovers-roadmap-3.md`.
- [ ] Investigate any future case where dictated text is queued and later delivered to the wrong surface. Source: `leftovers-roadmap-3.md`.
- [ ] Maintain a first-class Stacker command registry shared by shortcuts, toolbar buttons, command palette, native menus, and future external tools. Source: `external-command-handoff-contract-05-04-2026.md`.
- [ ] Keep command-palette entries available for Stacker formatting commands when Stacker is active. Source: `external-command-handoff-contract-05-04-2026.md`.
- [ ] Document and preserve saved prompt edit/delete behavior, including delete confirmation modal. Source: later Stacker roadmap decisions.
- [ ] Document how users delete saved prompts from the saved prompt list. Source: later Stacker roadmap decisions.
- [ ] Keep public local IPC deferred until there is a security and permissions design. Source: `external-command-handoff-contract-05-04-2026.md`.
- [ ] Define future external tool permission checks and user-visible settings before opening command execution to local clients. Source: `external-command-handoff-contract-05-04-2026.md`.
- [ ] Treat future shell automation as a separate terminal execution contract, not as editor insertion. Source: `external-command-handoff-contract-05-04-2026.md`.
- [ ] Consider direct insert/replace-selection support for the code editor as a later external command handoff extension. Source: `external-command-handoff-contract-05-04-2026.md`.

## Terminal

- [ ] Continue terminal lifecycle hardening around startup, restart, resize, close, and long-running sessions. Sources: `enterprise-editor-readiness-review-05-02-2026.md`, `llnzy-critical-review-04-28-2026.md`.
- [ ] Add or verify terminal conformance coverage for selection, paste, control keys, alternate screen, ANSI color, and scrollback. Source: `llnzy-critical-review-04-28-2026.md`.
- [ ] Profile terminal selection drag latency, especially while a mouse-reporting TUI is running. Sources: `leftovers-roadmap-3.md`, `reliability-manual-testing-05-02-2026.md`.
- [ ] Verify copy correctness after forward drag, backward drag, word selection, line selection, and select all. Source: `leftovers-roadmap-3.md`.
- [ ] Add terminal title from the running process. Source: `tweaks.md`.
- [ ] Add or extend terminal link detection. Source: `tweaks.md`.
- [ ] Add copy-on-select option. Source: `tweaks.md`.
- [ ] Add scrollback search. Source: `tweaks.md`.
- [ ] Recheck terminal right-click/menu behavior after each interaction pass. Source: `manual-smoke-checklist.md`.

## Git

- [ ] Share Git repository discovery between the Git tab and editor git gutter. Source: `llnzy-local-git-tab-roadmap-05-01-2026.md`.
- [ ] Keep Git tab process work off the UI frame path. Source: `llnzy-local-git-tab-roadmap-05-01-2026.md`.
- [ ] Consider a `git2` backend later, behind the existing model boundary. Source: `llnzy-local-git-tab-roadmap-05-01-2026.md`.
- [ ] Let Git file rows open files or diffs. Source: `llnzy-local-git-tab-roadmap-05-01-2026.md`.
- [ ] Add branch/all and first-parent toggles. Source: `llnzy-local-git-tab-roadmap-05-01-2026.md`.
- [ ] Add debounced repo-root file watcher for Git refresh. Source: `llnzy-local-git-tab-roadmap-05-01-2026.md`.
- [ ] Add worktree display support. Source: `llnzy-local-git-tab-roadmap-05-01-2026.md`.
- [ ] Add optional file-specific history from the active editor file. Source: `llnzy-local-git-tab-roadmap-05-01-2026.md`.
- [ ] Improve keyboard-friendly Git selection. Source: `llnzy-local-git-tab-roadmap-05-01-2026.md`.
- [ ] Introduce mutating Git actions only with explicit safety UX. Potential actions include stash, commit, reset, checkout, rebase, revert, stash pop, and amend. Source: `llnzy-local-git-tab-roadmap-05-01-2026.md`.

## Sidebar, Files, Drag And Drop, Tabs

- [x] Verify clean file move through the sidebar. Sources: `manual-smoke-checklist.md`, `reliability-manual-testing-05-02-2026.md`.
- [x] Verify dirty open file move is blocked or explicitly prompted. Sources: `manual-smoke-checklist.md`, `reliability-manual-testing-05-02-2026.md`.
- [x] Verify dirty open file rename/delete is blocked or explicitly prompted. Source: `sidebar-file-explorer-hardening-05-03-2026.md`.
- [x] Decide whether folder rename should remap clean open descendant buffers or require descendants to be closed first. Sources: `manual-smoke-checklist.md`, `sidebar-file-explorer-hardening-05-03-2026.md`.
- [x] Decide folder rename behavior for dirty open descendant buffers. Source: `sidebar-file-explorer-hardening-05-03-2026.md`.
- [x] Optionally open newly created files immediately after creation. Source: `sidebar-file-explorer-hardening-05-03-2026.md`.
- [ ] Consider inline rename/create instead of modal flows. Source: `sidebar-file-explorer-hardening-05-03-2026.md`.
- [x] Restore selection/focus for newly created or renamed tree items. Source: `sidebar-file-explorer-hardening-05-03-2026.md`.
- [x] Confirm tab paths remap correctly after clean rename/move. Sources: `manual-smoke-checklist.md`, `sidebar-file-explorer-hardening-05-03-2026.md`.
- [x] Confirm dirty tabs never get silently remapped, closed, or overwritten. Sources: `manual-smoke-checklist.md`, `sidebar-file-explorer-hardening-05-03-2026.md`.
- [x] Add or polish sidebar right-click context menus on files and folders. Source: `tweaks.md`.
- [x] Add Close Folder action. Source: `tweaks.md`.
- [x] Add configurable sidebar font size. Source: `tweaks.md`.
- [x] Add or refine file icons. Source: `tweaks.md`.
- [ ] Replace first-pass drag geometry checks with per-surface drop target registration. Source: `llnzy-drag-and-drop-roadmap-04-29-2026.md`.
- [ ] Aggregate multi-file hover/drop state because window events may arrive one file at a time. Source: `llnzy-drag-and-drop-roadmap-04-29-2026.md`.
- [x] Support native file and folder drops onto sidebar root and folders to import external assets into the workspace without overwriting existing items. Source: later sidebar workflow decision.
- [x] Route binary/image drops to an image viewer, preview, or clear unsupported message. Source: `llnzy-drag-and-drop-roadmap-04-29-2026.md`.
- [ ] Add modifier/context-menu alternate drag operations later. Source: `llnzy-drag-and-drop-roadmap-04-29-2026.md`.
- [x] Ensure drag copy/move never overwrites files without an explicit overwrite design. Source: `llnzy-drag-and-drop-roadmap-04-29-2026.md`.
- [ ] Ensure one undo step can restore a whole drag move/copy operation if undoable file operations are added. Source: `llnzy-drag-and-drop-roadmap-04-29-2026.md`.
- [ ] Consider native outbound drag and drop to Finder, Explorer, and other apps as a future platform bridge. Source: `llnzy-drag-and-drop-roadmap-04-29-2026.md`.
- [ ] Keep drag and drop as a command-producing interaction layer, not feature business logic. Source: `llnzy-drag-and-drop-roadmap-04-29-2026.md`.
- [ ] Support internal app payloads for editor text movement, tab reorder/split/detach, Stacker prompt/queue movement, Sketch object movement, and cross-surface workflows. Source: `llnzy-drag-and-drop-roadmap-04-29-2026.md`.
- [ ] Continue joined-tab polish for mixed groups such as Terminal+CodeFile, Git+Stacker, and CodeFile+Sketch. Sources: `leftovers-roadmap-3.md`, `llnzy-phase-9-product-hierarchy-ux-consistency-05-02-2026.md`.
- [ ] Move joined-tab internals toward stable group IDs if index-based state becomes fragile again. Source: `leftovers-roadmap-3.md`.
- [ ] Verify singleton tabs focus existing tabs, keep stable titles, and preserve persisted state after close/reopen. Source: `llnzy-phase-9-product-hierarchy-ux-consistency-05-02-2026.md`.
- [ ] Verify joined panes keep divider bounds, close/remap behavior, tab switching, context menus, and narrow-pane degradation correct. Source: `llnzy-phase-9-product-hierarchy-ux-consistency-05-02-2026.md`.
- [ ] Polish tab right-click menu presentation and perceived latency. Source: `manual-smoke-checklist.md`.

## Sketch And Visual Workspace

- [ ] Add optional background and outline controls for Sketch text boxes. Source: `llnzy-sketch-pad-roadmap.md`.
- [ ] Decide whether Stacker prompts should be draggable into Sketch as text boxes. Source: `llnzy-sketch-pad-roadmap.md`.
- [ ] Decide whether selected Sketch content should copy to clipboard as text, image, or both. Source: `llnzy-sketch-pad-roadmap.md`.
- [ ] Continue Sketch text tool overhaul if the current interaction model still feels incomplete. Source: `tweaks.md`.
- [ ] Verify sketch save/recall and saved sketches browser against current behavior. Source: `tweaks.md`.
- [ ] Keep Sketch singleton behavior consistent with Home, Stacker, Git, Appearances, and Settings. Source: `llnzy-phase-9-product-hierarchy-ux-consistency-05-02-2026.md`.

## Rendering, Performance, And Graphics

- [ ] Add explicit performance budgets for terminal, editor, effects, LSP, and Git scenarios. Source: `llnzy-critical-review-04-28-2026.md`.
- [ ] Build repeatable performance harnesses for terminal, editor, effects, LSP, and Git workflows. Source: `llnzy-critical-review-04-28-2026.md`.
- [ ] Add adaptive quality when frame time exceeds budget. Source: `llnzy-roadmap.md`.
- [ ] Add power-aware rendering on battery. Source: `llnzy-roadmap.md`.
- [ ] Measure frame smoothness with effects on and off. Source: `llnzy-code-quality-cleanup-roadmap.md`.
- [ ] Validate stale text artifacts after scroll, split resize, tab switch, and theme change. Source: `llnzy-code-quality-cleanup-roadmap.md`.
- [ ] Verify terminal text correctness across resize, scrollback, splits, tabs, and themes. Source: `llnzy-code-quality-cleanup-roadmap.md`.
- [ ] Resolve mismatch between UI background options and built-in shader registration. Source: `llnzy-code-quality-cleanup-roadmap.md`.
- [ ] Decide whether `matrix`, `nebula`, and `tron` backgrounds should exist or be removed. Source: `llnzy-code-quality-cleanup-roadmap.md`.
- [ ] Verify custom shader loading. Source: `llnzy-code-quality-cleanup-roadmap.md`.
- [ ] Clarify `apply_time_of_day` behavior around local time versus UTC. Source: `llnzy-code-quality-cleanup-roadmap.md`.
- [ ] Migrate toward frame/layer vocabulary before any major renderer rewrite. Source: `llnzy-graphics-engine-roadmap.md`.
- [ ] Make feature code produce frame descriptions rather than directly knowing about bloom, CRT, glyph atlases, or other renderer internals. Source: `llnzy-graphics-engine-roadmap.md`.
- [ ] Treat text as a first-class engine primitive instead of terminal-only rendering data. Source: `llnzy-graphics-engine-roadmap.md`.
- [ ] Add layer-aware effects and effect masks. Source: `llnzy-graphics-engine-roadmap.md`.
- [ ] Consider future blur and color grading only after layer-aware passes exist. Source: `llnzy-graphics-engine-roadmap.md`.
- [ ] Add render-aligned hit regions for interaction. Source: `llnzy-graphics-engine-roadmap.md`.
- [ ] Migrate editor text rendering to GPU only after profiling shows a real bottleneck. Sources: `llnzy-source-editor-roadmap.md`, `leftovers-code-editor.md`, `tweaks.md`.
- [ ] Add lazy rendering for long lines if profiling identifies long-line cost. Source: `tweaks.md`.
- [ ] Add memory pressure monitoring and cache eviction. Source: `tweaks.md`.

## Workspaces, Settings, Themes, And UX

- [ ] Create a workspace settings page where a workspace bundles theme, project, and tab layout. Source: `tweaks.md`.
- [ ] Add workspace switcher from Home or command palette. Source: `tweaks.md`.
- [ ] Add workspace auto-save and restore. Source: `tweaks.md`.
- [ ] Keep Settings and Appearances as distinct product surfaces. Source: `llnzy-phase-9-product-hierarchy-ux-consistency-05-02-2026.md`.
- [ ] Verify Settings and Stacker UI still render and behave correctly after structural changes. Source: `llnzy-code-quality-cleanup-roadmap.md`.
- [ ] Verify optional asset lists have clear empty and unavailable states. Source: `reliability-manual-testing-05-02-2026.md`.
- [ ] Add or complete background image library behavior. Source: `tweaks.md`.
- [ ] Fix or verify background image changing. Source: `tweaks-2.md`.
- [ ] Add custom theme creation if the current theme system remains read-only. Source: `tweaks.md`.
- [ ] Add per-view theme application if it fits the product model. Source: `tweaks.md`.
- [ ] Consider theme hot-switch animation only after core theme behavior is stable. Source: `tweaks.md`.
- [ ] Add welcome/onboarding overlay. Sources: `roadmap-to-maintenance.md`, `tweaks.md`.
- [ ] Add keyboard shortcut cheat sheet overlay. Source: `tweaks.md`.
- [ ] Add notification toasts. Sources: `roadmap-to-maintenance.md`, `tweaks.md`.
- [ ] Fix or verify Cmd+/Cmd- zoom behavior. Source: `tweaks.md`.
- [ ] Finish setup with appearance if initial setup still feels incomplete. Source: `tweaks-2.md`.
- [ ] Audit README, config docs, known limitations, and product claims against current behavior. Source: `llnzy-critical-review-04-28-2026.md`.
- [ ] Document built-in theme count and config path behavior accurately. Source: `llnzy-code-quality-cleanup-roadmap.md`.

## Security, Governance, And Enterprise Readiness

- [ ] Write a documented threat model. Sources: `enterprise-editor-readiness-review-05-02-2026.md`, `llnzy-critical-review-04-28-2026.md`.
- [ ] Define boundaries around terminals, project files, LSP, future plugins, Stacker commands, and external tools. Source: `enterprise-editor-readiness-review-05-02-2026.md`.
- [ ] Add dependency review process. Source: `enterprise-editor-readiness-review-05-02-2026.md`.
- [ ] Define secure update chain requirements before auto-update ships. Source: `enterprise-editor-readiness-review-05-02-2026.md`.
- [ ] Define vulnerability handling and security response policy. Source: `enterprise-editor-readiness-review-05-02-2026.md`.
- [ ] Define secrets-handling policy for terminals, config, workspaces, future plugins, and external command integrations. Source: `enterprise-editor-readiness-review-05-02-2026.md`.
- [ ] Add admin/policy controls if LLNZY targets enterprise deployment. Source: `enterprise-editor-readiness-review-05-02-2026.md`.
- [ ] Include managed settings, disabled features, enforced defaults, approved language servers/extensions, telemetry policy, and profile locking in the enterprise policy model. Source: `enterprise-editor-readiness-review-05-02-2026.md`.
- [ ] Define compatibility policy, release cadence, deprecation process, and support expectations. Source: `enterprise-editor-readiness-review-05-02-2026.md`.
- [ ] Keep narrowing product promise around core reliability before adding broad enterprise features. Sources: `llnzy-critical-review-04-28-2026.md`, `llnzy-veteran-editor-review-05-01-2026.md`.

## Extension System And Future Platform Surface

- [ ] Design extension boundaries before exposing plugin-like behavior. Sources: `llnzy-source-editor-roadmap.md`, `leftovers-code-editor.md`, `roadmap-to-maintenance.md`, `llnzy-veteran-editor-review-05-01-2026.md`.
- [ ] Decide which APIs are stable enough for extensions: editor commands, Stacker commands, workspace data, terminal automation, Git data, Sketch objects, or theme assets. Source: `llnzy-veteran-editor-review-05-01-2026.md`.
- [ ] Keep public local IPC and extension execution behind security review and user-visible permissions. Sources: `external-command-handoff-contract-05-04-2026.md`, `enterprise-editor-readiness-review-05-02-2026.md`.
- [ ] Add platform clipboard support for rich text, images, and file lists if workflows require more than plain text. Source: `platform-boundary-architecture-05-04-2026.md`.
- [ ] Keep OS-specific implementations isolated behind platform services as new OS behavior lands. Source: `platform-boundary-architecture-05-04-2026.md`.

## Experimental And Long-Range Ideas

- [ ] Explore a 3D code map only as an optional workspace visualization, not as a replacement for the editor. Source: `llnzy-3d-code-editor-research-04-29-2026.md`.
- [ ] Render project folders and files as scene objects with click-to-open behavior. Source: `llnzy-3d-code-editor-research-04-29-2026.md`.
- [ ] Highlight the active editor file in a future code map. Source: `llnzy-3d-code-editor-research-04-29-2026.md`.
- [ ] Color future code-map objects by file type, diagnostics, or git recency. Source: `llnzy-3d-code-editor-research-04-29-2026.md`.
- [ ] Scale future code-map object height by lines of code only if it remains readable. Source: `llnzy-3d-code-editor-research-04-29-2026.md`.
- [ ] Add hover details and search focus for a future code map. Source: `llnzy-3d-code-editor-research-04-29-2026.md`.
- [ ] Consider dependency/reference arcs, tree-sitter symbol clusters, and LSP document symbol clusters in a future code map. Source: `llnzy-3d-code-editor-research-04-29-2026.md`.
- [ ] Consider git churn, author color mode, timeline scrubber, and files-changed-together relationship mode for future visualization. Source: `llnzy-3d-code-editor-research-04-29-2026.md`.
- [ ] Avoid free-flying controls and tiny hit targets in any future 3D workspace feature. Source: `llnzy-3d-code-editor-research-04-29-2026.md`.

## Archived Documents Reviewed

- `enterprise-editor-readiness-review-05-02-2026.md`
- `durable-editor-foundation.md`
- `external-command-handoff-contract-05-04-2026.md`
- `future-roadmap.md`
- `leftovers-code-editor.md`
- `leftovers-roadmap-3.md`
- `llnzy-3d-code-editor-research-04-29-2026.md`
- `llnzy-code-quality-cleanup-roadmap.md`
- `llnzy-critical-review-04-28-2026.md`
- `llnzy-drag-and-drop-roadmap-04-29-2026.md`
- `llnzy-graphics-engine-roadmap.md`
- `llnzy-local-git-tab-roadmap-05-01-2026.md`
- `llnzy-phase-9-product-hierarchy-ux-consistency-05-02-2026.md`
- `llnzy-reliability-durability-roadmap-05-01-2026.md`
- `llnzy-roadmap.md`
- `llnzy-sketch-pad-roadmap.md`
- `llnzy-source-editor-roadmap.md`
- `llnzy-veteran-editor-review-05-01-2026.md`
- `manual-smoke-checklist.md`
- `platform-boundary-architecture-05-04-2026.md`
- `reliability-manual-testing-05-02-2026.md`
- `roadmap-to-maintenance.md`
- `sidebar-file-explorer-hardening-05-03-2026.md`
- `tweaks-2.md`
- `tweaks.md`
