# Academy courses

Choose a course by what you already know and what you want to build:

| Course | Before you start | Shipped scope and outcome |
| --- | --- | --- |
| JavaScript | Basic file and terminal skills; no prior programming language | 11 lessons in five modules, from values to a tested expense-report CLI using Node.js. Browser interfaces are outside this course. |
| TypeScript | JavaScript functions, arrays, objects, modules, and promises | 10 lessons in five modules, from strict compilation to a modular async ledger report. Take JavaScript first if those prerequisites are unfamiliar. |
| Rust | Basic file and terminal skills; no prior Rust | Seven introductory lessons aligned with chapters 1–3 of *The Rust Programming Language*, 3rd edition. Ends with functions and compound types; ownership, borrowing, structs, and a capstone are not shipped yet. The book is optional companion reading. |
| Elixir | Basic programming familiarity; no prior Elixir or Erlang | 10 lessons in five modules, from immutable values to a supervised task tracker. Phoenix and durable storage are extensions. |

## Practice in LLNZY

1. Open a course and select its first lesson. Read the explanation and exercise prompt.
2. Choose **Open practice** on the exercise card. LLNZY prepares a persistent folder with the bundled starter files and opens the workspace. Returning to that exercise reuses your work.
3. Edit the implementation files and save your changes. Keep supplied checks intact unless the lesson explicitly asks you to add a test.
4. Choose **Check work** to run the exercise from its practice folder. Read the result and fix one issue at a time. The intentionally incomplete starters should fail; that first failure is part of learning.
5. If stuck, use the lesson’s **Hint before a solution** section. Predict one input and trace your code before comparing with a reference solution. After passing, try a new case and explain your approach in your own words.

The packaged app includes the course material. Students do not need Python or the
LLNZY source repository. Each lesson has its own workspace; later exercises do
not require copying build output from an earlier lesson. You may also run the
shown check command in the practice folder’s terminal.

Checks are feedback on supplied examples, not proof of complete understanding.
Reading, explaining, experimenting, and writing your own cases matter too.

## Install your course tools once

LLNZY currently targets macOS. Install the tools for your selected course, then
reopen the app so it inherits their updated PATH. Installing tools can require
network access; bundled exercise checks do not install packages.

| Course | Required tools | Verify in a terminal |
| --- | --- | --- |
| JavaScript | [Node.js](https://nodejs.org/en/download) 22 or newer | `node --version` |
| TypeScript | Node.js 22+ and TypeScript 5.9.3 | `node --version`, `tsc --version` |
| Rust | [Stable Rust through rustup](https://www.rust-lang.org/tools/install), including Cargo | `rustc --version`, `cargo --version` |
| Elixir | [Elixir and compatible Erlang/OTP](https://elixir-lang.org/install.html); fixtures validated with Elixir 1.19.5 and OTP 28 | `elixir --version`, `mix --version` |

For TypeScript, install the compiler once:

```sh
npm install --global typescript@5.9.3
```

Use a user-managed Node installation if global packages are not writable.
TypeScript checks compile with strict checking before running JavaScript, and
`tsconfig.json` keeps the editor and compiler settings together. JavaScript
uses Node’s built-in modules; Rust uses its standard library; Elixir needs no
Hex packages. Elixir L06 includes a Mix project and uses `mix test`; the other
Elixir lessons use `elixir check.exs`.

Editor hover, completion, and navigation depend on a separately installed
language server. They are optional: the course runtime can execute your check
without editor assistance. Elixir currently has no bundled editor grammar or
built-in language-server entry, so its editing assistance differs from Rust,
JavaScript, and TypeScript.

## When a check fails

- **Tool missing:** verify the tool in the table above, finish installation, and reopen LLNZY. This is a setup issue, not an incorrect answer.
- **File missing in a manual terminal run:** use the practice folder as the working directory. It should contain the files named in the exercise card. Open practice prepares those files.
- **Compiler error:** start with the first diagnostic pointing into your implementation. Repair its cause, save, and check again before chasing later errors.
- **Assertion or output mismatch:** compare expected and actual values. Trace the reported input through your function. Exact-output checks include capitalization, spacing, and the final newline; avoid extra debug printing on stdout.
- **Timeout:** look for a loop that never finishes, a missing reply, or interactive input. The bundled checks use deterministic inputs and should not wait for typing.

## Validate authored content

Course maintainers need Python 3.11+ and all four courses’ runtimes on PATH:

```sh
python3 scripts/check_academy_courses.py
cargo test --lib academy::
```

The Python runner executes each solution and starter in separate temporary
directories. Every currently bundled exercise, including Rust’s setup lessons,
has an intentionally incomplete starter: a solution must pass and a starter
must fail. A compiling starter is not necessarily a passing starter; Rust L00
must print the required line. If a future setup exercise deliberately starts
complete, add an explicit, narrowly scoped validator expectation with its
reason instead of silently accepting all passing starters.

Exact checks compare stdout including the final newline. Output checks also
require exit status zero. A timeout or missing executable fails validation.
CI uses Node 22, TypeScript 5.9.3, stable Rust, Elixir 1.19.5, and OTP 28.0.
The Rust loader tests validate every course against the app’s schema and
markdown parser. These checks validate curriculum fixtures; they do not replace
interactive testing of the packaged learning workflow.

To validate one course or export a standalone starter as an author:

```sh
python3 scripts/check_academy_courses.py --course rust
python3 scripts/check_academy_courses.py --export javascript L00 /tmp/llnzy-js-L00
```

The export destination must not exist, preventing accidental overwrites. Choose
a fresh directory per lesson. Solutions remain inline authoring fixtures; they
are separate from students’ persistent practice files.

The earlier combined `js-ts` roadmap is historical design context. The shipped
JavaScript and TypeScript courses are separate curricula and use `tsc` for
TypeScript compilation rather than the roadmap’s proposed `tsx` checks.
