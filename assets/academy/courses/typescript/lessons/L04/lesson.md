+++
title = "Interfaces, Aliases, and Safe Patches"
concepts = ["interface extends", "type aliases", "Partial and Pick", "immutable updates"]

[[exercise]]
prompt = "Define EntryPatch so only label and cents may be patched, implement patchEntry immutably, and implement describeState for both LoadState variants. A zero-cent patch and an empty patch are valid."
[exercise.check]
command = ["sh", "-c", "tsc --project tsconfig.json --pretty false && node dist/check.js"]
expected = "L04 passed\n"
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
import { patchEntry, describeState } from "./model.js";
import { equal, assert } from "./assert.js";
const original = Object.freeze({id: 4, label: "tea", cents: 250});
const patched = patchEntry(original, {label: "gift", cents: 0});
equal(patched, {id: 4, label: "gift", cents: 0}, "patch values");
equal(original.cents, 250, "original intact");
assert(patched !== original, "new object");
equal(patchEntry(original, {}), original, "empty patch");
const extraFields = {id: 99, label: "gift"};
equal(patchEntry(original, extraFields), {id: 4, label: "gift", cents: 250}, "ignore extra patch fields");
equal(describeState({kind: "empty"}), "empty", "empty state");
equal(describeState({kind: "ready", entry: patched}), "4: gift", "ready state");
if (false) {
  // @ts-expect-error IDs cannot be edited
  patchEntry(original, {id: 9});
  // @ts-expect-error optional does not mean explicitly undefined
  patchEntry(original, {label: undefined});
}
console.log("L04 passed");
'''
solution = '''
import { patchEntry, describeState } from "./model.js";
import { equal, assert } from "./assert.js";
const original = Object.freeze({id: 4, label: "tea", cents: 250});
const patched = patchEntry(original, {label: "gift", cents: 0});
equal(patched, {id: 4, label: "gift", cents: 0}, "patch values");
equal(original.cents, 250, "original intact");
assert(patched !== original, "new object");
equal(patchEntry(original, {}), original, "empty patch");
const extraFields = {id: 99, label: "gift"};
equal(patchEntry(original, extraFields), {id: 4, label: "gift", cents: 250}, "ignore extra patch fields");
equal(describeState({kind: "empty"}), "empty", "empty state");
equal(describeState({kind: "ready", entry: patched}), "4: gift", "ready state");
if (false) {
  // @ts-expect-error IDs cannot be edited
  patchEntry(original, {id: 9});
  // @ts-expect-error optional does not mean explicitly undefined
  patchEntry(original, {label: undefined});
}
console.log("L04 passed");
'''

[[exercise.files]]
path = "model.ts"
starter = '''
export interface Identified { id: number }
export interface Entry extends Identified { label: string; cents: number }
export type EntryPatch = Partial<Entry>;
export type LoadState = {kind: "empty"} | {kind: "ready"; entry: Entry};
export function patchEntry(entry: Readonly<Entry>, patch: EntryPatch): Entry {
  return {...entry};
}
export function describeState(state: LoadState): string { return "empty"; }
'''
solution = '''
export interface Identified { id: number }
export interface Entry extends Identified { label: string; cents: number }
export type EntryPatch = Partial<Pick<Entry, "label" | "cents">>;
export type LoadState = {kind: "empty"} | {kind: "ready"; entry: Entry};
export function patchEntry(entry: Readonly<Entry>, patch: EntryPatch): Entry {
  return {id: entry.id, label: patch.label ?? entry.label, cents: patch.cents ?? entry.cents};
}
export function describeState(state: LoadState): string {
  return state.kind === "ready" ? `${state.entry.id}: ${state.entry.label}` : "empty";
}
'''
+++
# Interfaces, Aliases, and Safe Patches

Use an interface to name an object contract that can be extended: `Entry extends Identified` adds fields to a shared shape. Interfaces can also participate in declaration merging, which is useful in extension APIs but can surprise you if you reuse a name.

A type alias can name an object too, and can additionally describe unions, tuples, and utility-type compositions. Neither choice changes runtime behavior.

Here the domain entity is an interface; the loading alternatives form a type alias. State the reason in one sentence for each. Both describe structurally compatible values: membership depends on shape, not a runtime class declaration.

`Partial<Pick<Entry, "label" | "cents">>` first selects editable fields and then makes them optional. It deliberately excludes the stable ID. With exact optional property checking, omitting `label` differs from explicitly passing `label: undefined`.

Construct a new object by explicitly copying the stable ID and choosing only the allowed patch fields. Structural typing permits a variable with additional properties to satisfy EntryPatch; the type does not remove those properties at runtime. Spreading the entire patch could therefore overwrite the ID.

Use nullish coalescing to preserve provided zero values. Avoid `patch.cents || entry.cents`: a legitimate zero would be discarded. This function trusts its typed arguments; validating arbitrary JSON remains a separate boundary responsibility. Narrow the state by its tag before reading the ready entry.

A small example to compare with your exercise:

```ts
interface Named { name: string }
interface User extends Named { id: number }
type UserNamePatch = Partial<Pick<User, "name">>;
```

Run the check from the exported exercise directory in your terminal:

```bash
tsc --project tsconfig.json --pretty false && node dist/check.js
```

Compilation must succeed before Node runs. Read the first compiler diagnostic, fix its cause, and rerun.

Keep `check.ts`, `assert.ts`, and `tsconfig.json` intact; edit the exercise implementation files.

The checks compare behavior and compile type examples. Printing the success message yourself does not implement the exercise.

After passing, hover the exported functions in the editor and explain their input and output types without executing them.
