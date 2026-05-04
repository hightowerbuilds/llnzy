# External Command Handoff Contract

Date: 2026-05-04

This document defines the contract external tools should eventually use when
they need to target LLNZY text surfaces such as Stacker and the code editor.
The immediate reason for the contract is Wispr Flow and other voice dictation
tools. The broader reason is that shortcuts, toolbar buttons, native menus,
command palette entries, WebView text input, accessibility tools, and future
external integrations should not each invent their own mutation path.

The contract is intentionally app-first. The first implementation should be an
internal Rust dispatcher. Public IPC can come later after the routing, security,
targeting, and result semantics are stable.

---

## Current State

Stacker now uses a WebView-backed native browser textarea for live text entry.
That is the correct surface for Wispr Flow because voice dictation tools already
understand native browser text controls. Rust still owns the durable Stacker
document state through `StackerDocumentEditor`, prompt persistence, dirty draft
tracking, saved prompt management, queue state, and formatting commands.

The code editor already has a command dispatcher centered around
`EditorCommand` and command palette `CommandId` values. Stacker has a parallel
registry centered around `StackerCommandId`, `StackerEditorCommand`, and
`StackerDocumentEditor`.

The handoff contract should join those patterns under a shared app-level
command envelope.

---

## Goals

1. Provide one command path for text insertion, selection replacement,
   formatting, copy, paste, select all, undo, and redo.
2. Let the active text surface decide how a command mutates state.
3. Preserve native text entry behavior where it is already working, especially
   the Stacker WebView textarea.
4. Avoid using the clipboard as the primary data transport for external tools.
5. Support joined tabs and split panes without guessing the wrong target.
6. Return structured success or failure information to callers.
7. Keep external execution narrow and permissioned before any public IPC is
   exposed.

---

## Non-Goals

- This is not a shell automation API.
- This is not a general plugin system.
- This is not a remote control server.
- This is not a replacement for native keyboard, IME, paste, or accessibility
  input.
- This should not allow arbitrary terminal commands without a separate,
  explicit terminal execution contract.

---

## Command Envelope

Every handoff command should be represented as a structured envelope:

```rust
pub struct ExternalCommand {
    pub id: CommandRequestId,
    pub source: CommandSource,
    pub target: CommandTarget,
    pub action: ExternalAction,
    pub focus_policy: FocusPolicy,
    pub selection_policy: SelectionPolicy,
}
```

The exact type names can change, but these concepts should remain.

### Command Source

The source identifies who is asking:

- `KeyboardShortcut`
- `CommandPalette`
- `Toolbar`
- `NativeMenu`
- `WebView`
- `Accessibility`
- `VoiceDictation`
- `ExternalTool`

The source matters for diagnostics and policy. For example, a keyboard shortcut
can act immediately on the focused surface. A future external tool may need a
permission check or a user-visible setting before it can mutate text.

### Command Target

The target must be explicit enough for joined tabs:

- `FocusedSurface`
- `ActiveTab`
- `TabId(TabId)`
- `Pane { tab_id: TabId }`
- `Surface(SurfaceKind)`

`FocusedSurface` should be the default for voice dictation. Voice tools operate
best when they target the native control that currently owns focus. In a joined
Terminal + Stacker layout, this prevents dictation from being sent to Stacker
unless the Stacker textarea is the active/focused pane.

`ActiveTab` is useful for command palette actions. `TabId` and `Pane` are useful
for future automation and tests.

### External Actions

The shared action enum should cover common text editor behavior:

```rust
pub enum ExternalAction {
    InsertText { text: String },
    ReplaceSelection { text: String },
    SetSelection { start: usize, end: usize },
    SelectAll,
    Copy,
    Paste,
    Undo,
    Redo,
    ApplyFormatting(FormattingCommand),
    Save,
    Submit,
}
```

The initial implementation does not need every action for every surface. It
does need consistent failure semantics when a surface does not support an
action.

### Focus Policy

Focus behavior must be explicit:

- `Preserve`: do not move focus.
- `FocusTarget`: focus the target before command execution.
- `FocusAfter`: execute, then focus the target.
- `NoFocus`: execute without any native focus changes.

For Wispr Flow, `Preserve` should be preferred. The Stacker cursor issue showed
that unnecessary focus calls can fight the native text control.

### Selection Policy

Selection behavior should be explicit:

- `UseCurrentSelection`
- `ReplaceCurrentSelection`
- `SetSelectionBefore { start, end }`
- `Append`
- `Prepend`

For Stacker and the code editor, indexes should be character indexes at the app
boundary. Surface adapters can translate to UTF-16 or byte offsets internally.
The Stacker WebView already needs this distinction because browser textarea
selection is UTF-16 while `StackerDocumentEditor` uses character indexes.

---

## Result Envelope

Every command should return a structured result:

```rust
pub struct ExternalCommandResult {
    pub id: CommandRequestId,
    pub status: CommandStatus,
    pub target: Option<ResolvedTarget>,
    pub changed: bool,
    pub selection: Option<TextSelection>,
    pub message: Option<String>,
}
```

Statuses should include:

- `Handled`
- `NoOp`
- `UnsupportedAction`
- `NoTarget`
- `TargetNotEditable`
- `PermissionDenied`
- `InvalidPayload`
- `InternalError`

This gives future tools enough information to retry, show a useful message, or
fall back to native input.

---

## Surface Contracts

### Stacker

Stacker supports:

- `InsertText`
- `ReplaceSelection`
- `SetSelection`
- `SelectAll`
- `Copy`
- `Paste`
- `Undo`
- `Redo`
- `ApplyFormatting`
- `Save` for the active saved prompt or scratch prompt flow

Stacker implementation rules:

1. Live typing and dictation should continue through the WebView textarea when
   it is focused.
2. WebView input must sync back into `StackerDocumentEditor`.
3. Rust-side commands must update `StackerDocumentEditor` first, then sync the
   resulting text and selection back into the WebView.
4. Formatting commands must continue to use `StackerCommandId` and
   `StackerEditorCommand`.
5. Dirty draft tracking must update after every text mutation.
6. Saved prompt edit/delete behavior must stay modal-safe and queue-safe.

### Code Editor

The code editor supports:

- `InsertText`
- `ReplaceSelection`
- `SetSelection`
- `SelectAll`
- `Copy`
- `Paste`
- `Undo`
- `Redo`
- `Save`
- existing editor commands exposed through `EditorCommand`

Code editor implementation rules:

1. Commands should resolve to the active code buffer before mutating.
2. Existing `EditorCommand` dispatch should remain the command backbone.
3. Clipboard commands may still use `arboard`, but direct text insertion should
   not require the clipboard.
4. The result should report whether the buffer changed.
5. Missing buffer, inactive editor, or unsupported command should return a
   structured failure instead of silently falling through to terminal input.

### Terminal

The terminal is not a normal text editor. It supports:

- write text to PTY
- paste text to PTY, including bracketed paste when active
- copy selected terminal text
- select all terminal output

Terminal implementation rules:

1. Terminal text input should continue through PTY write and paste paths.
2. External commands must not treat terminal text like an editable document.
3. Command execution inside the shell is out of scope for this contract.
4. Future shell automation needs a separate, explicit terminal execution
   contract.

---

## Joined Tab Targeting

Joined tabs are the main place where sloppy targeting will create bugs.

Rules:

1. If a native text control has focus, voice dictation should target that
   focused control.
2. If a joined pane has explicit active-pane state, command palette actions
   should target that pane.
3. If there is no focused text surface and no active pane, commands should
   return `NoTarget`.
4. Stacker in a joined pane should expose the same WebView textarea behavior as
   Stacker in a standalone tab.
5. Terminal in a joined pane should keep PTY ownership and should not receive
   Stacker formatting commands.

The remaining Wispr Flow manual check is specifically:

- Terminal + Stacker joined
- Stacker + CodeFile joined
- Git + Stacker joined if the layout permits it

The expected behavior is that dictation lands in Stacker only when the Stacker
textarea is focused.

---

## Security and Permissions

The first implementation should be internal only. Before adding public IPC,
LLNZY should define:

- which local processes can send commands
- whether commands require an app-generated token
- where the local IPC endpoint lives
- how commands are logged
- how users disable external control
- which actions are allowed by default
- whether terminal actions are excluded by default

The safe default is:

- allow native input
- allow internal commands
- keep external IPC disabled until explicitly enabled
- never expose shell execution through this contract

---

## Implementation Roadmap

1. Add shared command model types:
   - command source
   - target
   - action
   - focus policy
   - selection policy
   - result

2. Add an internal dispatcher:
   - resolve target
   - route to Stacker, code editor, or terminal adapter
   - return structured result

3. Add Stacker adapter:
   - wrap current Stacker document operations
   - sync WebView text/selection after Rust-side edits
   - preserve dirty draft tracking

4. Add code editor adapter:
   - wrap active buffer command dispatch
   - add direct insert/replace selection path if missing
   - preserve existing clipboard behavior for copy/cut/paste

5. Add terminal adapter:
   - expose write/paste/copy/select-all behavior only
   - do not expose shell command execution

6. Convert current callers gradually:
   - keyboard shortcuts
   - command palette
   - native menu actions
   - toolbar buttons
   - WebView messages

7. Add tracing:
   - source
   - target resolution
   - action
   - result status
   - changed/no-op

8. Add tests:
   - Stacker insert/replace/select/format
   - code editor insert/replace/select
   - terminal paste routing
   - joined tab target resolution
   - unsupported action failures

9. Only after the internal dispatcher is stable, add optional local IPC.

---

## Acceptance Criteria

- Stacker, code editor, and terminal commands route through explicit surface
  adapters.
- Unsupported actions return structured failures.
- Stacker WebView dictation remains fast and cursor-stable.
- Formatting commands mutate `StackerDocumentEditor` and sync back to WebView.
- Code editor commands continue to use the editor command stack.
- Terminal input still goes through PTY write/paste paths.
- Joined tab command routing chooses the focused/active pane predictably.
- No public external IPC is enabled without an explicit security decision.

