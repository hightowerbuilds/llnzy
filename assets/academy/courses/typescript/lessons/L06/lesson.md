+++
title = "Typed Async Results"
concepts = ["Promise<T>", "unknown catch values", "Promise.all", "bounded retry"]

[[exercise]]
prompt = "Implement attempt<T> to retry a rejected task at most tries times, stop immediately after success, and return a typed error on exhaustion. Invalid tries (noninteger or less than one) returns Error(\"tries must be a positive integer\") without calling the task. Normalize non-Error rejections with new Error(String(cause))."
[exercise.check]
command = ["sh", "-c", "tsc --project tsconfig.json --pretty false && node dist/check.js"]
expected = "L06 passed\n"
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
import { attempt } from "./async.js";
import { equal, assert } from "./assert.js";
let calls = 0;
const recovered = await attempt(async () => {calls++; if (calls < 2) throw new Error("temporary"); return 42;}, 3);
equal(recovered, {ok: true, value: 42}, "retry success");
equal(calls, 2, "stop after success");
let failures = 0;
const failed = await attempt(async () => {failures++; throw "offline";}, 2);
assert(!failed.ok, "failure result");
assert(failed.error instanceof Error, "normalize error");
equal(failed.error.message, "offline", "failure message");
equal(failures, 2, "bounded attempts");
for (const tries of [0, -1, 1.5, NaN]) {
  const invalid = await attempt(async () => {throw new Error("task must not run");}, tries);
  assert(!invalid.ok, "reject invalid budget");
  equal(invalid.error.message, "tries must be a positive integer", "budget error");
}
const pair = await Promise.all([attempt(async () => "first", 1), attempt(async () => "second", 1)]);
equal(pair, [{ok: true, value: "first"}, {ok: true, value: "second"}], "parallel ordered results");
console.log("L06 passed");
'''
solution = '''
import { attempt } from "./async.js";
import { equal, assert } from "./assert.js";
let calls = 0;
const recovered = await attempt(async () => {calls++; if (calls < 2) throw new Error("temporary"); return 42;}, 3);
equal(recovered, {ok: true, value: 42}, "retry success");
equal(calls, 2, "stop after success");
let failures = 0;
const failed = await attempt(async () => {failures++; throw "offline";}, 2);
assert(!failed.ok, "failure result");
assert(failed.error instanceof Error, "normalize error");
equal(failed.error.message, "offline", "failure message");
equal(failures, 2, "bounded attempts");
for (const tries of [0, -1, 1.5, NaN]) {
  const invalid = await attempt(async () => {throw new Error("task must not run");}, tries);
  assert(!invalid.ok, "reject invalid budget");
  equal(invalid.error.message, "tries must be a positive integer", "budget error");
}
const pair = await Promise.all([attempt(async () => "first", 1), attempt(async () => "second", 1)]);
equal(pair, [{ok: true, value: "first"}, {ok: true, value: "second"}], "parallel ordered results");
console.log("L06 passed");
'''

[[exercise.files]]
path = "async.ts"
starter = '''
export type Result<T, E> = {ok: true; value: T} | {ok: false; error: E};
export async function attempt<T>(task: () => Promise<T>, tries: number): Promise<Result<T, Error>> {
  return {ok: false, error: new Error("TODO")};
}
'''
solution = '''
export type Result<T, E> = {ok: true; value: T} | {ok: false; error: E};
export async function attempt<T>(task: () => Promise<T>, tries: number): Promise<Result<T, Error>> {
  if (!Number.isInteger(tries) || tries < 1) {
    return {ok: false, error: new Error("tries must be a positive integer")};
  }
  for (let index = 0; index < tries; index++) {
    try {
      return {ok: true, value: await task()};
    } catch (cause: unknown) {
      if (index === tries - 1) {
        return {ok: false, error: cause instanceof Error ? cause : new Error(String(cause))};
      }
    }
  }
  throw new Error("unreachable attempt state");
}
'''
+++
# Typed Async Results

An async function returns a promise even when its body returns an ordinary value. `Promise<Result<T, Error>>` tells the caller to await first and then inspect the result tag.

JavaScript promises do not have a separate generic parameter for their rejection type; a caught rejection is unknown because a task may throw a string or any other value.

Narrow caught values with `instanceof Error` before using `.message`. Normalize other values into an Error so consumers see one stable error contract.

Retrying belongs at the layer that knows whether another attempt is appropriate. This exercise uses deterministic injected tasks; it has no timers, network, or random failures.

Validate the attempt budget before invoking the task. Then use a bounded loop with await inside it, returning as soon as an attempt succeeds. On the final rejection, return the normalized error. An explicit unreachable throw after the loop can document that validation plus iteration must already have returned.

The tests also await two independent attempts with `Promise.all`. Calling the two async functions starts the operations; Promise.all waits for both and preserves input order in the returned array, regardless of completion order.

Do not write a floating async call: its failure could escape the flow you intended to handle.

A small example to compare with your exercise:

```ts
async function twice(task: () => Promise<number>): Promise<number> {
  const value = await task();
  return value * 2;
}
```

Run the check from the exported exercise directory in your terminal:

```bash
tsc --project tsconfig.json --pretty false && node dist/check.js
```

Compilation must succeed before Node runs. Read the first compiler diagnostic, fix its cause, and rerun.

Keep `check.ts`, `assert.ts`, and `tsconfig.json` intact; edit the exercise implementation files.

The checks compare behavior and compile type examples. Printing the success message yourself does not implement the exercise.

After passing, hover the exported functions in the editor and explain their input and output types without executing them.
