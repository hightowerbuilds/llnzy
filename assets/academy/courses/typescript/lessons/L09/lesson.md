+++
title = "Capstone: A Modular Async Ledger Report"
concepts = ["typed async boundaries", "generic selection", "union dispatch", "map filter reduce", "integration"]

[[exercise]]
prompt = "Complete ledger.ts: select matching entries by exact tag (or all when omitted), summarize sorted tags as \"tag: cents\" lines, and implement runLedger to await an injected unknown-data loader, validate entries, dispatch list or summary, and return Result<string, Error>. Normalize thrown values, retain exhaustive dispatch, and preserve input order for list output \"id: label (cents)\". Empty output is an empty string."
[exercise.check]
command = ["sh", "-c", "tsc --project tsconfig.json --pretty false && node dist/check.js"]
expected = "L09 passed\n"
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
import { select, summarize, runLedger } from "./ledger.js";
import { ValidationError } from "./model.js";
import { equal, assert } from "./assert.js";
const rows = Object.freeze([
  Object.freeze({id: 1, label: "train", cents: 450, tag: "travel", note: "keep"}),
  Object.freeze({id: 2, label: "tea", cents: 125, tag: "food", note: "warm"}),
  Object.freeze({id: 3, label: "rice", cents: 275, tag: "food", note: "dinner"}),
]);
const selected = select(rows, "food");
const note: string | undefined = selected[0]?.note;
equal(note, "warm", "generic subtype retained");
equal(selected.map(entry => entry.id), [2, 3], "filter");
equal(select(rows, ""), [], "explicit empty tag");
assert(select(rows) !== rows, "new array");
equal(summarize(rows), "food: 400\ntravel: 450", "sorted totals");
equal(summarize([]), "", "empty totals");
const load = async (): Promise<unknown> => rows;
const reports = await Promise.all([runLedger(load, {kind: "list", tag: "food"}), runLedger(load, {kind: "summary"})]);
equal(reports, [{ok: true, value: "2: tea (125)\n3: rice (275)"}, {ok: true, value: "food: 400\ntravel: 450"}], "integrated reports");
equal(await runLedger(load, {kind: "summary", tag: "missing"}), {ok: true, value: ""}, "empty filter");
const malformed = await runLedger(async () => [{id: 0}], {kind: "list"});
assert(!malformed.ok && malformed.error instanceof ValidationError, "boundary error preserved");
const failed = await runLedger(async () => {throw "unavailable";}, {kind: "list"});
assert(!failed.ok, "loader failure");
equal(failed.error.message, "unavailable", "normalized loader failure");
const overflow = await runLedger(async () => [{id: 1, label: "a", tag: "x", cents: Number.MAX_SAFE_INTEGER}, {id: 2, label: "b", tag: "x", cents: 1}], {kind: "summary"});
assert(!overflow.ok, "overflow rejected");
equal(overflow.error.message, "total exceeds safe integer range", "overflow message");
if (false) {
  // @ts-expect-error unsupported command
  runLedger(load, {kind: "erase"});
}
console.log("L09 passed");
'''
solution = '''
import { select, summarize, runLedger } from "./ledger.js";
import { ValidationError } from "./model.js";
import { equal, assert } from "./assert.js";
const rows = Object.freeze([
  Object.freeze({id: 1, label: "train", cents: 450, tag: "travel", note: "keep"}),
  Object.freeze({id: 2, label: "tea", cents: 125, tag: "food", note: "warm"}),
  Object.freeze({id: 3, label: "rice", cents: 275, tag: "food", note: "dinner"}),
]);
const selected = select(rows, "food");
const note: string | undefined = selected[0]?.note;
equal(note, "warm", "generic subtype retained");
equal(selected.map(entry => entry.id), [2, 3], "filter");
equal(select(rows, ""), [], "explicit empty tag");
assert(select(rows) !== rows, "new array");
equal(summarize(rows), "food: 400\ntravel: 450", "sorted totals");
equal(summarize([]), "", "empty totals");
const load = async (): Promise<unknown> => rows;
const reports = await Promise.all([runLedger(load, {kind: "list", tag: "food"}), runLedger(load, {kind: "summary"})]);
equal(reports, [{ok: true, value: "2: tea (125)\n3: rice (275)"}, {ok: true, value: "food: 400\ntravel: 450"}], "integrated reports");
equal(await runLedger(load, {kind: "summary", tag: "missing"}), {ok: true, value: ""}, "empty filter");
const malformed = await runLedger(async () => [{id: 0}], {kind: "list"});
assert(!malformed.ok && malformed.error instanceof ValidationError, "boundary error preserved");
const failed = await runLedger(async () => {throw "unavailable";}, {kind: "list"});
assert(!failed.ok, "loader failure");
equal(failed.error.message, "unavailable", "normalized loader failure");
const overflow = await runLedger(async () => [{id: 1, label: "a", tag: "x", cents: Number.MAX_SAFE_INTEGER}, {id: 2, label: "b", tag: "x", cents: 1}], {kind: "summary"});
assert(!overflow.ok, "overflow rejected");
equal(overflow.error.message, "total exceeds safe integer range", "overflow message");
if (false) {
  // @ts-expect-error unsupported command
  runLedger(load, {kind: "erase"});
}
console.log("L09 passed");
'''

[[exercise.files]]
path = "model.ts"
starter = '''
export interface Entry { id: number; label: string; cents: number; tag: string }
export type Result<T, E> = {ok: true; value: T} | {ok: false; error: E};
export class ValidationError extends Error {
  constructor(message: string) { super(message); this.name = "ValidationError"; }
}
export type Command = {kind: "list"; tag?: string} | {kind: "summary"; tag?: string};
'''
solution = '''
export interface Entry { id: number; label: string; cents: number; tag: string }
export type Result<T, E> = {ok: true; value: T} | {ok: false; error: E};
export class ValidationError extends Error {
  constructor(message: string) { super(message); this.name = "ValidationError"; }
}
export type Command = {kind: "list"; tag?: string} | {kind: "summary"; tag?: string};
'''

[[exercise.files]]
path = "boundary.ts"
starter = '''
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

[[exercise.files]]
path = "ledger.ts"
starter = '''
import type { Command, Entry, Result } from "./model.js";
import { parseEntries } from "./boundary.js";
export function select<T extends Entry>(entries: readonly T[], tag?: string): T[] { return []; }
export function summarize(entries: readonly Entry[]): string { return ""; }
export async function runLedger(load: () => Promise<unknown>, command: Command): Promise<Result<string, Error>> {
  return {ok: false, error: new Error("TODO: wire ledger")};
}
'''
solution = '''
import type { Command, Entry, Result } from "./model.js";
import { parseEntries } from "./boundary.js";
export function select<T extends Entry>(entries: readonly T[], tag?: string): T[] {
  return entries.filter(entry => tag === undefined || entry.tag === tag);
}
export function summarize(entries: readonly Entry[]): string {
  const totals = entries.reduce((groups, entry) => {
    const total = (groups.get(entry.tag) ?? 0) + entry.cents;
    if (!Number.isSafeInteger(total)) throw new Error("total exceeds safe integer range");
    groups.set(entry.tag, total);
    return groups;
  }, new Map<string, number>());
  return [...totals.keys()].sort().map(tag => `${tag}: ${totals.get(tag)}`).join("\n");
}
export async function runLedger(load: () => Promise<unknown>, command: Command): Promise<Result<string, Error>> {
  try {
    const entries = select(parseEntries(await load()), command.tag);
    switch (command.kind) {
      case "list": return {ok: true, value: entries.map(entry => `${entry.id}: ${entry.label} (${entry.cents})`).join("\n")};
      case "summary": return {ok: true, value: summarize(entries)};
      default: {
        const unreachable: never = command;
        throw new Error(`Unhandled command: ${unreachable}`);
      }
    }
  } catch (cause: unknown) {
    return {ok: false, error: cause instanceof Error ? cause : new Error(String(cause))};
  }
}
'''
+++
# Capstone: A Modular Async Ledger Report

The final project connects the preceding ideas across a small codebase. Read model.ts first, then boundary.ts, then ledger.ts, then the integration checks.

Use go-to-definition to trace how unknown data becomes trusted entries. Before editing, state what each exported function accepts and returns, where validation happens, and which layer converts exceptions into result data.

The loader is injected as `() => Promise<unknown>`. This makes the core independent of a filesystem, HTTP service, or test fixture.

These lessons deliberately use in-memory loaders so every check is offline and deterministic. A real application's adapter could read JSON and return it as unknown; the same boundary would still validate it.

`select<T extends Entry>` preserves additional fields of a more specific entry subtype. It should return a new array, match tags with strict equality, and leave the original objects alone.

The optional tag means omitted selects everything; an explicit empty string selects nothing in a valid ledger. Do not treat both cases as falsy.

For a summary, accumulate cents by tag with reduce and a Map, sort tags by ordinary string comparison, then map to output lines.

Reject a total that exceeds the safe-integer range with `Error("total exceeds safe integer range")`. Valid individual amounts do not guarantee their sum is safe. For list output, retain the original order.

In runLedger, await the loader and pass its value through parseEntries before dispatching the tagged command. An exhaustive switch catches future command additions.

Convert any thrown value into the error branch, narrowing with instanceof Error first. On success return the formatted string. No function should print a success marker; the supplied integration harness owns output.

Run the two reports together with Promise.all as the test does. Then inspect failure paths: invalid data, a rejected loader, and overflow. These are part of the program's behavior.

After completion, explain why Entry is an interface, Command is a union alias, select is generic, and the loader returns unknown. As an extension, add a new command and let compiler diagnostics lead you to its dispatcher.

A small example to compare with your exercise:

```ts
const loader = async (): Promise<unknown> => [];
const result = await runLedger(loader, {kind: "summary"});
if (result.ok) console.log(result.value);
else console.log(result.error.message);
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

Follow one successful load and one rejected load through the pipeline. Distinguish an omitted tag from an empty string, and check totals for overflow.

Try one focused change and check again. Before comparing with a reference solution, explain the failing case in your own words. A passing check covers the supplied examples; also try a new input and explain why your implementation handles it.
