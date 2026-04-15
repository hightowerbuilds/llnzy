# Changelog

All notable changes to llnzy will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- GPU-accelerated terminal rendering via wgpu and glyphon
- Full ANSI terminal emulation via alacritty_terminal
- PTY spawning with portable-pty
- Keyboard input encoding with full modifier support
- Mouse reporting (SGR and X10 protocols)
- Cursor rendering: block, beam, underline with configurable blink
- Scrollback history with mouse wheel and Shift+PageUp/Down
- Text selection: click-drag, double-click word, triple-click line
- Clipboard integration: Cmd+C, Cmd+V, Cmd+A
- Bold, italic, underline (single, double, curly, dotted, dashed), strikethrough
- Font fallback chain for emoji, CJK, and symbols
- Ligature support with toggle
- HiDPI/Retina display scaling
- 256-color palette and 24-bit true color support
- TOML configuration file with hot-reload (2-second poll)
- 5 built-in color schemes: Dracula, Nord, One Dark, Solarized Dark, Monokai
- Window opacity/transparency
- Configurable padding, line height, font size, font family
- Tabs: Cmd+T new, Cmd+W close, Cmd+1-9 switch, Cmd+]/[ cycle
- Split panes: Cmd+D vertical, Cmd+Shift+D horizontal, Cmd+Arrow focus
- In-terminal search: Cmd+F, incremental, case-insensitive, regex mode (Ctrl+R)
- URL detection with Cmd+click to open
- OSC sequence support: window title, clipboard, hyperlinks
- Visual bell on BEL character
- Bracketed paste mode
- Application cursor mode (DECCKM)
- Alternate screen buffer
- Shell exit detection with tab auto-close
- Fullscreen toggle (Cmd+Enter)
- Drag-and-drop file paths into terminal
- Window state persistence (size saved on close, restored on launch)
- Built-in diagnostics panel (Cmd+Shift+E) with timestamped log entries
- Performance: line-level dirty tracking, text buffer caching, adaptive frame rate
- Unit test suite (175 tests)
- Integration test suite (45 terminal emulation tests)
- PTY round-trip test suite (5 tests)
- CI/CD pipeline (GitHub Actions: build, test, clippy, rustfmt)
- Configuration reference documentation
- MIT license
