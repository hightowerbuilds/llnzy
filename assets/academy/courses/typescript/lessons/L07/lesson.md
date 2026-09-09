+++
title = "Refactor Across Module Boundaries"
concepts = ["ES modules", "import type", "behavior preservation", "compiler guided refactor"]

[[exercise]]
prompt = "Complete report.ts and summary.ts to produce typed line reports and a summary. Split on LF or CRLF, trim each line, omit blanks, count whitespace-separated words, and preserve line order. Update consumers to use wordCount."
[exercise.check]
command = ["sh", "-c", "tsc --project tsconfig.json --pretty false && node dist/check.js"]
expected = "L07 passed\n"
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
import { report } from "./report.js";
import { summary } from "./summary.js";
import { equal } from "./assert.js";
const lines = report("  hello world \r\n\r\n TypeScript   keeps contracts\n");
equal(lines, [{text: "hello world", wordCount: 2}, {text: "TypeScript   keeps contracts", wordCount: 3}], "parse lines");
equal(summary(lines), "2 lines / 5 words", "summary");
equal(summary(report(" \n\t")), "0 lines / 0 words", "empty summary");
equal(report("one\ttwo"), [{text: "one\ttwo", wordCount: 2}], "tab separator");
console.log("L07 passed");
'''
solution = '''
import { report } from "./report.js";
import { summary } from "./summary.js";
import { equal } from "./assert.js";
const lines = report("  hello world \r\n\r\n TypeScript   keeps contracts\n");
equal(lines, [{text: "hello world", wordCount: 2}, {text: "TypeScript   keeps contracts", wordCount: 3}], "parse lines");
equal(summary(lines), "2 lines / 5 words", "summary");
equal(summary(report(" \n\t")), "0 lines / 0 words", "empty summary");
equal(report("one\ttwo"), [{text: "one\ttwo", wordCount: 2}], "tab separator");
console.log("L07 passed");
'''

[[exercise.files]]
path = "model.ts"
starter = '''
export interface LineReport { text: string; wordCount: number }
'''
solution = '''
export interface LineReport { text: string; wordCount: number }
'''

[[exercise.files]]
path = "report.ts"
starter = '''
import type { LineReport } from "./model.js";
export function report(source: string): LineReport[] { return []; }
'''
solution = '''
import type { LineReport } from "./model.js";
export function report(source: string): LineReport[] {
  return source.split(/\r?\n/).map(line => line.trim()).filter(line => line.length > 0)
    .map(text => ({text, wordCount: text.split(/\s+/).length}));
}
'''

[[exercise.files]]
path = "summary.ts"
starter = '''
import type { LineReport } from "./model.js";
export function summary(lines: readonly LineReport[]): string {
  return `${lines.length} lines / ${lines.reduce((sum, line) => sum + line.count, 0)} words`;
}
'''
solution = '''
import type { LineReport } from "./model.js";
export function summary(lines: readonly LineReport[]): string {
  return `${lines.length} lines / ${lines.reduce((sum, line) => sum + line.wordCount, 0)} words`;
}
'''
+++
# Refactor Across Module Boundaries

A safe refactor starts with observable behavior. Here a text report turns each nonblank line into its trimmed text and word count, then computes a total. The interface is shared across files so the compiler can point out consumers still reading an obsolete field.

Open `model.ts` and follow references to `LineReport`. The model calls the count `wordCount`; the starter's summary still expects `count`. Repair the consumer rather than adding a second field that lets the mismatch persist. Keep line parsing in report.ts and aggregation in summary.ts.

Use `import type` for an interface needed only during checking. It vanishes from emitted JavaScript. Relative imports use `.js` because Node executes the emitted modules; TypeScript's NodeNext resolution maps those specifiers back to the `.ts` sources while checking.

A parsing pipeline can map each line to its trimmed form, filter empty lines, then map to reports. The summary can reduce reports from an initial total of zero. Test empty text and Windows line endings, not only a happy path. After passing, rename `text` in the interface temporarily and follow every resulting diagnostic before restoring it.

A small example to compare with your exercise:

```ts
// shape.ts
export interface Point { x: number; y: number }
// distance.ts
import type { Point } from "./shape.js";
export function horizontal(point: Point): number { return point.x; }
```

Run `tsc --project tsconfig.json --pretty false && node dist/check.js` from the exported exercise directory in your terminal. Compilation must succeed before Node runs. Read the first compiler diagnostic, fix its cause, and rerun. Keep `check.ts`, `assert.ts`, and `tsconfig.json` intact; edit the exercise implementation files. The checks compare behavior and compile type examples; printing the success message yourself does not implement the exercise.

After passing, hover the exported functions in the editor and explain their input and output types without executing them.

Reference: [TypeScript module resolution](https://www.typescriptlang.org/docs/handbook/modules/reference.html).
