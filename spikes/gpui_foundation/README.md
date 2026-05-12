# GPUI Foundation Spike

This is an isolated GPUI spike for LLNZY. It is intentionally outside the production app so GPUI can be tested without changing the current `winit`/`egui` runtime.

## Goals

- Prove a pinned GPUI dependency builds locally.
- Open a GPUI window with LLNZY-like layout regions.
- Exercise basic declarative layout, scrolling/list content, and custom canvas painting.
- Capture follow-up findings before any production migration begins.

## Run

```sh
cargo run --manifest-path spikes/gpui_foundation/Cargo.toml
```

## Build Status

Current blocker on this machine:

```text
gpui@0.2.2: metal shader compilation failed
xcrun: error: unable to find utility "metal", not a developer tool or in PATH
```

The active developer directory is currently:

```sh
xcode-select -p
# /Library/Developer/CommandLineTools
```

GPUI's macOS renderer needs the full Xcode toolchain, not only Command Line Tools, because its build script invokes Apple's `metal` shader compiler. After installing Xcode, point `xcode-select` at it and re-run:

```sh
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
xcrun -find metal
cargo check --manifest-path spikes/gpui_foundation/Cargo.toml
```

## Current Scope

The first build proves a basic GPUI shell and custom-painted panel. It does not yet prove native text input, IME/dictation behavior, or terminal texture bridging.
