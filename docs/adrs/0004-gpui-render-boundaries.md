# ADR 0004: GPUI Render Boundaries

Date: 2026-05-14

## Status

Accepted

## Context

The app has several high-change GPUI surfaces. Large files are not inherently
wrong, but combining state transitions, rendering, input, and persistence in one
file makes review and regression testing harder.

## Decision

GPUI top-level files should own entity wiring and orchestration. Rendering,
input, actions, sync planning, and pure state transitions should move into
focused modules when they become independently understandable or testable.

## Consequences

- New behavior should start in model modules when it does not require GPUI.
- Render helpers should use small context structs when argument clusters grow.
- Extracted pure boundaries need focused tests.
- The current architecture map is the source of truth for where new code
  belongs.
