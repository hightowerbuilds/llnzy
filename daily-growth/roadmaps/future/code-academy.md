# LLNZY Code Academy

Status: future roadmap, September 2026.

Owner decision, September 2026: give LLNZY a second reason to exist beyond
being a great terminal — use the surfaces we already own to teach
programming. Users pick a language, work through a structured course, and get
pushed to real proficiency. The honest way to learn a language is a real
shell, a real editor, and real toolchain feedback; LLNZY already ships all
three. A course platform bolted onto a terminal is not a gimmick here — it is
the terminal-first thesis played forward: the terminal *is* the REPL, the
editor *is* the workbook, and the app is the guide.

Grounded in the code at `6372478`. Verified before planning: the surface enum
(`WorkspaceSurface`, `src/gpui_workspace.rs:341`) and how panes route to
surfaces (`src/gpui_workspace/panes.rs:372-386`); the editor's tree-sitter
coverage (Rust, JS, TS, Python, Go, C, JSON, HTML, CSS, Bash — `Cargo.toml`)
and LSP feature set; markdown preview styles that already render styled
markdown in-app (`src/gpui_editor/render/markdown.rs`); task auto-detection
from project files (`src/tasks.rs` — Cargo.toml already yields build/check
tasks); versioned workspace recovery
(`src/gpui_workspace/recovery.rs:11`); platform path separation for state
(`src/platform/paths.rs:17-113`, including `data_dir` for progress and the
`LLNZY_PROFILE=dev` isolation that keeps academy dev data out of production);
`fs_watch` for course-file change invalidation; and the PTY/session stack the
lessons will talk to.

**What LLNZY already has** (verified; don't rebuild these): a real terminal
(true-color ANSI/VT emulation, PTY lifecycle, session restart), a real editor
(rope-backed, tree-sitter highlighting for the languages we would teach, LSP
hover/completion/diagnostics/rename/format), a project sidebar that scopes to
the terminal's CWD, styled markdown rendering, theme-aware UI, persisted
preferences, workspace recovery, and a command palette. No other course
platform runs the student's code on the student's actual machine in the same
window they will keep using after the course ends. That is the product.

## Design decisions

Decided up front so phases don't drift:

- **Courses are directories of markdown lessons with frontmatter.** A course
  manifest (`course.toml`) plus one markdown file per lesson, exercises as
  fenced blocks with metadata. Rationale: the editor already highlights and
  previews markdown, course authoring needs no new tooling, and courses are
  diffable text like everything else in this repo.
- **Verification runs out-of-band, not through the user's terminal.** The
  "Check" action runs the lesson's check command with `std::process::Command`
  in a per-lesson scratch workspace under `data_dir`, with a timeout, and
  compares exit code / output against expectations declared in the lesson.
  Rationale: parsing ANSI-painted terminal output is fragile and
  non-deterministic; a captured subprocess is honest and testable. The visible
  terminal stays the free-play surface where the student runs anything they
  want, including the same check command.
- **Progress is local JSON in `data_dir`.** No accounts, no cloud, no sync.
  Completion state survives restarts and respects the dev/production profile
  split.
- **Academy is a first-class surface**, a new `WorkspaceSurface` variant with
  its own tab, not a modal or a webview. Course exercise files open in the
  existing editor with LSP live — the student learns with production-grade
  tooling from minute one.
- **Launch languages: Rust, Python, TypeScript.** All three already have
  tree-sitter grammars and tested LSP integrations in this app. Go and C are
  cheap follow-ups (grammars already shipped) once the platform proves out.
- **No sandbox.** Check commands execute real code on the user's machine —
  the same trust level as typing in the terminal, which is the point of the
  app. The course picker states the prerequisite (e.g. "requires rustup") and
  the first lesson of each course verifies the toolchain and prints versions.
  We do not build a VM.

## Phase 0 — Course format and model (pure, tested)

New `src/academy/` module. Model code only, no GPUI — it must be unit-testable
without a window, per the architecture map's ownership rules.

- [ ] **Course manifest and lesson schema** — serde types: course (id, title,
  language, description, lesson order), lesson (id, title, concepts, one or
  more exercises), exercise (prompt, starter files, check command, expected
  exit code / output match mode: `exact` | `contains` | `exit_code`).
- [ ] **Lesson parser** — markdown + frontmatter parsing that yields typed
  lesson structs; fenced exercise blocks carry the check spec. Malformed
  lessons must fail with a positioned error (error-log policy: status, not
  crash).
- [ ] **Course library loader** — discovers courses in a bundled directory and
  a user courses dir (via `platform::paths`); `fs_watch` invalidates the
  course cache on change, mirroring the sidebar pattern.
- [ ] **Unit tests** — parser round-trips, malformed-input errors, loader
  ordering. These are the pyramid's base; no phase below this ships without
  them.

## Phase 1 — Academy surface

- [ ] **`WorkspaceSurface::Academy`** — new enum variant, tab bar entry,
  focus routing in `panes.rs`, recovery-snapshot support (bump
  `WORKSPACE_RECOVERY_VERSION` and handle old snapshots per the existing
  migration pattern).
- [ ] **Course picker** — language cards (Rust / Python / TypeScript),
  description, lesson count, progress percent, toolchain prerequisite note.
- [ ] **Lesson list** — per-course view: lesson titles, completion state,
  current-lesson marker, locked/sequential-vs-free navigation (decide:
  default sequential with a "skip" affordance).
- [ ] **Lesson view** — concept text rendered with the existing markdown
  styles, then exercise instructions with starter-file chips that open in the
  editor on click.

## Phase 2 — Lesson runtime (terminal + editor integration)

This is the phase that no other course product can copy cheaply.

- [ ] **Exercise materialization** — write starter files into a per-course,
  per-lesson scratch workspace under `data_dir`; re-materialize cleanly on
  retake; never overwrite student work without confirmation (destructive-op
  policy).
- [ ] **Editor integration** — open exercise files as a normal editor tab
  with LSP live; the lesson view's chips deep-link to the file.
- [ ] **Terminal integration** — "Open in terminal" drops the student's shell
  into the exercise workspace CWD (existing session restart machinery,
  `src/gpui_terminal.rs` Cmd+R path) so `cargo run` / `python3` /
  `npx tsx` work on the real PTY. Task detection (`src/tasks.rs`) already
  lights up Cargo-based Rust exercises.
- [ ] **Lesson-side next/prev navigation** with unsaved-editor guard.

## Phase 3 — Verification harness and progress

- [ ] **Check runner** — execute the exercise's check command via
  `std::process::Command` in the scratch workspace: timeout, captured
  stdout/stderr, match modes (`exact`, `contains`, `exit_code`), structured
  result (pass/fail + diff excerpt on failure).
- [ ] **Result surface** — pass marks the exercise complete and unlocks the
  next lesson; fail shows expected-vs-actual with a "show in terminal" button
  that reruns the command on the PTY where the student can debug it.
- [ ] **Progress persistence** — JSON in `data_dir`; course/lesson/exercise
  completion with timestamps; resume restores the Academy view to the current
  lesson on app relaunch.
- [ ] **Error-log integration** — check-runner timeouts, spawn failures, and
  missing toolchains log with the failed command and lesson id, and surface a
  readable reason in the Academy view (error policy: status, not silent).

## Phase 4 — Course content

The platform is only as good as the courses. Written in the same repo, same
review bar as code.

- [ ] **Authoring conventions doc** — lesson structure, exercise sizing
  (5–15 min), check-command rules (deterministic output, no network), and a
  "course lint" that validates every bundled course parses and every check
  command passes from starter files' *solution* state and fails from the
  starter state. Course lint runs in CI.
- [ ] **Rust course** — hello world → ownership → structs/enums → traits →
  error handling → collections/iterators → a final real CLI project that
  builds with `cargo`. Target: a student who can read intermediate Rust and
  ship a small tool.
- [ ] **Python course** — same shape: basics → data structures → functions →
  modules/venv → a final script project. Checks run under `python3`.
- [ ] **TypeScript course** — basics → types → interfaces/generics → async →
  a final `tsx`-run project; leverages the existing TS tree-sitter + LSP
  support.
- [ ] **Proficiency bar review** — per course, a written definition of
  "proficient" the final project must actually exercise (e.g. Rust: ownership
  moves in real code, `Result` propagation, a trait with two impls).

## Phase 5 — Fit and finish

- [ ] **Home surface course card** — current course, next lesson, one-click
  resume.
- [ ] **Command palette + menus** — "Academy: continue course", per-language
  course entries.
- [ ] **Theme integration** — Academy UI derives from app theme (light/dark
  parity, matching the appearances work).
- [ ] **Docs** — architecture map update (`src/academy/` ownership), README
  feature entry, manual smoke tests appended to
  `docs/manual-smoke-tests.md`.

## Non-goals

- No accounts, cloud sync, or telemetry.
- No sandboxed execution environment or VM.
- No in-app REPL of our own — the terminal already is one.
- No auto-grading of style or design; checks verify behavior only.
- No editor-chrome tutorials teaching LLNZY itself (that is a separate,
  smaller idea; this roadmap teaches languages).

## Risks

- **Course maintenance** is the long-term cost, not the code. The course lint
  in CI is the mitigation — a course that stops passing its own checks fails
  the build.
- **Toolchain drift** (rustup/python/node versions) changes check output.
  Check commands must assert on stable, version-independent output; the
  course lint catches regressions on the CI machine.
- **Scope creep toward LMS features.** The non-goals list is the fence.
