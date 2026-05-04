# Platform Boundary Architecture Addendum

Date: 2026-05-04

This addendum starts the implementation foundation for the Cross-Platform
Compatibility Roadmap. The goal is to make operating-system behavior explicit
without letting platform conditionals leak through product modules.

LLNZY should treat platform behavior as a narrow set of services. Terminal
tabs, editor buffers, Stacker documents, Sketch state, themes, workspaces, Git
views, command palette actions, and layout code should operate on product
concepts. They should not need to know whether the app is running on macOS,
Windows, X11, Wayland, or a packaged sandbox.

## Boundary Shape

Create a `platform` module that exposes stable app-facing contracts and hides
OS-specific implementations behind `cfg`-selected modules.

Suggested shape:

```text
src/platform/
  mod.rs
  shell.rs
  terminal_host.rs
  paths.rs
  open.rs
  clipboard.rs
  menu.rs
  input.rs
  packaging.rs
  macos.rs
  windows.rs
  linux.rs
```

`mod.rs` should export the shared types and a small `PlatformServices` value.
Product modules should receive either this service value or narrower service
traits when testing needs isolation. The OS-specific files should be the main
place where `#[cfg(target_os = "...")]` appears.

The boundary should be concrete before it is clever. Do not build a broad
"platform abstraction layer" that owns rendering, workspace state, or product
commands. Put only OS-sensitive behavior here.

Implementation update:

- Added the initial `src/platform` module skeleton.
- Added typed model files for shell profiles, terminal host launch specs,
  platform paths, desktop open/reveal requests, clipboard status, menu
  capability, input intent, and packaging metadata.
- Added `PlatformServices::current()` as a development-mode service root.
- The skeleton is intentionally not wired into runtime behavior yet; the next
  compatibility pass should migrate one platform-sensitive behavior at a time
  behind these types.
- Routed terminal shell startup through `ShellProfile` and
  `TerminalLaunchSpec` while preserving the existing portable-pty runtime.
- Kept the old interactive Unix defaults intact: login shell argument, `TERM`,
  `COLORTERM`, configured shell program, cwd, PTY resize, process id, and exit
  reporting.
- Added concrete `PlatformPathSet` resolution for config, data, cache, themes,
  workspaces, logs, crash reports, exports, Stacker, Sketch, saved sessions,
  recent projects, window state, backgrounds, and shaders.
- Migrated app-owned `dirs::config_dir()` call sites behind `PlatformPaths`
  while preserving the current on-disk `llnzy` directory layout.
- Added `PlatformOpen` execution helpers for URLs, files, folders, and reveal
  requests with macOS, Windows, and Linux command policies.
- Routed terminal hyperlink opening through `PlatformOpen` instead of calling
  macOS `open` directly from the main app loop.
- Added `PlatformClipboard` as the only direct `arboard` owner and routed app
  clipboard reads/writes through plain-text platform helpers.

## PlatformShell

`PlatformShell` owns shell discovery and command launch policy. It should return
structured shell profiles instead of raw command strings.

Responsibilities:

- Detect the default interactive shell for a terminal tab.
- Provide login and interactive flags per OS and shell family.
- Build task command invocations for project commands.
- Prepare environment variables that LLNZY owns.
- Provide user-facing display names such as `zsh`, `bash`, `PowerShell`, or
  `Command Prompt`.
- Report when a configured shell is missing and offer a fallback.

macOS and Linux should share the Unix shell model where possible, but they
should not assume identical defaults. Windows should model PowerShell, Windows
PowerShell, `cmd.exe`, Git Bash, WSL profiles, and user-configured shells as
distinct profiles because quoting, path translation, startup flags, and process
control differ.

Product code should ask for `ShellProfile` or `TaskLaunchSpec`, not concatenate
`/bin/sh -lc` or `powershell.exe -Command` itself.

## TerminalHost

`TerminalHost` owns process hosting for terminal sessions. It should be the only
place that knows whether LLNZY is using a Unix PTY or Windows ConPTY.

Responsibilities:

- Spawn a terminal process from a `ShellProfile` or explicit launch spec.
- Stream output bytes into the terminal emulator.
- Accept input bytes from the app's terminal input encoder.
- Resize the underlying PTY or pseudoconsole.
- Expose process id, child state, exit status, and failure reason.
- Terminate or interrupt the child using platform-appropriate behavior.
- Preserve enough diagnostics to debug shell launch failures.

The app-facing terminal session contract should stay uniform: a session has a
grid size, output stream, writable input sink, scrollback, title, optional cwd,
process id when available, and lifecycle state. The implementation underneath
can differ.

Unix implementations should continue to use PTY semantics. Windows
implementations should be designed as ConPTY hosting, even if a library hides
some of that detail. Do not force Windows into a POSIX mental model in product
code. Process groups, signal behavior, executable extensions, path separators,
and shell startup semantics must stay inside the terminal host and shell
services.

## PlatformPaths

`PlatformPaths` owns all app, user, project, and diagnostic paths that depend on
the OS or packaging mode.

Responsibilities:

- Resolve config, data, cache, logs, crash reports, themes, workspaces, and
  extension directories.
- Keep path creation and migration logic centralized.
- Normalize display paths for UI without changing durable stored paths.
- Provide temporary directories for app-owned operations.
- Surface unavailable or permission-denied directories as typed errors.
- Account for packaged, portable, sandboxed, and development builds.

Product modules should not call platform directory APIs directly. They should
receive paths from `PlatformPaths` and store durable app data through existing
store modules. Workspace files may contain platform-specific project paths, but
restore logic should tolerate missing paths and preserve the rest of the
workspace.

## PlatformOpen

`PlatformOpen` owns interactions with the user's desktop environment.

Responsibilities:

- Open URLs in the default browser.
- Open files with the system default application.
- Reveal files or folders in Finder, Explorer, or the active Linux file manager
  when supported.
- Open project folders from recent workspaces.
- Return structured errors for unsupported or failed operations.

Product code should express intent: `open_url`, `open_file`, `reveal_path`, or
`open_folder`. It should not shell out to `open`, `xdg-open`, `gio`, `explorer`,
or `start` directly.

Linux needs a fallback policy because desktop environments vary. A failed
reveal should degrade to opening the parent folder when possible and should
surface a plain status message rather than failing silently.

## PlatformClipboard

`PlatformClipboard` should initially be thin if the current clipboard crate is
reliable across targets. The reason to keep it behind a boundary anyway is that
clipboard behavior becomes platform-specific as soon as LLNZY supports rich
text, images, terminal bracketed paste policy, Wayland quirks, or sandboxed
builds.

Responsibilities:

- Read and write plain text.
- Preserve terminal paste behavior, including bracketed paste decisions outside
  the platform clipboard itself.
- Add future support for rich text, images, or file lists without changing
  product modules.
- Report clipboard unavailability as a normal app status, not a panic.

Editor, terminal, Stacker, and command palette code should call clipboard
commands through app services. They should not import clipboard crates directly.

## PlatformMenu

`PlatformMenu` owns native menu integration and the fallback path when native
menus are incomplete or unavailable.

Responsibilities:

- Provide macOS application menu behavior where expected.
- Define Windows and Linux menu support or an explicit egui fallback.
- Route menu selections into app commands.
- Keep labels, accelerators, and enabled/disabled states synchronized with the
  command registry.
- Avoid duplicating command logic inside menu callbacks.

Menus should be command dispatchers, not independent business logic. The menu
layer should emit app command IDs, and existing product modules should handle
the command through the same path used by keyboard shortcuts and the command
palette.

## PlatformInput

`PlatformInput` owns translation from raw window events into LLNZY input
intent. This is especially important for terminal input, IME, and shortcut
display.

Responsibilities:

- Normalize modifier names and shortcut display per OS.
- Translate keyboard events into app shortcuts or terminal input bytes.
- Handle AltGr, dead keys, compose input, and IME text entry.
- Preserve native text input behavior for WebView-backed or OS-backed controls.
- Keep terminal-specific encoding separate from editor text mutation.
- Define platform differences for paste, selection, mouse reporting, and focus.

The input layer should produce clear app intents such as `AppShortcut`,
`TerminalInput`, `TextInput`, `MouseReport`, or `FocusChange`. Product modules
should consume those intents and avoid inspecting low-level winit platform
details.

## Packaging Metadata

Packaging metadata should be a first-class platform boundary because app
identity affects paths, OS integration, file associations, update channels,
signing, and user trust.

Create a `PlatformPackagingMetadata` model that describes:

- App identifier and bundle/package id.
- Executable name and display name.
- Version, build channel, and update channel.
- Icons and resource paths.
- File associations and protocol handlers.
- Signing and notarization identity where applicable.
- Installer/package format and portable-build behavior.
- Whether the current build is development, packaged, sandboxed, or portable.

macOS metadata should cover bundle id, `Info.plist`, icon resources, Developer
ID signing, notarization, and DMG packaging. Windows metadata should cover
application user model id, icon resources, installer or MSIX choice, code
signing, file associations, protocol handlers, and portable build constraints.
Linux metadata should cover `.desktop` files, icons, AppImage or package
metadata, Flatpak permissions if pursued, MIME associations, and Wayland/X11
expectations.

Runtime code should not infer packaging behavior from ad hoc path checks. It
should ask packaging metadata what mode it is running in.

## Keeping Product Modules OS-Agnostic

The rule is simple: product modules may depend on platform capabilities, but
not platform mechanisms.

Allowed product concepts:

- `OpenProject`
- `RevealInFileManager`
- `CopyText`
- `PasteText`
- `SpawnTerminal`
- `RunTask`
- `MenuCommand`
- `KeyboardShortcut`
- `RestoreWorkspace`
- `WriteAppLog`

Disallowed product-module behavior:

- Direct `#[cfg(target_os = "...")]` branches outside the platform boundary,
  except for narrow framework integration that cannot be moved yet.
- Direct calls to OS commands for opening files or URLs.
- Direct imports of clipboard, native menu, or platform directory crates from
  editor, terminal, Stacker, Git, Sketch, or workspace modules.
- Hard-coded shell paths, executable extensions, or path separators.
- Treating Windows paths as strings that can be safely rewritten with simple
  slash replacement.

Where existing code already violates this boundary, migrate incrementally. Add
the service first, move one behavior at a time, and leave call sites clearer
than they were. The end state should be that a new product feature can be built
once and rely on platform services for OS-specific behavior.

## Implementation Sequence

Start by introducing type definitions and no-surprise wrappers, then move
behavior behind them.

1. [x] Add `src/platform` with shared service types and OS-specific modules.
2. [x] Move shell discovery and terminal spawning behind `PlatformShell` and
   `TerminalHost`.
3. [x] Route config, data, cache, logs, themes, and workspaces through
   `PlatformPaths`.
4. [x] Move open/reveal behavior behind `PlatformOpen`.
5. [x] Move clipboard imports behind `PlatformClipboard`.
6. [ ] Route native menu callbacks through app command IDs via `PlatformMenu`.
7. [ ] Normalize keyboard, IME, and terminal input through `PlatformInput`.
8. [ ] Define packaging metadata and make build scripts consume the same values
   where practical.

This sequence keeps the terminal-hosting risk visible while avoiding a large
rewrite. Each step should leave product modules with fewer platform assumptions
and more typed intent.
