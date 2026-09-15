+++
title = "Discriminated Unions and Exhaustiveness"
concepts = ["literal tags", "discriminated unions", "never", "refactoring"]

[[exercise]]
prompt = "Implement apply in commands.ts for add, reset, and scale. Retain the never exhaustiveness assignment in the default arm and avoid casts."
[exercise.check]
command = ["sh", "-c", "tsc --project tsconfig.json --pretty false && node dist/check.js"]
expected = "L03 passed\n"
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
import { apply } from "./commands.js";
import { equal } from "./assert.js";
equal(apply(10, {kind: "add", amount: -3}), 7, "add");
equal(apply(10, {kind: "reset"}), 0, "reset");
equal(apply(10, {kind: "scale", factor: 3}), 30, "scale");
equal(apply(10, {kind: "scale", factor: 0}), 0, "zero factor");
if (false) {
  // @ts-expect-error each variant requires its own fields
  apply(0, {kind: "add"});
}
console.log("L03 passed");
'''
solution = '''
import { apply } from "./commands.js";
import { equal } from "./assert.js";
equal(apply(10, {kind: "add", amount: -3}), 7, "add");
equal(apply(10, {kind: "reset"}), 0, "reset");
equal(apply(10, {kind: "scale", factor: 3}), 30, "scale");
equal(apply(10, {kind: "scale", factor: 0}), 0, "zero factor");
if (false) {
  // @ts-expect-error each variant requires its own fields
  apply(0, {kind: "add"});
}
console.log("L03 passed");
'''

[[exercise.files]]
path = "commands.ts"
starter = '''
export type Command = {kind: "add"; amount: number} | {kind: "reset"} | {kind: "scale"; factor: number};
export function apply(value: number, command: Command): number {
  switch (command.kind) {
    case "add": return value + command.amount;
    case "reset": return 0;
    default: {
      const unreachable: never = command;
      throw new Error(`Unhandled: ${unreachable}`);
    }
  }
}
'''
solution = '''
export type Command = {kind: "add"; amount: number} | {kind: "reset"} | {kind: "scale"; factor: number};
export function apply(value: number, command: Command): number {
  switch (command.kind) {
    case "add": return value + command.amount;
    case "reset": return 0;
    case "scale": return value * command.factor;
    default: {
      const unreachable: never = command;
      throw new Error(`Unhandled: ${unreachable}`);
    }
  }
}
'''
+++
# Discriminated Unions and Exhaustiveness

A discriminated union describes alternatives with different required fields. An `add` command carries `amount`; a `scale` command carries `factor`; `reset` needs neither. A single object with optional fields would allow confusing combinations and force repeated undefined checks.

Switch on the shared literal `kind`. Within each case TypeScript knows the corresponding variant, so `command.amount` is available only in the add case. Return the new state from each branch. In the default arm, assigning the remaining command to `never` asks the compiler to prove that no variant remains.

The starter omits a case deliberately. Read the diagnostic on the never assignment: it identifies the unhandled variant. Handle it instead of deleting the assignment. This is how the compiler helps you update all consumers when a model grows.

After passing, temporarily add `{ kind: "subtract"; amount: number }` to Command. Run the compiler and locate the missed branch; then remove that exploratory variant. Runtime tests show known examples work, while exhaustiveness protects a future edit.

A small example to compare with your exercise:

```ts
type Light = {kind: "off"} | {kind: "on"; brightness: number};
function brightness(light: Light): number {
  return light.kind === "on" ? light.brightness : 0;
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

The never diagnostic names the missing variant. Handle that variant rather than removing the exhaustiveness check.

Try one focused change and check again. Before comparing with a reference solution, explain the failing case in your own words. A passing check covers the supplied examples; also try a new input and explain why your implementation handles it.
