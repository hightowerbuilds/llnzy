# GPUI Foundation Spike

This is an isolated GPUI spike for LLNZY. It is intentionally outside the production app so GPUI can be tested without changing the current `winit`/`egui` runtime.

## Goals

- Prove a pinned GPUI dependency builds locally.
- Open a GPUI window with LLNZY-like layout regions.
- Exercise basic declarative layout, scrolling/list content, and custom canvas painting.
- Exercise GPUI's native text input path with a Stacker-like prompt field.
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

## Verified

- `cargo check --manifest-path spikes/gpui_foundation/Cargo.toml`
- `cargo run --manifest-path spikes/gpui_foundation/Cargo.toml`

The current spike includes a basic app shell, Explorer-like list, custom-painted panel, and focused text input field.

## Still Open

- Manual verification of text selection, paste/cut/copy, IME composition, dictation/Wispr-style input, and command-key editing behavior.
- Terminal texture/surface bridging.
- Resize, scrolling, and high-frequency redraw measurements.

## Manual Text Input Test Pass

Run the spike, then use the focused prompt field:

```sh
cargo run --manifest-path spikes/gpui_foundation/Cargo.toml
```

Checklist:

- [ ] Type normal ASCII text.
- [ ] Type emoji and accented characters.
- [ ] Move by character with Left/Right.
- [ ] Select text with Shift+Left/Shift+Right.
- [ ] Select all with Cmd+A.
- [ ] Copy, cut, and paste with Cmd+C/Cmd+X/Cmd+V.
- [ ] Click to move the cursor.
- [ ] Drag to select text.
- [ ] Use macOS dictation or Wispr-style input.
- [ ] Use an IME/composition input method and confirm marked text appears correctly.
- [ ] Resize the window and confirm the input field remains usable.

Record failures as concrete observations: what action was taken, expected result, actual result, and whether the app froze, dropped input, or rendered incorrectly.
