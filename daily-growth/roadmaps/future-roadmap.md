# Future Roadmap

This document tracks larger follow-up work that should stay visible but does
not need to block the current leftovers roadmap.

---

## Wispr Flow Optimization

- [ ] Continue optimizing Wispr Flow input behavior with multiple workspace tabs
  open and multiple shell sessions running at the same time.
- [ ] Verify Wispr Flow delivery when switching between grouped shells,
  standalone shells, Stacker, and editor tabs.
- [ ] Investigate whether delayed Wispr Flow delivery is tied to active tab
  focus, AppKit text bridge ownership, terminal paste routing, or a combination
  of those paths.
- [ ] Keep the Stacker input engine as the owner of Stacker text mutation while
  improving external text ingress.
- [ ] Define the command handoff contract external tools need when they target
  Stacker or the code editor: insert text, replace selection, apply formatting,
  submit command, copy, paste, select all, undo, and redo.
- [ ] Verify whether Wispr Flow sends text through IME commit, pasteboard paste,
  accessibility insertion, synthetic key events, or a mix of those paths.
- [x] Route external text delivery through the same editor command/document
  mutation path as keyboard and toolbar actions.
- [x] Add a narrow debug trace for external ingress events so Wispr Flow can be
  diagnosed without noisy normal logs.

---

## Stacker Document Editor Buildout

- [x] Replace raw Stacker prompt input state with `StackerDocumentEditor`.
- [x] Centralize Stacker text mutation through document/input-engine operations
  for committed text, paste, selection replacement, backward delete, forward
  delete, select all, copy selection, undo, and redo.
- [x] Add undo/redo history with cursor and selection restoration.
- [x] Add a reusable Stacker command layer for bold, unordered list, ordered
  list, heading, blockquote, inline code, code block, checklist, clear, load
  text, undo, and redo.
- [x] Preserve cursor and selection behavior through formatting toolbar actions.
- [x] Add reusable egui cursor/selection synchronization helpers for Stacker.
- [x] Add dirty draft tracking for scratch prompts and saved-prompt edits.
- [x] Warn before switching prompts or starting a new prompt when the current
  draft has unsaved changes.
- [x] Keep normal `Command-V` clipboard paste working in the styled editor.
- [x] Keep normal typing working in the styled editor.
- [x] Add Stacker-owned formatting shortcuts so editor commands do not leak into
  global workspace commands while Stacker is active.
- [x] Add a compact editor status line showing source, dirty state, character
  count, word count, line count, and selection size.
- [x] Add a first-class Stacker command registry that can be used by keyboard
  shortcuts, toolbar buttons, command palette entries, native menus, and future
  external input tools.
- [x] Add command-palette entries for Stacker formatting commands when Stacker is
  the active tab.
- [x] Keep Stacker as a focused prompt editor without Source/Split/Preview
  rendering modes.
- [x] Add explicit saved-prompt management actions for editing and deleting saved
  prompts.
- [x] Define saved-prompt edit behavior: click loads a prompt into the editor,
  editing marks it dirty, `Save` updates the existing prompt, and `Save Prompt`
  creates a new prompt from scratch.
- [x] Add a delete action to each saved prompt row that asks for confirmation
  before removing the prompt from the saved prompt list.
- [x] Make saved-prompt delete behavior safe around dirty drafts: deleting the
  currently edited prompt should either require saving/canceling the draft first
  or clearly discard the draft through the same warning modal pattern.
- [ ] Manually verify saved-prompt edit and delete behavior, including queue
  state, dirty draft state, persistence to `stacker.json`, and undo/redo
  expectations after loading another prompt.
- [x] Add an instrumentation flag or debug-only tracing point for future
  external input debugging without noisy logs in normal development.
- [x] Continue optimizing AppKit bridge ingress latency while keeping Stacker's
  own document editor as the text mutation owner.
- [x] Decide whether external text ingress needs a native text-client bridge, a
  paste-command capture layer, an accessibility-command bridge, or another
  platform integration point.
- [x] Implement the chosen external text ingress path only after the current
  delivery mechanism is understood.
- [ ] Manually verify typing, copy, paste, select all, undo, redo, formatting
  toolbar actions, dirty-draft warning, save behavior, and queue actions.
- [ ] Manually verify Wispr Flow or a comparable OS-level dictation/paste tool
  after the external ingress path is implemented.

---

## Terminal Selection Drag Performance

- [ ] Profile the drag path while selecting in a mouse-reporting TUI.
- [ ] Manually verify copy correctness after forward drag, backward drag, word
  selection, line selection, and select all.

---

## Terminal Highlighting Performance

- [ ] Continue improving terminal highlighting performance for selection,
  search matches, URL underlines, and other terminal overlay rectangles.
- [ ] Profile highlight generation and rendering cost in large scrollback
  sessions and busy TUI apps.
- [ ] Cache or incrementally rebuild highlight geometry where safe.
- [ ] Verify highlighting remains visually correct after resize, scrollback,
  selection changes, search updates, and joined-tab layout changes.

---

## Tab Grouping Manual Verification

- [ ] Manually verify mixed joined groups such as Terminal + CodeFile, Git +
  Stacker, and CodeFile + Sketch.

---

## Cross-Platform Compatibility Addendum

Source foundation:
`daily-growth/roadmaps/cross-platform-compatibility-roadmap-05-04-2026.md`

### Compatibility Contract

- [ ] Define the public compatibility promise: same llnzy workspace model across
  macOS, Windows, and Linux, with native terminal behavior per OS.
- [ ] Keep editor, renderer, themes, project navigation, Stacker, Sketch, local
  Git dashboard, and workspace tabs as near-identical cross-platform surfaces.
- [ ] Treat macOS and Linux as Unix PTY platforms.
- [ ] Treat Windows as a Windows ConPTY platform, not as a Unix PTY clone.
- [ ] Document that platform differences are expected where shells, paths,
  shortcuts, packaging, and process control are OS-native.

### Platform Boundary

- [ ] Audit every `cfg(target_os = "...")` platform-specific path in the app.
- [ ] Define a `PlatformShell` boundary for default shell detection, shell
  arguments, interactive/login behavior, task execution, and environment setup.
- [ ] Define a `TerminalHost` boundary for spawn, read, write, resize, process
  id, terminate, and exit reporting.
- [ ] Define a `PlatformPaths` boundary for config, data, cache, themes,
  workspaces, logs, crash reports, and exported assets.
- [ ] Define a `PlatformOpen` boundary for opening URLs, revealing files,
  opening folders, and opening files with system default applications.
- [ ] Define a `PlatformClipboard` fallback boundary if `arboard` is not enough
  on any supported OS.
- [ ] Define a `PlatformMenu` boundary for native macOS menus and in-window
  command fallbacks elsewhere.
- [ ] Define a `PlatformInput` boundary for modifier naming, IME, dead keys,
  compose input, AltGr, and terminal special-key encoding.
- [ ] Define platform packaging metadata for app id, executable name, icons,
  file associations, protocol handlers, signing identity, and update channel.
- [ ] Keep product modules operating on app concepts such as tabs, sessions,
  buffers, projects, snapshots, themes, and UI commands.

### Rendering and Windowing

- [ ] Confirm macOS uses Metal as the supported renderer path.
- [ ] Confirm Windows uses DX12 as the primary renderer path.
- [ ] Decide whether Windows should allow Vulkan fallback.
- [ ] Confirm Linux uses Vulkan as the primary renderer path.
- [ ] Decide the Linux GL/GLES fallback policy.
- [ ] Add renderer diagnostics for selected backend, adapter, OS, driver hints,
  and failure reason.
- [ ] Add startup/home visual smoke coverage for each OS.
- [ ] Add terminal ANSI color visual smoke coverage for each OS.
- [ ] Add editor syntax highlighting visual smoke coverage for each OS.
- [ ] Add joined terminal pane visual smoke coverage for each OS.
- [ ] Add terminal plus editor joined-layout smoke coverage for each OS.
- [ ] Add settings and appearances visual smoke coverage for each OS.
- [ ] Add effects enabled/disabled smoke coverage for each OS.
- [ ] Add high-DPI and window-resize smoke coverage for each OS.
- [ ] Test Linux under both Wayland and X11.

### macOS Terminal and App Behavior

- [ ] Preserve Unix PTY shell hosting on macOS.
- [ ] Detect default shell from `$SHELL`, with `/bin/zsh` fallback.
- [ ] Preserve login-shell behavior for interactive terminal tabs.
- [ ] Preserve cwd inheritance from OSC 7, active project root, or last known
  workspace root.
- [ ] Preserve PTY resize on pane resize and cell-size changes.
- [ ] Preserve Ctrl-C and related terminal control behavior through PTY input.
- [ ] Preserve UTF-8 output handling and graceful invalid-byte behavior.
- [ ] Set useful terminal environment variables such as `TERM`,
  `COLORTERM`, and possibly `TERM_PROGRAM`.
- [ ] Test zsh, bash, and fish on macOS.
- [ ] Test Apple Silicon builds.
- [ ] Decide whether Intel macOS builds remain supported.
- [ ] Verify macOS IME, dead-key input, and native menu command routing.
- [ ] Verify Gatekeeper launch after signing and notarization.
- [ ] Harden GUI-app PATH discovery for Git and language servers.

### Linux Terminal and App Behavior

- [ ] Preserve Unix PTY shell hosting on Linux.
- [ ] Detect default shell from environment/passwd data with safe fallback.
- [ ] Preserve login-shell behavior where appropriate.
- [ ] Preserve cwd inheritance from OSC 7, active project root, or last known
  workspace root.
- [ ] Preserve PTY resize on pane resize and cell-size changes.
- [ ] Preserve terminal control behavior through PTY input.
- [ ] Preserve UTF-8 output handling and graceful invalid-byte behavior.
- [ ] Test bash, zsh, and fish on Linux.
- [ ] Test Ubuntu LTS or equivalent glibc baseline.
- [ ] Test Fedora or another modern Wayland-first desktop.
- [ ] Test GNOME Wayland.
- [ ] Test KDE Wayland or X11.
- [ ] Add `.desktop` file metadata and icon installation.
- [ ] Verify Linux clipboard, file dialogs, drag/drop, and IME behavior.

### Windows Terminal and App Behavior

- [ ] Validate Windows terminal hosting through ConPTY.
- [ ] Decide whether `portable-pty` remains sufficient on Windows or whether a
  dedicated Windows host layer is needed.
- [ ] Detect PowerShell 7 when installed.
- [ ] Fall back to Windows PowerShell when PowerShell 7 is unavailable.
- [ ] Fall back to `cmd.exe` when PowerShell options are unavailable.
- [ ] Add shell profile options for PowerShell, Command Prompt, Git Bash, WSL,
  and custom shells.
- [ ] Implement ConPTY resize behavior.
- [ ] Preserve UTF-8 input/output boundary through ConPTY.
- [ ] Add Windows path parsing for drive-letter paths.
- [ ] Add Windows path parsing for slash-normalized paths.
- [ ] Add UNC path parsing.
- [ ] Add `file:line:column` parsing for Windows paths where possible.
- [ ] Implement Windows process termination/restart behavior.
- [ ] Explain terminate-process versus shell-level exit behavior in the UI or
  docs.
- [ ] Add Windows terminal environment policy for `TERM`, `COLORTERM`, and
  Windows-specific variables.
- [ ] Verify Windows clipboard behavior.
- [ ] Verify Ctrl, Alt, Windows key, AltGr, function keys, and terminal
  application modes.
- [ ] Preserve mouse reporting through the same terminal-level app API.
- [ ] Treat WSL as an explicit shell profile, not the default Windows behavior.
- [ ] Document Windows shell startup, path, signal, executable lookup, ANSI/VT,
  and quoting differences.

### Shell Profiles and Task Execution

- [ ] Replace the single-shell assumption with a shell-profile model.
- [ ] Store shell profile name, platform, program, args, cwd policy,
  environment additions, default flag, and interactive/task behavior.
- [ ] Preserve structured task execution using command, args, and cwd.
- [ ] Avoid blind string concatenation into whichever shell is active.
- [ ] Route Unix direct tasks through structured process spawn attached to PTY.
- [ ] Route Windows `.exe`, `.cmd`, and `.bat` tasks according to Windows
  process rules.
- [ ] Route Windows `.ps1` tasks through PowerShell explicitly.
- [ ] Add per-platform custom shell snippet support only with clear quoting
  rules.
- [ ] Document that custom task snippets may need per-platform variants.

### Input, Keyboard, and IME

- [ ] Keep macOS app shortcuts Command-based.
- [ ] Keep Windows app shortcuts Control-based.
- [ ] Keep Linux app shortcuts Control-based.
- [ ] Preserve terminal control shortcuts when terminal focus requires them.
- [ ] Decide macOS Option-as-text versus Option-as-Alt behavior.
- [ ] Prevent Windows AltGr from being misread as app Ctrl+Alt shortcuts.
- [ ] Test Linux compose and dead keys under Wayland and X11.
- [ ] Test regular text typing in terminal, editor, and Stacker.
- [ ] Test paste into terminal, editor, and Stacker.
- [ ] Test CJK IME composition.
- [ ] Test emoji/symbol input.
- [ ] Test keyboard shortcuts with terminal, editor, Stacker, and command
  palette focused.

### Filesystem, Paths, and Workspace Restore

- [ ] Save native paths exactly as selected on the current OS.
- [ ] Keep session restore graceful when project paths or file paths are missing.
- [ ] Show clear restore warnings for skipped project folders and files.
- [ ] Preserve restorable theme/tab intent even when some paths are missing.
- [ ] Add optional future path mappings for synced workspaces across OSes.
- [ ] Support Unix absolute path detection.
- [ ] Support macOS home-relative path detection.
- [ ] Support Windows drive-letter path detection.
- [ ] Support Windows slash-normalized path detection.
- [ ] Support UNC path detection.
- [ ] Define initial WSL path-opening limitations.
- [ ] Later, add WSL `/mnt/<drive>/...` translation or `wslpath` integration if
  WSL workflows justify it.

### LSP and Developer Tools

- [ ] Add platform-aware PATH initialization.
- [ ] Add per-language server command overrides in config.
- [ ] Show LSP statuses for unsupported, command not found, starting, running,
  stopped, and unavailable states.
- [ ] Document that language features require installed language servers.
- [ ] Add Windows guidance for installing language servers through `rustup`,
  npm, Go, Python, or package managers.
- [ ] Add macOS guidance for GUI app PATH differences.
- [ ] Add Linux guidance for system package managers and language server PATH.
- [ ] Decide how Flatpak builds handle host language servers before shipping
  Flatpak as a main Linux package.

### Git Integration

- [ ] Detect `/usr/bin/git`, Xcode command line tools git, Homebrew git, and
  user PATH git on macOS.
- [ ] Detect Git for Windows in PATH and common install locations.
- [ ] Detect system Git through PATH on Linux.
- [ ] Add platform-specific missing-Git guidance.
- [ ] Preserve local-only Git behavior with no GitHub/GitLab account
  requirement.
- [ ] Verify branch, status, commits, stashes, reflog, and commit patch views on
  each OS.

### Native Menus and Desktop Integration

- [ ] Keep native menu bar behavior on macOS.
- [ ] Mirror every native macOS menu command through an in-app command palette
  or shortcut available on Windows and Linux.
- [ ] Decide whether Windows needs a native menu bar or only in-window command
  UI.
- [ ] Rely on in-window command UI and `.desktop` integration on Linux.
- [ ] Add Windows taskbar identity, icon, Start menu entry, and uninstall
  behavior through packaging.
- [ ] Add Linux launcher identity through `.desktop` file and icon resources.
- [ ] Ensure no feature exists only in a native menu.

### Packaging and Distribution

- [ ] Produce macOS `.app` bundle.
- [ ] Sign macOS executable and nested resources.
- [ ] Enable hardened runtime if needed for notarization.
- [ ] Notarize macOS builds.
- [ ] Staple notarization ticket where applicable.
- [ ] Produce macOS DMG.
- [ ] Decide universal macOS binary versus separate Apple Silicon and Intel
  downloads.
- [ ] Declare minimum supported macOS version.
- [ ] Produce signed Windows executable.
- [ ] Choose Windows installer strategy: MSI, NSIS, Wix, MSIX, or staged
  combination.
- [ ] Evaluate MSIX install-location and filesystem implications before making
  MSIX the default.
- [ ] Produce Windows installer with Start menu shortcut, icon, and uninstall
  entry.
- [ ] Decide whether to offer Windows portable `.zip`.
- [ ] Document Windows crash/log locations.
- [ ] Declare minimum supported Windows version, likely Windows 10 or newer.
- [ ] Produce Linux tarball or AppImage-style preview build.
- [ ] Produce `.deb` package if Ubuntu/Debian-family support is targeted.
- [ ] Produce `.rpm` package if Fedora/openSUSE-family support is targeted.
- [ ] Defer Flatpak until filesystem, terminal spawning, language server, and
  developer-tool sandbox permissions are decided.
- [ ] If Flatpak ships, label it as a sandboxed edition and document its limits.

### Automated Testing

- [ ] Add CI `cargo check` on macOS, Windows, and Linux.
- [ ] Add CI `cargo test --lib` on macOS, Windows, and Linux.
- [ ] Run terminal parsing tests on all target OSes.
- [ ] Run editor buffer/search/syntax tests on all target OSes.
- [ ] Run tab grouping/layout tests on all target OSes.
- [ ] Run workspace/session restore tests on all target OSes.
- [ ] Run Git parser tests without requiring a live remote.
- [ ] Add platform path parser tests for Unix, Windows, UNC, and WSL-like paths.
- [ ] Add OS-specific PTY/ConPTY integration tests with clear skip rules where
  host shell availability differs.

### Manual Release Smoke Testing

- [ ] Smoke test macOS Apple Silicon.
- [ ] Smoke test macOS Intel if supported.
- [ ] Smoke test Windows 10 or Windows 11 x64.
- [ ] Smoke test Windows ARM64 if supported.
- [ ] Smoke test Ubuntu LTS or equivalent.
- [ ] Smoke test Fedora/GNOME Wayland or equivalent.
- [ ] Smoke test KDE Wayland or X11 if feasible.
- [ ] Verify fresh install launch.
- [ ] Verify opening project folder.
- [ ] Verify terminal tab creation and shell command execution.
- [ ] Verify TUI app execution.
- [ ] Verify window resize.
- [ ] Verify split/joined terminal panes.
- [ ] Verify active and inactive joined terminal pane scrolling.
- [ ] Verify terminal copy/paste.
- [ ] Verify code open/edit/save.
- [ ] Verify syntax highlighting.
- [ ] Verify LSP available and unavailable states.
- [ ] Verify detected Cargo/npm task execution.
- [ ] Verify Git dashboard in clean and dirty repos.
- [ ] Verify command palette.
- [ ] Verify theme/effects switching.
- [ ] Verify unsaved file prompt.
- [ ] Verify last-session restore.
- [ ] Verify drag/drop where supported.

### Support Levels and Release Messaging

- [ ] Define support labels: Supported, Preview, Experimental, Unsupported.
- [ ] Initially keep macOS as Supported.
- [ ] Treat Windows as Preview until terminal host, installer, PATH/LSP, and
  input polish are proven.
- [ ] Treat Linux as Preview until target distros/desktops, Wayland/X11,
  packaging, and GPU backend coverage are stable.
- [ ] Add platform labels to bug triage.
- [ ] Add severity rules for platform-specific regressions.

### User-Facing Communication

- [ ] Publish the core platform message: same llnzy workspace, native terminal
  hosting per operating system.
- [ ] Create a platform comparison page for macOS, Windows, and Linux.
- [ ] Include status, terminal host, default shell, GPU backend, package,
  shortcut modifier, Git requirement, LSP requirement, and known differences in
  the comparison page.
- [ ] Label downloads by platform and support level.
- [ ] Avoid vague "beta" language unless the support meaning is defined.
- [ ] Add macOS install docs with signing/notarization, architecture, shell,
  PATH, and language server notes.
- [ ] Add Windows install docs with supported Windows versions, installer versus
  portable, shell profiles, Git for Windows, paths, and ConPTY caveats.
- [ ] Add Linux install docs with distros/desktops, packages, Wayland/X11, GPU
  drivers, shell discovery, and language server discovery.
- [ ] Add a platform differences doc covering terminal host model, shortcuts,
  paths, shell profiles, packaging/sandboxing, limitations, and workarounds.
- [ ] Add troubleshooting docs for GPU failure, shell startup failure, missing
  language servers, missing Git, failed path restore, copy/paste, and IME.
- [ ] Add platform-specific sections to every release note.
- [ ] Keep known platform-specific issues visible instead of burying them in
  commit messages.

---

## Notes

Add future work here when it is important enough to remember but not ready to
be implemented in the current pass.
