# Academy Course: JavaScript / TypeScript

Status: future roadmap, September 2026.

Owner decision, September 2026: this is the JS/TS launch course for the
Academy. One course, two acts — Act I teaches JavaScript on the real node
runtime, Act II re-covers the same ground in TypeScript and goes deeper —
because the honest path to TS fluency is a working mental model of JS
semantics first, then the type system layered on top of artifacts the student
already wrote. Every lesson runs in the surfaces we already own: the student
edits exercise files in the LLNZY editor with live TS LSP and tree-sitter
highlighting, and free-plays in the real terminal. Grounded in the platform
contract in `daily-growth/roadmaps/future/code-academy.md` (course.toml + markdown lessons,
captured-subprocess checks with `exact` | `contains` | `exit_code` match
modes, per-lesson scratch workspace under `data_dir`, no sandbox, lesson 0
verifies the toolchain).

Course id: `js-ts`. Language card: "JavaScript → TypeScript". Prerequisite
note in the picker: "requires node >= 20".

## Proficiency bar

The contract. The final project must exercise every line of this list; a
student who finishes the course can:

1. **Read typed code.** Open an unfamiliar ~500-line TypeScript codebase and
   answer "what does this accept, what does it return, where does this data
   come from" without running it — by reading types and using LSP
   hover/go-to-definition.
2. **Modify without breaking.** Change a function signature in that codebase
   and follow the compiler to every call site, fixing them.
3. **Write typed async.** Write `async` functions with explicit `Promise<T>`
   return types, `await` them correctly (incl. parallel `Promise.all`), and
   type the error path — no floating promises, no `any` escapes.
4. **Type a boundary.** Take `unknown` data (parsed JSON, argv) and narrow it
   to a trusted type with zero `as` casts and zero `any`.
5. **Choose `interface` vs `type`** and say why in one sentence each way.
6. **Write one generic** instead of three copies — a function or type that
   parameterizes over a type and stays readable.
7. **Use the array trinity fluently** — `map`/`filter`/`reduce`, chained —
   and pick the right one for a job instead of reaching for `for` loops.
8. **Explain the JS traps**: `==` coercion, `this`, closures capturing
   mutable bindings, floating-point money. Avoid them on purpose.
9. **Structure code in ES modules**: `import`/`export`, one responsibility
   per file, no cross-file copy-paste.
10. **Fail loudly and typed**: `throw`/`catch` at the right layer, custom
    `Error` subclasses, discriminated-union results where errors are data.
11. **Drive the terminal**: run `node` / `npx --no-install tsx`, read a stack
    trace, and bisect a failure with `console.log` or a breakpoint-free
    probe. This is the skill the Academy exists to build.

## Toolchain & constraints

- **node >= 20**, verified in Lesson 0 by feature probe, not version string
  (version strings are check-fragile; see Authoring & lint).
- **TypeScript executes via `tsx`.** No `tsc` build step, no `tsconfig`
  required for lessons (the final project adds a minimal one, compile-only).
- **The tsx network problem, resolved.** `npx tsx` downloads the package on
  first run — that is a network call from inside a check command, which
  violates the deterministic/offline check contract. Mitigation, in three
  parts:
  1. **One-time explicit install at course start, student-initiated.** Lesson
     0 instructs the student to run, in their own terminal:
     `npm install --no-save tsx` in the course root workspace
     (`data_dir/academy/courses/js-ts/`). One network call, consented,
     visible, never repeated by a check.
  2. **Layout guarantee.** Per-lesson scratch dirs nest under the course
     root, so `node_modules/` is reachable by directory walk-up:
     ```
     data_dir/academy/courses/js-ts/
       package.json          # {"type":"module","private":true} — no deps
       node_modules/         # tsx only, from step 1
       lessons/L05/          # per-lesson scratch, rematerialized per retake
     ```
  3. **Every TS check uses `npx --no-install tsx <file>`**, which resolves
     from `node_modules/.bin` on the walk-up path and never hits the
     registry; `npm_config_yes=false` in the check env makes any accidental
     fetch a hard failure instead of a silent download. Lesson 0's own check
     proves resolution works **from inside a nested lesson dir** — the exact
     geometry later checks rely on. If npm's walk-up ever changes, fallback
     is to prepend `<course root>/node_modules/.bin` to `PATH` and call
     `tsx` directly; same files, no behavior change.
- **Zero npm dependencies in every exercise.** Stdlib only: `node:fs`,
  `node:path`, `node:readline`, `node:util`, `console`, timers. No network,
  no `fetch`, no clocks, no randomness in anything a check observes.
- **ESM everywhere.** The root `package.json` sets `"type":"module"`, so
  `.js` lessons use `import` from Lesson 7 onward and `.ts` lessons always
  do. No CommonJS in this course.
- Checks run captured, offline, with a timeout, per the platform contract.
  The student's terminal remains free-play — they can run the check command
  themselves, and the lesson text tells them to.

## Curriculum

18 lessons + final project. Exercises target 5–15 minutes. Check commands run
in the lesson scratch dir; expected output is stdout unless noted.

### Act I — JavaScript (Lessons 0–9)

**L0 · Toolchain and workspace verify**
- Concepts: what runs where — node, tsx, the scratch workspace, the check
  contract; the one-time tsx install.
- Exercise: run the tsx install in your terminal, then finish `verify.js` so
  it prints `node-ok` iff the runtime supports stable ESM + `structuredClone`
  (feature probe, not version compare), and `verify.ts` so it prints
  `tsx-ok`.
- Check: `node verify.js && npx --no-install tsx verify.ts`
- Expected: `node-ok\ntsx-ok` — **exact**.

**L1 · Values, types, coercion**
- Concepts: seven primitives + objects; `typeof` results; `==` vs `===`;
  truthiness; the `NaN` trap.
- Exercise: in `coerce.js`, implement `describe(v)` returning a fixed label
  per input type, and `equals(a, b)` that mirrors `===` without using it for
  the two given tricky pairs.
- Check: `node coerce.js`
- Expected: 12 fixed lines, one per test case — **exact**.

**L2 · Control flow**
- Concepts: `if`/`else`, `switch`, `for`/`for..of`, `break`/`continue`,
  early returns over nested conditionals.
- Exercise: `fizzbuzz.js` — classic loop to 30, plus `classify(n)` rewritten
  as a `switch` with no `if` at all.
- Check: `node fizzbuzz.js`
- Expected: 30 fixed lines — **exact**.

**L3 · Functions, scope, closures**
- Concepts: declarations vs expressions vs arrows; lexical scope; closures
  capturing bindings; default/rest params.
- Exercise: `counter.js` — write `makeCounter(start)` and a `memoize(fn)`
  that caches by argument, then log three probe sequences that prove the
  bindings are per-closure.
- Check: `node counter.js`
- Expected: fixed probe lines (e.g. `c1: 1, 2, 3 / c2: 10, 11 / memo hits: 1`)
  — **exact**.

**L4 · Arrays and objects**
- Concepts: literal construction, index access, spread/rest, destructuring,
  reference semantics, `Object.keys/values/entries`.
- Exercise: `records.js` — given a fixed in-file array of player objects,
  destructure and rebuild a summary object using spread only (no mutation of
  the input).
- Check: `node records.js`
- Expected: one `JSON.stringify`'d summary line — **exact**.

**L5 · map / filter / reduce**
- Concepts: the array trinity, chaining, when each applies, `reduce` with an
  object accumulator, avoiding the `for` loop reflex.
- Exercise: `pipeline.js` — from a fixed in-file dataset produce total,
  top-3, and a grouped-by-tag count in three chained pipelines.
- Check: `node pipeline.js`
- Expected: three fixed lines — **exact**.

**L6 · Error handling**
- Concepts: `throw`/`try`/`catch`/`finally`, `Error` subclasses, stack
  traces, failing fast vs recovering, rethrowing.
- Exercise: `errors.js` — define `ValidationError extends Error`; write
  `parsePlayer(raw)` that throws it on bad input, and a driver that catches,
  prints `name: message` per failure, and still prints the final tally.
- Check: `node errors.js`
- Expected: fixed lines including `ValidationError: age must be a number` —
  **contains**.

**L7 · Modules**
- Concepts: `export`/`import`, default vs named, one responsibility per
  file, `node` resolving relative paths under `"type":"module"`.
- Exercise: split L5's `pipeline.js` into `data.js`, `stats.js`, `main.js`;
  `main.js` imports and prints the same three lines.
- Check: `node main.js`
- Expected: same three fixed lines as L5 — **exact** (proves the refactor
  changed nothing).

**L8 · Promises and async/await**
- Concepts: `Promise` construction, `.then` vs `await`, `Promise.all`,
  async function = promise-returning function, `try`/`catch` around `await`.
- Exercise: `async.js` — wrap two deterministic timer-based tasks
  (`setTimeout` 10ms/20ms), await them sequentially then in parallel, print
  elapsed ordering markers, and catch a timer task that rejects.
- Check: `node async.js`
- Expected: `seq done / par done / caught: boom` — **contains**.

**L9 · Deterministic async patterns (no network)**
- Concepts: streaming input via `node:readline`, chunked/queued work,
  backpressure-by-batch, sequential vs concurrent processing of a list.
- Exercise: `lines.js` — feed a fixed in-file string through
  `readline.createInterface` over a `StringReader`, count and uppercase
  words per line, print per-line counts then a total.
- Check: `node lines.js`
- Expected: fixed per-line counts + `total: N` — **exact**. This artifact is
  the refactor target for L17.

### Act II — TypeScript (Lessons 10–17)

**L10 · Why TypeScript**
- Concepts: types as a reading tool; the editor's diagnostics as a
  superpower; what tsx does (strip types, run on node); first squiggles.
- Exercise: fix `broken.ts` until LSP diagnostics are clean and it prints
  the expected lines; the starter has three deliberate type errors.
- Check: `npx --no-install tsx broken.ts`
- Expected: two fixed lines — **exact**.

**L11 · Annotations and inference**
- Concepts: where inference works (leave it), where annotation is the
  contract (function params and returns), `noImplicitAny` thinking,
  `type` aliases for domain nouns.
- Exercise: `shapes.ts` — annotate every exported function's params and
  returns explicitly, let locals infer; compute fixed areas.
- Check: `npx --no-install tsx shapes.ts`
- Expected: fixed area lines — **exact**.

**L12 · Narrowing**
- Concepts: `typeof`/`instanceof`/`in` guards, truthiness narrowing,
  discriminated unions with a `kind` tag, custom type predicates.
- Exercise: `narrow.ts` — one `shape` union with four `kind`s; write
  `area(shape)` with a `switch` the compiler can verify exhaustive, plus a
  `isPlayer(v): v is Player` predicate.
- Check: `npx --no-install tsx narrow.ts`
- Expected: four fixed areas — **exact**.

**L13 · Unions, literals, exhaustive switch**
- Concepts: literal types, union of string literals vs enum, `never` in the
  default arm, why exhaustiveness is a refactoring safety net.
- Exercise: `commands.ts` — model three CLI verbs as a literal-tagged union;
  add a fourth verb and let the `never` check catch the missed arm (log
  proof it was caught).
- Check: `npx --no-install tsx commands.ts`
- Expected: `handled: add, list, sum / unreachable arm verified` —
  **exact**.

**L14 · `interface` vs `type`**
- Concepts: declaration merging, extends vs intersection, when a type alias
  is the only option (unions, mapped/utility compositions), the one-sentence
  rule for each.
- Exercise: `model.ts` — same domain twice: an `interface` hierarchy with
  `extends`, and a `type` composition using `&` and a union; prove both
  accept the same data.
- Check: `npx --no-install tsx model.ts`
- Expected: `interface ok / type ok` — **exact**.

**L15 · Generics and utility types**
- Concepts: type parameters with constraints (`<T extends ...>`), generic
  functions vs generic types, `Partial`/`Required`/`Readonly`/`Pick`/`Omit`/
  `Record`, when a generic earns its keep.
- Exercise: `generics.ts` — write `pluck<T, K extends keyof T>(rows, key)`
  and `Result<T, E>`; derive `PlayerPatch = Partial<Pick<...>>` and use both.
- Check: `npx --no-install tsx generics.ts`
- Expected: fixed pluck output + `patch applied` — **exact**.

**L16 · Typing async and typed API boundaries**
- Concepts: `Promise<T>` in signatures, typed errors on async paths,
  `unknown` at every external edge (JSON, argv) narrowed by hand, zero `any`
  and zero `as`.
- Exercise: `boundary.ts` — `loadConfig(raw: unknown): Config` that
  validates field-by-field and throws `ValidationError`; an async
  `withRetry(fn, tries)` using timers that returns `Result<T, Error>`.
- Check: `npx --no-install tsx boundary.ts`
- Expected: `ok / ok / retried 2x: boom` — **exact**.

**L17 · Typed refactor of an Act I artifact**
- Concepts: porting working JS to TS without changing behavior; letting the
  compiler find the latent bugs; incremental typing order.
- Exercise: copy L9's `lines.js` to `lines/` as `.ts` modules; type every
  boundary, keep output byte-identical. The starter ships with two latent
  type errors the port must surface and fix (an implicit any and a wrong
  field name that JS silently produced `undefined` for).
- Check: `npx --no-install tsx lines/main.ts`
- Expected: identical to L9's output — **exact** (behavior-preserving proof).

### Final project (Lesson 18)

See next section. Check: `npx --no-install tsx src/main.ts --selftest` —
**exact**, plus `--selftest` exit 0.

## Final project — `ledger`, a zero-dependency TS CLI

A local expense journal. ~300–400 lines across 6–8 modules in the lesson
scratch, run as `npx --no-install tsx src/main.ts <command>`. Stdlib only
(`node:fs/promises`, `node:path`). Deterministic by construction: the data
file lives in the scratch dir, IDs are monotonic counters, timestamps are an
explicit `--at <iso>` flag the tests set (never `Date.now()`).

**Spec.**
- Commands: `add --label <s> --cents <n> --tag <s> [--at <iso>]`,
  `list [--tag <s>]`, `summary [--tag <s>]`, `edit <id> --label|--cents`
  (via a `Patch` type), `export`, `selftest`.
- Storage: `data/ledger.json`, an object with `nextId` and `entries: Entry[]`;
  every read goes through `loadLedger(raw: unknown): Ledger` (field-by-field
  validation, `ValidationError` on bad shape, no `as`, no `any`).
- Domain: `Entry` = `interface`; `Command` = discriminated union of literal
  verbs parsed from `argv` (`parseCommand(args: string[]): Command`); the
  dispatch `switch` is exhaustive with a `never` default.
- Async: all I/O via `async`/`await` with explicit `Promise<T>` returns; one
  `withRetry` wrapper (timers) around writes; `Promise.all` where two reads
  can go in parallel.
- Generics + utilities: `Result<T, E>`, `pluck`, `Patch = Partial<Pick<Entry,
  'label' | 'cents'>>`, `Readonly<Entry>` on read paths.
- Output: plain, uncolored, line-per-record; `summary` prints per-tag totals
  using `reduce` over a `Record<string, number>`.
- `--selftest`: runs an in-repo scenario against a temp file — add, edit,
  list-filter, retry-once, one bad-JSON rejection — and prints
  `selftest ok`. This is what the check runs.

**Bar mapping.**

| Bar item | Exercised by |
| --- | --- |
| 1. Read typed code | the 6–8 module layout; navigation needed to wire commands |
| 2. Modify without breaking | `edit` + `Patch` changing `Entry` consumers |
| 3. Typed async | `Promise<T>` I/O, `Promise.all`, `withRetry` |
| 4. Type a boundary | `loadLedger(unknown)`, `parseCommand(argv)` |
| 5. interface vs type | `Entry` vs `Command`/`Patch`/`Result` |
| 6. Generics | `Result`, `pluck`, `withRetry<T>` |
| 7. Array trinity | filtering `list`, `summary` totals via `reduce` |
| 8. JS traps | integer cents (no float money), `===` in filters |
| 9. Modules | one file per command + storage + domain |
| 10. Fail loudly, typed | `ValidationError` subclass, `Result` on retry |
| 11. Terminal fluency | student runs every command themselves first |

## Authoring and course lint

Same bar as code; the course lint runs in CI and a course that stops passing
its own checks fails the build.

- **Pass from solution, fail from starter.** Every check must be validated
  both ways, with fixtures: the lesson's solution files (shipped to the lint,
  not the student) produce the expected output exactly; the starter files
  produce something else. `exit_code` mode alone is too weak to use except
  where output is genuinely unobservable.
- **No version-dependent expected values.** Never assert on `node --version`,
  `tsx --version`, stack traces, file paths, or error text we don't control.
  L0 probes features, not versions. If an error message must be matched,
  match `ValidationError: <our own fixed string>` — **contains**.
- **No nondeterminism.** No `Date.now()`, `Math.random`, network, process
  ids, iteration order beyond spec, or locale-sensitive formatting in
  observed output. Check env pins `TZ=UTC`, `LC_ALL=C`, `NO_COLOR=1`,
  `npm_config_yes=false`.
- **Timeouts.** Every check declares one (default 15s; async lessons 30s).
  Timer-based exercises use short, fixed delays.
- **Solution files live next to lessons** as `solution/` siblings, consumed
  by the lint, excluded from student materialization.

## LLNZY integration notes

- TS/JS/TSX tree-sitter grammars and the TS language server integration
  already ship; exercise files open as normal editor tabs with live
  diagnostics, hover, completions, and go-to-definition. Lesson text should
  *assign* LSP usage ("hover `Entry`, read the inferred type") so students
  form the habit — this is the differentiation vs. any browser course.
- Act II lessons intentionally start with starter files that produce
  diagnostics; the visible red squiggles are the lesson.
- The terminal is free-play. Lessons say "run the check yourself first" —
  the Check button is confirmation, not the first attempt.
- Starter-file chips in the lesson view deep-link into the editor; the
  "Open in terminal" path drops the shell into the lesson scratch dir, where
  `node` / `npx --no-install tsx` work identically to the check.
- Task detection will not light up for node exercises (no task file). Do not
  add one; the check command is the task.

## Risks

- **node version drift** (v20 → v22+ output or semantics changes). Checks
  assert only on our own fixed strings; L0 probes features; course lint in
  CI on a pinned node catches regressions before students do.
- **tsx first-run network.** Solved by the L0 one-time consented install +
  `npx --no-install` everywhere + walk-up layout verified by L0's check from
  a nested dir; `npm_config_yes=false` turns any accidental fetch into a
  loud failure. Residual risk: registry outage during setup — the lesson
  text says plainly "you need network once, now."
- **Curriculum length creep** (18 lessons is already the ceiling of the
  brief). Fence: no new lessons without cutting one; deepen exercises, don't
  add stages. If a split is unavoidable, the merge candidates are L13 into
  L12 and L15 into L16.
- **npm walk-up resolution changes** break every TS check at once. Mitigation
  is documented above (PATH prepend fallback) and L0 fails loudly at setup,
  not at lesson 11.
- **500-line-bar overreach.** The final project is ~400 lines. If the bar
  review finds a bar item unexercised, the fix is a project requirement, not
  a new lesson.

## Work items

Production checklist. Order matters; lint lands before bulk content.

- [ ] Course scaffold: `courses/js-ts/course.toml`, root `package.json`
      (`{"type":"module","private":true}`), lesson ordering, `.gitignore` for
      `node_modules/` and lesson `data/`.
- [ ] Course lint: parse-all-lessons, run-every-check-from-solution,
      run-every-check-from-starter-must-fail, banned-pattern scan
      (`Date.now`, `Math.random`, `fetch`, `process.version` in observed
      output, `any`/`as` in Act II starters). CI job on pinned node 20.
- [ ] Act I lesson files L0–L9 with starters + `solution/` siblings +
      check fixtures (both directions).
- [ ] Act II lesson files L10–L17, same; L10 and L17 starters must produce
      LSP diagnostics on open (verify in the app, not just tsx).
- [ ] Final project: spec lesson, starter tree, full `solution/`, minimal
      `tsconfig.json` (strict, `noEmit`), and a `--selftest` that the check
      asserts on exactly.
- [ ] tsx setup flow: L0 install instructions, the nested-dir resolution
      check, and the PATH-prepend fallback implemented and tested.
- [ ] Proficiency bar review: map each of the 11 bar items to a concrete
      project requirement (table above) and sign off that the map holds.
- [ ] Manual smoke pass in LLNZY: open L12, confirm diagnostics render,
      completions fire in an exercise file, "Open in terminal" lands in the
      right scratch dir, Check passes from solution and fails from starter.
- [ ] Docs: README feature line, architecture map entry for the course dir,
      manual smoke test entries.
