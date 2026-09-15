+++
title = "Your First Strict Build"
concepts = ["compiler versus runtime", "strict mode", "ES modules", "reading diagnostics"]

[[exercise]]
prompt = "Fix the two incorrect values in setup.ts while keeping their declared types. Save your changes and choose Check work."
[exercise.check]
command = ["sh", "-c", "tsc --project tsconfig.json --pretty false && node dist/check.js"]
expected = "L00 passed\n"
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
import { learners, ready } from "./setup.js";
import { equal } from "./assert.js";
equal(learners + 2, 5, "numeric addition");
equal(ready, true, "boolean readiness");
console.log("L00 passed");
'''
solution = '''
import { learners, ready } from "./setup.js";
import { equal } from "./assert.js";
equal(learners + 2, 5, "numeric addition");
equal(ready, true, "boolean readiness");
console.log("L00 passed");
'''

[[exercise.files]]
path = "setup.ts"
starter = '''
export const learners: number = "3";
export const ready: boolean = "yes";
'''
solution = '''
export const learners: number = 3;
export const ready: boolean = true;
'''
+++
# Your First Strict Build

This course assumes you can already write JavaScript functions, arrays, objects, imports, and promises. Complete the JavaScript course first if those are unfamiliar.

TypeScript adds a static analysis stage: it checks a program before execution, then emits JavaScript. Its annotations do not validate user input at runtime.

Use Node.js 22 or newer and TypeScript 5.9.3. In your own terminal, run `node --version` and `tsc --version`. If the compiler is missing, install it once with `npm install --global typescript@5.9.3` (this step needs network access), then reopen LLNZY so its checks inherit the updated PATH.

If your Node installation does not allow global packages, use a user-managed Node installation instead of changing system permissions. Lesson checks never install packages or access the network.

Each lesson is self-contained: `package.json` selects ES modules and `tsconfig.json` enables strict checking. The DOM library supplies the type of `console`; these exercises do not use browser APIs or require Node type packages.

`noEmitOnError` prevents new output after errors, and the `&&` in the command prevents running stale output when compilation fails.

Open `setup.ts` and hover each declaration. A variable declared `number` cannot contain a string even when that string looks numeric. Correct the value rather than weakening the annotation. The compiler and runtime checks serve different purposes: a well-typed implementation can still return the wrong answer.

Try changing the boolean to the string `"true"` after you pass. Predict which stage will reject it, then restore the working value.

A small example to compare with your exercise:

```ts
const label: string = "ready";
console.log(label.toUpperCase()); // READY
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

Reference: [TypeScript strict compiler options](https://www.typescriptlang.org/tsconfig/strict.html).

## Hint before a solution

Read each declared type before changing its value. Quotation marks create strings; a number or boolean declaration needs a matching value.

Try one focused change and check again. Before comparing with a reference solution, explain the failing case in your own words. A passing check covers the supplied examples; also try a new input and explain why your implementation handles it.
