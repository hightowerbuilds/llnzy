# llnzy

A GPU-accelerated terminal emulator with visual effects, built from scratch in Rust.

## What it is

A native terminal emulator that runs your shell and renders everything through the GPU via wgpu. On top of standard terminal functionality, it has a visual effects engine — animated shader backgrounds, bloom/glow, compute-shader particles, CRT retro effects, and cursor glow — all configurable through a built-in settings panel.

It also includes a prompt queue manager called Stacker for saving and copying prompts.

## Status

This is an active personal project. It works as a daily terminal on macOS. There are no prebuilt binaries. Linux and Windows are untested. Things will break.

## Building

```sh
git clone https://github.com/hightowerbuilds/llnzy.git
cd llnzy
cargo run --release
```

Requires Rust 1.75+, a GPU that supports wgpu (Metal on macOS, Vulkan/DX12 on Linux/Windows).

## Features

- Full ANSI/VT100 terminal emulation (alacritty_terminal)
- GPU text rendering (glyphon/cosmic-text) with bold, italic, underline, strikethrough
- 256-color + 24-bit true color
- Tabs, split panes, scrollback, search (regex)
- TOML config with hot-reload
- Visual effects: smoke background, bloom/glow, GPU particles, CRT scanlines, cursor glow/trail
- 6 built-in themes: Minimalist, Cyberpunk, Retro, Deep Space, Synthwave, Forest
- Settings panel with real-time sliders and toggles
- Stacker: prompt queue manager with copy-to-clipboard
- Footer navigation: Shells / Stacker / Settings
- FPS overlay (Cmd+Shift+P)
- Effect kill switch (Cmd+Shift+F)

## Tech

| Layer | Crate |
|---|---|
| Window | winit 0.30 |
| GPU | wgpu 22 |
| Text | glyphon 0.6 |
| Terminal | alacritty_terminal 0.26 |
| PTY | portable-pty 0.8 |
| UI | egui 0.29 |
| Layout | taffy 0.7 |
| Clipboard | arboard |
| Config | serde + toml |

## Config

`~/Library/Application Support/llnzy/config.toml` on macOS. Changes apply within 2 seconds.

```toml
[effects]
enabled = true
background = "smoke"
background_intensity = 0.25
bloom_enabled = true
particles_enabled = true
particles_count = 800
cursor_glow = true
```

## Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| Cmd+T | New tab |
| Cmd+W | Close tab |
| Cmd+D | Split vertical |
| Cmd+Shift+D | Split horizontal |
| Cmd+F | Search |
| Cmd+Shift+F | Toggle all effects |
| Cmd+Shift+P | Toggle FPS overlay |
| Cmd+Enter | Fullscreen |

## License

[MIT](LICENSE)
