# ADR 0002: Editor Position Model

Date: 2026-05-14

## Status

Accepted

## Context

LLNZY edits Unicode text, talks to LSP servers, and receives platform text-input
positions. Rust strings use UTF-8 bytes, the buffer model uses character
positions, and LSP/AppKit-style boundaries often use UTF-16 offsets.

## Decision

The editor buffer stores positions as line plus character column. Buffer
operations convert through rope character indices. UTF-16 conversion is isolated
in `src/utf16.rs` (originally `src/stacker/utf16.rs`, relocated when the
Stacker surface was removed in September 2026) and tested against Unicode
boundary corpora.

## Consequences

- UI and protocol adapters must translate into buffer positions rather than
  mutating by byte offsets directly.
- Tests must cover emoji, combining/multi-scalar text, CJK, newlines, and
  surrogate-pair boundaries.
- Byte offsets remain valid only inside parser/protocol adapters that
  explicitly require bytes.
