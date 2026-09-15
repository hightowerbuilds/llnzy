+++
title = "Inference and Function Contracts"
concepts = ["inference", "parameter annotations", "return types", "readonly input"]

[[exercise]]
prompt = "Implement totalCents in totals.ts with typed parameters and a number return. Sum quantity times unitCents without mutating the readonly cart; an empty cart totals zero."
[exercise.check]
command = ["sh", "-c", "tsc --project tsconfig.json --pretty false && node dist/check.js"]
expected = "L01 passed\n"
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
import { totalCents } from "./totals.js";
import { equal } from "./assert.js";
const cart = Object.freeze([Object.freeze({ unitCents: 125, quantity: 3 }), Object.freeze({ unitCents: 50, quantity: 2 })]);
equal(totalCents(cart), 475, "cart total");
equal(totalCents([]), 0, "empty total");
equal(totalCents([{ unitCents: 0, quantity: 7 }]), 0, "free item");
if (false) {
  // @ts-expect-error a price is numeric, not text
  totalCents([{ unitCents: "125", quantity: 1 }]);
}
console.log("L01 passed");
'''
solution = '''
import { totalCents } from "./totals.js";
import { equal } from "./assert.js";
const cart = Object.freeze([Object.freeze({ unitCents: 125, quantity: 3 }), Object.freeze({ unitCents: 50, quantity: 2 })]);
equal(totalCents(cart), 475, "cart total");
equal(totalCents([]), 0, "empty total");
equal(totalCents([{ unitCents: 0, quantity: 7 }]), 0, "free item");
if (false) {
  // @ts-expect-error a price is numeric, not text
  totalCents([{ unitCents: "125", quantity: 1 }]);
}
console.log("L01 passed");
'''

[[exercise.files]]
path = "totals.ts"
starter = '''
export type Line = { unitCents: number; quantity: number };
export function totalCents(lines) {
  return 0;
}
'''
solution = '''
export type Line = { unitCents: number; quantity: number };
export function totalCents(lines: readonly Readonly<Line>[]): number {
  return lines.reduce((sum, line) => sum + line.unitCents * line.quantity, 0);
}
'''
+++
# Inference and Function Contracts

Inference saves repetition when the compiler already has evidence: `const count = 2` needs no number annotation.

A function parameter is a boundary; without an annotation an ordinary exported parameter may become an implicit `any`, which strict mode rejects. Annotate public inputs and outputs, then let intermediate expressions infer.

A type alias gives a domain concept a name. `readonly Line[]` promises that the function will not rearrange or append to the input array through this reference; it does not freeze JavaScript objects at runtime.

`Readonly<Line>` also prevents assignment to a line's properties through that type. The tests freeze their fixture to catch runtime mutation too.

The cart stores money as integer cents. Multiplying integer quantities by integer prices avoids the familiar `0.1 + 0.2` decimal surprise for these small amounts. This lesson assumes valid nonnegative prices; later boundary lessons establish those assumptions from unknown data.

Implement a reducer with an initial accumulator of zero. Hover its accumulator to see why the result is a number. Explain what would happen to an empty array if you omitted the initial value.

A small example to compare with your exercise:

```ts
function double(value: number): number {
  const result = value * 2; // inferred number
  return result;
}
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

Give the function’s boundary an explicit type, then let local calculations infer. Which initial accumulator also handles an empty list?

Try one focused change and check again. Before comparing with a reference solution, explain the failing case in your own words. A passing check covers the supplied examples; also try a new input and explain why your implementation handles it.
