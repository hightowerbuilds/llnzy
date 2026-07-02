# LLNZY Development Baseline

This document defines the current local quality gate for LLNZY. Run it before
opening a pull request or treating a local branch as ready to merge.

## Required Local Gate

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

These commands are intentionally the same checks enforced by CI:

- formatting must be stable;
- all clippy warnings are treated as errors;
- every target and enabled feature must compile and test cleanly.

## Development Instance

Use `./dev.sh` to run a work-in-progress build alongside the installed daily
driver. It sets `LLNZY_PROFILE=dev`, which moves every platform path (config,
data, cache, themes, workspaces, logs, prompts, recovery state) from the
`llnzy` app directory to `llnzy-dev`, so the dev instance cannot read or
corrupt production state. Arguments pass through to `cargo run`
(`./dev.sh --release` for a release build).

## Supported Cargo Shapes

The supported build shapes are:

```sh
cargo check
cargo check --all-features
cargo test --lib
cargo test --all-targets --all-features
```

`cargo check --no-default-features` is not currently a supported shape. The
default `llnzy` binary starts the GPUI workspace, while that workspace module is
behind the `gpui-workspace` default feature. If LLNZY later needs a headless
library-only profile, add an explicit headless binary or gate `src/main.rs`
before treating no-default-features as supported.

On macOS, the default and all-features shapes also compile the Metal-backed
shader/effects dependencies declared under the macOS target section in
`Cargo.toml`.

## Current Platform

The enforced CI baseline runs on `macos-latest`. That matches the current app
surface, including GPUI and the macOS effects pipeline.

If a future change claims cross-platform support, add the matching CI job before
calling the platform supported.

## Current Architecture Map

Use `docs/architecture.md` as the current module ownership map. Historical
roadmaps are useful context, but they are not the source of truth for where new
code belongs.

Use `docs/quality-policy.md` for the current branch quality bar, test pyramid,
error policy, and dependency review policy.

## Performance Budgets

Use `docs/performance.md` for the current release-mode performance budget
command and policy. These checks are separate from the normal local gate because
desktop timing is noisier than pure correctness tests.

## Operations And Diagnostics

Use `docs/operations.md` for diagnostics locations, report-export behavior, and
the release-readiness command set.

## Manual Smoke Tests

Use `docs/manual-smoke-tests.md` for the deferred human-in-the-loop checks that
complete the roadmap after the automated gate is green.

## Expected Working Tree

Generated build output, screenshots, local assistant/tool directories, and
private local state should stay out of commits. Shader sources, packaging
metadata, docs, and tests required for the quality gate must be tracked.
