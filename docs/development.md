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

## Performance Budgets

Use `docs/performance.md` for the current release-mode performance budget
command and policy. These checks are separate from the normal local gate because
desktop timing is noisier than pure correctness tests.

## Expected Working Tree

Generated build output, screenshots, local assistant/tool directories, and
private local state should stay out of commits. Shader sources, packaging
metadata, docs, and tests required for the quality gate must be tracked.
