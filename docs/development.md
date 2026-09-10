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
cargo check --lib --no-default-features
cargo test --lib
cargo test --all-targets --all-features
```

`cargo check --lib --no-default-features` works because the gpui-gated modules
(`gpui_editor` behind `gpui-editor`; `effects`, `gpui_tabs`, `gpui_terminal`,
and `gpui_workspace` behind `gpui-workspace`) are feature-gated in `src/lib.rs`,
so the library compiles without them. The `--lib` flag means the workspace
binary is not built in this shape, which is also why `src/main.rs` never enters
it. CI's `linux-check` job runs this shape on `ubuntu-latest` as a non-blocking
cross-compile smoke check of the platform-independent modules (editor, config,
lsp, terminal model, error log, and friends).

A full `--no-default-features` build of all targets is not supported:
`src/main.rs` starts the GPUI workspace and needs the `gpui-workspace` default
feature. If LLNZY later needs a headless library-only profile, add an explicit
headless binary or gate `src/main.rs` before treating no-default-features as
supported.

GPUI uses its native Metal renderer with `runtime_shaders`, so building only
requires Command Line Tools rather than the offline Metal compiler from Xcode.
GPUI's own shader source compiles on the device at launch; no shader download
is required. Keep `macos-blade` disabled: Blade 0.7.1 unwraps a missing Metal
drawable, and upstream reports crashes when moving windows between displays
([GPUI issue #27767](https://github.com/zed-industries/zed/issues/27767)). Native
Metal skips drawing when no drawable is available. The display-transition smoke
test remains required because compilation cannot reproduce HDMI hotplug or GPU
switching.

## Current Platform

The required CI jobs (check, fmt, clippy, test, build-release) run on
`macos-latest`. That matches the current app surface, including GPUI and the
macOS effects pipeline. `linux-check` is a separate advisory job on
`ubuntu-latest`, scoped to `cargo check --lib --no-default-features`; it is a
smoke check of the platform-independent modules, not a claim of Linux support.

The `academy-courses` job runs on Ubuntu with Node 22, Python 3.13, and
TypeScript 5.9.3, Elixir 1.19.5, and OTP 28. It runs `python3 scripts/check_academy_courses.py` to verify
all JavaScript, TypeScript, and Elixir solutions pass and starters fail. Release
bundling also depends on this job. See `assets/academy/courses/README.md`
for local prerequisites and starter export commands.

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
private local state should stay out of commits. Packaging metadata, docs, and
tests required for the quality gate must be tracked.
