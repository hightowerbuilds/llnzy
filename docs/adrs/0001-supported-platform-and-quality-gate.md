# ADR 0001: Supported Platform And Quality Gate

Date: 2026-05-14

## Status

Accepted

## Context

LLNZY is a native GPUI desktop app with macOS-specific packaging and a
Metal-backed effects path. The source contains cross-platform Rust modules, but
the product is not currently packaged or tested as a supported Linux or Windows
app.

## Decision

macOS is the current supported release target. The required local and CI gate is
formatting, clippy with warnings denied, and the full all-features test suite.
Release readiness adds the performance budget command and macOS bundle build.

## Consequences

- `cargo check --no-default-features` is not a supported build shape yet.
- New platform claims require matching CI and packaging work.
- Build and release docs must state macOS honestly instead of implying broader
  support.
- The app can still keep pure modules portable, but portability is not a
  product promise until CI enforces it.
