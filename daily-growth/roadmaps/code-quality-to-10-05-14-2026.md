# LLNZY Code Quality To 10 Roadmap - 05-14-2026

Purpose: turn the code-quality conversation into an executable roadmap. This is not a feature wishlist. It is the quality ladder for making LLNZY feel like a professional desktop app codebase: always buildable, easy to change, hard to regress, observable when it fails, and disciplined enough that new work does not make the system more fragile.

Current read: the app has real foundations. It is a Rust desktop workspace with editor, terminal, LSP, sketch, stacker, config, git, preferences, platform, and effects layers. The test surface is unusually broad for a project at this stage. The weak point is not ambition or lack of coverage. The weak point is that the current working tree is not build-clean, several central UI surfaces are too large, and operational quality gates are not yet strong enough to stop regressions before they land.

Baseline from the 05-14-2026 review:

- Current score: 5/10 because the current workspace is build-broken.
- Underlying architecture score if build/test health is restored: roughly 7/10.
- Rust: about 49.6k lines across 157 files.
- Markdown: about 27.3k lines across 87 files.
- Immediate blocker: `src/effects/host.rs` does not compile against pinned `wgpu = 29.0.3`.
- Secondary blocker: packaging metadata test expects `LLNZY`, while `assets/packaging.env` currently sets `DISPLAY_NAME=llnzy`.
- Structural pressure: `gpui_terminal.rs`, `gpui_workspace.rs`, `gpui_editor.rs`, `gpui_sketch.rs`, and `stacker/cli.rs` are still large orchestration files.
- Positive signals: hundreds of unit tests, clear domain modules, rope-backed editor primitives, terminal emulation tests, LSP request tests, platform-specific boundaries, crash-log hook, and performance-minded dev profile settings.

## What A 10 Means

A 10/10 LLNZY codebase is not perfect in the abstract. It is excellent for this product's shape.

It means:

- A fresh checkout builds on the supported platform without local knowledge.
- The default branch is always green.
- CI blocks formatting, clippy, test, packaging, security, and feature-flag regressions.
- The large surfaces have clear state/action/render boundaries.
- Platform-specific and unsafe code is isolated behind small, reviewed modules.
- Panics in production paths are rare, intentional, and documented.
- Error handling preserves user work wherever possible.
- Integration tests cover the workflows users actually rely on.
- Performance has budgets and regression checks.
- Release artifacts are reproducible.
- Current architecture docs describe what exists now, not what used to exist.
- Contributors can tell where a change belongs before opening five files.

The roadmap below is staged. Do not try to jump straight to 10. The correct order is to make the project green, then make it easier to change, then make it hard to regress, then make it operationally boring.

## Get To 7: Green, Trusted, Contributor-Safe

Goal: restore basic trust. A 7/10 codebase can still have design debt, but it compiles, tests pass, common commands are documented, and obvious regressions cannot land unnoticed.

### 1. Restore The Build

Fix the current hard blockers first.

- Update `src/effects/host.rs` to the actual `wgpu = 29.0.3` API.
- Remove the stale descriptor fields and adjust the new required fields.
- Confirm the host path compiles under the default feature set.
- Decide whether effects should be default-on right now. If the shader path is still experimental, gate it behind a feature until the host is stable.
- Fix the packaging name mismatch by choosing one truth:
  - If the product display name is `LLNZY`, change `assets/packaging.env` to `DISPLAY_NAME=LLNZY`.
  - If the product display name is `llnzy`, change the test expectation in `src/platform/packaging.rs`.

Acceptance gate:

```sh
cargo test --lib
cargo test --all-targets --all-features
```

Both must compile and pass on a clean tree.

### 2. Make The Standard Quality Commands Official

The project needs one blessed local verification path and one blessed CI path. They should match closely enough that a local green result means something.

Recommended local gate:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features
cargo test --all-targets --all-features
```

Add a short `docs/development.md` or a top-level README section with the exact commands. Do not leave contributors guessing which subset matters.

Acceptance gate:

- The commands are documented.
- The same commands run in CI.
- CI status is required before merge.

### 3. Clean Up Clippy Warnings

Current clippy warnings are ordinary, but they matter because warning tolerance spreads.

Known warning categories:

- `unnecessary_map_or`
- `too_many_arguments`
- `type_complexity`
- `nonminimal_bool`
- `cloned_ref_to_slice_refs`
- `manual_contains`

Fix the mechanical ones immediately. For `too_many_arguments`, do not silence it casually. Use it as a signal that a render or command helper wants a context struct.

Acceptance gate:

```sh
cargo clippy --all-targets --all-features -- -D warnings
```

This should be green before the project claims 7/10.

### 4. Audit Production Panics

Tests can use `unwrap()` and `expect()` freely. Production code should not panic on recoverable user, file, OS, LSP, terminal, or GPU failure.

Audit all non-test `unwrap`, `expect`, `panic!`, `todo!`, and `unimplemented!` sites. Sort them into:

- Impossible invariant, keep with a precise `expect`.
- Recoverable runtime failure, convert to `Result` or graceful fallback.
- User-facing failure, route to diagnostics or error log.
- Crash-worthy corruption, keep but write enough context to `crash.log`.

High-value areas:

- GPUI window creation and entity updates.
- LSP request serialization and server lifecycle.
- PTY spawn and shell process management.
- File saves, recovery snapshots, theme imports, and image decoding.
- Effects/GPU initialization.

Acceptance gate:

- No casual production `unwrap()` remains in app, platform, editor, terminal, LSP, or effects paths.
- Every retained production `expect()` explains the invariant in human language.
- Failure paths preserve unsaved editor and stacker data where possible.

### 5. Stabilize Feature Flags

The current app has feature gates for GPUI surfaces. The effect pipeline adds more platform-specific pressure. Feature flags should make build intent obvious.

Required build shapes:

- Default app build.
- Library tests.
- All features.
- No-default-features, if the library is intended to support headless use.
- macOS-specific effects build.

Acceptance gate:

```sh
cargo check
cargo check --all-features
cargo test --lib
```

If `--no-default-features` is not supported, document that explicitly rather than letting it fail mysteriously.

### 6. Separate Active Docs From Historical Docs

The repository has a useful but heavy `daily-growth` history. That is fine, but active docs and historical thinking should be visibly separated.

Recommended layout:

- `docs/` for current user and developer truth.
- `daily-growth/roadmaps/` for active roadmaps.
- `daily-growth/roadmaps/old/` and `cold-storage/` for historical material.

Acceptance gate:

- New contributors can find current build, test, packaging, and architecture docs without reading historical roadmaps.
- Active roadmaps have dates and completion status.

## Get To 8: Maintainable Architecture

Goal: reduce change risk. A codebase at 8/10 is not just green. It is shaped so that new work naturally lands in the right place.

### 1. Shrink The Big Surface Files

Large files are not automatically bad, but these files are central and change-heavy. That combination creates merge risk and makes review harder.

Targets:

- `src/gpui_workspace.rs`
- `src/gpui_editor.rs`
- `src/gpui_terminal.rs`
- `src/gpui_sketch.rs`
- `src/gpui_stacker.rs`
- `src/stacker/cli.rs`

The goal is not arbitrary line-count reduction. The goal is ownership clarity.

Preferred pattern:

- Top-level GPUI file owns app/entity wiring and high-level orchestration.
- `state.rs` owns state structs and invariants.
- `actions.rs` owns command handlers.
- `render.rs` owns view construction.
- `input.rs` owns keyboard/mouse/text input translation.
- `effects.rs`, `lsp.rs`, `files.rs`, `search.rs`, etc. own feature-specific behavior.
- Tests sit near pure logic where possible.

Acceptance gate:

- No central GPUI file should keep growing as the default place for new feature logic.
- Each extracted module has a short responsibility statement at the top or obvious naming.
- Refactors preserve behavior and land with tests for the extracted pure logic.

### 2. Introduce Context Structs For Dense Render Helpers

The clippy `too_many_arguments` warnings point at a real problem: render helpers become hard to call correctly as UI complexity grows.

Replace long signatures with small context structs when helpers need shared state.

Example shape:

```rust
struct TerminalRowRenderContext<'a> {
    session: &'a Session,
    config: &'a Config,
    metrics: TerminalMetrics,
    selection: &'a SelectionState,
}
```

The win is not fewer characters. The win is that future fields can be added without touching every call site and without losing argument meaning.

Acceptance gate:

- Render helpers with repeated argument clusters use context structs.
- Context structs do not become dumping grounds. Split them by responsibility.

### 3. Make State Transitions Testable Without GPUI

GPUI rendering is expensive to exercise directly. The codebase should push behavior into pure state machines and keep GPUI as a thin shell.

Candidates:

- Workspace tab lifecycle.
- Joined pane behavior.
- Sidebar move and rename state.
- Command palette filtering and execution selection.
- Editor close/reopen/dirty-buffer flows.
- Terminal session restart and title/cwd updates.
- Appearance preference changes.
- Effects selection and uniform normalization.

Acceptance gate:

- New behavior can usually be tested without opening a GPUI window.
- UI handlers mostly translate events into state transitions.
- Pure logic modules have focused unit tests.

### 4. Isolate Platform And Unsafe Code

Unsafe and platform-specific code is acceptable in this app. It is not acceptable for that code to leak everywhere.

Rules:

- Every `unsafe` block gets a `SAFETY:` comment that names the actual invariant.
- OS APIs live in `platform/` or tightly scoped modules like `effects/host.rs`.
- Callers receive safe Rust types or explicit `Result`/`Option` outcomes.
- GPU failures skip a frame or disable an effect, not crash the workspace.

Acceptance gate:

- `rg "unsafe" src` produces a short, reviewable list.
- Each unsafe site has a narrow reason to exist.
- Effects and platform modules have tests for validation logic even if hardware paths require manual smoke tests.

### 5. Clarify Error Handling Policy

Different errors deserve different UX.

Recommended tiers:

- Silent fallback: optional decoration fails, such as a shader frame skip.
- Status/error log: LSP unavailable, theme import rejected, background image invalid.
- User prompt: destructive file operation, overwrite, unsaved close.
- Crash log: invariant violation or unrecoverable corruption.

Acceptance gate:

- Each major subsystem follows the same tiers.
- Error messages include the failed operation and relevant path/server/effect name.
- Recoverable errors do not panic.

### 6. Define Module Ownership

Write a short `docs/architecture.md` that maps the codebase.

It should answer:

- Where does editor model logic live?
- Where does editor GPUI rendering live?
- Where does terminal emulation stop and terminal UI begin?
- Where does LSP transport stop and editor integration begin?
- Where do config defaults, user preferences, and runtime applied config meet?
- Where should a new appearance setting be added?
- Where should platform-specific packaging logic live?

Acceptance gate:

- A contributor can place a new feature without reverse-engineering the whole app.
- Roadmaps link to the modules they intend to touch.

## Get To 9: Durable Under Real Usage

Goal: move from maintainable to robust. A 9/10 codebase has tests and tooling that model real user workflows, not just small units.

### 1. Add Workflow Smoke Tests

Unit tests are already a strength. The next layer is workflow coverage.

High-value smoke tests:

- App startup reaches workspace initialization.
- Open project, populate sidebar, select file, open editor.
- Edit file, save file, reload file.
- Dirty buffer close is blocked or handled intentionally.
- Terminal starts a shell, receives output, restarts after exit.
- LSP missing-server path is user-visible and non-fatal.
- LSP fake-server path covers completion, hover, references, rename, and format.
- Theme/background import handles missing, oversized, invalid, and valid images.
- Effects disabled path works on machines without usable GPU setup.

Acceptance gate:

- Critical user workflows have at least one automated test or documented manual smoke test.
- Manual smoke tests are short, dated, and repeatable.

### 2. Add Property Tests Where Bugs Hide

Text editors fail at boundaries: Unicode, selections, undo, redo, multi-cursor, line endings, and byte/char/UTF-16 conversion.

Add `proptest` or equivalent for:

- Insert/delete round trips.
- Undo/redo restores exact text and selection.
- UTF-8 char positions and UTF-16 LSP positions round-trip where valid.
- Selection ranges remain ordered and clamped.
- CRLF save/reload preserves intended line endings.
- Search/replace does not panic on arbitrary input.
- Stacker markdown storage round-trips frontmatter and body.

Acceptance gate:

- Property tests run in CI with a practical case count.
- Regressions save seeds when they fail.

### 3. Add Performance Budgets

LLNZY is an editor and terminal. Performance regressions are correctness regressions.

Add benchmarks for:

- Opening large files.
- Typing into medium and large buffers.
- Syntax parse/highlight on common languages.
- Project search on fixture trees.
- Terminal output throughput.
- Terminal scrollback rendering.
- LSP diagnostic remapping.
- Effects frame render cost, if effects remain default-on.

Suggested budget style:

- Hard CI gate for obvious disasters.
- Soft tracked numbers for noisy desktop/GPU paths.
- Store benchmark notes in `docs/performance.md`.

Acceptance gate:

- There is a repeatable benchmark command.
- At least editor input, syntax, search, and terminal throughput have baseline numbers.
- New performance-sensitive work states whether it affects a budget.

### 4. Build Real Packaging Confidence

Packaging should not rely on memory or one-off local scripts.

Required coverage:

- Packaging metadata tests agree with `assets/packaging.env`.
- Bundle includes icon, plist, executable, CLI install scripts if expected.
- macOS minimum version is checked.
- Signing identity behavior is explicit for dev and release.
- DMG or app bundle generation is reproducible.
- Release artifacts do not live in git.

Acceptance gate:

- A documented release command creates the expected artifact.
- CI or a release workflow validates packaging metadata.
- The app name, executable name, bundle id, icon, and version come from one source of truth.

### 5. Add Security And Dependency Gates

This project depends on substantial parsing, graphics, async, image, and terminal code. Supply-chain checks are warranted.

Add:

- `cargo audit` or equivalent vulnerability check.
- `cargo deny` for license and duplicate dependency policy.
- Review policy for exact pins like `gpui = "=0.2.2"` and `wgpu = "=29.0.3"`.
- A documented dependency upgrade cadence.

Acceptance gate:

- CI reports dependency vulnerabilities and license issues.
- Exact pins are intentional and documented.

### 6. Improve Diagnostics And Crash Recovery

The project already writes crash logs. The next step is making failures actionable.

Add:

- Structured diagnostic entries with subsystem, operation, and error.
- Last-opened workspace/session context where privacy-safe.
- Recovery snapshots for dirty editor buffers.
- Clear user-facing status when LSP, PTY, or effects fail.
- A diagnostics export command or menu item.

Acceptance gate:

- A user can send one diagnostics bundle after a failure.
- A developer can tell whether the failure was editor, terminal, LSP, config, platform, or effects.

## Get To 10: Professional-Grade, Boringly Reliable

Goal: make quality self-sustaining. A 10/10 codebase does not depend on heroic memory. It has systems that keep it clean.

### 1. Require Green Main

Main should always represent a usable app.

Required:

- Branch protection.
- Required CI checks.
- No merging with known failing tests unless an explicit quarantine process exists.
- Fast rollback path for broken main.

Acceptance gate:

- A fresh clone of main builds and passes the documented gate.
- Broken main is treated as a stop-the-line event.

### 2. Establish A Test Pyramid

The test suite should have layers with different costs.

Recommended layers:

- Unit tests for pure logic.
- Property tests for text and state invariants.
- Integration tests for PTY/LSP/storage workflows.
- Smoke tests for app startup and packaging.
- Manual visual tests for GPUI rendering and effects where automation is not practical yet.

Acceptance gate:

- The project documents which layer catches which class of bug.
- Slow tests are named and scheduled intentionally.
- New subsystems add tests at the right layer.

### 3. Make Architecture Decisions Explicit

Write short ADRs for decisions that will otherwise be rediscovered repeatedly.

Needed ADRs:

- Why GPUI and what version policy?
- Why alacritty_terminal and portable-pty?
- How LSP clients are managed.
- How editor positions handle bytes, chars, graphemes, and UTF-16.
- How terminal cell metrics are computed.
- How effects render into GPUI.
- What platforms are supported.
- What the product thesis is.

Acceptance gate:

- Major architecture choices have a dated record.
- New work can cite or update an ADR instead of relitigating the choice.

### 4. Make Platform Support Honest

If LLNZY is macOS-only, own that. If it is macOS-first, prepare for portability now.

MacOS-only quality bar:

- macOS CI is exhaustive.
- macOS packaging, signing, notarization, and crash logs are polished.
- Non-macOS builds fail with clear messaging or are unsupported by design.

MacOS-first quality bar:

- Linux `cargo check` runs in CI for non-GPUI/headless parts.
- Platform modules compile conditionally.
- Terminal/editor core avoids unnecessary macOS assumptions.
- Packaging roadmaps name the future target platforms.

Acceptance gate:

- The README and CI agree on platform support.
- Unsupported platforms fail clearly.

### 5. Finish The Operational Loop

A professional app has a loop from failure to fix.

Required:

- Crash log generated.
- Diagnostics export available.
- Version/build channel included in diagnostics.
- User-facing error paths preserve work.
- Reproduction steps can be linked to tests.
- Regression tests are added for fixed bugs.

Acceptance gate:

- Every serious bug fix either adds a test or documents why it cannot.
- Diagnostics contain enough context to classify the failure.

### 6. Keep Historical Roadmaps From Becoming Product Truth

The `daily-growth` folder is valuable because it preserves thinking. It should not compete with current docs.

For 10/10:

- Active roadmaps have status markers.
- Completed roadmaps move to `old/`.
- Current architecture docs live in `docs/`.
- Historical claims are not the source of truth for current behavior.

Acceptance gate:

- Someone can read `README.md`, `docs/development.md`, and `docs/architecture.md` and understand the current project.
- Roadmaps are for execution, not required onboarding.

## Suggested Execution Order

This order is deliberately conservative.

1. Fix `wgpu` compile errors in `src/effects/host.rs`.
2. Fix packaging display-name mismatch.
3. Run and document the full local quality gate.
4. Make CI enforce fmt, clippy with warnings denied, and tests.
5. Audit production panics.
6. Stabilize feature-flag build shapes.
7. Extract state/action/render boundaries from the largest GPUI files.
8. Add pure state tests for each extracted boundary.
9. Add workflow smoke tests.
10. Add property tests for editor text invariants.
11. Add performance benchmarks and budgets.
12. Harden packaging and release workflow.
13. Add dependency/security checks.
14. Write current architecture docs and ADRs.
15. Keep main green as policy.

## Scorecard

Use this as the recurring review checklist.

## Execution Status - 05-14-2026

The 7/10 reliability gate has now been worked through for the supported build shapes. The app builds, formats, passes clippy with warnings denied, and passes the full automated test suite under `--all-targets --all-features`.

Completed reliability work:

- Restored the effects host against the pinned `wgpu` API and made missing GPU/effects setup non-fatal.
- Fixed the packaging display-name mismatch so packaging metadata tests agree with `assets/packaging.env`.
- Added the standard local quality gate and documented supported/unsupported Cargo build shapes in `docs/development.md`.
- Made CI enforce formatting, clippy with `-D warnings`, and the full test suite.
- Hardened GPUI startup, workspace focus, Stacker UI paint, editor startup, image preview, shared error log state, LSP runtime setup, LSP request serialization, LSP transport stdio setup, terminal URL detection, and Stacker storage formatting against recoverable panics.
- Replaced the Stacker command descriptor searched-array `expect` with an exhaustive enum mapping.
- Audited the remaining panic-pattern matches. The remaining `unwrap`, `expect`, and deliberate `panic!` matches are test-only fixtures, `#[cfg(test)]` helpers, assertion paths, or clippy expectation attributes rather than production runtime paths.

Known status:

- `cargo check --no-default-features` is intentionally unsupported for the current binary shape and is documented as such.
- Manual GPUI and visual workflow checks are deferred to the final smoke-test list, per the working agreement for this roadmap.
- The 8/10 architecture pass extracted Stacker rendering, Stacker refresh planning, Stacker CLI argument parsing, and terminal render geometry into focused modules.
- The 9/10 durability pass added packaging metadata/bundle validation, release-mode performance budgets, text/UTF-16 invariant tests, and a diagnostics report export backend.
- The 10/10 sustainability pass added current architecture, operations, performance, quality-policy, ADR, and manual-smoke-test docs under `docs/`.

Completed architecture and durability work:

- `src/gpui_stacker/render.rs` now owns Stacker view construction.
- `src/stacker/sync.rs` now owns pure prompt refresh planning with focused tests.
- `src/stacker/cli/args.rs` now owns Stacker CLI argument parsing.
- `src/gpui_terminal/render.rs` now owns terminal render geometry and display-mode helper logic.
- `assets/packaging.env`, `assets/Info.plist`, `bundle.sh`, and CI are locked together by packaging metadata tests and a release bundle build.
- `tests/performance_budgets.rs` provides release-mode budgets for editor large insertion, Rust syntax parsing, and terminal output throughput.
- Unicode/UTF-16 and editor buffer position/undo/redo invariants have broader corpus coverage.
- `src/diagnostics.rs` can render and write a diagnostics report containing version, platform/path context, and recent runtime log entries.
- Remaining `unsafe` sites now carry explicit safety context.

Current active docs:

- `docs/development.md`: local gate and current doc index.
- `docs/architecture.md`: source ownership map.
- `docs/quality-policy.md`: branch, test, error, and dependency policy.
- `docs/performance.md`: release performance budget command and baselines.
- `docs/operations.md`: diagnostics, release readiness, and operational loop.
- `docs/manual-smoke-tests.md`: deferred human-in-the-loop smoke checklist.
- `docs/adrs/`: accepted architecture decisions for platform support, editor positions, terminal/PTY boundaries, and GPUI render boundaries.

Remaining manual completion:

- Run the checklist in `docs/manual-smoke-tests.md` after the automated gate is green.
- Wire a visible UI/menu action to `export_diagnostics_report` if diagnostics export should be user-facing rather than backend-only.

### 7/10 Gate

- Full build and tests pass.
- Clippy is clean with warnings denied.
- Formatting is enforced.
- Packaging metadata tests pass.
- Current quality commands are documented.
- No casual production panics in recoverable paths.
- Feature-flag expectations are documented.

### 8/10 Gate

- Large UI/controller files are no longer default homes for feature logic.
- State transitions are testable without GPUI where practical.
- Platform and unsafe code is isolated.
- Error handling policy is consistent.
- Architecture map exists and matches the code.

### 9/10 Gate

- Critical workflows have smoke tests.
- Text/state invariants have property tests.
- Performance budgets exist for editor, terminal, search, syntax, and LSP hotspots.
- Packaging is reproducible and validated.
- Dependency security/license checks run in CI.
- Diagnostics are actionable.

### 10/10 Gate

- Main is always green.
- CI and release gates are trusted.
- Architecture decisions are documented.
- Platform support is honest and enforced.
- Serious bug fixes add regression coverage.
- Historical docs are separated from current truth.
- New contributors can make a focused change without first learning the whole codebase.

## Final Standard

The final standard is not "no debt." The final standard is controlled debt.

LLNZY can keep being ambitious. It can keep having editor, terminal, LSP, stacker, sketch, appearances, and shader systems in one app. But at 10/10, every one of those systems has a clear boundary, a test strategy, an error strategy, and a maintenance story. The app should feel boring to build, boring to test, and boring to release, so the product itself can stay interesting.
