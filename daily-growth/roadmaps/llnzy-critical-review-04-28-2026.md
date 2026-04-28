# llnzy Critical Review
## April 28, 2026

This document is a deliberately critical senior engineering review of `llnzy` as it exists today: a native Rust terminal emulator with a code editor, sketch pad, prompt stacker, theme system, project explorer, and workspace shell built around it.

The tone is intentionally direct. The goal is not to be flattering. The goal is to expose the gaps that matter if this app is meant to graduate from impressive personal tool to trustworthy daily development environment.

---

## Executive Summary

`llnzy` is impressive, but it is carrying too much ambition on too little architecture.

The strongest part of the app is the terminal foundation: PTY integration, `alacritty_terminal`, GPU text rendering, input encoding, terminal search, URL detection, mouse reporting, and scrollback are real work. This is not a fake terminal skin. The code has enough serious implementation detail to deserve respect.

The weakest part is product and architectural focus. The app wants to be a terminal emulator, source editor, lightweight IDE, sketch pad, prompt queue, theme playground, visual effects engine, and workspace launcher. That breadth is exciting, but it has pushed core state management into large central objects and long frame-driven mutation flows. Right now, the app feels like a terminal emulator with a code editor attached to it, plus several other productivity ideas orbiting around the same event loop.

The core risk is not that the code is bad. The core risk is that the app is becoming a pile of impressive features before it has a boring, reliable spine.

If a senior engineer at a top software company reviewed this, the likely read would be:

- This is a serious prototype.
- The terminal has credible bones.
- The editor is brave but not yet trustworthy.
- The architecture is already straining.
- The product identity needs discipline.
- The next phase should be reliability and separation, not more features.

---

## The Blunt Product Read

The app currently has a positioning problem.

It is not exactly a terminal emulator because it has an editor, explorer, prompt manager, workspace system, sketch pad, themes, and a home screen.

It is not exactly an IDE because the editor stack is not yet mature enough to compete with editors that have years of edge-case hardening.

It is not exactly a visual coding workspace because the sketch and prompt tools are useful but not yet integrated deeply enough into a coherent developer workflow.

So the product currently lands as:

> A highly personalized terminal-first development environment with a lot of interesting attached tools.

That can become a compelling product. But it needs to choose a primary promise.

The most believable promise is:

> A beautiful, fast, terminal-first dev environment with just enough editor intelligence for quick code navigation and edits.

That is a stronger promise than:

> A full VS Code replacement, terminal replacement, sketch app, prompt manager, visual effects platform, and workspace manager all at once.

The second promise is where projects go to become permanently unfinished.

---

## What Is Actually Strong

Before the critique, it is important to separate real strengths from surface polish.

### Terminal Core

The terminal side is the most credible part of the app.

Relevant files:

- `src/pty.rs`
- `src/terminal.rs`
- `src/session.rs`
- `src/input.rs`
- `src/renderer/text.rs`
- `tests/terminal_emulation.rs`
- `tests/pty_roundtrip.rs`

Good decisions:

- Uses `portable-pty` instead of pretending stdin/stdout is enough.
- Uses `alacritty_terminal` instead of trying to hand-roll VT parsing.
- Separates PTY, terminal emulation, and session state.
- Runs PTY reads and writes off the render thread.
- Supports bracketed paste.
- Supports mouse reporting.
- Supports app cursor mode.
- Supports OSC title events.
- Handles clipboard store events.
- Has terminal emulation integration tests.

This is real work. Many "terminal" projects never get this far.

### GPU Rendering

The renderer is a legitimate custom GPU pipeline, not just egui text in a panel.

Relevant files:

- `src/renderer/mod.rs`
- `src/renderer/state.rs`
- `src/renderer/text.rs`
- `src/renderer/rect.rs`
- `src/renderer/bloom.rs`
- `src/renderer/crt.rs`
- `src/renderer/background.rs`
- `src/renderer/particles.rs`
- `src/renderer/cursor.rs`

Good decisions:

- Uses `wgpu`.
- Uses `glyphon`.
- Caches terminal line text.
- Splits scene rendering from post-processing.
- Supports bloom, CRT, particles, image backgrounds, shader backgrounds, cursor glow.
- Allows egui to render either into the scene texture or on top, depending on effects behavior.

This part has personality and technical depth. It also carries performance risk, but the ambition is coherent.

### Editor Building Blocks

The editor is over-ambitious, but it is not fake.

Relevant files:

- `src/editor/buffer.rs`
- `src/editor/cursor.rs`
- `src/editor/history.rs`
- `src/editor/syntax.rs`
- `src/editor/keymap.rs`
- `src/ui/editor_view.rs`
- `src/ui/explorer_view.rs`

Good decisions:

- Uses `ropey` for text storage.
- Preserves line endings.
- Detects indentation style.
- Has undo/redo.
- Cursor movement is grapheme-aware.
- Tree-sitter is integrated.
- Syntax parsing is moved off the main thread.
- LSP exists.
- Keybinding presets exist.

This is not a toy text area. It is a real attempt at an editor.

### Project Workflow

The app already thinks about developer workflows:

- project explorer
- fuzzy file finder
- command palette
- task detection
- code navigation
- project search
- workspace storage
- recent projects
- prompt stacker
- sketch notes

The issue is not lack of ideas. The issue is lack of a strong boundary around the ideas.

---

## The Main Engineering Critique

The app has an architecture problem before it has a feature problem.

The top-level app object is doing too much. The UI state object is doing too much. The editor view is doing too much. The LSP manager blocks too often. The command model is implicit. The render loop doubles as an action dispatcher. The result is a codebase that is impressive today but likely painful tomorrow.

The app needs a spine.

Right now, too many workflows are implemented as:

1. Render UI.
2. Put side effects into `Option` fields.
3. Return to `main.rs`.
4. Drain those fields.
5. Mutate global state.
6. Request another redraw.

That is survivable at small scale. It becomes increasingly brittle as features grow.

---

## Finding 1: `main.rs` Is Carrying the Whole App

Relevant file:

- `src/main.rs`

The `App` struct starts at `src/main.rs:55` and owns almost every major subsystem:

- config
- window
- renderer
- tabs
- active tab
- event proxy
- modifiers
- terminal selection
- terminal search
- error log
- clipboard
- mouse state
- click state
- UI state
- screen layout
- visual bell
- config reload timers
- blink timers
- keypress timers

That is already too much.

The `window_event` implementation then coordinates:

- egui event routing
- terminal input suppression
- close prompts
- scale factor updates
- resize handling
- redraw/rendering
- terminal output draining
- sidebar layout updates
- save prompt responses
- tab bar actions
- config changes
- project opening
- workspace launching
- singleton tab creation
- file tab creation
- task execution
- clipboard bridge
- tab renaming
- mouse wheel behavior
- mouse reporting
- selection behavior
- URL/file click behavior
- keybinding dispatch
- terminal key encoding
- IME
- drag and drop
- native macOS menu actions
- config hot reload
- animation control flow

This is not orchestration anymore. This is a central switchboard with business logic embedded everywhere.

### Why This Matters

When a bug happens, there is no obvious owner.

If a tab closes incorrectly, is that workspace state, editor state, UI state, tab bar state, or main loop state?

If config changes do not apply correctly, is that settings UI, `Config`, `Renderer`, `UiState`, or hot reload?

If LSP freezes the editor, is that editor view, LSP manager, runtime, event loop, or egui?

The answer is often "several of them."

That is the warning sign.

### Recommended Fix

Create explicit controllers:

- `AppController`
- `TerminalController`
- `EditorController`
- `WorkspaceController`
- `ConfigController`
- `UiActionDispatcher`
- `ProcessManager`

The main loop should receive events and dispatch commands. It should not personally implement every command.

Target shape:

```rust
enum AppCommand {
    NewTerminal { cwd: Option<PathBuf> },
    CloseTab { tab_id: TabId },
    OpenFile { path: PathBuf },
    SaveFile { buffer_id: BufferId },
    LaunchWorkspace { workspace: SavedWorkspace },
    ApplyConfig { config: Config },
    RunTask { task: Task },
}
```

Then make subsystems return commands instead of setting random `Option` fields.

---

## Finding 2: `UiState` Is a Storage Unit

Relevant file:

- `src/ui/mod.rs`

`UiState` starts at `src/ui/mod.rs:28`. It contains:

- egui context
- winit state
- egui renderer
- active view
- settings tab
- pending config
- clipboard text
- sidebar state
- FPS state
- Stacker state
- prompt bar state
- copy ghosts
- Sketch state
- Explorer state
- Editor state
- tab rename state
- tab context
- terminal panel state
- command palette state
- recent projects
- footer actions
- active tab kind
- tab bar actions
- tab names
- split view
- workspace launch
- pending close prompt
- save prompt response

That is not a UI state object. That is half the app.

The egui render function starts pulling state out around `src/ui/mod.rs:300`, runs the frame, then writes it back around `src/ui/mod.rs:625`. This is clever Rust ownership management, but it is also a maintainability smell.

The code is saying: "I cannot borrow this state in a sane shape, so I will temporarily disassemble the app."

### Why This Matters

This style makes it easy to accidentally:

- drop state
- restore stale state
- ignore an action
- overwrite a concurrent change
- make UI behavior depend on render ordering
- hide business logic inside rendering code

It also makes tests harder. You cannot easily test "when user clicks X, command Y is emitted" because the action lives as a mutation to a field inside a large render closure.

### Recommended Fix

Split UI state by feature:

- `ShellUiState`
- `TabBarState`
- `SidebarState`
- `EditorUiState`
- `StackerUiState`
- `SketchUiState`
- `SettingsUiState`
- `CommandPaletteState`

Then make rendering functions return typed outputs:

```rust
struct UiFrameOutput {
    commands: Vec<AppCommand>,
    clipboard: Option<String>,
    config_patch: Option<ConfigPatch>,
}
```

The UI should not own project behavior. It should request behavior.

---

## Finding 3: The Render Loop Is Also the Action Loop

Relevant area:

- `src/main.rs:626`

The redraw handler does all of this in one flow:

1. process terminal output
2. calculate selection/search overlays
3. update UI frame time
4. render terminal and egui
5. detect sidebar width changes
6. consume UI actions
7. mutate app state
8. create tabs
9. open files
10. run tasks
11. update config
12. request redraw again

This is too much responsibility for a frame.

Rendering should be as close as possible to:

```rust
let frame = app_state.snapshot_for_render();
renderer.render(frame);
```

Instead, rendering is part of the state transition pipeline.

### Why This Matters

Frame rate and business logic should not be tightly coupled.

If the app stops rendering because the window is occluded or throttled, do actions still process correctly?

If UI rendering fails, do commands get lost?

If a command triggers a redraw inside the redraw handler, are there reentrancy or ordering issues?

This is the kind of architecture that feels fine in manual testing and then behaves strangely under load.

### Recommended Fix

Separate:

- event collection
- command processing
- state mutation
- rendering

A minimal version:

```rust
fn window_event(...) {
    let commands = self.handle_window_event(event);
    self.apply_commands(commands);
    self.request_redraw();
}

fn redraw(...) {
    self.drain_async_sources();
    let frame = self.build_render_frame();
    self.renderer.render(frame);
}
```

---

## Finding 4: Save Errors Are Not Treated as Critical

Relevant area:

- `src/main.rs:776`

When the save prompt response is `Save`, the code calls `buf.save()` and ignores the result before closing the tab or exiting.

That is not acceptable for an editor.

An editor gets one job before all others: do not lose work.

Ignoring save errors violates that.

### Why This Matters

Save can fail for ordinary reasons:

- permission denied
- deleted parent directory
- disk full
- file locked
- network volume gone
- race with external process
- path moved

If the app silently closes after a failed save, the user may lose work. That is an existential trust failure.

### Recommended Fix

Make save return an explicit result and keep the tab/window open on failure.

Behavior should be:

- attempt save
- if success, close
- if failure, show error dialog and keep buffer open
- log detailed error
- keep modified state intact

This needs tests.

---

## Finding 5: PTY Process Lifecycle Is Too Thin

Relevant file:

- `src/pty.rs`

The PTY spawn logic creates the child at `src/pty.rs:58`, but the child handle is discarded:

```rust
let _child = pair.slave.spawn_command(cmd).map_err(io::Error::other)?;
```

The session later infers exit by reader channel disconnect.

That is useful, but insufficient.

### Why This Matters

A terminal emulator should have serious process lifecycle semantics:

- know the child process
- know exit status
- handle shell exit intentionally
- optionally keep exited tabs visible
- support restart/relaunch
- clean up child processes
- distinguish normal exit from failure
- avoid zombie-like lifecycle ambiguity

Auto-closing terminal tabs whose shells exit, implemented around `src/main.rs:172`, is also questionable. A user often wants to see the final output of a failed command or shell process. Auto-close destroys context.

### Recommended Fix

Retain child handle or an explicit process supervisor.

Add:

- `PtyProcess`
- child id
- exit status
- termination time
- kill/close semantics
- "process exited" terminal overlay
- setting for auto-close behavior, default off

---

## Finding 6: LSP Is Useful but Not Yet Robust

Relevant files:

- `src/lsp/mod.rs`
- `src/lsp/client.rs`
- `src/lsp/transport.rs`
- `src/lsp/registry.rs`

The LSP layer is a major achievement for a personal editor. But it is not yet a mature LSP client.

### Blocking Calls

`LspManager` owns a runtime and calls `block_on` for operations like:

- ensure server
- did open
- did change
- did save
- did close

Examples start around `src/lsp/mod.rs:168`.

This risks UI stalls. Language servers can be slow. They can hang. They can crash. They can emit unexpected requests. They can do expensive startup indexing. Blocking editor behavior on them is not safe.

### Server Requests Are Not Answered

`src/lsp/transport.rs:187` detects server-initiated requests but only forwards them as notifications. There is no visible response handling path.

Many servers ask for:

- `workspace/configuration`
- `client/registerCapability`
- `workspace/applyEdit`
- `window/showMessageRequest`
- `window/workDoneProgress/create`

If the client ignores or fails to respond, some servers degrade or hang waiting.

### Stderr Is Dropped

`src/lsp/transport.rs:51` sends server stderr to null.

That makes debugging LSP failures much harder. A dev tool should not hide the one stream servers use to explain what went wrong.

### Recommended Fix

Move to fully async LSP:

- requests return futures or channels
- no UI-thread `block_on`
- add request timeouts
- respond to required server requests
- capture stderr into error log
- implement server restart
- track server health
- use capability checks before enabling features
- make root/workspace ownership explicit

---

## Finding 7: The Editor Is Trying to Be Too Much Too Soon

Relevant files:

- `src/editor/buffer.rs`
- `src/editor/cursor.rs`
- `src/editor/keymap.rs`
- `src/editor/mod.rs`
- `src/ui/editor_view.rs`
- `src/ui/explorer_view.rs`

The editor has:

- buffers
- tabs
- tree-sitter
- LSP
- diagnostics
- completion
- hover
- go-to-definition
- references
- rename
- formatting
- code actions
- symbols
- workspace symbols
- inlay hints
- code lens
- project search
- minimap
- git gutter
- snippets
- folding
- word wrap
- visible whitespace
- VS Code keybindings
- Vim mode
- Emacs mode
- multicursor

That is a large chunk of VS Code's conceptual surface.

The hard truth: every one of those features has deep edge cases. A custom editor can absolutely succeed, but only if it narrows the first production promise.

### The Current Risk

The editor may feel powerful in demos and fragile in daily use.

Problems users will punish immediately:

- save failure
- cursor weirdness
- bad undo grouping
- broken paste
- slow typing
- broken IME
- broken selection
- LSP freezes
- file reload conflicts
- bad external change handling
- wrong diagnostics
- project search missing files
- large file stalls

Visual effects will not compensate for any of those.

### Recommended Fix

Define editor tiers:

#### Tier 1: Must Be Excellent

- open file
- edit text
- save safely
- undo/redo
- selection
- copy/paste
- search
- syntax highlight
- diagnostics
- go to definition
- file tree
- fuzzy file open

#### Tier 2: Can Be Rough

- rename
- code actions
- formatting
- references
- symbols
- project search
- git gutter

#### Tier 3: Should Wait

- full Vim parity
- full Emacs parity
- code lens
- inlay hints
- minimap polish
- snippet engine depth
- advanced multicursor behavior

Right now the app is pulling Tier 3 into the same arena before Tier 1 is boringly reliable.

---

## Finding 8: Syntax Parsing Model Is Expensive

Relevant area:

- `src/editor/mod.rs:205`

The current parse path copies the active buffer to a full string:

```rust
let source = self.buffers[self.active].text();
```

Then it spawns a parser thread with a fresh `SyntaxEngine`.

That is fine as a guardrail against blocking the UI, but it is not a scalable parser architecture.

### Why This Matters

For large files:

- copying full source is expensive
- spawning threads repeatedly is expensive
- reconstructing parser state is expensive
- parse results can race user edits
- incremental parsing loses some value if every parse starts from copied full text

There is generation tracking, which is good. But the architecture is still heavy.

### Recommended Fix

Move toward:

- persistent parser worker
- per-buffer parse job queue
- cancellation/coalescing
- full parse on open
- incremental parse after edits
- hard file-size thresholds
- visible "syntax disabled for large file" state

---

## Finding 9: Word Wrap and Rendering Need Performance Budgets

Relevant area:

- `src/ui/editor_view.rs:115`

Word wrap computes wrap rows during rendering. That is dangerous because wrapping can become expensive with large files or long lines.

The app has performance guards, but it needs explicit budgets:

- max frame time for terminal idle
- max frame time during output flood
- max frame time editing a 10k-line file
- max frame time editing a 50k-line file
- max frame time with word wrap
- max frame time with diagnostics
- max frame time with effects on
- max memory for scrollback
- max memory for editor buffers

### Recommended Fix

Add benchmark scenarios:

- 100k lines terminal output
- 10MB log file
- minified JS file
- Rust project open
- continuous `yes` output
- LSP indexing
- effects enabled
- effects disabled

Then make performance regressions visible.

---

## Finding 10: Workspace Restore Is Half-Implemented

Relevant files:

- `src/workspace_store.rs`
- `src/main.rs`

`src/workspace_store.rs:135` defines `load_last_session`, and `src/main.rs` saves a session snapshot on close. But startup restore does not appear wired.

That is a credibility issue because the README claims session auto-save on close.

### Recommended Fix

Either:

- implement restore on startup, or
- remove the claim until it exists.

If implemented, restore needs:

- missing project handling
- missing file handling
- stale paths
- theme not found
- terminal tab restore semantics
- clear user feedback

---

## Finding 11: Docs and Code Drift

Examples:

- Docs say padding defaults are `2.0`.
- Code defaults are `20.0` and `70.0`.
- README says 11 languages; TOML is detected but parser registration is skipped.
- Workspace/session language implies restore, but restore appears incomplete.

Small doc mismatches matter because they signal that the codebase is moving faster than its truth layer.

### Recommended Fix

Add a "truth audit" pass:

- README claims
- config docs
- keyboard shortcut table
- supported languages
- workspaces
- session restore
- platform support
- known limitations

If something is partial, call it partial.

---

## Finding 12: The Product Surface Is Too Wide

Current major product surfaces:

- terminal
- editor
- explorer
- command palette
- task runner
- themes
- effects
- background gallery
- sketch pad
- stacker
- home screen
- workspace manager
- settings
- keybinding presets
- macOS native menu

This is a lot for a project that still has save-error gaps and central state issues.

### Senior Review Take

The app needs fewer features, not more.

Specifically, it needs:

- fewer panels
- fewer one-off state flows
- fewer "almost done" features
- fewer claims
- more reliability
- more tests
- more clear ownership

The honest roast:

`llnzy` currently looks like it keeps finding new rooms to add to the house before finishing the foundation inspection.

That is normal for creative tools. It is also exactly how creative tools become unmaintainable.

---

## UX Critique

The app has a strong aesthetic direction, but the UX hierarchy is unclear.

### Good UX Ideas

- terminal-first workflow
- bottom footer for tool switching
- home screen with recent projects
- prompt bar for terminal/editor
- theme presets
- project tree bumper
- click terminal file paths into editor
- task runner creates terminal tabs

### UX Problems

- The footer, sidebar, overlays, tab bar, settings, and singleton tabs blur navigation models.
- It is not always obvious whether a tool is an overlay, a tab, a persistent mode, or a panel.
- The code editor is visually subordinate to the terminal architecture, but feature-wise it wants to be first-class.
- Stacker and Sketch are interesting, but their relationship to coding workflows needs sharper definition.
- Effects can become the product story, which is risky if core reliability is not yet solved.

### Recommended UX Direction

Pick one mental model:

#### Option A: Terminal With Attached Tools

Terminal is primary. Editor is lightweight. Sketch/Stacker are utility tabs. This is credible and distinctive.

#### Option B: Native IDE

Editor and terminal are peers. Requires much more architecture, file safety, LSP reliability, project model, and extension story.

#### Option C: Personal Dev Workbench

Everything is a workspace surface. Requires a command bus and state model strong enough to unify all tools.

Today the app is halfway between all three.

---

## Testing Critique

There are useful tests. But the most dangerous workflows are not yet sufficiently covered.

### Existing Strengths

- terminal emulation tests
- PTY roundtrip tests
- input encoding tests
- config parsing tests
- stacker tests
- workspace serialization tests
- task detection tests
- editor buffer/cursor/history tests

### Missing Critical Tests

Add tests for:

- save failure keeps tab open
- close window with multiple modified buffers and save failure
- external file modification prompt
- LSP server unavailable behavior
- LSP request timeout behavior
- server-initiated request response behavior
- terminal tab exit behavior
- workspace restore with missing files
- config hot reload while settings open
- file rename/delete through sidebar
- large file open threshold
- word wrap performance guard
- project finder cap behavior
- terminal mouse reporting correctness
- bracketed paste correctness
- IME text entry into editor and terminal

Also add screenshot/interaction tests if possible for UI-critical flows.

---

## Security and Safety Critique

This is a local developer tool, but it still needs a threat model.

Areas to consider:

- running task commands from project files
- opening paths from terminal output
- URL click behavior
- file path parsing from terminal text
- shell spawning behavior
- config-specified shell
- loading custom background images
- importing/exporting prompts
- workspace files stored under config

Specific concern:

Task execution builds a command string and writes it into a shell tab. That is convenient, but commands and arguments should be escaped or represented more safely. It is not immediately catastrophic because tasks are local project-driven, but anything that converts structured command arrays into shell strings deserves scrutiny.

---

## Release Readiness

This app is not ready to be marketed as a general-purpose editor or terminal replacement.

It is ready to be described as:

> Active personal project. Works for the author. Expect rough edges.

That matches the README and is honest.

Before broader users, minimum release hardening should include:

- save error safety
- crash reporting path
- PTY lifecycle cleanup
- no silent data loss
- session restore truthfulness
- LSP timeout behavior
- clear unsupported platform story
- known limitations document
- versioned config migration plan
- smoke test checklist

---

## Scorecard

These are intentionally harsh.

| Area | Score | Notes |
|---|---:|---|
| Terminal architecture | 7/10 | Good foundation, needs lifecycle hardening. |
| Terminal correctness | 6/10 | Uses strong emulator, needs broader conformance and exit behavior. |
| Renderer | 7/10 | Real GPU pipeline, needs performance budgets. |
| Visual identity | 8/10 | Distinctive and memorable. |
| Editor foundation | 6/10 | Good primitives, too much surface too early. |
| Editor trustworthiness | 4/10 | Save and LSP risks are serious. |
| LSP architecture | 4/10 | Useful, but blocking and incomplete request handling. |
| UI architecture | 3/10 | Large mutable state object and render-side effects. |
| Product focus | 4/10 | Interesting but diffuse. |
| Testing | 5/10 | Good start, missing critical workflow tests. |
| Release readiness | 3/10 | Personal daily driver, not broad release. |
| Long-term maintainability | 4/10 | Needs subsystem separation soon. |

Overall:

> Strong prototype, weak spine.

---

## P0 Roadmap: Trust Before Features

These should happen before major new feature work.

### P0.1 Save Safety

Fix every path where save errors are ignored.

Requirements:

- save failure keeps file open
- user sees clear error
- modified state remains
- close operation aborts
- logs include path and error
- tests cover tab close and window close

### P0.2 PTY Lifecycle

Make terminal session process state explicit.

Requirements:

- retain child/process identity
- expose exit status
- do not auto-close by default
- show exited-state UI
- support close/restart action
- tests for exit behavior

### P0.3 LSP Nonblocking Baseline

Remove UI-thread blocking for server operations.

Requirements:

- no `block_on` in user-interaction path
- request timeout
- server unavailable state
- server stderr captured
- server request response path

### P0.4 Command Bus

Introduce typed commands instead of pending option fields.

Requirements:

- UI emits commands
- main loop applies commands
- command results are explicit
- no feature-specific action fields for every new UI surface

### P0.5 Session Restore Truth

Either implement restore or remove the claim.

Requirements:

- restore last project
- restore file tabs
- restore singleton tabs
- handle missing paths
- visible restore errors

---

## P1 Roadmap: Architecture Cleanup

### P1.1 Split `main.rs`

Move logic into:

- `app/controller.rs`
- `app/commands.rs`
- `app/tabs.rs`
- `app/window.rs`
- `app/input.rs`
- `app/session_restore.rs`

Goal: `main.rs` should be mostly event loop wiring.

### P1.2 Split `UiState`

Move feature state into feature-owned modules:

- `ui/shell_state.rs`
- `ui/sidebar_state.rs`
- `ui/editor_state.rs`
- `ui/stacker_state.rs`
- `ui/sketch_state.rs`
- `ui/settings_state.rs`

Goal: `UiState` should coordinate egui integration, not own the whole product.

### P1.3 Stabilize Editor Core

Focus on:

- reliable text edit operations
- save/reload conflicts
- search/replace
- selection
- undo/redo
- large file behavior
- syntax highlighting fallback

Postpone fancy editor features until these are dull and reliable.

### P1.4 Performance Harness

Add repeatable performance scenarios.

Measure:

- terminal output flood
- large file open
- large file scroll
- editor typing latency
- effects on/off
- LSP active/inactive
- word wrap

---

## P2 Roadmap: Product Clarity

### P2.1 Decide the Product Promise

Pick one:

- terminal-first dev environment
- full native IDE
- personal dev workbench

Write the README around that.

### P2.2 Reduce Feature Claims

Every public claim should map to implemented, tested behavior.

If partial, say partial.

### P2.3 Make Stacker and Sketch Earn Their Place

These tools are interesting, but they need stronger workflow integration.

Examples:

- attach sketches to workspaces
- attach prompts to projects
- send prompt to active terminal
- insert prompt into editor
- command palette integration
- export workspace notes

Otherwise they feel like side quests.

---

## The "Roast" Version

`llnzy` is what happens when a very capable engineer says "I can build my own terminal" and then three weeks later accidentally starts rebuilding VS Code, Warp, Excalidraw, a prompt manager, and a shader toy in the same process.

The terminal is the adult in the room. The renderer has taste. The editor has ambition. The UI state has seen things. The main loop is doing five jobs and pretending that is a lifestyle choice.

The app has the energy of a brilliant prototype sprint. It does not yet have the calm, boring reliability of a tool people should trust with a workday.

That is fine. But the next phase cannot be "add debugger" or "add AI chat" or "add more themes." The next phase has to be making the existing app less likely to surprise its author.

The best version of `llnzy` is not the one with the longest feature list. It is the one where the terminal never lies, the editor never loses work, and the architecture makes adding features feel routine instead of heroic.

---

## Final Verdict

`llnzy` is promising because the hard parts are being attempted for real. It is risky because too many hard parts are being attempted at once.

The path forward is not to stop being ambitious. The path forward is to become more disciplined:

- terminal correctness first
- file safety first
- async LSP first
- command architecture first
- performance budgets first
- product identity first

After that, the app can grow.

Until then, the honest status is:

> A beautiful, terminal-first personal dev environment with serious foundations and serious architectural debt.

