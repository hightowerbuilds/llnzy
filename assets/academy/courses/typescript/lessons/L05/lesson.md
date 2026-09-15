+++
title = "Generics That Preserve Relationships"
concepts = ["type parameters", "keyof", "indexed access", "generic Result"]

[[exercise]]
prompt = "Implement pluck and mapResult in generic.ts. Preserve selected property types, transform only successful results, and retain the error type and value on failure."
[exercise.check]
command = ["sh", "-c", "tsc --project tsconfig.json --pretty false && node dist/check.js"]
expected = "L05 passed\n"
mode = "exact"
timeout_secs = 30

[[exercise.files]]
path = "package.json"
starter = '''
{"type":"module","private":true}
'''
solution = '''
{"type":"module","private":true}
'''

[[exercise.files]]
path = "tsconfig.json"
starter = '''
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "NodeNext",
    "moduleResolution": "NodeNext",
    "lib": [
      "ES2022",
      "DOM"
    ],
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "exactOptionalPropertyTypes": true,
    "noEmitOnError": true,
    "outDir": "dist"
  },
  "include": [
    "*.ts"
  ]
}
'''
solution = '''
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "NodeNext",
    "moduleResolution": "NodeNext",
    "lib": [
      "ES2022",
      "DOM"
    ],
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "exactOptionalPropertyTypes": true,
    "noEmitOnError": true,
    "outDir": "dist"
  },
  "include": [
    "*.ts"
  ]
}
'''

[[exercise.files]]
path = "assert.ts"
starter = '''
export function equal(actual: unknown, expected: unknown, label: string): void {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(`${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
  }
}
export function assert(condition: unknown, label: string): asserts condition {
  if (!condition) throw new Error(label);
}
'''
solution = '''
export function equal(actual: unknown, expected: unknown, label: string): void {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(`${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
  }
}
export function assert(condition: unknown, label: string): asserts condition {
  if (!condition) throw new Error(label);
}
'''

[[exercise.files]]
path = "check.ts"
starter = '''
import { pluck, mapResult } from "./generic.js";
import { equal } from "./assert.js";
const rows = [{name: "Ada", cents: 125}, {name: "Lin", cents: 75}];
const names: string[] = pluck(rows, "name");
const cents: number[] = pluck(rows, "cents");
equal(names, ["Ada", "Lin"], "names");
equal(cents, [125, 75], "prices");
equal(pluck(rows.slice(0, 0), "name"), [], "empty pluck");
equal(mapResult({ok: true, value: 3}, n => n * 2), {ok: true, value: 6}, "transform success");
let calls = 0;
equal(mapResult<number, string, string>({ok: false, error: "missing"}, n => {calls++; return String(n);}), {ok: false, error: "missing"}, "preserve error");
equal(calls, 0, "skip transform on failure");
if (false) {
  // @ts-expect-error nonexistent property
  pluck(rows, "missing");
  // @ts-expect-error names are not numbers
  const wrong: number[] = pluck(rows, "name");
}
console.log("L05 passed");
'''
solution = '''
import { pluck, mapResult } from "./generic.js";
import { equal } from "./assert.js";
const rows = [{name: "Ada", cents: 125}, {name: "Lin", cents: 75}];
const names: string[] = pluck(rows, "name");
const cents: number[] = pluck(rows, "cents");
equal(names, ["Ada", "Lin"], "names");
equal(cents, [125, 75], "prices");
equal(pluck(rows.slice(0, 0), "name"), [], "empty pluck");
equal(mapResult({ok: true, value: 3}, n => n * 2), {ok: true, value: 6}, "transform success");
let calls = 0;
equal(mapResult<number, string, string>({ok: false, error: "missing"}, n => {calls++; return String(n);}), {ok: false, error: "missing"}, "preserve error");
equal(calls, 0, "skip transform on failure");
if (false) {
  // @ts-expect-error nonexistent property
  pluck(rows, "missing");
  // @ts-expect-error names are not numbers
  const wrong: number[] = pluck(rows, "name");
}
console.log("L05 passed");
'''

[[exercise.files]]
path = "generic.ts"
starter = '''
export type Result<T, E> = {ok: true; value: T} | {ok: false; error: E};
export function pluck<T, K extends keyof T>(rows: readonly T[], key: K): T[K][] {
  return [];
}
export function mapResult<T, U, E>(result: Result<T, E>, transform: (value: T) => U): Result<U, E> {
  throw new Error("TODO: map only success");
}
'''
solution = '''
export type Result<T, E> = {ok: true; value: T} | {ok: false; error: E};
export function pluck<T, K extends keyof T>(rows: readonly T[], key: K): T[K][] {
  return rows.map(row => row[key]);
}
export function mapResult<T, U, E>(result: Result<T, E>, transform: (value: T) => U): Result<U, E> {
  return result.ok ? {ok: true, value: transform(result.value)} : result;
}
'''
+++
# Generics That Preserve Relationships

A useful generic expresses a relationship rather than merely accepting many values. `pluck<T, K extends keyof T>` says that the key belongs to the row type, and its result contains the values at that key: `T[K][]`. The same implementation can extract names or prices while retaining string or number information.

Compare this with a function returning `unknown[]`: every caller would need to narrow again. A function returning `any[]` is worse because mistakes pass silently. Let inference choose T and K at the call site. Most callers should not need explicit type arguments.

`Result<T, E>` models success and failure as data. `mapResult` has three roles: input value T, transformed value U, and unchanged error E. Narrow on `ok`, call the transform only for success, and return the error branch untouched otherwise. The type declaration carries this guarantee across callers.

The unreachable block in the test contains `@ts-expect-error` examples. Each must produce a compiler error; if a signature becomes too permissive, TypeScript reports the directive as unused. These checks verify the public type contract as well as runtime behavior.

A small example to compare with your exercise:

```ts
function identity<T>(value: T): T { return value; }
const name = identity("Ada"); // string information is retained
```

Choose **Open practice** in the exercise card to create or reopen this lesson’s files. Edit the implementation, save your changes, then choose **Check work**. Your practice folder is reused when you return; opening it again keeps your edits. No source checkout or Python is needed.

You can also run this command in the practice folder’s terminal:

```bash
tsc --project tsconfig.json --pretty false && node dist/check.js
```

Compilation must succeed before Node runs. Read the first compiler diagnostic, fix its cause, and rerun.

Keep `check.ts`, `assert.ts`, and `tsconfig.json` intact; edit the exercise implementation files.

The checks compare behavior and compile type examples. Printing the success message yourself does not implement the exercise.

After passing, read the exported declarations and explain their input and output types without executing them. If editor hover is available, use it to compare your prediction.

## Hint before a solution

Write down which input type determines the selected value type. In the error branch, should the transform run at all?

Try one focused change and check again. Before comparing with a reference solution, explain the failing case in your own words. A passing check covers the supplied examples; also try a new input and explain why your implementation handles it.
