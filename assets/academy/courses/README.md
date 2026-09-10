# Academy courses

The catalog contains independent Rust, JavaScript, TypeScript, and Elixir courses.
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

With Python 3.11+, Node, `tsc`, Elixir, and Mix on PATH:

```sh
python3 scripts/check_academy_courses.py
cargo test --lib academy::
```

The Python runner executes each solution and starter in separate temporary
directories. A solution must satisfy its declared check; a starter must fail
it. Exact checks compare stdout including its final newline, and output checks
also require exit status zero. A timeout or missing executable fails validation.
CI runs these fixtures with Node 22, TypeScript 5.9.3, Elixir 1.19.5, and OTP 28.0. The Rust tests load
every course through the application's strict schema and markdown parser.

The earlier combined `js-ts` roadmap is historical design context. These two
standalone courses implement the subsequent request for separate curricula;
they use `tsc` instead of the roadmap's proposed `tsx` runtime-only checks.

## Elixir

Elixir has ten ordered lessons across five modules:

| Module | Lessons |
| --- | --- |
| IEx & Immutable Values | First program; values, rebinding, and maps |
| Patterns & Functions | Tagged results; guards and recursion |
| Collections & Data Pipelines | Enum and pipes; input validation |
| Mix, Tests & Processes | Mix and ExUnit; messages and Tasks |
| OTP & Capstone | GenServer; a supervised task tracker |

Basic programming familiarity is enough; other courses are not prerequisites.
Install Elixir and compatible Erlang/OTP using the
[official instructions](https://elixir-lang.org/install.html). Fixtures are
validated with Elixir 1.19.5 and OTP 28; CI uses OTP 28.0. Check
`elixir --version` and `mix --version`. No Hex packages or network access are
needed for exercises. L06 exports a Mix project; other lessons use ExUnit scripts.

```sh
python3 scripts/check_academy_courses.py --export elixir L00 /tmp/llnzy-elixir-l00
python3 scripts/check_academy_courses.py --course elixir
```

Run `elixir check.exs` from the exported directory (`mix test` for L06).
The validator checks both passing solutions and failing starters, including
capstone worker restart and loss of in-memory state. The default validator now
runs all three exercise-backed courses and requires all their runtimes.

Reference material: [Elixir introduction](https://hexdocs.pm/elixir/introduction.html),
[GenServer](https://hexdocs.pm/elixir/GenServer.html), and
[Supervisor](https://hexdocs.pm/elixir/Supervisor.html). The lessons are an
independent foundation curriculum; Phoenix and durable storage are extensions.
Elixir files can be edited and run in the terminal; the editor does not yet
provide an Elixir tree-sitter grammar.
