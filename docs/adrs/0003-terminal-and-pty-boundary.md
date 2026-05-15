# ADR 0003: Terminal And PTY Boundary

Date: 2026-05-14

## Status

Accepted

## Context

LLNZY includes a terminal emulator, process/session management, and a GPUI
terminal surface. These are separate responsibilities and regress differently:
emulation bugs corrupt display state, PTY bugs affect child processes, and GPUI
bugs affect rendering/input.

## Decision

Terminal emulation stays in `src/terminal/`, PTY process management stays in
`src/pty.rs` and `src/session.rs`, and GPUI rendering/input stays in
`src/gpui_terminal.rs` plus `src/gpui_terminal/`.

## Consequences

- ANSI/VT behavior should be covered by terminal unit/integration tests.
- PTY spawn, shell output, resize, and restart behavior should be covered by PTY
  round-trip tests where possible.
- GPUI terminal code should translate events and render state instead of
  owning emulation rules.
- Manual smoke tests remain necessary for real shell/TUI behavior.
