# LLNZY Manual Laundry

Created: 2026-05-05

This document tracks laundry-list work that cannot be closed from code inspection
or unit tests alone. Items here need a live app pass, visual inspection, manual
interaction, platform-specific verification, or a documented manual test matrix.

Current scope: only sections already reached in `daily-growth/roadmaps/laundry-list.md`,
including the revisited `Stacker And External Input` section and the earlier
pass through `Workspaces, Settings, Themes, And UX`. Do not add items from
later sections until the main laundry-list pass reaches them.

## Cross-Platform Compatibility

- [ ] Validate macOS shell startup across zsh, bash, and fish. Source: `future-roadmap.md`.
- [ ] Validate Linux shell startup across bash, zsh, and fish. Source: `future-roadmap.md`.
- [ ] Validate Windows shell startup across PowerShell 7, Windows PowerShell, cmd, Git Bash, WSL, and custom profiles. Source: `future-roadmap.md`.
- [ ] Build visual smoke coverage for startup, Home, ANSI terminal colors, syntax highlighting, joined panes, Stacker/editor layouts, settings, appearances, resize, high DPI, and Linux Wayland/X11. Source: `future-roadmap.md`.
- [ ] Verify Linux dialogs, clipboard, drag and drop, and IME behavior under GNOME/KDE once `.desktop` integration exists. Source: `future-roadmap.md`.

## Reliability And Data Safety

- [ ] Run periodic manual smoke checks for tabs, splits, search, settings, Stacker, paste, terminal routing, and effects. Source: `llnzy-code-quality-cleanup-roadmap.md`.
- [ ] Re-test Wispr Flow and comparable dictation software after event loop, menu, clipboard, paste, or text-input changes. Source: `llnzy-code-quality-cleanup-roadmap.md`.
- [ ] Define and manually exercise "full day of normal use" durability expectations as a release gate. Source: `llnzy-reliability-durability-roadmap-05-01-2026.md`.

## Stacker And External Input

- [ ] Continue manual verification for typing, copy, paste, select all, undo, redo, formatting, saved prompt edit/delete, and queue actions. Source: `leftovers-roadmap-3.md`.
- [ ] Re-test normal Cmd+V paste and normal typing after any external input changes. Source: `leftovers-roadmap-3.md`.
- [ ] Revisit remaining external input delay if future AppKit/WebView text ingress changes reintroduce latency. Source: `leftovers-roadmap-3.md`.
- [ ] Investigate any future case where dictated text is queued and later delivered to the wrong surface. Source: `leftovers-roadmap-3.md`.
- [ ] Live-check saved prompt edit, save update, delete confirmation, delete cancel, Escape cancel, and deleting the currently open dirty saved prompt. Source: `docs/stacker-command-workflow-05-05-2026.md`.
- [ ] Live-check toolbar and command-palette formatting preserve the expected selected text or cursor position after button/palette interaction. Source: `docs/stacker-command-workflow-05-05-2026.md`.

## Sidebar, Files, Drag And Drop, Tabs

- [ ] Verify singleton tabs focus existing tabs, keep stable titles, and preserve persisted state after close/reopen. Source: `llnzy-phase-9-product-hierarchy-ux-consistency-05-02-2026.md`.
- [ ] Verify joined panes keep divider bounds, close/remap behavior, tab switching, context menus, and narrow-pane degradation correct. Source: `llnzy-phase-9-product-hierarchy-ux-consistency-05-02-2026.md`; existing matrix: `docs/joined-layout-manual-verification-05-04-2026.md`.
- [ ] Polish tab right-click menu presentation and perceived latency through a live interaction pass. Source: `manual-smoke-checklist.md`.

## Sketch And Visual Workspace

- [ ] Verify sketch save/recall and saved sketches browser against current behavior. Source: `tweaks.md`.

## Rendering, Performance, And Graphics

- [ ] Validate stale text artifacts after scroll, split resize, tab switch, and theme change. Source: `llnzy-code-quality-cleanup-roadmap.md`; matrix: `docs/terminal-rendering-visual-verification-05-05-2026.md`.
- [ ] Verify terminal text correctness across resize, scrollback, splits, tabs, and themes. Source: `llnzy-code-quality-cleanup-roadmap.md`; matrix: `docs/terminal-rendering-visual-verification-05-05-2026.md`.

## Workspaces, Settings, Themes, And UX

- [ ] Verify Settings and Stacker UI still render and behave correctly after structural changes. Source: `llnzy-code-quality-cleanup-roadmap.md`.
- [ ] Verify optional asset lists have clear empty and unavailable states. Source: `reliability-manual-testing-05-02-2026.md`; current partial coverage: background list has empty/unavailable states, saved workspaces and user themes still need review.
- [ ] Verify background image changing in a live app pass. Source: `tweaks-2.md`; code path exists, but image switching has not been live-verified.
