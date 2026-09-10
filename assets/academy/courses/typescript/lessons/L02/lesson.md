+++
title = "Validate Unknown Data"
concepts = ["unknown", "typeof and in guards", "null", "type predicates"]

[[exercise]]
prompt = "Implement isPlayer in player.ts without any or type assertions. Accept a nonblank name and a nonnegative integer score, reject arrays, null, missing fields, and invalid values. Preserve valid original names."
[exercise.check]
command = ["sh", "-c", "tsc --project tsconfig.json --pretty false && node dist/check.js"]
expected = "L02 passed\n"
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
import { isPlayer } from "./player.js";
import { equal, assert } from "./assert.js";
for (const value of [null, [], {}, {name: "A"}, {name: " ", score: 1}, {name: "A", score: -1}, {name: "A", score: 1.5}, {name: "A", score: NaN}, {name: "A", score: "2"}]) {
  equal(isPlayer(value), false, "reject malformed player");
}
const value: unknown = {name: " Ada ", score: 0};
assert(isPlayer(value), "valid player");
equal(value.name, " Ada ", "retain name");
equal(value.score + 1, 1, "narrowed score");
console.log("L02 passed");
'''
solution = '''
import { isPlayer } from "./player.js";
import { equal, assert } from "./assert.js";
for (const value of [null, [], {}, {name: "A"}, {name: " ", score: 1}, {name: "A", score: -1}, {name: "A", score: 1.5}, {name: "A", score: NaN}, {name: "A", score: "2"}]) {
  equal(isPlayer(value), false, "reject malformed player");
}
const value: unknown = {name: " Ada ", score: 0};
assert(isPlayer(value), "valid player");
equal(value.name, " Ada ", "retain name");
equal(value.score + 1, 1, "narrowed score");
console.log("L02 passed");
'''

[[exercise.files]]
path = "player.ts"
starter = '''
export interface Player { name: string; score: number }
export function isPlayer(value: unknown): value is Player {
  return false;
}
'''
solution = '''
export interface Player { name: string; score: number }
export function isPlayer(value: unknown): value is Player {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    && "name" in value && typeof value.name === "string" && value.name.trim().length > 0
    && "score" in value && typeof value.score === "number"
    && Number.isInteger(value.score) && value.score >= 0;
}
'''
+++
# Validate Unknown Data

External data is evidence to inspect, not a type to assume. `unknown` can hold anything, but you cannot read its properties until you narrow it. In contrast, `any` switches off useful checking. A type assertion such as `raw as Player` merely tells the compiler to trust you; it does not inspect the input.

Build your guard from the outside inward. First establish that the value is an object, is not null, and is not an array. Then use `"name" in value` and `"score" in value` before reading those fields.

Check field types before calling string or number operations. `typeof null` is `"object"`, and `typeof NaN` is `"number"`, so neither single test is enough.

A predicate return type `value is Player` allows callers to narrow after a successful test. The compiler trusts that claim, so the implementation must justify every property it promises.

Use `trim()` only to determine whether a name is blank; do not silently modify an accepted name. A zero score is valid, which is why truthiness is the wrong score check.

In `check.ts`, hover `value` inside the successful guard branch and outside it. Explain how control flow changes what operations are allowed.

A small example to compare with your exercise:

```ts
function lengthIfText(value: unknown): number {
  return typeof value === "string" ? value.length : 0;
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

Reference: [TypeScript narrowing](https://www.typescriptlang.org/docs/handbook/2/narrowing.html).
