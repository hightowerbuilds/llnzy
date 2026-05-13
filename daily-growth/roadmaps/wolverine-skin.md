# Wolverine Skin

Status: research roadmap

Date: 2026-05-12

## Purpose

Wolverine Skin is the self-healing layer for LLNZY. The goal is not for the app
to silently rewrite itself whenever something goes wrong. The goal is for LLNZY
to remember failures with enough context that an agent can inspect the evidence,
reproduce the problem in a bounded environment, propose a fix, verify the fix,
and leave the app stronger than it was before.

The effect should feel like this:

- LLNZY captures errors, warnings, panics, failed commands, failed saves, LSP
  crashes, terminal lifecycle problems, rendering failures, and recovery events.
- The captured information is structured, durable, privacy-aware, and tied to
  the user action or app subsystem that caused it.
- A user can open an administrative surface that shows what happened, how often
  it happened, whether it is known, and whether an agent can investigate it.
- An agent can read a diagnostic bundle, compare it against the current source,
  tests, recent changes, and prior fixes, then generate a patch plan.
- LLNZY never mutates project files or its own source without explicit user
  approval, visible diffs, and verification results.

The first useful version can be small. LLNZY already has an in-memory
`ErrorLog`, a diagnostics panel, platform-owned log and crash-report directories,
editor recovery snapshots, runtime recovery hooks, LSP lifecycle boundaries, and
local-first security guidance. Wolverine Skin should extend those foundations
rather than inventing a disconnected monitoring system.

## Current LLNZY Foundations

LLNZY already has several pieces that map cleanly onto a self-healing design:

- `src/error_log.rs` provides a thread-safe in-memory log with `Info`, `Warn`,
  and `Error` levels, recent-entry retrieval, counts, and render data for the
  diagnostics panel.
- `src/platform/paths.rs` already names `logs_dir` and `crash_reports_dir`, plus
  app-owned config, data, cache, workspace, export, and theme directories.
- `src/editor/recovery.rs` writes durable editor recovery snapshots into the app
  data directory, uses atomic writes, and avoids losing unsaved buffer content.
- `src/runtime/recovery.rs` periodically saves editor recovery snapshots and
  reports failures back into the error log.
- Runtime command, terminal, tab, drag/drop, save prompt, and session restore
  modules already send important failures into `error_log`.
- The security baseline already states that logs, recovery snapshots, terminal
  scrollback, prompts, config, and diagnostic bundles may contain sensitive user
  data and must not be treated casually.

The missing layer is durability, structure, correlation, reproduction, and
administration. The current error log is useful for immediate debugging inside a
running process, but Wolverine Skin needs a longer-lived medical record for the
app.

## Product Definition

Wolverine Skin is composed of four product surfaces:

1. **Capture:** a structured event recorder that turns runtime failures and
   suspicious states into durable diagnostic records.
2. **Quarantine:** a sandboxed diagnostic area where LLNZY stores redacted event
   trails, state snapshots, crash records, reproduction metadata, and generated
   analysis without mixing them into normal project files.
3. **Clinic:** an administrative UI where the user can inspect incidents,
   choose retention and privacy settings, start agent analysis, review proposed
   patches, and approve or reject repairs.
4. **Reinforcement:** a verified fix workflow that converts one incident into a
   regression test, source patch, release note, or known-issue rule.

The name should stay internal until the behavior is trustworthy. In user-facing
text, the feature can be called "Diagnostics and Repair", "Self-Healing", or
"App Clinic". The phrase "Wolverine Skin effect" is useful in roadmap language
because it captures the intended compounding loop: every injury should leave
stronger tissue behind.

## Design Principles

- **Local-first:** diagnostic capture and analysis should work without sending
  user data to a remote service by default.
- **Explicit repair:** an agent may analyze and propose; the user decides when
  code is changed.
- **Structured over string-only logs:** text messages are useful, but durable
  incident records need typed fields, subsystem labels, IDs, timestamps, and
  causal links.
- **Privacy by default:** never capture full terminal input, full environment
  variables, tokens, private keys, large prompt bodies, or whole project files
  unless the user explicitly exports a bundle with clear warning.
- **Bounded storage:** diagnostic history needs retention limits, compaction,
  and deletion controls.
- **Reproducibility:** the best repair input is not "something broke"; it is a
  minimized timeline that says which subsystem broke, what command was running,
  what state changed before failure, what files were involved, and what test or
  replay can trigger the same condition.
- **No hidden self-modification:** self-healing means evidence-driven repair, not
  unreviewed automatic patching.

## Architecture

### 1. Durable Diagnostic Store

Add a durable store under the app-owned diagnostics area:

```text
<config_dir>/logs/
  current.jsonl
  archived/
  indexes/

<config_dir>/crash-reports/
  panic-<timestamp>-<id>.toml

<data_dir>/wolverine-skin/
  incidents/
  bundles/
  analyses/
  reproductions/
  patches/
  quarantined/
```

The exact root can be refined, but the separation matters:

- Normal log streams belong in `logs_dir`.
- Crash-specific records belong in `crash_reports_dir`.
- Agent-readable diagnostic bundles, reproduction fixtures, analysis output, and
  generated patch drafts should live in a dedicated data directory such as
  `data_dir/wolverine-skin`.

The durable log should use newline-delimited JSON for append-friendly writes and
easy external inspection:

```json
{"schema":1,"event_id":"01HX...","session_id":"...","ts":"2026-05-12T10:30:00Z","level":"error","subsystem":"editor.save","message":"Failed to save buffer","error_kind":"io.permission_denied","surface":"CodeFile","workspace_id":"...","file_id":"hash:...","correlation_id":"..."}
```

The current `ErrorLog` can remain the fast in-memory display buffer, but writes
should flow through a new diagnostic recorder:

```text
Runtime subsystem
  -> DiagnosticEvent
  -> DiagnosticRecorder
  -> in-memory ErrorLog for UI
  -> durable JSONL writer
  -> incident classifier
```

### 2. Event Model

The event model should be explicit enough for agent analysis without requiring
full access to sensitive user content.

Core fields:

- `event_id`: unique sortable ID.
- `session_id`: generated at app startup.
- `correlation_id`: shared across a user action or async operation.
- `timestamp_unix_ms`: wall-clock timestamp.
- `elapsed_ms`: time since app start.
- `level`: trace, info, warn, error, fatal.
- `subsystem`: editor, terminal, renderer, lsp, stacker, config, workspace,
  git, sketch, sidebar, app.lifecycle, platform.
- `surface`: active tab or app surface when available.
- `operation`: save, open_file, close_tab, lsp_request, terminal_spawn,
  render_frame, parse_config, restore_session.
- `message`: human-readable summary.
- `error_kind`: stable category such as io.not_found, io.permission_denied,
  parse.invalid_toml, lsp.process_exit, renderer.device_lost, panic.unwind.
- `source_location`: module and optional line when known.
- `workspace_id`: hashed workspace path or stable workspace identity.
- `file_id`: hashed file path when a file is involved.
- `safe_path_hint`: basename and extension only, unless user enables full paths.
- `counts`: repetition count for deduplicated events.
- `diagnostic_context_id`: link to richer context when captured.

Optional context records can include:

- Active tab kind, tab group shape, and focus target.
- Editor buffer metadata: dirty state, line count, encoding, file extension,
  selection count, cursor positions, and whether a recovery snapshot exists.
- LSP metadata: language ID, server command hash/name, request method, response
  status, exit code, stderr summary, restart count.
- Terminal metadata: shell name, PTY size, process exit status, command title,
  current working directory hash, scrollback line count, alternate-screen state.
- Renderer metadata: backend, surface size, frame timing, effect settings,
  device-loss reason where available.
- Config metadata: config schema version, settings key path, parse/apply phase.
- Git metadata: repository root hash, operation, branch name when safe, command
  exit status.

### 3. Incident Classifier

Raw logs are not enough. Wolverine Skin should group events into incidents.

An incident is a durable object:

```text
Incident
  id
  first_seen
  last_seen
  status
  severity
  fingerprint
  subsystem
  title
  summary
  affected_sessions
  event_ids
  reproduction_status
  analysis_status
  patch_status
```

Fingerprinting should be stable across sessions. Good fingerprint inputs:

- Subsystem.
- Error kind.
- Normalized message template.
- Top source frame or module.
- Operation.
- Involved file extension or surface type.
- LSP method or terminal lifecycle phase if applicable.

Bad fingerprint inputs:

- Full file paths.
- Full terminal commands.
- Full prompt text.
- Absolute timestamps.
- Random IDs.

Incident statuses:

- `new`: seen but not reviewed.
- `known`: reviewed and understood, no patch yet.
- `needs-user-data`: agent needs a larger diagnostic export.
- `reproducible`: a replay or test fixture exists.
- `patch-proposed`: agent produced a candidate fix.
- `verified`: patch has passed relevant checks.
- `applied`: user accepted and applied the fix.
- `dismissed`: user decided not to act.
- `blocked`: repair requires external dependency, OS permission, or product
  decision.

### 4. Context Capture Without Overcapture

The strongest version of Wolverine Skin captures just enough context to debug
without becoming spyware against the user's own machine.

Default capture should include:

- App version, build profile, git commit if available.
- OS, architecture, renderer backend, screen scale, and app path.
- LLNZY config schema version and the names of non-default settings, not
  necessarily their full values.
- Workspace identity hashes.
- File extension, basename, size, and line count for active editor files.
- Command names and exit codes for LLNZY-owned commands, with arguments redacted
  by default.
- LSP method names, server process status, and bounded stderr summaries.
- Stack traces for panics.
- Recent diagnostic events around the failure.
- Recovery snapshot existence and content hash.

Default capture should avoid:

- Full source file contents.
- Full terminal scrollback.
- Full terminal input.
- Full prompt bodies.
- Full environment variables.
- Tokens, auth headers, SSH keys, private keys, cookies, API keys, and secret
  config values.
- Large binary assets.

The administrative UI can offer an explicit "Include more context" export flow.
That flow should show a clear checklist of what will be included and should
produce a local bundle the user can inspect before sending or letting an agent
read it.

### 5. Sandboxed Reproduction Area

The sandbox is a local workspace for evidence and repair attempts. It should not
be the user's project directory, and it should not automatically run arbitrary
commands.

The reproduction area should contain:

- Redacted diagnostic bundle.
- Minimized app config needed to trigger the issue.
- Synthetic project fixture when possible.
- Captured app state summary.
- Replay script or harness where possible.
- Agent notes.
- Proposed tests.
- Proposed patch.
- Verification logs.

For LLNZY, likely reproduction harnesses include:

- Unit tests for editor buffer, cursor, recovery, config, terminal grid, LSP
  request shaping, tab-group models, sketch serialization, and sidebar move
  planning.
- Integration tests for PTY roundtrip and terminal emulation.
- App-level replay fixtures for user commands once command dispatch is fully
  structured.
- Rendering verification snapshots for frame adapter and diagnostics panel
  behavior.
- Manual smoke checklist output when no automated reproduction is feasible.

Sandbox rules:

- Agent analysis reads bundles and source code.
- Patch generation writes only to a dedicated patch draft or branch.
- Test execution uses an approved command list.
- External network use is disabled unless the user explicitly allows it.
- Destructive file operations are blocked unless the user approves a clear plan.
- The user can delete any incident, bundle, analysis, or patch.

### 6. Agent Analysis Workflow

The agent should work from evidence to patch, with a clear chain:

1. **Read incident:** load the grouped incident, recent events, crash report, and
   safe context.
2. **Map subsystem:** identify likely source modules and ownership boundaries.
3. **Compare:** inspect current code, recent related changes, existing tests,
   and similar incidents.
4. **Hypothesize:** produce one or more likely root causes with confidence.
5. **Reproduce:** create or identify a test, replay, or manual scenario.
6. **Patch:** make a minimal source change.
7. **Verify:** run focused tests first, then broader checks if the patch touches
   shared code.
8. **Explain:** present root cause, changed files, verification results, and
   residual risk.
9. **Reinforce:** add regression coverage or a durable known-issue rule.

The agent should not skip directly from error text to code edits. The value of
Wolverine Skin is the comparative work: what changed, what failed, how often it
failed, which code paths own the operation, and which test proves it will not
come back.

### 7. Patch Proposal Model

Patch proposals should be first-class records:

```text
PatchProposal
  id
  incident_id
  created_at
  agent_id_or_tool
  status
  summary
  root_cause
  changed_files
  diff_path
  tests_run
  verification_result
  rollback_notes
  user_decision
```

Statuses:

- `draft`
- `needs-review`
- `approved-to-apply`
- `applied`
- `rejected`
- `superseded`
- `failed-verification`

The user-facing flow should be:

1. User opens an incident in the Clinic.
2. User clicks "Analyze".
3. Agent produces a root-cause report and patch proposal.
4. User reviews summary, changed files, and diff.
5. User clicks "Run Verification".
6. LLNZY shows test output and pass/fail status.
7. User clicks "Apply Patch" only after review.

For a developer build, "Apply Patch" can mean applying changes to the current
working tree. For a packaged app, repair may instead produce a local report,
open an issue, or queue the fix for the developer source tree. A shipped end-user
app should not rewrite its signed binary or install untrusted code.

## Administrative UX

The user should administer Wolverine Skin from a dedicated settings or
diagnostics surface. This should be practical and quiet, closer to a maintenance
console than a marketing feature.

### Clinic Dashboard

Primary sections:

- Incident list with severity, subsystem, first seen, last seen, frequency, and
  status.
- Session timeline showing recent warnings, errors, crashes, repairs, and
  recovery actions.
- Storage panel showing log size, bundle size, retention policy, and cleanup
  controls.
- Privacy panel showing what is captured by default and what requires explicit
  export approval.
- Repair queue showing analyses in progress, patch proposals, verification
  state, and applied fixes.

Useful filters:

- Severity.
- Subsystem.
- Current session only.
- Repeated incidents.
- Needs attention.
- Patch proposed.
- Verified.
- Dismissed.

Useful actions:

- View incident.
- Analyze with agent.
- Create diagnostic bundle.
- Redact and export bundle.
- Mark as known.
- Dismiss.
- Delete.
- Run verification.
- Review patch.
- Apply patch.
- Open changed files.
- Copy incident summary.

### Incident Detail

The detail view should answer:

- What happened?
- When did it first and last happen?
- How many times did it happen?
- What was the user doing?
- Which subsystem owns it?
- Is there a recovery snapshot?
- Is data at risk?
- Is it reproducible?
- Has an agent analyzed it?
- Is there a proposed fix?
- What tests or checks prove the fix?

Suggested layout:

- Header: title, severity, status, subsystem.
- Timeline: related events around the failure.
- Context: safe metadata and redaction notices.
- Reproduction: current state and available replay/test.
- Agent Notes: root cause, confidence, next action.
- Patch: summary, files, diff, verification.
- Administration: retention, delete, export, mark known.

### Settings

Wolverine Skin settings should include:

- Enable durable diagnostics: on by default for warnings/errors, off or sampled
  for verbose traces.
- Retention: for example 7 days, 30 days, 90 days, or manual.
- Max storage: cap logs and bundles.
- Capture level: minimal, balanced, verbose.
- Include full paths: off by default.
- Include terminal command arguments: off by default.
- Include LSP stderr: bounded and redacted by default.
- Include prompt bodies: off by default.
- Agent analysis: manual only by default.
- Auto-create regression task: optional.
- Safe mode: disable agent analysis, extensions, shaders, external commands, and
  nonessential background work.

## Implementation Phases

### Phase 0: Tighten The Current Error Log

Objective: make the current diagnostic panel and error log a reliable source of
truth for immediate failures.

- [ ] Audit all `error_log.error`, `warn`, and `info` calls for subsystem and
  operation clarity.
- [ ] Replace vague strings with stable, actionable messages.
- [ ] Add helper constructors or macros for common subsystems so new events are
  not free-form everywhere.
- [ ] Add source module or operation labels where useful.
- [ ] Keep the existing diagnostics panel working exactly as it does now.

Exit criteria:

- The visible diagnostics panel still works.
- Most runtime failures say which subsystem and operation failed.
- No sensitive content is newly exposed.

### Phase 1: Durable JSONL Diagnostics

Objective: write warning and error events to disk with bounded retention.

- [ ] Add `DiagnosticEvent`, `DiagnosticRecorder`, and `DiagnosticWriter`.
- [ ] Route `ErrorLog` writes through the recorder.
- [ ] Write JSONL logs under `paths.logs_dir`.
- [ ] Rotate logs by size and session.
- [ ] Add retention cleanup.
- [ ] Add tests for event serialization, rotation, redaction, and failure-safe
  writer behavior.

Exit criteria:

- LLNZY can restart and still expose recent durable diagnostic history.
- A disk write failure does not crash the app.
- The in-memory panel and durable log agree on counts and severity for current
  session events.

### Phase 2: Incident Grouping

Objective: turn repeated low-level events into actionable incident records.

- [ ] Add incident fingerprinting.
- [ ] Persist incidents under `data_dir/wolverine-skin/incidents`.
- [ ] Group repeated events across sessions.
- [ ] Add incident status transitions.
- [ ] Add a simple incident list view in diagnostics or settings.
- [ ] Add tests for stable fingerprinting and deduplication.

Exit criteria:

- Repeated failures are grouped.
- The user can distinguish one-off noise from recurring app wounds.
- Incidents can be dismissed, deleted, or marked known.

### Phase 3: Context Bundles

Objective: create agent-readable diagnostic bundles without overcapturing user
data.

- [ ] Define `DiagnosticBundle` schema.
- [ ] Include app version, platform, safe config metadata, incident summary,
  recent events, and subsystem context.
- [ ] Add redaction rules for paths, secrets, prompt text, terminal input, and
  environment variables.
- [ ] Add explicit export flow for larger bundles.
- [ ] Store bundles under `data_dir/wolverine-skin/bundles`.
- [ ] Add tests for redaction and bundle schema compatibility.

Exit criteria:

- An agent can understand the incident without needing the full user project.
- The user can inspect what will be shared or analyzed.
- Sensitive fields are omitted or redacted by default.

### Phase 4: Reproduction Harnesses

Objective: give agents a way to prove bugs instead of guessing.

- [ ] Add command/event correlation IDs around user actions.
- [ ] Capture lightweight replay metadata for editor, config, LSP, terminal, and
  tab operations.
- [ ] Build minimal reproduction fixture generation for pure model modules.
- [ ] Connect incidents to existing unit and integration tests.
- [ ] Add a manual reproduction checklist template for issues that cannot be
  replayed yet.

Exit criteria:

- Common editor/config/tab/recovery issues can produce a test or fixture.
- Incidents can record whether reproduction is automatic, manual, blocked, or
  unknown.
- Agent reports clearly separate proven facts from hypotheses.

### Phase 5: Agent Clinic

Objective: let a user ask an agent to analyze an incident from inside LLNZY.

- [ ] Add an admin action that starts an analysis job.
- [ ] Feed the agent the incident, diagnostic bundle, relevant source paths, and
  available tests.
- [ ] Store analysis output under `data_dir/wolverine-skin/analyses`.
- [ ] Show root cause, confidence, proposed files to inspect, and recommended
  verification.
- [ ] Keep analysis read-only until the user explicitly requests a patch.

Exit criteria:

- The agent can produce useful root-cause reports from local evidence.
- The user can see what evidence the agent used.
- Analysis does not mutate source or project files.

### Phase 6: Patch Proposals And Verification

Objective: convert selected analyses into reviewed, tested repairs.

- [ ] Add patch proposal records.
- [ ] Generate diffs into `data_dir/wolverine-skin/patches` or a dedicated branch
  in developer builds.
- [ ] Show changed files and diff before application.
- [ ] Run focused verification commands.
- [ ] Attach verification logs to the patch proposal.
- [ ] Apply only after user approval.
- [ ] Add rollback notes or a reverse patch where practical.

Exit criteria:

- A repeated bug can become a proposed patch with test results.
- The user remains in control of source changes.
- Applied fixes leave regression coverage or a known-issue rule.

### Phase 7: Reinforcement Loop

Objective: make LLNZY stronger each time a wound is healed.

- [ ] Track whether incidents stop recurring after a fix.
- [ ] Mark fixes as effective only after clean sessions or passing regression
  checks.
- [ ] Promote recurring incident fingerprints into health checks.
- [ ] Add release-note snippets for important repairs.
- [ ] Add a maintenance dashboard showing repaired, recurring, and blocked
  incident trends.

Exit criteria:

- LLNZY can answer "what did we learn from this failure?"
- Fixes are measured against recurrence, not just patch application.
- The app accumulates regression knowledge over time.

## Subsystem-Specific Notes

### Editor

The editor is the best first subsystem because it already has model tests and
recovery snapshots. Wolverine Skin should capture dirty state, recovery snapshot
status, file extension, buffer size, cursor/selection metadata, save operation
results, and file watcher events. It should not capture full file contents by
default.

Good first repairs:

- Failed save paths.
- Recovery snapshot write/read failures.
- Incorrect dirty-state transitions.
- Buffer remap issues after sidebar move/rename.
- Cursor or selection panics.

### Terminal

Terminal diagnostics need special privacy handling. Terminal input and scrollback
can contain secrets. Default capture should focus on lifecycle and structure:
spawn failure, shell path, PTY size, resize events, exit code, active alternate
screen, scrollback size, and renderer state. Full command arguments and
scrollback should require explicit user inclusion.

Good first repairs:

- Terminal spawn failures.
- Close/restart lifecycle bugs.
- Resize and grid consistency failures.
- Link parsing or title parsing panics.

### LSP

LSP failures are ideal for incident grouping because servers crash or time out
in repeated patterns. Capture language ID, server command name, request method,
response status, timeout, exit code, restart count, and bounded stderr summary.
Avoid logging whole documents or edits unless the user explicitly exports a
bundle.

Good first repairs:

- Stale response handling.
- Server restart behavior.
- Request timeout classification.
- Diagnostic routing to the wrong buffer.

### Renderer

Renderer issues need timing and device metadata. Capture frame size, renderer
backend, device-loss reason, active effects, adaptive quality state, and surface
resize events. Avoid storing screenshots by default; screenshots should be
manual exports.

Good first repairs:

- Device lost handling.
- Stale text artifacts.
- Diagnostics panel layout regressions.
- Frame adapter panics under resize or split changes.

### Config And Settings

Config issues are usually reproducible with small fixtures. Capture config
schema version, key path, parse/apply phase, and redacted value type. Do not
store raw secrets or large user paths by default.

Good first repairs:

- Invalid config parse errors with poor recovery.
- Settings that apply to the wrong surface.
- Theme/background asset load failures.
- Keybinding mapping conflicts.

### Stacker

Stacker contains user prompts and draft text, so default diagnostics should
capture command names, queue lengths, saved prompt IDs, and mutation operation
results without full prompt bodies. Explicit bundle export can include selected
prompt content if the user chooses it.

Good first repairs:

- Queue persistence failures.
- Draft save/load problems.
- External input routing mistakes.
- Command dispatch inconsistencies.

## Data Schema Sketches

### Diagnostic Event

```rust
pub struct DiagnosticEvent {
    pub schema: u16,
    pub event_id: String,
    pub session_id: String,
    pub correlation_id: Option<String>,
    pub timestamp_unix_ms: u64,
    pub elapsed_ms: u64,
    pub level: DiagnosticLevel,
    pub subsystem: DiagnosticSubsystem,
    pub operation: String,
    pub message: String,
    pub error_kind: Option<String>,
    pub source_location: Option<SourceLocation>,
    pub context_id: Option<String>,
    pub redaction: RedactionSummary,
}
```

### Incident

```rust
pub struct Incident {
    pub schema: u16,
    pub id: String,
    pub fingerprint: String,
    pub title: String,
    pub severity: IncidentSeverity,
    pub status: IncidentStatus,
    pub subsystem: DiagnosticSubsystem,
    pub first_seen_unix_ms: u64,
    pub last_seen_unix_ms: u64,
    pub occurrence_count: u64,
    pub affected_session_count: u64,
    pub event_ids: Vec<String>,
    pub latest_bundle_id: Option<String>,
    pub latest_analysis_id: Option<String>,
    pub latest_patch_id: Option<String>,
}
```

### Diagnostic Bundle

```rust
pub struct DiagnosticBundle {
    pub schema: u16,
    pub id: String,
    pub incident_id: String,
    pub created_at_unix_ms: u64,
    pub capture_level: CaptureLevel,
    pub app: AppDiagnosticInfo,
    pub platform: PlatformDiagnosticInfo,
    pub incident: IncidentSummary,
    pub events: Vec<DiagnosticEvent>,
    pub contexts: Vec<DiagnosticContext>,
    pub redactions: Vec<RedactionNotice>,
}
```

## Safety And Governance

Wolverine Skin touches sensitive surfaces. It should follow the existing
security-governance baseline and add stricter local rules:

- Diagnostic capture must be inspectable.
- Diagnostic deletion must be available.
- Export must be explicit.
- Agent analysis must say what it read.
- Patch application must show a diff.
- Verification output must be retained with the patch proposal.
- Remote upload or network analysis must not be assumed.
- Packaged builds must not self-modify signed application code.
- Developer builds may patch the working tree only with explicit approval.

The strongest near-term implementation is local and developer-focused:

- Capture locally.
- Analyze locally.
- Patch local source during development.
- Convert discoveries into tests and roadmap items.
- Keep packaged end-user repair limited to diagnostics, recovery, support
  bundles, and update recommendations until the update chain is signed and
  trusted.

## Success Criteria

Wolverine Skin is working when:

- LLNZY can show durable incident history across restarts.
- Repeated failures are grouped and searchable.
- A user can tell whether an error is new, known, recurring, fixed, or blocked.
- An agent can inspect a bundle and produce a specific root-cause report.
- At least one class of bug can be reproduced from captured context.
- Patch proposals include diffs and verification output.
- Fixes add tests or durable checks where practical.
- Sensitive user data is not captured by default.
- The user can delete diagnostics and control retention.
- The system improves LLNZY without making the app feel risky or autonomous in
  the wrong way.

## First Build Recommendation

The first build should be intentionally narrow:

1. Add durable JSONL diagnostics behind the existing `ErrorLog`.
2. Persist warning/error events under `logs_dir` with rotation.
3. Add incident fingerprinting for repeated runtime errors.
4. Add a simple admin list of incidents in the diagnostics surface.
5. Add diagnostic bundle creation for one subsystem, preferably editor recovery
   or config loading.
6. Add one agent-readable bundle format and one regression-test workflow.

That version would not yet be fully self-healing, but it would create the skin:
the app would remember injuries, classify them, preserve the right evidence, and
give an agent a practical place to start repairing the tissue.

## Addendum: Extension Observability And Developer Repair

Wolverine Skin should become one of the trust foundations for LLNZY's future
extensibility system. If LLNZY becomes a place where developers can build their
own extensions, commands, themes, language helpers, panels, automations, or
workspace tools, then the app needs a disciplined way to observe extension
failures without letting those failures damage the host application or disappear
into invisible logs.

The extension story should be:

- A developer builds an extension on top of LLNZY.
- The extension runs inside a constrained extension host with a clear manifest,
  permissions, resource limits, and crash isolation.
- Every extension lifecycle event, permission denial, runtime error, command
  failure, timeout, rejected API call, schema mismatch, and crash can flow into
  Wolverine Skin as structured diagnostic evidence.
- The extension developer can open a Clinic view filtered to their extension,
  inspect failures, create diagnostic bundles, ask an agent to analyze the
  extension behavior, and produce a fix or compatibility note.
- The app user can see whether a problem belongs to LLNZY core, a first-party
  extension, or a third-party extension.
- LLNZY can disable, quarantine, or safe-mode a broken extension without taking
  down the whole app.

This is important because extensibility changes the product promise. LLNZY would
not only be an app users operate; it would become a platform other developers
build against. Wolverine Skin gives that platform a nervous system. It lets
developers see what went wrong, lets users understand which component is
misbehaving, and lets the LLNZY maintainers distinguish host bugs from extension
bugs.

### What Wolverine Skin Should Cover For Extensions

Extension diagnostics should cover the full extension lifecycle:

- **Installation:** manifest parse failures, incompatible LLNZY version,
  missing files, invalid signatures once signing exists, unsupported API
  versions, duplicate extension IDs, and denied permissions.
- **Activation:** startup errors, dependency loading failures, slow activation,
  missing entrypoints, rejected host capabilities, and extension-host crashes.
- **Commands:** command registration failures, command execution errors,
  timeouts, cancellation, invalid arguments, and attempts to mutate a surface
  without the right permission.
- **UI Contributions:** panel render failures, bad layout data, failed asset
  loads, invalid theme tokens, crashed webview-like surfaces if those ever
  exist, and focus/input routing errors.
- **Editor Contributions:** diagnostics provider failures, formatter errors,
  completion provider failures, code action failures, snippet expansion errors,
  and attempts to edit stale or unauthorized buffers.
- **Terminal Contributions:** rejected terminal automation requests, blocked
  command execution, timeout/cancellation behavior, and permission violations.
- **Workspace Contributions:** file watcher failures, project indexing errors,
  denied filesystem reads/writes, excessive scan cost, and broken cache state.
- **Network Or External Tool Use:** denied network requests, process spawn
  failures, nonzero exit statuses, timeout behavior, and missing toolchain
  dependencies.
- **Compatibility:** API deprecation warnings, host/extension version mismatch,
  manifest schema migration failures, and extension API calls that changed
  behavior across LLNZY releases.
- **Resource Health:** memory pressure, high CPU, excessive event volume,
  runaway loops, repeated crashes, and extensions that slow down app startup or
  interaction.

Each of those events should be tagged with extension identity:

- `extension_id`
- `extension_name`
- `extension_version`
- `extension_kind`
- `extension_host_id`
- `activation_event`
- `declared_permissions`
- `granted_permissions`
- `extension_api_version`
- `developer_mode_enabled`
- `marketplace_or_local_source` once distribution exists

The host should never rely only on human-readable strings like "extension
failed". Agent analysis needs stable fields that identify which extension,
which API, which permission, which command, and which host boundary failed.

### Extension Incident Ownership

Extension incidents should make ownership visible. A failure should be
classified as one of:

- `host-core`: LLNZY core broke independently of extensions.
- `host-api`: an extension used a documented API correctly, but LLNZY handled
  it incorrectly.
- `extension-code`: the extension threw, crashed, timed out, or returned invalid
  data.
- `extension-permission`: the extension attempted something outside its granted
  permission set.
- `extension-compatibility`: the extension targets an API, manifest schema, or
  LLNZY version that is not compatible with the current app.
- `external-dependency`: the extension depends on a missing or broken tool,
  process, network endpoint, language server, or local file.
- `unknown`: Wolverine Skin does not yet have enough evidence.

That classification matters for administration. If the bug is in LLNZY's host
API, the right repair is a core LLNZY patch. If the bug is in extension code,
the right repair is an extension patch. If the issue is permission-related, the
right repair might be a permission grant, a manifest correction, or a safer API
path. Wolverine Skin should not collapse all extension problems into generic app
errors.

### Extension Developer Clinic

When developer mode is enabled, LLNZY should expose an extension-focused Clinic
view. This view should be useful to someone actively building on the platform.

Developer Clinic sections:

- Installed/local extensions with health status.
- Activation timeline for the selected extension.
- Command registration and command execution logs.
- Permission grants, denials, and requested-but-missing capabilities.
- Host API calls grouped by operation and failure rate.
- Recent crashes, timeouts, and resource-limit violations.
- Compatibility warnings against the current LLNZY version.
- Generated diagnostic bundle for the extension.
- Agent analysis and patch proposal records.

Useful developer actions:

- Reload extension.
- Disable extension.
- Open extension folder.
- Open manifest.
- Validate manifest.
- Run extension self-test.
- Create extension diagnostic bundle.
- Analyze extension failure with agent.
- Generate compatibility report.
- Propose extension patch.
- Mark incident as host bug, extension bug, or external dependency.

This surface should not only help LLNZY maintainers. It should help outside
developers build higher-quality extensions without needing to attach a debugger
for every failure. If LLNZY wants a healthy extension ecosystem, error visibility
has to be part of the developer experience.

### Extension Diagnostic Bundle

An extension diagnostic bundle should include enough information to debug the
extension without leaking user projects by default.

Default extension bundle contents:

- Extension manifest.
- Extension ID, name, version, source, and API version.
- Granted and requested permissions.
- LLNZY app version and extension host version.
- Activation events and timing.
- Recent extension-scoped diagnostic events.
- Repeated incident fingerprints.
- Host API calls that failed, timed out, or were denied.
- Resource-limit events.
- Stack trace or crash summary where available.
- Redacted external tool command names and exit statuses.
- Compatibility warnings.

Default extension bundle exclusions:

- Full source content from the user's workspace.
- Full terminal input or scrollback.
- Secrets, tokens, auth headers, private keys, cookies, and environment values.
- Full prompt bodies.
- Full extension cache content unless explicitly included.
- Network response bodies unless explicitly included.

If the extension itself is local source code under active development, the
developer may choose to include extension source files in the bundle. That should
be an explicit developer-mode choice, not the default behavior for ordinary app
users.

### Extension Sandboxing And Safe Failure

Wolverine Skin can only be trustworthy if extension execution is isolated.
Extension observability should assume these host boundaries:

- Extensions run out of process, in WASM, or behind another constrained
  extension-host boundary once programmable extensions exist.
- Extension crashes become extension incidents, not LLNZY process crashes.
- Extension API calls require declared permissions.
- Host APIs validate all extension input.
- Long-running extension work is cancellable.
- Extensions have CPU, memory, filesystem, process, and event-rate limits.
- Repeatedly crashing extensions can be automatically disabled after user-visible
  notice.
- Safe mode starts LLNZY with third-party extensions disabled.

The failure mode should be boring: LLNZY logs the incident, marks the extension
unhealthy, preserves a bundle, and keeps the rest of the app usable.

### Agent Repair For Extensions

Agent repair should support both sides of the extension boundary:

1. **Extension repair:** the agent reads the extension incident, manifest,
   extension source if the developer approves, host API documentation, and
   compatibility notes. It proposes a patch to the extension.
2. **Host API repair:** the agent determines that the extension used a supported
   API correctly and LLNZY mishandled it. It proposes a patch to LLNZY core and
   adds a host API regression test.
3. **Compatibility repair:** the agent identifies that the extension targets an
   older API. It proposes a manifest migration, shim, warning, or documentation
   update.
4. **Permission repair:** the agent identifies a missing or excessive
   permission. It proposes a manifest correction or safer API use.

Agent reports should include:

- Whether the failure is likely in LLNZY, the extension, the manifest, user
  configuration, or an external dependency.
- Which host API boundary was involved.
- Which permission was required or denied.
- Whether the extension is safe to keep enabled.
- Whether a minimal reproduction or extension self-test exists.
- What patch is proposed and where it applies.
- What verification command or manual test proves the repair.

For extension developers, this is the real platform value: LLNZY can become an
environment where building on top of the app includes a repair loop. A developer
does not just receive an opaque crash. They receive a structured account of
where the extension failed, what the host allowed or denied, what the agent
believes caused it, and what change would make it stronger.

### Extension Marketplace And Trust Signals

If LLNZY later supports a marketplace or curated extension index, Wolverine Skin
can provide local trust signals without becoming surveillance:

- Local crash count.
- Local activation time trend.
- Local permission denial count.
- Local resource-limit violations.
- Compatibility status against the current LLNZY version.
- Whether the extension has unresolved incidents.
- Whether the extension has passed its self-tests locally.
- Whether the extension has requested new permissions after update.

These signals should remain local by default. Sharing aggregate extension health
data would require a separate telemetry policy and explicit user consent. The
near-term value is not remote analytics; it is local clarity and local repair.

### Extension-Specific Success Criteria

The extension layer of Wolverine Skin is working when:

- A broken extension cannot take down the LLNZY host.
- Extension failures are visible as extension incidents, not anonymous app
  errors.
- Developers can filter diagnostics to a single extension.
- Permission denials explain which capability was missing and why it was
  blocked.
- Repeated extension crashes produce a stable incident fingerprint.
- Agent analysis can distinguish host API bugs from extension implementation
  bugs.
- A developer can generate a local diagnostic bundle for their extension.
- LLNZY can disable or quarantine an unhealthy extension while keeping the app
  usable.
- Compatibility problems produce migration guidance rather than silent failure.
- Extension repair proposals include diffs, verification steps, and a clear
  statement of whether the patch applies to LLNZY core or the extension.
