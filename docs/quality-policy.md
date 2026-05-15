# LLNZY Quality Policy

This policy defines the current quality bar for LLNZY. It is intentionally
practical: green builds, clear ownership, recoverable errors, and repeatable
release checks matter more than theoretical purity.

## Required Gate

Run before treating a branch as ready:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Release candidates also run:

```sh
cargo test --release --test performance_budgets -- --ignored --nocapture
./bundle.sh --release
```

## Main Branch Standard

- Main should build, format, lint, and test cleanly.
- Broken main is a stop-the-line issue.
- Known failing tests require an explicit quarantine note and a follow-up issue.
- Serious bug fixes should add regression coverage or state why the behavior
  cannot be automated yet.

## Test Pyramid

- Unit tests: pure editor, terminal, Stacker, LSP, config, platform, sketch, and
  parsing logic.
- Invariant tests: Unicode, UTF-16, buffer position, undo/redo, storage, and
  command parsing boundaries.
- Integration tests: PTY round trips, terminal emulation, storage workflows,
  LSP request payloads, and packaging metadata.
- Release budget tests: editor throughput, syntax parse, and terminal output.
- Manual smoke tests: GPUI visuals, effects, app-window behavior, packaging
  launch, and workflows that require human judgment.

## Architecture Ownership

Use `docs/architecture.md` before placing new code. Large GPUI surfaces should
coordinate subsystems; model logic belongs in pure modules where it can be
tested without opening a window.

New feature work should answer:

- Which module owns the state?
- Which module owns rendering?
- Which command or input path mutates it?
- Which automated test catches a regression?
- Which manual smoke test, if any, remains?

## Error Policy

- Optional visual failures can silently degrade if user work is unaffected.
- LSP, PTY, config, theme, image, and effects failures should log status with
  the failed operation and relevant path/server/effect name.
- Destructive file operations and unsaved close paths need user confirmation.
- Invariant violations may crash, but should preserve enough context in
  diagnostics for a developer to classify the failure.

Production code should not `unwrap` recoverable user, file, OS, LSP, PTY, or
GPU failures. Test code may use `unwrap` and `expect` for setup clarity.

## Dependency Policy

Dependency changes require a short justification:

- why the dependency is needed;
- license compatibility;
- maintenance and security posture;
- transitive surface area;
- platform impact;
- whether the dependency needs a local wrapper or focused tests.

Run advisory and license tooling before release when available:

```sh
cargo audit
cargo deny check
```

If the tooling is unavailable locally, record that in the release notes rather
than silently skipping the check.
