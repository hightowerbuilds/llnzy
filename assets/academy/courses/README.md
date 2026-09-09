# Academy courses

The catalog contains independent Rust, JavaScript, and TypeScript courses.
Start with JavaScript if functions, arrays, modules, or promises are unfamiliar;
TypeScript builds on those skills. Each JS/TS course has five ordered modules,
worked explanations, practice exercises, and a final project.

| Module | JavaScript (11 lessons) | TypeScript (10 lessons) |
| --- | --- | --- |
| 1 | Runtime & Values | The Compiler and Contracts |
| 2 | Control Flow & Functions | Trust and Control Flow |
| 3 | Objects & Data Pipelines | Modeling Reusable Data |
| 4 | Errors & Modules | Async Work and Module Boundaries |
| 5 | Async, Streams & Capstone | A Typed Ledger |

## Toolchain

Use Node.js 22 or newer for these courses. JavaScript exercises use only Node's
built-in modules. TypeScript exercises also require TypeScript 5.9.3 on PATH:

```sh
npm install --global typescript@5.9.3
```

This is a one-time setup command requiring network access. Exercise checks do
not install packages. TypeScript exercises compile with strict checking before
running the generated JavaScript. See the [compiler documentation](https://www.typescriptlang.org/docs/handbook/compiler-options.html)
for the distinction between checking types and emitting JavaScript.

## Practice today

The Academy displays course modules, lesson text, exercise prompts, file paths,
and check commands. Creating exercise workspaces and running checks from the UI
are still pending. To practice now, export starter files from the repository root
using Python 3.11+:

```sh
python3 scripts/check_academy_courses.py --export javascript L00 /tmp/llnzy-js-L00
python3 scripts/check_academy_courses.py --export typescript L00 /tmp/llnzy-ts-L00
```

The destination must not exist; this prevents overwriting earlier work. Open the
exported directory in LLNZY, read the matching Academy lesson, edit the files,
and run its check command from that directory. Export each subsequent lesson
into a new directory so files and compiler output cannot mix.

Solutions are inline authoring fixtures for review and validation. Try the
exercise before consulting them. TypeScript lessons include `tsconfig.json` so
the editor and terminal compiler use the same settings.

## Validate authored content

With Python 3.11+, Node, and `tsc` on PATH:

```sh
python3 scripts/check_academy_courses.py
cargo test --lib academy::
```

The Python runner executes each solution and starter in separate temporary
directories. A solution must satisfy its declared check; a starter must fail
it. Exact checks compare stdout including its final newline, and output checks
also require exit status zero. A timeout or missing executable fails validation.
CI runs these fixtures with Node 22 and TypeScript 5.9.3. The Rust tests load
every course through the application's strict schema and markdown parser.

The earlier combined `js-ts` roadmap is historical design context. These two
standalone courses implement the subsequent request for separate curricula;
they use `tsc` instead of the roadmap's proposed `tsx` runtime-only checks.
