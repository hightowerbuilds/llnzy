# Code Quality Review - 05-16-2026

## Scope

This review audited the LLNZY codebase as a senior-developer quality pass. The
constraint for this pass was strict: the current app behavior is presumed good,
and the goal is to understand code quality and future change risk without
changing functionality.

No source edits were made during the audit. The worktree already had unrelated
dirty state before the review:

- `daily-growth/mm-dd-yyyy-summary/05-16-2026.md`
- `daily-growth/roadmaps/editor-roadmap.md` moved to
  `daily-growth/roadmaps/old/editor-roadmap.md`
- `src/gpui_editor/render.rs`

## Verification Snapshot

The current branch meets the documented local quality gate:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Results:

- Formatting passed.
- Clippy passed with warnings denied.
- Full all-targets/all-features tests passed.
- The normal test suite included 606 library tests, 11 PTY round-trip tests,
  and 56 terminal emulation integration tests.

Release performance budgets also passed:

```sh
cargo test --release --test performance_budgets -- --ignored --nocapture
```

Observed release budget results:

- Editor large insert: 14.35 ms against a 500 ms budget.
- Terminal output throughput: 37.33 ms against a 500 ms budget.
- Rust syntax parse: 73.53 ms against a 1 second budget.

The headless library shape also passed:

```sh
cargo check --lib --no-default-features
```

That command emitted unused-code warnings around editor syntax preset exports in
the no-default feature profile. It still compiled successfully, but the warning
noise is worth cleaning up if the no-default library profile remains part of the
local release gate.

## Executive Assessment

LLNZY is a serious, above-average Rust desktop codebase. It has an explicit
architecture policy, a real local quality gate, meaningful model tests, and
honest platform positioning. The core app surfaces are not just
feature-complete; they are backed by enough model and integration coverage that
future development can proceed with some confidence.

The dominant risk is not whether the app works today. The dominant risk is how
safely the codebase can keep changing without accidental behavior drift. Several
large GPUI surfaces still carry a lot of orchestration state, and a handful of
file/protocol edge cases are not as hardened as the rest of the project.

The right next move is behavior-preserving hardening: add focused regression
tests and tighten a few internals without changing user-visible behavior.

## What Is Good

### Architecture And Ownership

The repo has unusually clear internal standards for a personal desktop app.
`docs/architecture.md`, `docs/quality-policy.md`, and the ADRs define where code
belongs, what the quality gate is, and which platform is actually supported.
That matters because LLNZY has a broad surface: terminal, editor, LSP, Git,
Stacker, Sketch, appearances, config, themes, packaging, and diagnostics.

The crate boundary mostly supports that architecture. Pure modules such as
`src/editor`, `src/terminal`, `src/lsp`, `src/stacker`, `src/sketch`,
`src/config`, and `src/platform` are separated from GPUI surfaces. GPUI modules
are still large, but they generally adapt UI events to tested model code rather
than owning every rule directly.

### Editor Core

The editor foundation is strong. It uses a rope-backed buffer, character-based
positions, undo/redo history, line-ending preservation, tree-sitter language
detection, async parsing with generation checks, search/replace, markdown
preview state, LSP integration, and large-file degradation.

Important details are handled well:

- Syntax parsing is deferred off the UI path.
- Parse results include buffer id, path, language id, line count, and generation
  so stale parse results are dropped safely.
- Large files disable syntax/minimap/live LSP via explicit thresholds.
- UTF-16 conversion is isolated and tested.
- Cursor, buffer, history, search, editorconfig, project search, syntax, and
  LSP adapters have focused tests.

### Terminal And PTY

The terminal/PTY split is one of the healthiest boundaries in the app.
Terminal emulation lives in `src/terminal`, session behavior in `src/session.rs`,
process management in `src/pty.rs`, and GPUI rendering/input in the GPUI
terminal modules.

PTY reads and writes are moved off the render path. Session output is budgeted
per frame with chunk and byte caps. The terminal integration suite exercises
ANSI colors, cursor movement, selection, scroll regions, OSC title/CWD events,
resize preservation, bracketed paste, and other behavior that tends to regress
in terminal apps.

### Stacker

Stacker is production-minded. The CLI identifies its trust boundary directly:
argv, stdin, and files are external input and are sized, parsed, and sanitized
before reaching app-owned storage.

Prompt storage uses file-per-prompt Markdown with frontmatter, staged atomic
writes, archive state, migration from legacy JSON, quota enforcement for inbox
suggestions, and a tested command parser. This is much better than a loose JSON
blob for long-term local data.

### LSP

The LSP layer is more modular than the UI code. Transport, document lifecycle,
diagnostics, requests, workspace edits, registry, and GPUI adaptation are split.
The code handles startup state, unavailable servers, pending open documents,
diagnostics remapping, progress status, server requests, and async request
polling.

This is a good shape for incremental hardening because the riskier protocol
logic can be tested without opening a window.

### Diagnostics And Failure Policy

The error log and diagnostics report are practical. Runtime warnings/errors are
captured, persisted, replayed, compacted, and exposed in the UI. The panic hook
writes a crash diagnostic. Effects failures degrade rather than crash the app
when possible. This matches the documented policy: user work should continue
when optional visual systems fail.

### Performance Discipline

Performance is not hand-waved. There are explicit release budgets for editor
large inserts, syntax parsing, and terminal throughput, and the current numbers
are comfortably below budget. The dev profile also intentionally optimizes
dependencies and lightly optimizes the local crate, which is pragmatic for a
desktop editor/terminal app.

## What Needs Improvement

### Large GPUI Surfaces

The largest maintainability risk is surface size and state density in GPUI
modules. Files such as `src/gpui_workspace.rs`, `src/gpui_editor.rs`,
`src/gpui_terminal.rs`, `src/gpui_sketch.rs`, and
`src/gpui_workspace/appearances.rs` still combine broad orchestration state,
render wiring, input routing, menu actions, modal state, persistence hooks, and
cross-surface coordination.

This is not an immediate correctness defect. The code is organized enough to
work. The risk is reviewability: small changes in these surfaces can have hidden
effects because the state graph is wide.

Future work should continue extracting:

- Pure state transitions.
- Render context structs.
- Small command/action planners.
- Focused helper modules with tests.

The important constraint is to avoid broad rewrites. Extraction should happen
only where a pure boundary can be tested and behavior can be locked down.

### Rope Line Access

The most concrete technical concern is `Buffer::line`.

`src/editor/buffer/model.rs` calls `rope.line(idx).as_str().unwrap_or("")`.
Rope slices are not guaranteed to be contiguous. If a valid line is stored
across rope chunks, `as_str()` can return `None`, and this implementation would
return an empty string for real content.

That affects a lot of downstream behavior:

- Rendering.
- Cursor movement.
- Search.
- Indentation.
- Comment toggling.
- Git gutter calculations.
- LSP position calculation surfaces.

The current test suite is green, so this is not proven broken in normal usage.
It is still the first hardening target because a single low-level assumption
feeds many editor paths.

### Save Semantics

`Buffer::save_to` stages to a temp file and renames, which is good. It does not
yet preserve permissions/xattrs, fsync the parent directory, or enforce an
external-change check at the save operation itself.

The editor does track last-seen disk text and has manual external-change
detection. The remaining issue is that save is not itself a guarded boundary.
If a file changes on disk between checks, the save path can still overwrite it.

Unopened-file LSP workspace edits currently use direct `fs::write`, which is
less durable than the buffer save path and should eventually share the safer
write helper.

### EditorConfig Save-Time Policies

The `.editorconfig` parser is solid and tested, but several save-time fields are
explicitly inert:

- `insert_final_newline`
- `trim_trailing_whitespace`
- `end_of_line`
- `charset`

This is documented in the buffer model, so it is not hidden debt. It is still a
product expectation gap: users may assume those settings affect save behavior.

### LSP File URI Handling

The LSP document layer constructs file URIs manually with `file://` plus
`Path::display()`, and URI-to-path conversion strips the prefix. That is too
thin for paths containing spaces, `#`, `%`, non-UTF-8 segments, or platform
quirks.

The current supported platform is macOS, which lowers the immediate blast
radius, but this is protocol code and should be boringly correct.

### Project Search Semantics

Project search is intentionally simple. It uses a manual directory stack,
hard-coded ignored directories, extension allow-lists, and hard caps. That is
fine for current behavior. The quality concern is that it does not fully honor
`.gitignore` semantics and may surprise users coming from mature editors.

This is lower priority than buffer and file/protocol hardening because changing
search behavior is more user-visible.

### Dependency And Release Policy Enforcement

The repo documents advisory and license checks with `cargo audit` and
`cargo deny`. The project decision is to avoid adding GitHub Actions
integration for these checks, so they should remain local/manual
release-readiness checks unless a different automation target is chosen later.

Release performance budgets should follow the same rule. They pass locally, but
they are intentionally a human-run gate rather than a GitHub Actions job.

## Recommended First Step

Start with rope line-access hardening. It has the best ratio of risk reduction
to behavior change because it sits under the editor but can be tested directly.

Behavior-preserving plan:

1. Add regression coverage that forces a non-contiguous or stress-shaped rope
   line and proves line text, line length, search, cursor movement, and save
   round-trip behavior still work.
2. Replace `Buffer::line` internals with an implementation that does not treat
   non-contiguous rope slices as empty content.
3. Keep the public behavior the same: callers should still receive line text
   without trailing newline or carriage return.
4. Run the full gate after the change:
   - `cargo fmt --check`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test --all-targets --all-features`

The main design choice for that step is whether `Buffer::line` should continue
returning `&str` or whether the buffer API should expose a borrowed-or-owned
line type. Keeping the current `&str` signature may force an internal scratch
buffer or continued assumptions. A cleaner API may use `Cow<'_, str>` or a
dedicated helper for callers that need owned text. Because this touches a broad
call graph, the safest first patch is test-first and tightly scoped.

## Follow-Up Roadmap

After rope line-access hardening:

1. Harden LSP file URI/path conversion with focused tests for spaces, encoded
   characters, and relative paths.
2. Introduce a shared safe-write helper for buffer saves and unopened-file LSP
   workspace edits.
3. Add explicit save conflict tests around last-seen disk text and external
   edits.
4. Decide whether `.editorconfig` save-time settings should be implemented or
   surfaced as unsupported in UI/docs.
5. Continue extracting pure state from large GPUI surfaces only when a tested
   boundary naturally appears.
6. Keep release performance budgets and dependency advisory/license checks as
   local/manual release-readiness checks. Do not add GitHub Actions integration
   for them.

## Final Judgment

The codebase is in good shape. It is not fragile prototype code. It has real
tests, real architecture, good failure handling, and strong performance posture.

The next quality gains should be surgical, not sweeping. Lock down the low-level
editor invariants first, then tighten file/protocol edges, then keep trimming
large GPUI surfaces as behavior-preserving opportunities appear.
