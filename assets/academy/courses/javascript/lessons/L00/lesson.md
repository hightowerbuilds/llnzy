+++
title = "Run JavaScript with Node"
concepts = ["Node runtime", "ES modules", "const and let", "feature probes", "assertions"]

[[exercise]]
prompt = "Implement greeting(name) with a template literal and cloneRecord(record) with structuredClone. Return values; the harness handles printing."
[exercise.check]
command = ["node", "check.js"]
expected = "L00 passed\n"
mode = "exact"
timeout_secs = 15

[[exercise.files]]
path = "package.json"
starter = '''
{"type":"module","private":true}
'''
solution = '''
{"type":"module","private":true}
'''

[[exercise.files]]
path = "runtime.js"
starter = '''
export function greeting(name) { return "TODO"; }
export function cloneRecord(record) { return record; }
'''
solution = '''
export function greeting(name) {
  return `Hello, ${name}!`;
}
export function cloneRecord(record) {
  return structuredClone(record);
}
'''

[[exercise.files]]
path = "check.js"
starter = '''
import assert from 'node:assert/strict';
import { greeting, cloneRecord } from './runtime.js';
assert.equal(typeof structuredClone, 'function', 'structuredClone must be available');
assert.equal(greeting('Ada'), 'Hello, Ada!');
assert.equal(greeting('Lin'), 'Hello, Lin!');
const original = { profile: { name: 'Ada' }, scores: [2, 5] };
const copy = cloneRecord(original);
assert.deepEqual(copy, original);
assert.notEqual(copy, original);
copy.profile.name = 'Grace';
copy.scores.push(8);
assert.equal(original.profile.name, 'Ada');
assert.deepEqual(original.scores, [2, 5]);
console.log('L00 passed');
'''
solution = '''
import assert from 'node:assert/strict';
import { greeting, cloneRecord } from './runtime.js';
assert.equal(typeof structuredClone, 'function', 'structuredClone must be available');
assert.equal(greeting('Ada'), 'Hello, Ada!');
assert.equal(greeting('Lin'), 'Hello, Lin!');
const original = { profile: { name: 'Ada' }, scores: [2, 5] };
const copy = cloneRecord(original);
assert.deepEqual(copy, original);
assert.notEqual(copy, original);
copy.profile.name = 'Grace';
copy.scores.push(8);
assert.equal(original.profile.name, 'Ada');
assert.deepEqual(original.scores, [2, 5]);
console.log('L00 passed');
'''
+++
# Run JavaScript with Node

JavaScript is the language; Node.js is the runtime that executes it here. This course assumes you can create a file and navigate a terminal, but does not assume another programming language. Use Node.js 22 or newer. Exercises use only built-in features and modules, so there is nothing to install with npm. Browser APIs such as `document` are outside this course.

A program evaluates expressions and executes statements. `const` names a value without allowing reassignment; `let` names a binding you plan to update. Strings hold text. A function accepts arguments and returns a result to its caller:

```js
const subject = 'JavaScript';
function welcome(topic) {
  return `Learning ${topic}`;
}
console.log(welcome(subject)); // Learning JavaScript
```

Save that example as `hello.js` and run `node hello.js`. Backticks make a template literal; `${topic}` inserts a value. `console.log` displays a value, while `return` gives it to another function. Confusing those operations is a common cause of an unexpected `undefined` result.

Each exercise includes `package.json` with `"type":"module"`, enabling the `import` and `export` syntax already present in the starter. You will design your own module boundaries in L07. For now, preserve those keywords and implement the function bodies.

An object groups named fields. Assigning an object to another variable shares the same object; it does not copy it. `structuredClone(record)` creates an independent copy of the plain records used here, including nested fields. It is not a universal copier: functions cannot be cloned this way. The first harness checks that the runtime supports this feature and that editing the copy leaves the original intact.

After the exercise, explain why `const copy = original` would fail the independence check. Use the editor's hover on `greeting` to inspect its inferred return type.

## Practice

Implement greeting(name) with a template literal and cloneRecord(record) with structuredClone. Return values; the harness handles printing.

Using Python 3.11 or newer, export a fresh workspace from the repository root with `python3 scripts/check_academy_courses.py --export javascript L00 /tmp/llnzy-js-l00`. The destination must not already exist; choose a new path for a retake. Open those files in LLNZY, then run `node check.js` from the exported directory. The starter intentionally fails. Leave `check.js` unchanged: it calls your functions with several inputs and reports the first failed assertion. Read that failure, inspect the relevant input, and rerun after one focused edit.
