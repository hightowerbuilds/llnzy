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

The spike uses GPUI's `macos-blade` feature:

```toml
gpui = { version = "=0.2.2", features = ["macos-blade"] }
```

This avoids GPUI's default macOS build path that invokes Apple's `metal` shader compiler. With the current Command Line Tools setup, the default path fails because `xcrun -find metal` does not resolve, but the Blade-backed path compiles:

```sh
cargo check --manifest-path spikes/gpui_foundation/Cargo.toml
```

## Current Scope

The first build proves a basic GPUI shell and custom-painted panel. It does not yet prove native text input, IME/dictation behavior, or terminal texture bridging.
