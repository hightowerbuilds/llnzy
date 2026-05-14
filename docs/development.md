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

## Current Platform

The enforced CI baseline runs on `macos-latest`. That matches the current app
surface, including GPUI and the macOS effects pipeline.

If a future change claims cross-platform support, add the matching CI job before
calling the platform supported.

## Expected Working Tree

Generated build output, screenshots, local assistant/tool directories, and
private local state should stay out of commits. Shader sources, packaging
metadata, docs, and tests required for the quality gate must be tracked.
