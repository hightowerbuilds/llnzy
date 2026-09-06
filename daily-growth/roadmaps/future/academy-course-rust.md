# Academy Course — Rust (`academy-course-rust`)

Status: future roadmap, September 2026.

Owner decision, September 2026: this is the Rust launch course for the
Academy. Rust goes first because it is the language LLNZY is written in, so
the app's own tooling — rust-analyzer, cargo task detection, a real PTY — is
the best in class for exactly this course. Scope: one course, 18 lessons plus
a final project, std-only, taking a beginner to the proficiency bar below.
Content lives in this repo and passes course lint like any other code.

Grounded in the platform contract in `daily-growth/roadmaps/future/code-academy.md`: markdown
lessons + `course.toml`, exercises with prompt / starter files / check
command / expected result, checks run as captured subprocesses in a
per-lesson scratch workspace under `data_dir` with a timeout, match modes
`exact` | `contains` | `exit_code`, no sandbox, no network, no accounts.

## Proficiency bar

A student who finishes this course can, without looking anything up:

1. **Read ownership.** Given ~40 lines of reviewed Rust, say for each
   non-trivial value whether it moves, borrows, or clones — and why the
   alternative would not compile (or would be wasted work).
2. **Propagate errors.** Write a fallible call path three layers deep, use
   `?` at every layer, convert error types once at the boundary, and produce
   a user-facing message plus a nonzero exit code at the top.
3. **Abstract.** Define a trait with at least one method, implement it for
   two distinct types, and consume it both generically (`impl Trait` /
   `<T: Trait>`) and dynamically (`&dyn Trait`) — and say when each is right.
4. **Use iterators.** Replace an index-based loop with a `map`/`filter`/
   `collect` chain over `Vec`/`HashMap`, and read the resulting type
   signature aloud.
5. **Model data.** Express a domain with structs and enums, including one
   enum with data-carrying variants, and exhaustively match on it.
6. **Test.** Write `#[test]` functions with `assert_eq!`, run them with
   `cargo test`, and read a failing test's panic location.
7. **Read the compiler.** Given a borrowck or type error, name the cause in
   one sentence and the fix in one more. This is drilled, not assumed.
8. **Ship.** Take a multi-file crate from `cargo new` to `cargo build` with
   modules laid out by responsibility and zero external crates.

The final project must exercise every item; mapping is listed there.

## Toolchain & constraints

- **Prerequisite: rustup.** The course picker states "requires rustup". There
  is no sandbox; the student's real toolchain runs their real code.
- **Lesson 0 verifies the toolchain** before any concept work: run
  `rustc --version` and `cargo --version`, require exit 0, and print both
  versions to stdout for the student to see. Version *text* is never asserted
  on — only presence and exit code (`exit_code` mode).
- **Zero external crates, std only, in every lesson and the final project.**
  Cargo never touches the network for a check. Starter `Cargo.toml` files are
  hand-authored, not `cargo new` output: no `[dependencies]` section, edition
  **pinned to "2021"** in every starter so behavior does not shift when the
  student's cargo defaults to a newer edition. No `cargo update`, no lockfile
  churn, nothing to fetch.
- **Check timeout: 120s for compile-inclusive checks** (`cargo run`,
  `cargo check`, `cargo test`). Rationale: a cold debug build of a std-only
  binary is typically well under 30s, but first-compiles on older Intel
  laptops plus cargo's target-dir spin-up can spike; 120s covers it without
  letting a hung process eat the harness. `cargo check`-only lessons may use
  90s. **Warm builds are fast** — after the first check, incremental rebuilds
  of these tiny crates are 1–3s, and each lesson has its own scratch
  workspace so builds never share a target dir and never go stale.
- **One binary per lesson.** Starter files: `Cargo.toml` + `src/main.rs`
  (plus `src/lib.rs`/submodules from lesson 16 on). Every lesson's check runs
  from the lesson's scratch workspace root, where `Cargo.toml` lives.

## Curriculum

18 lessons (0–17) + final project. Exercise sizing 5–15 minutes each.
Notation: **check** is the command the harness runs; **expect** is the
assertion with its match mode. All `cargo run` expectations assert on the
*program's* stdout, never cargo's chatty wrapper output.

| # | Lesson | Concepts | Exercise & check |
|---|--------|----------|------------------|
| 0 | Toolchain & Hello Cargo | rustup, cargo new/run, main, println! | Fix a broken `main` that fails to compile (missing `fn`, bad macro). **Check:** `cargo run`. **Expect** stdout `hello from the llnzy academy` — `exact`. |
| 1 | Variables & Mutability | let, mut, shadowing, const | Compute a running total with `mut`, then rewrite with shadowing. **Check:** `cargo run`. **Expect** `total: 30` — `exact`. |
| 2 | Types & Control Flow | i32/u32/bool/char, if/else, loops, overflow discipline | FizzBuzz variant to stdout with fixed labels. **Check:** `cargo run`. **Expect** the 15-line output for 1..=15 — `exact`. |
| 3 | Functions | params, returns, expression bodies, unit type | Extract two helpers from inline code; print results. **Check:** `cargo run`. **Expect** `area: 24` / `perimeter: 20` on separate lines — `contains`. |
| 4 | Ownership & Moves | move semantics, drop order, clone vs move | Pass a `String` into a function that consumes it, then use it again *correctly*. **Check:** `cargo check`. **Expect** exit `0` — `exit_code`. |
| 5 | Borrows & References | `&`/`&mut`, one-writer rule, reading borrowck errors | Fix three borrowck errors; each fix is a one-line change. **Check:** `cargo check`. **Expect** exit `0` — `exit_code`. |
| 6 | Slices & Strings | `&str` vs `String`, `&[T]`, why two string types exist | Count words in a `&str` and return `String` pieces; print counts. **Check:** `cargo run`. **Expect** `words: 5` — `exact`. |
| 7 | Structs | named fields, methods, `impl`, associated fns, `Self` | Build a `Task` struct with `new`, `complete`, `is_done`. **Check:** `cargo run`. **Expect** `[x] write roadmap` — `exact`. |
| 8 | Enums | unit vs data-carrying variants, exhaustive switch | Model a shell event (`Prompt`, `Input(String)`, `Exit(u8)`) and print each. **Check:** `cargo run`. **Expect** three labeled lines — `contains`. |
| 9 | Pattern Matching | match arms, bindings, `_`, if let, while let | Destructure the lesson-8 enum incl. nested data; print exit code. **Check:** `cargo run`. **Expect** `exit code: 3` — `exact`. |
| 10 | Option | `Option<T>`, no nulls, `unwrap_or`, combinators | Safe `first_word` lookup with no `unwrap` allowed in solution. **Check:** `cargo run`. **Expect** `some: ok` / `none: empty` — `contains`. |
| 11 | Result & `?` | `Result<T,E>`, `?`, `From`, error at the boundary | Read a missing file, propagate with `?`, print friendly error, exit nonzero. **Check:** `cargo run`. **Expect** exit `1` — `exit_code`. |
| 12 | Collections | `Vec`, `HashMap`, entry API, ownership in containers | Tally command frequencies in a fixed `Vec<&str>`. **Check:** `cargo run`. **Expect** sorted, deterministic tally lines — `exact`. |
| 13 | Iterators & Closures | `map/filter/sum/collect`, closure capture, laziness | Rewrite an index loop as an iterator chain (bar item 4). **Check:** `cargo run`. **Expect** `evens squared: [4, 16, 36]` — `exact`. |
| 14 | Traits & Generics | trait def, two impls, `impl Trait`, bounds | `Display`-like trait implemented for two types, consumed generically. **Check:** `cargo run`. **Expect** both renderings — `contains`. |
| 15 | Trait Objects | `&dyn Trait` / `Box<dyn Trait>`, static vs dynamic dispatch | Hold mixed impls in a `Vec<Box<dyn Trait>>` and iterate. **Check:** `cargo run`. **Expect** three lines in insertion order — `exact`. |
| 16 | Modules & Layout | `mod`, file-per-module, `pub`, `use`, crate root | Split a 100-line main into `lib.rs` + two submodules. **Check:** `cargo check`. **Expect** exit `0` — `exit_code`. |
| 17 | Testing | `#[test]`, `assert_eq!`, `#[should_panic]`, cargo test | Port lessons 10 and 12 logic into tests; make 5 pass. **Check:** `cargo test`. **Expect** `test result: ok. 5 passed` — `contains`. |
| — | **Final project: `sift`** | all of the above | Spec below. Checks: `cargo test`, then three `cargo run --` invocations. |

Pacing note (see Risks): lessons 4–6 are the cliff. They ship as three short
lessons, not one, each with a reversible, one-line fix and a compiler-error
reading exercise; a student who finishes lesson 5 can name a borrow conflict
from the error text alone.

## Final project — `sift`, a search CLI in the spirit of llnzy

LLNZY is a terminal app; the graduate builds a terminal tool. `sift` is a
std-only line-search CLI — a spiritual miniature of the grep every terminal
user lives in. No external crates; argument parsing is hand-rolled
(`std::env::args`), which is instructive in itself.

**Spec.**

- `sift PATTERN FILE` — print every matching line, prefixed `N:line` with the
  1-based line number. Exit 0 if at least one match, exit 1 if none.
- `sift -c PATTERN FILE` — print only `N matches` and exit 0 on any match,
  `0 matches` + exit 1 on none.
- `sift -w PATTERN FILE` — match whole words only (word-boundary check by
  hand; no regex crate).
- `--help` / no args — print fixed usage text to stdout, exit 0.
- Missing or unreadable file — one clear line to **stderr**, exit 2. Error is
  built with a project `SiftError` enum and propagated with `?` from the read
  helper through `run()` to `main()` — three layers, one boundary conversion.
- **Trait requirement:** `trait Matcher { fn matches_line(&self, line: &str) -> bool; }`
  with two impls — `Literal` (substring) and `Word` (word-bounded). Flag
  parsing selects the impl; `run()` takes `&dyn Matcher` (or `impl Matcher`,
  student's choice, defended in one comment line).
- **Module layout:** `src/main.rs` (args + boundary), `src/matcher.rs`
  (trait + impls), `src/search.rs` (file read + scan), `src/error.rs`.
- **Tests:** at least six `#[test]` functions covering both `Matcher` impls,
  zero-match behavior, and usage-text presence; `cargo test` green.

**Checks.** Four, run in order: `cargo test` (expect `test result: ok` —
`contains`); `cargo run -- alpha sample.txt` (expect two numbered lines —
`exact`); `cargo run -- -w to sample.txt` (expect `contains` `4:`); and
`cargo run -- -c zz sample.txt` (expect exit `1` — `exit_code`). All on a
fixed `sample.txt` shipped as a starter file with LF endings and no trailing
whitespace.

**Bar mapping.** Ownership moves/borrows: `String` pattern moved into the
matcher, lines borrowed as `&str` (items 1). Error propagation across three
layers with `?` and a boundary message (item 2). `Matcher` trait with two
impls plus dynamic or generic dispatch (item 3). Line scanning as iterator
chains, not index loops (item 4). `SiftError` enum with data-carrying
variants, matched exhaustively (item 5). Six tests under `cargo test` (item
6). Compile-error recovery is exercised by the build-fix loop inherent to the
project (item 7). Four-module crate from scratch (item 8). No bar item is
optional for a passing grade.

## Authoring & course lint

- **Course lint runs in CI** and is the review bar for content: every bundled
  lesson parses, and every check command **passes from the solution state and
  fails from the starter state**. A lesson whose starter already passes is a
  no-op lesson; a lesson whose solution fails is a broken lesson. Both fail
  the build.
- **Expected outputs contain no nondeterminism.** No memory addresses, no
  timings, no durations, no paths from the student's machine, no
  `rustc`/`cargo` version text, no file-count or temp-dir noise. Where a
  lesson prints something unstable, the expected value is a fixed literal the
  student's code must produce.
- **Assert on program stdout, not cargo's output.** Cargo wraps `cargo run`
  output in compilation chatter and progress lines that change between
  versions; that is exactly the toolchain drift the platform contract warns
  about. Use `contains` for multi-line program output, `exact` only where the
  full stdout is a short fixed literal, and `exit_code` wherever stdout is
  empty or irrelevant (compile-only lessons, error paths).
- **stderr is not asserted on**, except where a lesson explicitly teaches
  exit codes — there, assert the exit code only. rustc's error text changes
  across versions and must never appear in an expectation.
- **Starters fail for the taught reason.** A starter that fails to compile
  must fail on the lesson's target concept, not on a stray typo; lint runs
  the starter's compile error and the author confirms the concept matches.
- **Solutions live in the repo** beside each lesson (e.g. `solutions/` per
  course dir), consumed by lint. Never shipped to the student's scratch
  workspace.

## LLNZY integration notes

- **Editor.** Exercise files open in the real editor with tree-sitter Rust
  highlighting shipped in the app today. rust-analyzer LSP gives live
  completions, diagnostics, hover, and rename in exercise files — a beginner
  meets production-grade Rust tooling in lesson 0, which is the whole thesis.
- **Diagnostics as curriculum.** Lessons 4 and 5 lean on the in-editor red
  squiggles deliberately: the student sees the borrow error in context before
  running a check. Lesson prompts instruct "hover the error" as a step.
- **Task detection.** `src/tasks.rs` auto-detects cargo build/check/run from
  the starter `Cargo.toml`, so every Rust exercise gets task integration for
  free — the student can run the check command as a task or in the terminal.
- **Terminal free-play.** The visible PTY is never the graded surface. The
  lesson view's "open in terminal" drops the shell into the scratch
  workspace, where the same check command can be run and debugged by hand.
- **Markdown.** Lesson bodies render with the app's existing markdown styles;
  no new rendering work for this course.

## Risks

- **Toolchain drift** (new stable editions, changed rustc diagnostics, cargo
  output format changes) breaks expectations written against unstable text.
  *Mitigation:* assert only on the student program's stdout and exit codes —
  never on cargo or rustc output; pin `edition = "2021"` in every starter;
  course lint in CI runs the whole course on current stable and fails the
  build when a check stops passing.
- **Compile time inside checks.** A cold debug build on a slow laptop can
  outrun a naive timeout; a timed-out check reads as "broken lesson" to a
  student. *Mitigation:* 120s timeout for compile-inclusive checks; std-only
  crates with no dependency resolution; per-lesson scratch workspaces whose
  warm target dirs make repeat checks 1–3s; `cargo check` instead of
  `cargo run` for compile-only lessons.
- **The motivation cliff at ownership/borrowck.** Lessons 4–6 are where
  beginners quit — the language appears to fight them. *Mitigation:* three
  short lessons instead of one long one; every exercise is a small,
  reversible, one-or-few-line fix with a provided broken starter; explicit
  compiler-error-reading drills (name the cause, then the fix) so the errors
  become signal rather than noise; rust-analyzer's in-editor diagnostics show
  the problem where the student is already looking.
- **Starter rot / no-op lessons.** A starter that already passes teaches
  nothing; a solution that fails ships broken. *Mitigation:* the lint rule is
  bidirectional (pass from solution, fail from starter) and runs in CI on
  every change to lesson content.
- **Student machines without rustup** hit a confusing spawn failure in lesson
  0. *Mitigation:* the course picker states the prerequisite up front, and
  lesson 0's failure path renders a readable "install rustup" message via the
  check-runner's error surface rather than a raw spawn error.

## Work items

Content production for `academy-course-rust`. Each lesson PR ships lesson
markdown, starter files, solution, and a lint-green check.

- [ ] Author `course.toml` (id, title, language, lesson order) and course
      directory skeleton.
- [ ] Write lessons 0–3 (toolchain, variables, types/control flow, functions)
      with starters + solutions; lint both directions.
- [ ] Write lessons 4–6 (moves, borrows, slices/Strings) with the
      compiler-error-reading drills; review pacing with a real beginner.
- [ ] Write lessons 7–9 (structs, enums, pattern matching).
- [ ] Write lessons 10–11 (Option, Result/`?`) incl. the nonzero-exit lesson.
- [ ] Write lessons 12–13 (collections, iterators/closures).
- [ ] Write lessons 14–15 (traits/generics, `dyn` trait objects).
- [ ] Write lessons 16–17 (modules/layout, `cargo test`).
- [ ] Author `sift` final project: spec lesson, four starters, reference
      solution, `sample.txt`, four checks; verify bar mapping covers items
      1–8.
- [ ] Run the course end-to-end in the Academy surface on a clean profile
      (`LLNZY_PROFILE=dev`), timing cold and warm checks; confirm 120s holds
      on the slowest available machine.
- [ ] Wire the course into course lint in CI; block merge on red.
- [ ] Post-launch: second-pass pass through lesson expectations for
      nondeterminism (addresses, paths, versions) with fresh eyes.
