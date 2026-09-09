+++
title = "Functions, Closures & this"
concepts = ["function declarations", "arrow functions", "lexical scope", "closures", "default parameters", "this binding"]

[[exercise]]
prompt = "Implement makeCounter(start = 0), returning a function that increments then returns its private count. Implement makeCallbacks(values), returning one zero-argument callback per value. Implement bindLabel(record), returning record.label bound to record so it works when detached."
[exercise.check]
command = ["node", "check.js"]
expected = "L03 passed\n"
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
path = "functions.js"
starter = '''
export function makeCounter(start = 0) { return () => start; }
export function makeCallbacks(values) { return []; }
export function bindLabel(record) { return record.label; }
'''
solution = '''
export function makeCounter(start = 0) {
  let count = start;
  return () => {
    count += 1;
    return count;
  };
}
export function makeCallbacks(values) {
  const callbacks = [];
  for (const value of values) callbacks.push(() => value);
  return callbacks;
}
export function bindLabel(record) {
  return record.label.bind(record);
}
'''

[[exercise.files]]
path = "check.js"
starter = '''
import assert from 'node:assert/strict';
import { makeCounter, makeCallbacks, bindLabel } from './functions.js';
const first = makeCounter();
const second = makeCounter(10);
assert.deepEqual([first(), second(), first(), second()], [1, 11, 2, 12]);
assert.equal(makeCounter(-2)(), -1);
const callbacks = makeCallbacks(['red', 'green', 'blue']);
assert.deepEqual(callbacks.map(fn => fn()), ['red', 'green', 'blue']);
assert.deepEqual(makeCallbacks([]), []);
const record = { name: 'Ada', label() { return `member: ${this.name}`; } };
const detached = bindLabel(record);
assert.equal(detached(), 'member: Ada');
record.name = 'Grace';
assert.equal(detached(), 'member: Grace');
assert.equal(bindLabel({ name: 'Lin', label: record.label })(), 'member: Lin');
console.log('L03 passed');
'''
solution = '''
import assert from 'node:assert/strict';
import { makeCounter, makeCallbacks, bindLabel } from './functions.js';
const first = makeCounter();
const second = makeCounter(10);
assert.deepEqual([first(), second(), first(), second()], [1, 11, 2, 12]);
assert.equal(makeCounter(-2)(), -1);
const callbacks = makeCallbacks(['red', 'green', 'blue']);
assert.deepEqual(callbacks.map(fn => fn()), ['red', 'green', 'blue']);
assert.deepEqual(makeCallbacks([]), []);
const record = { name: 'Ada', label() { return `member: ${this.name}`; } };
const detached = bindLabel(record);
assert.equal(detached(), 'member: Ada');
record.name = 'Grace';
assert.equal(detached(), 'member: Grace');
assert.equal(bindLabel({ name: 'Lin', label: record.label })(), 'member: Lin');
console.log('L03 passed');
'''
+++
# Functions, Closures & this

Functions are values: you can store them, pass them to another function, or return them. A declaration such as `function double(n) { return n * 2; }` names a function. An arrow expression such as `n => n * 2` creates a function value with a concise return expression. With braces, arrows need an explicit `return`.

A closure keeps access to bindings in the scope where the function was created. Those bindings can outlive the original call:

```js
function makePrefix(prefix) {
  return text => `${prefix}: ${text}`;
}
const warn = makePrefix('warning');
console.log(warn('low battery')); // warning: low battery
```

Every call to `makePrefix` creates a separate parameter binding. A counter uses the same idea with a mutable `let` binding. Each call to its returned function updates the existing count; creating a second counter creates independent state. Do not put that state at module scope, where all counters would share it.

Closures capture bindings, not automatically frozen snapshots of values. `for (const value of values)` creates a binding per iteration, so callbacks keep the intended value. A single mutable variable outside the loop would be shared by every callback. Compare these patterns in a probe before writing `makeCallbacks`.

`this` is a different mechanism. For an ordinary method call `record.label()`, the receiver is `record`. Detaching the method as `const label = record.label` removes that receiver. ES modules run in strict mode, so calling a detached ordinary function does not supply a useful default object. `method.bind(record)` creates a callable with a fixed receiver:

```js
const meter = { value: 4, read() { return this.value; } };
const readLater = meter.read.bind(meter);
console.log(readLater()); // 4
```

An arrow inherits `this` from its surrounding scope and cannot be rebound in this way. Use a regular method when you want receiver-dependent behavior. After passing, explain why changing `record.name` affects the bound function's output while creating another counter does not affect an existing counter.

Further reading: [MDN on closures](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Closures).

## Practice

Implement makeCounter(start = 0), returning a function that increments then returns its private count. Implement makeCallbacks(values), returning one zero-argument callback per value. Implement bindLabel(record), returning record.label bound to record so it works when detached.

Using Python 3.11 or newer, export a fresh workspace from the repository root with `python3 scripts/check_academy_courses.py --export javascript L03 /tmp/llnzy-js-l03`. The destination must not already exist; choose a new path for a retake. Open those files in LLNZY, then run `node check.js` from the exported directory. The starter intentionally fails. Leave `check.js` unchanged: it calls your functions with several inputs and reports the first failed assertion. Read that failure, inspect the relevant input, and rerun after one focused edit.
