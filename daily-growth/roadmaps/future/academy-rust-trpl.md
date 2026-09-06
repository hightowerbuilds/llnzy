# LLNZY Academy — The Rust Course, Aligned to TRPL (3rd Edition)

Status: future roadmap, September 2026.

Owner decision, September 2026: rebuild the Academy Rust course around
*The Rust Programming Language*, 3rd Edition (Klabnik, Nichols, and Krycho;
No Starch Press; print 2025, covering the Rust 2024 edition — the "2024"
refers to the language edition). The existing 18-lesson course
(`academy-course-rust.md`) maps surprisingly well onto the book's first
thirteen chapters, so it becomes the seed rather than the discard: this
roadmap extends it to full book coverage and defines the in-app teaching
runtime both courses need. The book is the spine; LLNZY is the gym.

## Research summary (what was verified, September 2026)

- Chapters 1–13 confirmed from No Starch's own catalog page: Getting
  Started; Programming a Guessing Game; Common Programming Concepts;
  Understanding Ownership; Using Structs; Enums and Pattern Matching;
  Packages, Crates, and Modules; Common Collections; Error Handling;
  Generic Types, Traits, and Lifetimes; Writing Automated Tests; An I/O
  Project: Building a Command Line Program; Functional Language Features:
  Iterators and Closures.
- New in the 3rd edition (confirmed via publisher page and community
  sources): a complete **async programming chapter (Chapter 17)**, written
  by Chris Krycho; Miri coverage for analyzing unsafe code; Rust 2024
  edition idioms throughout.
- Three project chapters anchor the book: the guessing game (ch 2), a
  command-line tool (ch 12), and a multithreaded web server (final).
- Back-half chapter numbering (18+) varies between the 2nd edition and the
  2024-edition online book; exact print titles for 18–20 are to be verified
  against the print ToC before Part C lessons are authored (tracked as a
  Phase 0 checklist item, not a blocker).
- **License reality, decided now:** the online book is open (rust-lang
  repos, MIT/Apache-2.0), but the 3rd-edition print text is No Starch
  copyright. Course lessons are **original prose**: our own explanations,
  our own exercises, our own code, aligned to chapter topics and explicitly
  cross-referenced ("read ch 4, then do this"). No copied passages, in any
  phase. This is both the legal line and the pedagogical one — a course
  that re-teaches in the app's own voice, exercises through the app's real
  terminal, is the product.

## The teaching app (lesson runtime) — what we build

This is Phase 0–3 of `code-academy.md` scoped to ship with the Rust course.
Five pieces, each independently landable:

1. **Lesson reader** (Academy surface). Lesson markdown rendered with the
   existing markdown pipeline; code blocks get a copy-to-clipboard affordance
   and an "open this file" affordance when the block maps to a starter file.
   Lesson list with completion state comes from `academy_progress` (already
   shipped: store, tests, Home cards).
2. **Exercise materialization.** Per-lesson scratch workspace under
   `data_dir/academy/courses/rust/lessons/LXX/` — a cargo project for
   compile-inclusive lessons, plain files otherwise. Starter files written on
   first open; re-materialize only with confirmation once student edits
   exist (destructive-op policy).
3. **Editor + terminal integration.** "Open in editor" opens the exercise
   file with rust-analyzer live (already wired via the editor's LSP stack);
   "Open in terminal" restarts the session in the lesson workspace CWD
   (existing session restart machinery) so `cargo run` works on the real
   PTY. Task detection already lights up cargo tasks.
4. **Check runner.** Captured `std::process::Command` in the lesson
   workspace: 120s timeout for compile-inclusive checks (90s for
   `cargo check`-only — resolves the open question from
   `academy-course-rust.md`), stdout/stderr captured, match modes
   `exact` / `contains` / `exit_code`. Assert on program stdout only;
   cargo's own chatter is never part of expectations.
5. **Progress writing.** The one write path is
   `AcademyProgress::record_lesson_complete` (shipped, tested). Check pass
   → record → Home progress cards update. Failure shows expected-vs-actual
   with a "run it in the terminal" button.

## Course structure — book chapters as modules

Each book chapter becomes one Academy module (2–4 lessons each, 5–15 min per
lesson). The existing 18-lesson course maps to ch 1–13 with minor renumbering;
new modules cover the back half. Lesson-level detail for ch 1–13 carries over
from `academy-course-rust.md` (check commands and match modes already
specified there); the table below is the alignment and the delta.

| Book ch. | Module | Lessons | Notes vs. existing course |
|---|---|---|---|
| 1 | Toolchain & cargo | 2 | Merge of existing L0–L1 |
| 2 | Guessing game tour | 2 | New: build it book-style (stdin, rand-free: LCG secret), `Result` preview |
| 3 | Variables, types, control flow | 3 | Existing L2–L3 |
| 4 | Ownership & moves | 3 | Existing L4, split for pacing (the cliff) |
| 4 | Borrows & references, slices | 2 | Existing L5–L6 |
| 5 | Structs & methods | 2 | Existing L7 |
| 6 | Enums, `Option`, pattern matching | 3 | Existing L8 + matching depth |
| 7 | Modules & project layout | 1 | Existing L14 |
| 8 | Collections: Vec, HashMap, String | 2 | Existing L10 |
| 9 | Error handling: `Result`, `?`, panic | 2 | Existing L9 |
| 10 | Generics, traits, lifetimes | 4 | Existing L12–L13 + lifetimes intro |
| 11 | Testing | 2 | Existing L17 |
| 12 | **Project: `sift` CLI** (book builds minigrep; we build our grep-lite) | 3 | Existing final project becomes mid-course capstone |
| 13 | Iterators & closures | 3 | Existing L11 expanded |
| 14 | Cargo & crates.io practices | 1 | New; reading-forward module, checks are `cargo publish --dry-run`-shaped local simulations |
| 15 | Smart pointers: `Box`, `Rc`, `RefCell` | 2 | New |
| 16 | Fearless concurrency: threads, channels, `Mutex` | 2 | New |
| 17 | **Async Rust** (new in 3rd ed.) | 2 | New; zero-dep single-threaded executor lesson + `async fn`/`.await` semantics; no Tokio (out of scope, stated) |
| 18–19 | Trait objects & OOP-ish patterns; patterns/matching depth; advanced features tour | 3 | Titles to verify against print ToC; `dyn`, advanced traits, unsafe-awareness + Miri mention |
| 20 | **Final project: multithreaded web server** (book-style) | 4 | New capstone: TCP listener, thread pool, graceful shutdown; std-only |

Totals: 21 modules, ~50 lessons, two capstones. That is the "good distance"
of the book this roadmap commits to; appendices are non-goals.

## Phases

### Phase A — Lesson runtime (the teaching app)

- [ ] `src/academy/` module: course manifest (`course.toml`), lesson
      frontmatter schema, markdown lesson parser, loader with `fs_watch`
      invalidation — pure, unit-tested (Phase 0 of the platform roadmap).
- [ ] Lesson reader in the Academy surface: chapter/module list with
      completion state, lesson view with rendered markdown.
- [ ] Exercise materialization + "Open in editor" + "Open in terminal".
- [ ] Check runner with timeout + match modes; failure diff view.
- [ ] Wire check pass → `record_lesson_complete`; Home cards live-update.
- [ ] Course lint (CI): every lesson's check passes from its solution
      state and fails from its starter state.

### Phase B — Author Part One (ch 1–6)

- [ ] Verify lesson/check specs against the shipped 18-lesson course;
      renumber to the module map above.
- [ ] Write the guessing-game module (new) and the ownership pacing split.
- [ ] Manual smoke: lessons 1–20 pass lint and play well in `./dev.sh`.

### Phase C — Author the middle (ch 7–13)

- [ ] Collections, error handling, generics/traits/lifetimes, testing.
- [ ] `sift` capstone module (3 lessons) at the ch 12 position.
- [ ] Confirm print ToC for ch 18–19 titles before Part D authoring.

### Phase D — Author the back half (ch 14–20)

- [ ] Cargo practices, smart pointers, concurrency, async (std-only),
      trait objects/advanced tour.
- [ ] Web server capstone (4 lessons), std-only, with tests.

### Phase E — Finish

- [ ] Proficiency-bar review against the final projects (both capstones).
- [ ] Docs: architecture map entry for `src/academy/`, README feature
      line, manual smoke test entries for the full course loop.

## Non-goals

- No copying of book text (license line above); lessons are original.
- No Tokio/async-ecosystem runtime in the async module — std semantics only.
- No appendices (keywords, operators, derivable traits, macros reference).
- No second edition support; the course targets the 3rd edition's chapter
  structure only.

## Risks

- **Book drift**: the online book moves ahead of print. Mitigation: the
  course pins to the print 3rd-edition ToC; the module map records chapter
  numbers, not URLs.
- **Chapter 18–19 title uncertainty** (2nd vs 3rd edition renumbering):
  verified in Phase C before Part D is authored; nothing before ch 14
  depends on it.
- **Volume**: ~50 lessons is the real cost. Mitigation: the existing
  18-lesson course already covers a third of it, and the course lint keeps
  content honest in CI without human re-review of every lesson.
- **Compile-time timeouts on CI runners**: course lint runs the same
  120s/90s budget; cold-cache cargo builds on GitHub macOS runners fit,
  but the lint job caches `target/` per lesson workspace to keep it fast.
