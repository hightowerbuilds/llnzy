# Crossover Compatibility

Status: future roadmap

Source notes:
- `daily-growth/roadmaps/old/laundry-list.md`
- `daily-growth/roadmaps/old/cross-platform-compatibility-roadmap-05-04-2026.md`
- `daily-growth/roadmaps/old/cold-storage/future-roadmap.md`
- `daily-growth/roadmaps/old/cold-storage/platform-boundary-architecture-05-04-2026.md`

## Goal

Make LLNZY a credible desktop app on macOS, Windows, and Linux without splitting the product into three separate versions. The workspace, tabs, editor, Stacker, Sketcher, Git tools, settings, and appearance system should feel shared across platforms, while terminal hosting, process execution, paths, packaging, and OS integration stay platform-native.

## Compatibility Promise

- [ ] Define which parts of LLNZY are expected to behave the same on macOS, Windows, and Linux.
- [ ] Document which behaviors are intentionally platform-specific.
- [ ] Keep project files portable enough that a repository opened on one OS remains recognizable on another OS.
- [ ] Preserve tab intent, theme choices, layout state, editor state, and sketch/stacker documents across operating systems when possible.
- [ ] Skip or recover gracefully from unavailable absolute paths instead of breaking the whole workspace.
- [ ] Decide support levels for each operating system, such as supported, preview, experimental, or unsupported.

## Platform Boundary

- [ ] Keep OS-specific behavior isolated behind platform services.
- [ ] Create a shell profile model with platform, profile name, executable path, args, cwd, environment, PTY host, and task policy.
- [ ] Normalize path handling for Unix paths, Windows drive-letter paths, UNC paths, slash-normalized paths, and `file:line:column` references.
- [ ] Treat WSL as an explicit terminal profile instead of an invisible fallback.
- [ ] Decide app identity and install locations for config, data, cache, logs, and temporary files on each OS.
- [ ] Add platform clipboard support for richer data if workflows require rich text, images, or file lists.
- [ ] Verify native dialogs, drag and drop, file drops, clipboard, and IME behavior on each platform.

## Terminal And Shell Hosts

- [ ] Document macOS and Linux as Unix PTY platforms.
- [ ] Document Windows as a ConPTY platform.
- [ ] Validate macOS shell startup across zsh, bash, and fish.
- [ ] Validate Linux shell startup across bash, zsh, and fish.
- [ ] Validate Windows shell startup across PowerShell 7, Windows PowerShell, cmd, Git Bash, WSL, and custom profiles.
- [ ] Document shell differences by OS, including default shell discovery, login shell behavior, control sequences, signal behavior, path syntax, encoding, and terminal profile behavior.
- [ ] Decide Windows command execution policy for `.exe`, `.cmd`, `.bat`, PowerShell scripts, and structured tasks.
- [ ] Make terminal failures visible with useful diagnostics instead of silent fallback behavior.

## Rendering And Visual Parity

- [ ] Define rendering backend policy: Metal on macOS, DX12 on Windows, Vulkan on Linux, with fallback behavior where needed.
- [ ] Add renderer diagnostics that show backend, adapter, OS, driver hints, scale factor, and fallback state.
- [ ] Keep the visual model shared across platforms, allowing for small font rendering differences.
- [ ] Add visual smoke coverage for startup, Home, ANSI terminal colors, syntax highlighting, joined panes, Stacker/editor layouts, Sketcher, settings, appearances, resize behavior, high DPI, and Linux Wayland/X11.
- [ ] Verify that terminal appearances, background images, CRT effects, and opacity settings survive packaged builds on each OS.

## Packaging And Distribution

- [ ] Add macOS signing and notarization.
- [ ] Decide Apple Silicon and Intel support policy.
- [ ] Add macOS release builds through GitHub Releases, starting with universal binaries if practical.
- [ ] Create a Homebrew formula when macOS distribution is stable enough.
- [ ] Add Windows code signing.
- [ ] Choose Windows installer and portable distribution formats.
- [ ] Decide Linux distribution formats such as AppImage, deb, rpm, or Flatpak.
- [ ] Add Linux `.desktop` integration.
- [ ] Verify Linux behavior under GNOME and KDE.
- [ ] Plan an auto-update mechanism after signing and packaging decisions are settled.

## Test Matrix

- [ ] Add CI checks for macOS, Windows, and Linux.
- [ ] Run Rust unit tests and integration tests on all supported platforms.
- [ ] Add path normalization tests for Windows, Unix, UNC, and `file:line:column` paths.
- [ ] Add PTY and ConPTY smoke tests where automation is practical.
- [ ] Add packaged-app smoke tests for permissions, bundled assets, terminal appearances, file explorer updates, drag and drop, and exported files.
- [ ] Build a manual verification pass for Windows 10, Windows 11, Ubuntu LTS, Fedora, GNOME, KDE, Apple Silicon macOS, and Intel macOS if Intel remains supported.

## Rough Phases

### Phase 1: Contract And Boundaries

- [ ] Write the public compatibility promise.
- [ ] Lock down platform service boundaries.
- [ ] Define shell profiles and path normalization rules.
- [ ] Decide supported, preview, and experimental platform labels.

### Phase 2: Windows Terminal And Paths

- [ ] Spike ConPTY hosting.
- [ ] Validate PowerShell, cmd, Git Bash, WSL, and custom profiles.
- [ ] Implement Windows path normalization and command execution rules.
- [ ] Prove that workspace state, tabs, editor files, and settings survive a Windows run.

### Phase 3: Linux Desktop Behavior

- [ ] Validate Unix PTY behavior on common Linux shells.
- [ ] Test Wayland and X11 rendering behavior.
- [ ] Verify clipboard, dialogs, drag and drop, IME, and file explorer behavior under GNOME and KDE.
- [ ] Choose initial Linux package format.

### Phase 4: Packaging

- [ ] Finish macOS signing and notarization.
- [ ] Add Windows signing and installer packaging.
- [ ] Add Linux package generation.
- [ ] Publish release artifacts through GitHub Releases.

### Phase 5: Release Confidence

- [ ] Build visual smoke coverage across supported platforms.
- [ ] Add diagnostics for terminal, renderer, shell, path, and packaging failures.
- [ ] Document known limitations.
- [ ] Decide when Windows and Linux move from preview to supported.
