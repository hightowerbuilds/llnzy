+++
title = "Ledger Boundary: From JSON to Trusted Entries"
concepts = ["runtime validation", "safe integer cents", "unique identifiers", "custom errors"]

[[exercise]]
prompt = "Implement parseEntries in boundary.ts. Require an array of entries with unique positive safe-integer IDs, nonblank labels and tags, and nonnegative safe-integer cents. Return newly constructed entries containing only those four fields. Throw ValidationError with the messages documented in the tests."
[exercise.check]
command = ["sh", "-c", "tsc --project tsconfig.json --pretty false && node dist/check.js"]
expected = "L08 passed\n"
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
import { parseEntries } from "./boundary.js";
import { ValidationError } from "./model.js";
import { equal, assert } from "./assert.js";
const source = {id: 1, label: "tea", cents: 250, tag: "food", extra: true};
const entries = parseEntries([source]);
equal(entries, [{id: 1, label: "tea", cents: 250, tag: "food"}], "trusted fields only");
source.cents = 999;
equal(entries[0]?.cents, 250, "copy boundary");
equal(parseEntries([]), [], "empty ledger");
equal(parseEntries([{...source, cents: 0}])[0]?.cents, 0, "zero cents");
function rejects(raw: unknown, message: string): void {
  let caught: unknown;
  try { parseEntries(raw); } catch (cause: unknown) { caught = cause; }
  assert(caught instanceof ValidationError, "typed validation error");
  equal(caught.message, message, "validation message");
}
rejects({}, "entries must be an array");
rejects([source, source], "duplicate id");
for (const value of [null, {}, {...source, id: 0}, {...source, id: 1.5}, {...source, id: Number.MAX_SAFE_INTEGER + 1}, {...source, label: " "}, {...source, tag: ""}, {...source, cents: -1}, {...source, cents: 1.1}, {...source, cents: Infinity}, {...source, cents: "10"}]) {
  rejects([value], "invalid entry");
}
console.log("L08 passed");
'''
solution = '''
import { parseEntries } from "./boundary.js";
import { ValidationError } from "./model.js";
import { equal, assert } from "./assert.js";
const source = {id: 1, label: "tea", cents: 250, tag: "food", extra: true};
const entries = parseEntries([source]);
equal(entries, [{id: 1, label: "tea", cents: 250, tag: "food"}], "trusted fields only");
source.cents = 999;
equal(entries[0]?.cents, 250, "copy boundary");
equal(parseEntries([]), [], "empty ledger");
equal(parseEntries([{...source, cents: 0}])[0]?.cents, 0, "zero cents");
function rejects(raw: unknown, message: string): void {
  let caught: unknown;
  try { parseEntries(raw); } catch (cause: unknown) { caught = cause; }
  assert(caught instanceof ValidationError, "typed validation error");
  equal(caught.message, message, "validation message");
}
rejects({}, "entries must be an array");
rejects([source, source], "duplicate id");
for (const value of [null, {}, {...source, id: 0}, {...source, id: 1.5}, {...source, id: Number.MAX_SAFE_INTEGER + 1}, {...source, label: " "}, {...source, tag: ""}, {...source, cents: -1}, {...source, cents: 1.1}, {...source, cents: Infinity}, {...source, cents: "10"}]) {
  rejects([value], "invalid entry");
}
console.log("L08 passed");
'''

[[exercise.files]]
path = "model.ts"
starter = '''
export interface Entry { id: number; label: string; cents: number; tag: string }
export type Result<T, E> = {ok: true; value: T} | {ok: false; error: E};
export class ValidationError extends Error {
  constructor(message: string) { super(message); this.name = "ValidationError"; }
}
'''
solution = '''
export interface Entry { id: number; label: string; cents: number; tag: string }
export type Result<T, E> = {ok: true; value: T} | {ok: false; error: E};
export class ValidationError extends Error {
  constructor(message: string) { super(message); this.name = "ValidationError"; }
}
'''

[[exercise.files]]
path = "boundary.ts"
starter = '''
import { ValidationError } from "./model.js";
import type { Entry } from "./model.js";
export function parseEntries(raw: unknown): Entry[] {
  throw new ValidationError("TODO: validate entries");
}
'''
solution = '''
import { ValidationError } from "./model.js";
import type { Entry } from "./model.js";
export function parseEntries(raw: unknown): Entry[] {
  if (!Array.isArray(raw)) throw new ValidationError("entries must be an array");
  const entries: Entry[] = [];
  const ids = new Set<number>();
  for (const item of raw) {
    const value: unknown = item;
    if (typeof value !== "object" || value === null
      || !("id" in value) || typeof value.id !== "number" || !Number.isSafeInteger(value.id) || value.id < 1
      || !("label" in value) || typeof value.label !== "string" || value.label.trim().length === 0
      || !("cents" in value) || typeof value.cents !== "number" || !Number.isSafeInteger(value.cents) || value.cents < 0
      || !("tag" in value) || typeof value.tag !== "string" || value.tag.trim().length === 0) {
      throw new ValidationError("invalid entry");
    }
    if (ids.has(value.id)) throw new ValidationError("duplicate id");
    ids.add(value.id);
    entries.push({id: value.id, label: value.label, cents: value.cents, tag: value.tag});
  }
  return entries;
}
'''
+++
# Ledger Boundary: From JSON to Trusted Entries

This is the first half of the capstone: a local expense ledger. Its domain interface is small enough to read completely, but the data boundary has several independent promises to establish. Arbitrary JSON does not become an Entry[] because a variable has that annotation. Receive it as unknown and validate before exposing trusted objects to the rest of the program.

Require positive safe-integer IDs and nonnegative safe-integer cents. `Number.isSafeInteger` rejects fractions, infinities, and integers beyond reliable JavaScript representation. A Set records seen IDs so duplicates fail even if both objects are otherwise valid. An empty ledger is allowed.

Build fresh objects containing exactly the declared fields. Structural typing does not remove extra runtime properties automatically; explicit reconstruction does. It also prevents later mutation of an input object from changing the returned ledger. Use `trim()` for blank detection while preserving accepted strings.

Array.isArray narrows an unknown value to an array whose elements are permissively typed by the standard library. Assign each element to a local `unknown` before checking its fields so that accidental property access cannot bypass your guard. Do not use type assertions or explicit any.

Separate errors by contract: a non-array throws `entries must be an array`, malformed fields throw `invalid entry`, and reused IDs throw `duplicate id`. The custom class lets callers distinguish expected validation failures from unrelated programming errors. The next lesson reuses this boundary in a multi-module asynchronous report.

A small example to compare with your exercise:

```ts
const input: unknown = JSON.parse('{"count": 2}');
if (typeof input === "object" && input !== null && "count" in input) {
  // The property exists; its value still needs validation.
  console.log(typeof input.count);
}
```

Run `tsc --project tsconfig.json --pretty false && node dist/check.js` from the exported exercise directory in your terminal. Compilation must succeed before Node runs. Read the first compiler diagnostic, fix its cause, and rerun. Keep `check.ts`, `assert.ts`, and `tsconfig.json` intact; edit the exercise implementation files. The checks compare behavior and compile type examples; printing the success message yourself does not implement the exercise.

After passing, hover the exported functions in the editor and explain their input and output types without executing them.
