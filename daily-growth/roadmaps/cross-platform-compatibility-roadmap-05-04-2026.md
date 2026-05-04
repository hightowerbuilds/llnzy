# Cross-Platform Compatibility Roadmap
## May 4, 2026

## Purpose

This document defines how llnzy should become a credible cross-platform desktop
application across macOS, Windows, and Linux. It is intentionally not an
implementation plan for one narrow bug. It is the foundation for a platform
strategy: what we will build, where the operating systems differ, how we will
test those differences, how we will package each version, and how we will
explain the product honestly to users.

The core product should stay the same everywhere: a GPU-accelerated native
workspace that combines terminal tabs, code editing, local Git inspection,
project navigation, saved workspaces, visual effects, Stacker, and Sketch.
However, terminal applications live close to the operating system. A terminal
emulator is not only a window with text. It is a process host, a PTY or console
host, an input translation layer, a filesystem-facing project tool, an LSP
launcher, a shell environment, and a packaged desktop app. Those pieces are not
identical across macOS, Linux, and Windows.

The right goal is not to pretend that all versions are identical. The right
goal is to make the cross-platform contract explicit:

- The editor, renderer, themes, project navigation, Stacker, Sketch, local Git
  dashboard, and workspace model should have near-identical behavior across all
  supported desktop platforms.
- The terminal host should feel native to each platform while exposing the same
  llnzy-level concepts: tabs, split/joined panes, scrollback, search, mouse
  reporting, selection, process lifecycle, cwd tracking where available, and
  task execution.
- macOS and Linux should share the Unix PTY model. They will differ mostly in
  packaging, desktop integration, shell defaults, font discovery, and window
  manager behavior.
- Windows should use Windows-native console hosting through ConPTY, with a
  Windows-specific shell policy, path model, process-control behavior, and user
  documentation.
- The public website, README, onboarding, release notes, and in-app labels
  should explain the product as "same workspace, native terminal behavior per
  OS" instead of burying platform caveats in support threads.

## Current Starting Point

llnzy is already structured in a way that makes cross-platform work realistic.
The application is Rust, not a macOS-only Swift/AppKit app. The windowing layer
uses `winit`, rendering uses `wgpu`, the UI overlay uses `egui`, terminal
emulation is based on `alacritty_terminal`, PTY/process hosting is isolated
behind `src/pty.rs`, and the app's primary product state is expressed as
workspace tabs rather than platform-specific window objects.

The main platform-sensitive areas in the current codebase are:

- `src/pty.rs`: spawns a shell through `portable-pty`, starts reader/writer
  threads, resizes the PTY, writes bytes, and kills the child process.
- `src/session.rs`: bridges PTY output into terminal emulation, tracks title,
  cwd, process id, and exited state.
- `src/terminal.rs`: wraps `alacritty_terminal` for VT processing, scrollback,
  selection, colors, hyperlinks, and terminal events.
- `src/main.rs`: owns winit event routing, terminal input routing, native file
  drop, mouse reporting, macOS-only menu/text bridge hooks, and app lifecycle.
- `src/menu.rs` and `src/macos_text_bridge.rs`: macOS-specific native
  integration.
- `src/config.rs`, `src/theme_store.rs`, `src/workspace_store.rs`, and related
  persistence modules: currently use user config/data paths that should be
  audited per OS.
- `bundle.sh` and `assets/Info.plist`: macOS packaging exists; Windows and
  Linux packaging do not appear to be equally established.

The most important architectural observation is that we do not need to rewrite
the app. We need to harden the platform boundary. The product surface should
continue to call into a small set of platform services: shell discovery,
terminal host spawning, process lifecycle, open-url/open-file, clipboard,
native menu capabilities, config/data path resolution, font discovery, and
packaging metadata.

## Research Baseline

The cross-platform plan rests on a few external facts:

- `winit` targets desktop Windows, macOS, and Unix through X11 and Wayland, but
  its project scope is window creation and input handling, not drawing, native
  menus, or a complete platform abstraction. Source:
  https://docs.rs/crate/winit/latest/source/FEATURES.md
- `wgpu` exposes platform GPU backends that match our target operating systems:
  Metal on macOS, DX12 on Windows, Vulkan on Windows/Linux, and GL/GLES as a
  secondary backend. Source: https://docs.rs/wgpu/latest/wgpu/
- Windows terminal hosting should be treated as ConPTY hosting, not POSIX PTY
  hosting. Microsoft describes pseudoconsoles as the mechanism that lets a
  terminal app host character-mode applications without the default console
  window, with UTF-8/VT crossing the pseudoconsole channel. Sources:
  https://learn.microsoft.com/en-us/windows/console/pseudoconsoles and
  https://devblogs.microsoft.com/commandline/windows-command-line-introducing-the-windows-pseudo-console-conpty/
- Rust has first-class targets for common macOS, Windows, and Linux desktop
  targets, but the practical build toolchain differs by OS. Windows MSVC targets
  require Windows tooling for fully supported Windows builds. Sources:
  https://doc.rust-lang.org/rustc/platform-support.html and
  https://doc.rust-lang.org/stable/rustc/platform-support/windows-msvc.html
- macOS distribution outside the App Store requires Developer ID signing and
  notarization for a smooth Gatekeeper experience. Sources:
  https://developer.apple.com/support/developer-id/ and
  https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution
- Windows has multiple packaging choices. MSIX is the modern Windows package
  format, but packaged desktop apps have filesystem/install-location behavior
  that must be understood before we choose it as the default. Sources:
  https://learn.microsoft.com/en-us/windows/msix/ and
  https://learn.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-behind-the-scenes
- Linux desktop integration should follow freedesktop `.desktop` conventions,
  and sandboxed Flatpak distribution would require an explicit permissions
  strategy. Sources:
  https://specifications.freedesktop.org/desktop-entry-spec/latest/ and
  https://docs.flatpak.org/en/latest/sandbox-permissions.html

## Product Compatibility Contract

Before implementation, we should define what "compatible" means. Without a
contract, every platform-specific edge case becomes a debate about whether the
app is broken or simply native.

The compatibility contract should be:

1. A project opened on one platform should be recognizable on another. Saved
   workspaces may contain platform-specific paths, but the data model should be
   able to skip unavailable paths, show clear restore messages, and preserve
   theme/tab intent wherever possible.
2. The visual model should be the same. The same theme should mean the same
   colors, effects, typography intent, tab bar model, joined panes, sidebar,
   editor layout, and Git dashboard hierarchy. Small font rendering differences
   are acceptable; layout collapse or missing surfaces are not.
3. The terminal model should be the same at the app level but native at the OS
   level. A llnzy terminal tab always has a shell process, a grid size, input
   encoding, output stream, scrollback, selection, search, title, cwd when
   detectable, and process lifecycle. The underlying implementation differs:
   Unix PTY on macOS/Linux, ConPTY on Windows.
4. The editor model should be cross-platform. Rope editing, syntax
   highlighting, LSP, diagnostics, search, formatting, file watching, project
   search, snippets, markdown preview, and git gutter should work on all
   supported operating systems, with language-server availability depending on
   what the user has installed.
5. Build tasks should be cross-platform in detection but platform-native in
   execution. Cargo, npm, pnpm, yarn, make, Go, and Python tasks may be present
   everywhere, but shells, path separators, executable extensions, and process
   launch behavior differ.
6. Distribution should be native enough that the user does not feel like they
   are running a development artifact. macOS gets a signed/notarized app bundle
   and DMG. Windows gets a signed installer or package plus a portable option
   only if we can support it. Linux gets at least one portable binary format and
   one distro-friendly integration path.

## Platform Architecture We Should Build

The practical implementation should introduce a small "platform services"
boundary. The current code already has platform-specific modules in places, but
we should make that boundary explicit enough that new platform behavior does
not spread throughout `main.rs`.

The boundary should cover:

- `PlatformShell`: default shell detection, shell login/interactive flags,
  environment preparation, command execution for tasks, and display name.
- `TerminalHost`: spawn terminal process, read output, write input, resize,
  query process id, terminate, and report exit.
- `PlatformPaths`: config directory, data directory, cache directory, themes,
  workspaces, logs, crash reports, and exported assets.
- `PlatformOpen`: open URL, reveal file, open project folder, and open file
  with system default app.
- `PlatformClipboard`: if `arboard` remains sufficient, this can be thin; if
  not, this boundary absorbs platform-specific fallback behavior.
- `PlatformMenu`: native menu support where available, egui menu fallback where
  native menus are absent or incomplete.
- `PlatformInput`: keyboard modifier names, IME behavior, dead keys, compose
  input, AltGr, and special terminal key encoding.
- `PlatformPackagingMetadata`: app identifier, executable name, icon resources,
  file associations, protocol handlers, signing identity, and update channel.

This boundary should not become an abstract "everything" layer. It should only
own things whose behavior truly differs by OS. The rest of the app should stay
plain Rust modules that operate on product concepts: tabs, sessions, buffers,
projects, snapshots, themes, and UI commands.

## Rendering and Windowing Plan

Rendering is the best-positioned part of the current stack. `wgpu` gives us a
shared rendering API over Metal, DX12, Vulkan, and GL/GLES. `winit` gives us a
shared event loop and window abstraction over Windows, macOS, X11, and Wayland.
That does not mean the renderer is automatically done. It means the platform
work is about validation, adapter selection, fallback policy, and window-system
edge cases rather than rewriting the renderer.

We should build the renderer policy in three layers.

First, define preferred backend order per OS:

- macOS: Metal only for supported builds. We should not plan a Vulkan
  portability story unless there is a specific reason; Metal is the native path.
- Windows: DX12 primary, Vulkan fallback if available and useful, GL/GLES only
  as a last-resort debug path.
- Linux: Vulkan primary, GL/GLES fallback. Wayland and X11 should both be
  tested because users will encounter both.

Second, expose a diagnostic surface. If GPU initialization fails, the app should
not simply crash or print an opaque error. It should produce a user-readable
failure explaining the selected backend, adapter candidates if available, OS,
driver hint, and suggested next step. For development builds this can be a log.
For packaged builds it should become a small native or minimal fallback dialog
if possible.

Third, build a render validation matrix. We need screenshots or pixel-level
smoke tests on each platform for:

- startup/home view
- terminal output with ANSI colors
- editor with syntax highlighting
- joined terminal panes
- terminal plus editor joined layout
- settings/appearances views
- visual effects enabled and disabled
- high DPI scaling
- window resize
- transparency disabled and enabled where supported

User-facing promise: visual features are cross-platform, but GPU backend and
driver quality may vary. The docs should say "Metal on macOS, DirectX 12 on
Windows, Vulkan on Linux" in the technical requirements section, not in the
marketing headline.

## Terminal Hosting Plan

Terminal hosting is the most important platform split.

### macOS and Linux

macOS and Linux both use Unix-style PTYs. The app creates a pseudoterminal,
spawns the user's shell attached to the slave side, reads bytes from the master,
writes input bytes to the master, and resizes the PTY when the terminal grid
changes. This is the model the current code already assumes. It aligns with how
shells, TUIs, terminal control sequences, job control, signals, and environment
variables work on Unix systems.

The Unix host should support:

- default shell from `$SHELL`, with fallback to `/bin/zsh` on macOS and
  `/bin/sh` or detected login shell on Linux
- login-shell behavior when starting an interactive terminal tab
- cwd inheritance from the active terminal's OSC 7 cwd, from active project
  root, or from last known workspace root
- PTY resize on pane resize and DPI/cell-size changes
- signal behavior through PTY input where possible, such as Ctrl-C as ETX
- process termination through the PTY child handle
- UTF-8 output with graceful handling for invalid bytes
- environment variables: `TERM=xterm-256color`, `COLORTERM=truecolor`, and
  potentially `TERM_PROGRAM=llnzy`
- OSC 7 working-directory tracking and title tracking

The Linux host should also be tested against common shells and distributions:

- bash, zsh, fish
- Ubuntu LTS or equivalent glibc baseline
- Fedora or another modern Wayland-first desktop
- Arch or rolling distro if we want early driver/windowing signal
- GNOME Wayland, KDE Wayland, and at least one X11 session

The macOS host should be tested against:

- zsh default shell
- bash users
- fish users
- Apple Silicon and Intel builds if we distribute both or universal binaries
- IME text input
- dead-key text input
- native menu command routing
- Gatekeeper launch after notarization

### Windows

Windows is different enough that we should document it as a separate terminal
host. We should not describe Windows as "Unix PTY but with backslashes." Windows
console applications historically use Win32 Console APIs as well as VT output.
ConPTY is the native bridge that lets a terminal app host those applications
and receive UTF-8/VT streams. The llnzy Windows terminal host must be built and
tested as a ConPTY host, whether we rely on `portable-pty`'s Windows backend or
eventually replace that layer with a dedicated implementation.

The Windows host should support:

- default shell detection with a clear policy: PowerShell 7 if installed,
  Windows PowerShell as fallback, then `cmd.exe`; optionally expose a setting
  for Command Prompt, PowerShell, Git Bash, WSL, or custom shell
- ConPTY process creation and resize
- UTF-8 input/output boundary
- Windows path display and path parsing, including drive-letter paths,
  UNC paths, and `file:line:column` patterns where possible
- process termination using Windows process handles, with a user-facing
  distinction between "terminate process" and shell-level exit
- environment setup for terminal apps, including `TERM`, `COLORTERM`, and
  Windows-specific variables only when they help rather than confuse tools
- clipboard integration compatible with Windows conventions
- key handling for Ctrl, Alt, Windows key, AltGr, function keys, and terminal
  application modes
- mouse reporting through the same terminal-level API the Unix host uses
- WSL launch as an explicit shell profile, not an invisible emulation layer

The Windows host should not promise perfect Unix parity. Differences we should
expect and document:

- Shell startup files differ. PowerShell profiles are not `.zshrc` or
  `.bashrc`.
- Paths differ. `C:\Users\name\project` is not `/Users/name/project`.
- Some CLI applications use Win32 Console API behavior internally and rely on
  ConPTY translation.
- Signal semantics differ. Ctrl-C should work for normal console apps, but the
  underlying mechanism is not POSIX signals.
- Executable lookup differs. `.exe`, `.cmd`, `.bat`, and PowerShell scripts have
  Windows-specific execution rules.
- ANSI/VT support depends on the console application and ConPTY path, although
  modern Windows tooling is broadly VT-capable.
- Shell quoting and task execution differ sharply between PowerShell and
  POSIX-style shells.

The user-facing promise for Windows should be: "llnzy for Windows is native
Windows terminal hosting, not a Linux emulator. It works best with PowerShell,
Command Prompt, Windows-native developer tools, and WSL profiles when you choose
them."

## Shell Profiles and Task Execution

We should add a shell-profile model before trying to make Windows feel complete.
Right now the config has a single shell program. Cross-platform support needs a
more explicit shape:

```toml
[[shells]]
name = "PowerShell"
platform = "windows"
program = "pwsh.exe"
args = ["-NoLogo"]
default = true

[[shells]]
name = "zsh"
platform = "macos"
program = "/bin/zsh"
args = ["-l"]
default = true

[[shells]]
name = "bash"
platform = "linux"
program = "/bin/bash"
args = ["-l"]
default = true
```

We do not need to implement exactly this syntax, but the data model should
support equivalent concepts: profile name, platform, program, args, cwd policy,
environment additions, default flag, and whether the profile is interactive or
task-oriented.

Task execution should not blindly concatenate strings into whatever shell
happens to be active. For detected tasks, we already have `command`, `args`, and
`cwd`. That is good. The cross-platform direction should preserve structured
command execution as long as possible. When a shell is needed, the platform
service should choose the right invocation:

- Unix interactive task tab: shell profile may execute `command args` in the
  user's login shell only if shell features are needed.
- Unix direct task: spawn `command` with `args` and cwd, attached to PTY.
- Windows direct task: spawn `.exe`, `.cmd`, or `.bat` according to Windows
  process rules. For `.ps1`, route through PowerShell explicitly.
- Windows shell task: use PowerShell or cmd profile intentionally, with correct
  quoting.

User-facing docs should say that detected tasks are run with platform-native
process rules. A Cargo task should feel like Cargo on every platform. A custom
shell snippet may need per-platform variants.

## Input, Keyboard, and IME

Input is another area where "same UI" is not enough. Terminal users expect
their keyboard to behave like the host OS.

We should define platform input requirements:

- macOS: Command is the application shortcut modifier; Control should be passed
  to terminal/editor commands where expected; Option may be text composition or
  Alt depending on setting; native menu shortcuts should match macOS
  conventions.
- Windows: Control is the primary shortcut modifier; Alt activates terminal
  meta behavior or app accelerators depending on focus; AltGr must not be
  mistaken for Ctrl+Alt shortcuts in text entry contexts.
- Linux: Control is the primary shortcut modifier; Alt and Super vary by window
  manager; compose/dead keys and IME behavior must be tested under Wayland and
  X11.

The code already contains macOS-specific text bridge work for Stacker. That is
a signal that text input cannot be treated as fully abstract. We should keep
platform-specific text handling behind `PlatformInput` or a closely related
module, with tests/manual scripts for:

- regular text typing
- dead keys
- emoji/symbol input
- CJK IME composition
- paste into terminal
- paste into editor
- paste into Stacker
- terminal control sequences
- keyboard shortcuts when terminal, editor, Stacker, and command palette are
  focused

The user-facing docs should avoid overwhelming users with implementation
details. We should state modifier conventions plainly:

- macOS uses Command for app shortcuts.
- Windows and Linux use Control for app shortcuts.
- Terminal control shortcuts are passed through when terminal focus requires
  them.

## Filesystem, Paths, and Workspace Restore

The workspace model needs a cross-platform path strategy. A saved workspace can
contain a project path and code file paths. Those paths are inherently
platform-specific. We should not promise that a workspace saved on macOS will
open the exact same directory on Windows unless the user has configured a path
mapping.

We should build three layers:

1. Native path persistence. Save the path exactly as selected on that OS.
2. Graceful restore. If the path is missing, skip it, show a clear warning, and
   preserve the rest of the session. The current session-restore plan already
   moves in this direction.
3. Optional path mappings later. For teams that sync workspaces across OSes,
   allow mappings such as `/Users/luke/code` to `C:\Users\Luke\code` or
   `/home/luke/code`. This should be a later feature, not a blocker for initial
   compatibility.

Path parsing must also become platform-aware:

- Unix absolute paths: `/home/me/project/src/main.rs:10:2`
- macOS home paths: `~/project/src/main.rs`
- Windows drive paths: `C:\Users\me\project\src\main.rs:10:2`
- Windows slash-normalized paths from tools: `C:/Users/me/project/src/main.rs`
- UNC paths: `\\server\share\project\file.rs`
- WSL paths when using a WSL shell: `/mnt/c/Users/me/project` and Linux paths
  inside the WSL filesystem

We should be careful with WSL. A Windows llnzy process cannot treat every WSL
path as a normal Windows path. We need an explicit translation story if we want
Cmd-click file opening from WSL output to open Windows-side files. The minimal
first version can document that WSL path opening is limited. A stronger version
can call `wslpath` through the WSL profile or implement a path translation layer
for `/mnt/<drive>/...`.

## LSP and Developer Tooling

The LSP layer is cross-platform in protocol terms but platform-specific in
server discovery. `rust-analyzer`, `typescript-language-server`, `pyright`,
`gopls`, and others are found through PATH or configured command names. PATH
discovery differs by OS and packaging model.

We should build:

- a platform-aware PATH initialization policy
- a user-visible LSP status that distinguishes "server unsupported",
  "server command not found", "server starting", "server running", and "server
  stopped"
- per-language command overrides in config
- documentation that language features require language servers installed on
  the user's machine
- Windows-specific guidance for installing language servers through `rustup`,
  npm, Go, Python, or package managers
- macOS guidance for PATH differences between GUI apps and terminal shells
- Linux Flatpak guidance if sandboxing limits access to host language servers

macOS deserves special attention here. GUI apps launched from Finder often do
not inherit the same PATH as a terminal shell. If llnzy is a developer tool, it
must either initialize PATH from a login shell, allow explicit tool paths, or
both. This is not just a macOS problem, but macOS users hit it frequently.

Linux sandboxed packaging also deserves attention. A Flatpak build may not be
able to freely execute host language servers or inspect arbitrary project files
unless permissions are broad. That might make Flatpak a poor first packaging
target for a terminal/editor unless we choose broad filesystem/process
permissions and explain them. An AppImage or distro package may be a better
first Linux developer-tool distribution.

## Git Integration

The Git dashboard is local-only and shells out to `git`. This is good for
cross-platform privacy and avoids GitHub/GitLab account complexity. It also
means the app must find `git` on each OS.

Platform work:

- macOS: find `/usr/bin/git`, Xcode command line tools git, Homebrew git, and
  user PATH git. If Git is missing, show a specific install hint.
- Windows: find Git for Windows (`git.exe`) in PATH, common install locations,
  or user config. Handle CRLF paths and quoting. If Git is missing, point users
  to Git for Windows or their preferred package manager.
- Linux: rely mostly on PATH/package manager installation. Show distro-neutral
  "install git with your package manager" language.

The user-facing docs should state that Git features are local repository
inspection: branch, status, commits, stashes, reflog, and commit patches. They
do not require remote hosting accounts and do not perform network fetch/push.

## Native Menus, App Chrome, and Desktop Integration

macOS expects a native menu bar. The current code already has macOS menu work.
That should remain native because users expect app-level menu items such as
File, Edit, View, Window, and Help to live in the system menu. macOS also needs
proper app bundle metadata, icon, document/window behavior, signing,
notarization, and Gatekeeper-friendly distribution.

Windows expects an app window with native window controls, taskbar identity,
Start menu entries, installer registration, uninstall behavior, and standard
keyboard shortcuts. We do not necessarily need a native Win32 menu bar if the
egui command palette and app chrome provide the product commands, but we need
to decide intentionally. If we do not provide a native Windows menu, docs should
not mention one.

Linux varies by desktop. Some environments still show app menus, some do not.
We should rely on in-window UI, command palette, and `.desktop` integration.
The `.desktop` file should provide the app name, icon, executable, categories,
and optional file/project open behavior according to freedesktop conventions.

The rule should be:

- Native menu on macOS.
- In-window command system everywhere.
- Platform installer metadata everywhere.
- No feature should exist only in a native menu. If macOS has `View > Toggle
  Word Wrap`, Windows and Linux should expose the same command through the
  command palette or in-app menu/shortcut.

## Packaging and Distribution

### macOS

macOS distribution should produce:

- `.app` bundle
- signed executable and nested resources
- hardened runtime if required for notarization
- notarized app
- stapled ticket where applicable
- DMG for drag-install distribution
- optionally a zip for update systems, only if Gatekeeper behavior remains good
- universal binary if we choose to support both Intel and Apple Silicon in one
  package, or separate `arm64` and `x86_64` builds if size/build complexity
  argues for separation

Minimum macOS version should be declared in docs and enforced in build metadata
if possible. Since `wgpu` Metal support and Rust target support are strong
here, the main blockers are packaging, signing, PATH/tool discovery, and native
input/menu polish.

### Windows

Windows distribution should produce:

- signed executable
- installer with Start menu shortcut, uninstall entry, icon, and optional PATH
  integration if we add CLI helpers later
- MSIX investigation, but not necessarily MSIX as first default
- portable `.zip` only if settings/log paths and auto-update behavior are clear
- crash/log location documented
- Windows Defender/SmartScreen reputation strategy through code signing and
  consistent release identity

We should evaluate MSI/NSIS/Wix versus MSIX. MSIX has modern installation and
update advantages, but packaged desktop apps have install-location and
filesystem behavior that may surprise a developer tool if not planned. A
traditional installer may be simpler for early Windows users, especially if the
app needs to spawn arbitrary shells and tools from user projects.

Minimum Windows version should likely be Windows 10 or newer because ConPTY is
the realistic terminal-hosting baseline and current Rust Windows MSVC target
requirements align with modern Windows. We should not spend time supporting
older Windows unless there is a strong user need.

### Linux

Linux distribution should be staged:

1. Tarball/AppImage-style portable build for early testers.
2. `.deb` for Ubuntu/Debian-family users if we want the most common developer
   desktop path.
3. `.rpm` for Fedora/openSUSE-family users if adoption justifies it.
4. Flatpak only after we decide how to handle filesystem, terminal spawning,
   language servers, and developer-tool sandbox permissions.

Linux packaging must include:

- binary
- icon resources
- `.desktop` file
- app metadata if distributing through stores/repositories
- dependency strategy for graphics/runtime libraries
- tested behavior on Wayland and X11
- clear GPU backend requirements

Flatpak is attractive for user installation but difficult for a terminal/editor
that needs broad access to user projects and host developer tools. If we ship a
Flatpak, we should present it as the sandboxed edition and document any
limitations around shells, PATH, project filesystem access, external tools, and
language servers.

## Testing Strategy

Cross-platform compatibility should be driven by a test matrix, not by one
developer manually launching the app occasionally.

### Automated Tests

We should add CI jobs for:

- `cargo check` on macOS, Windows, and Linux
- `cargo test --lib` on macOS, Windows, and Linux
- terminal parsing tests everywhere
- editor buffer/search/syntax tests everywhere
- tab grouping/layout tests everywhere
- workspace/session restore tests everywhere
- Git parser tests without requiring a live remote
- platform path parser tests for Unix, Windows, UNC, and WSL-like paths

PTY integration tests need platform handling. Some can run everywhere; some may
need OS-specific shells and should be conditionally compiled or conditionally
skipped with clear names.

### Manual Smoke Tests

Every release candidate should be manually tested on:

- macOS Apple Silicon
- macOS Intel if supported
- Windows 10 or Windows 11 x64
- Windows ARM64 if we decide to support it
- Ubuntu LTS or equivalent
- Fedora/GNOME Wayland or equivalent
- KDE Wayland or X11 if feasible

Manual scenarios:

- launch fresh install
- open project folder
- create terminal tab
- run shell command
- run TUI app
- resize window
- split/join terminal panes
- scroll active and inactive joined terminal panes
- copy/paste terminal selection
- open code file
- edit/save file
- trigger syntax highlighting
- start LSP when server is installed
- show LSP unavailable when server is absent
- run detected Cargo/npm task
- view Git dashboard in clean and dirty repo
- use command palette
- switch theme/effects
- close with unsaved file prompt
- restore last session
- drag/drop file or folder where supported

### Release Acceptance Levels

We should define support levels:

- Supported: tested in CI and manual release pass; bugs prioritized.
- Preview: builds and mostly works; known caveats; feedback requested.
- Experimental: hidden or separate download; not marketed as reliable.
- Unsupported: may compile, but no release promise.

Initial target should be:

- macOS: Supported
- Windows: Preview until terminal host, installer, PATH/LSP, and input polish
  are proven
- Linux: Preview with specific distros/desktops named; Supported after Wayland,
  X11, packaging, and GPU backend coverage are stable

## User Communication Strategy

The product messaging should be honest, positive, and specific. Users will
accept differences if they are framed as native platform behavior. Users will
not accept surprise incompatibilities after installation.

The core phrase should be:

> llnzy is the same workspace across macOS, Windows, and Linux, with native
> terminal hosting on each operating system.

That phrase gives us room to explain why PowerShell on Windows does not behave
like zsh on macOS, without making Windows sound like a lesser port.

### Platform Comparison Page

We should create a public "Platforms" or "Downloads" page with a comparison
table:

| Area | macOS | Windows | Linux |
|---|---|---|---|
| Status | Supported | Preview/Supported when ready | Preview/Supported by distro |
| Terminal host | Unix PTY | Windows ConPTY | Unix PTY |
| Default shell | zsh or user shell | PowerShell/cmd/custom | user shell |
| GPU backend | Metal | DX12 primary | Vulkan primary |
| Package | signed/notarized app + DMG | signed installer/package | AppImage/deb/rpm/Flatpak as available |
| App shortcuts | Command-based | Control-based | Control-based |
| Git | local git required | Git for Windows recommended | system git required |
| LSP | installed language servers | installed language servers | installed language servers |
| Known differences | Finder PATH considerations | Windows path/shell semantics | Wayland/X11 and distro packaging |

The table should be concise. Each row should link to deeper docs.

### Download Labels

Downloads should be labeled by support level:

- "macOS - Stable"
- "Windows - Preview" or "Windows - Stable"
- "Linux - Preview: AppImage"
- "Linux - Debian/Ubuntu package"
- "Linux - Flatpak sandboxed edition" if/when applicable

Avoid vague labels like "beta" unless we define what beta means. "Preview"
communicates that the app is usable but still being hardened.

### In-App Onboarding

On first run, the app should show platform-specific setup hints only when
needed:

- macOS: "If language servers are not found, set tool paths in Settings." Do
  not show unless an LSP command is missing.
- Windows: "Choose your default shell: PowerShell, Command Prompt, Git Bash,
  WSL, or custom." This should be an actual setup control, not a wall of text.
- Linux: "Running under Wayland/X11" and GPU backend diagnostics should be
  available in About/Diagnostics, not front-and-center unless there is a
  failure.

### Documentation Pages

We should maintain these docs:

1. `Installing on macOS`
   - supported macOS versions
   - Apple Silicon/Intel download choice
   - Gatekeeper/notarization reassurance
   - default shell
   - PATH and language server notes

2. `Installing on Windows`
   - supported Windows versions
   - installer versus portable
   - PowerShell/cmd/Git Bash/WSL shell profiles
   - Git for Windows
   - path differences
   - terminal caveats around ConPTY and legacy console apps

3. `Installing on Linux`
   - supported distros/desktops
   - AppImage/deb/rpm/Flatpak differences
   - Wayland/X11
   - GPU backend/drivers
   - system package dependencies
   - shell and language server discovery

4. `Platform Differences`
   - terminal host model
   - shortcuts
   - paths
   - shell profiles
   - packaging/sandboxing
   - known limitations and workarounds

5. `Troubleshooting`
   - GPU initialization failure
   - shell fails to start
   - language server not found
   - Git not found
   - project paths fail to restore
   - copy/paste or IME issues

### Release Notes

Release notes should include a platform section every time:

- macOS changes
- Windows changes
- Linux changes
- Cross-platform changes
- Known platform-specific issues

If Windows terminal behavior changes, say so in the Windows section. If Linux
Wayland behavior changes, say so in the Linux section. Users should not have to
infer platform impact from commit messages.

## Roadmap

### Phase 1: Platform Boundary and Audit

Goal: make platform-specific behavior visible and controlled.

Work:

- Inventory all `cfg(target_os = "...")` usage.
- Define `platform` module boundaries for shell, terminal host, paths, open,
  menu, and input.
- Keep existing behavior intact while moving calls behind those boundaries.
- Add path parsing tests for macOS/Linux/Windows examples.
- Add diagnostics for OS, renderer backend, shell profile, config path, data
  path, and Git/LSP command lookup.

Exit criteria:

- The app still works on macOS.
- Platform services have clear APIs.
- The Windows/Linux work has a place to land without spreading conditionals
  through product code.

### Phase 2: Windows Terminal Preview

Goal: produce a Windows build that launches, renders, opens a shell, and runs
basic developer workflows.

Work:

- Validate `portable-pty` Windows behavior with ConPTY.
- Add Windows shell profile detection.
- Add Windows path parsing.
- Add Windows process termination/restart behavior.
- Add Windows modifier/input tests.
- Add Git for Windows detection.
- Add LSP command discovery and user override path.
- Build signed or unsigned internal installer for testing.

Exit criteria:

- PowerShell tab works.
- Command Prompt tab works.
- Git Bash or WSL profile works if explicitly configured.
- Terminal scrollback/search/selection work.
- Editor open/edit/save works.
- Git dashboard works when Git is installed.
- At least one Cargo or npm task works in a terminal tab.

### Phase 3: Linux Preview

Goal: produce a Linux build that works on a defined distro/windowing baseline.

Work:

- Validate Wayland and X11 startup.
- Validate Vulkan and GL fallback.
- Define Linux shell detection.
- Add `.desktop` metadata and icon installation.
- Build AppImage or tarball preview.
- Test on Ubuntu LTS and Fedora/GNOME Wayland.
- Validate file dialogs, drag/drop, clipboard, and IME.
- Validate system Git and LSP discovery.

Exit criteria:

- App launches on target distro(s).
- Terminal works with bash/zsh/fish.
- Editor and Git dashboard work.
- GPU diagnostics identify backend.
- Packaging creates launcher integration.

### Phase 4: macOS Production Hardening

Goal: keep macOS as the polished reference platform while cross-platform work
expands.

Work:

- Finish signed/notarized release process.
- Decide universal versus per-arch binaries.
- Harden PATH/tool discovery.
- Expand IME/dead-key/manual input tests.
- Keep native menu commands mirrored in command palette.
- Document macOS-specific behavior clearly.

Exit criteria:

- User can download, install, and launch without Gatekeeper friction.
- Language server failures are explainable.
- macOS release remains stable while Windows/Linux previews evolve.

### Phase 5: Platform Support Graduation

Goal: move Windows and Linux from Preview to Supported.

Work:

- Add CI matrix for all target OSes.
- Add release smoke checklist per OS.
- Add crash/log collection path documentation.
- Build signed Windows installer.
- Build Linux package set.
- Close top platform-specific input/rendering/terminal issues.
- Publish platform comparison page.

Exit criteria:

- Windows and Linux release candidates pass defined smoke tests.
- Known differences are documented.
- Downloads page labels match actual support level.
- Bug triage has platform labels and severity rules.

## Known Differences We Should Embrace

The app should not hide these differences:

- Windows terminal tabs are ConPTY-hosted Windows console sessions. macOS and
  Linux terminal tabs are Unix PTY sessions.
- macOS uses Command for app shortcuts. Windows and Linux use Control.
- Shell profile startup files differ by OS and shell.
- Paths differ and saved workspaces may need path mapping across machines.
- Language servers are external tools and must be installed per OS.
- Git must be installed per OS.
- Linux packaging differs by distro and sandbox choice.
- Flatpak, if offered, may have different filesystem/tool access from AppImage
  or distro packages.
- Windows WSL profiles are useful but should be described as WSL profiles, not
  as the default Windows shell behavior.

These differences do not make one platform "lesser." They are the cost of
building a native terminal/editor instead of shipping a generic web wrapper.

## Final Product Positioning

The product should be described this way:

llnzy is a native GPU-accelerated terminal and code workspace for macOS,
Windows, and Linux. The workspace model, editor, local Git tools, themes,
Stacker, Sketch, and project navigation are shared across platforms. The
terminal layer is native to each operating system: Unix PTY on macOS/Linux and
Windows ConPTY on Windows. That gives users the shell and command-line behavior
they expect on their machine while preserving one llnzy workflow.

That positioning is technically accurate, easy to defend, and gives us a clean
roadmap. We are not promising impossible sameness. We are promising a coherent
product with native platform behavior and clear documentation.
