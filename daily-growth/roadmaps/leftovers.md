# llnzy Leftovers

This document tracks follow-up work that is real but should not block the current manual testing pass.

---

## Terminal Selection Performance

- [ ] Tune terminal selection drag latency in mouse-reporting CLI/TUI apps.
  - Context: The May 3, 2026 manual pass confirmed terminal copy now works in Codex, Gemini, Claude Code, and similar mouse-reporting apps, but drag selection still feels slower than it should.
  - Likely direction: profile the terminal drag path, selection range rebuilding, redraw cadence, and renderer invalidation while selecting emulator grid text.

---

## Stacker Input And Command Routing

- [ ] Repair Stacker text ingress for OS-level dictation/voice text and paste-like committed text.
  - Context: Normal copy/paste inside the Stacker text editor works, and Stacker toolbar actions for undo, select all, and line deletion work.
  - Current failure: Wispr Flow/simple dictated paste does not insert anything into Stacker with the Wispr toggle either on or off, while the same input works in the terminal.
  - Direction: treat Stacker more like the terminal by giving it a small surface-owned input engine below egui widget handling. The engine should own committed text, paste-like text, cursor/selection replacement, newline normalization, and focus-independent text ingress when Stacker is the active surface.

- [ ] Fix Stacker Backspace inserting question marks.
  - Context: During the Stacker input-routing pass, Backspace was observed inserting question-mark replacement characters instead of deleting text.
  - Direction: ensure Backspace/Delete and other non-text editing keys are command operations, never committed text payloads.

---

## Tab Engine

- [ ] Replace the current single joined-tab pair model with a tab grouping engine.
  - Context: The current joined-tabs implementation supports one special pair. The product direction needs developers to quickly create, separate, and manage multiple joined tab groups, such as two joined tabs in one area and two joined tabs elsewhere.
  - Direction: model tab groups as first-class layout state instead of one global joined pair. The engine should support multiple groups, fast join/separate actions, stable tab identity through reorder/close, and predictable context-menu behavior for each group.
