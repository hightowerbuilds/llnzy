+++
title = "Values, Equality & Defaults"
concepts = ["primitive values", "typeof", "strict equality", "NaN", "nullish coalescing"]

[[exercise]]
prompt = "Implement describe(value), same(a,b), and label(value). describe returns null, array, nan, or the typeof label; same uses strict equality; label defaults only null and undefined to anonymous."
[exercise.check]
command = ["node", "check.js"]
expected = "L01 passed\n"
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
path = "values.js"
starter = '''
export function describe(value) { return "TODO"; }
export function same(a, b) { return false; }
export function label(value) { return value || "anonymous"; }
'''
solution = '''
export function describe(value) {
  if (value === null) return 'null';
  if (Array.isArray(value)) return 'array';
  if (Number.isNaN(value)) return 'nan';
  return typeof value;
}
export function same(a, b) { return a === b; }
export function label(value) { return value ?? 'anonymous'; }
'''

[[exercise.files]]
path = "check.js"
starter = '''
import assert from 'node:assert/strict';
import { describe, same, label } from './values.js';
for (const [value, expected] of [[null,'null'], [[], 'array'], [NaN,'nan'], [3,'number'], ['3','string'], [false,'boolean'], [undefined,'undefined'], [4n,'bigint'], [Symbol('x'),'symbol'], [{},'object'], [() => 1,'function']]) {
  assert.equal(describe(value), expected);
}
assert.equal(same(3, '3'), false);
assert.equal(same(3, 3), true);
assert.equal(same(NaN, NaN), false);
const shared = {};
assert.equal(same(shared, shared), true);
assert.equal(same({}, {}), false);
for (const value of [0, false, '']) assert.equal(label(value), value);
assert.equal(label(null), 'anonymous');
assert.equal(label(undefined), 'anonymous');
assert.equal(label('Ada'), 'Ada');
console.log('L01 passed');
'''
solution = '''
import assert from 'node:assert/strict';
import { describe, same, label } from './values.js';
for (const [value, expected] of [[null,'null'], [[], 'array'], [NaN,'nan'], [3,'number'], ['3','string'], [false,'boolean'], [undefined,'undefined'], [4n,'bigint'], [Symbol('x'),'symbol'], [{},'object'], [() => 1,'function']]) {
  assert.equal(describe(value), expected);
}
assert.equal(same(3, '3'), false);
assert.equal(same(3, 3), true);
assert.equal(same(NaN, NaN), false);
const shared = {};
assert.equal(same(shared, shared), true);
assert.equal(same({}, {}), false);
for (const value of [0, false, '']) assert.equal(label(value), value);
assert.equal(label(null), 'anonymous');
assert.equal(label(undefined), 'anonymous');
assert.equal(label('Ada'), 'Ada');
console.log('L01 passed');
'''
+++
# Values, Equality & Defaults

JavaScript has seven primitive types: string, number, bigint, boolean, undefined, symbol, and null. Objects are a separate category; arrays and functions are special kinds of objects.

`typeof` is useful but has historical exceptions: `typeof null` is `"object"`, arrays also report `"object"`, and callable functions report `"function"`. Use explicit checks when your domain needs a more specific label.

```js
console.log(8 === '8');       // false: different types
console.log(8 == '8');        // true: coercion occurs
console.log(Number('8'));    // 8: deliberate conversion
console.log(Number.isNaN(Number('eight'))); // true
```

Prefer `===` when comparing values. Objects compare by identity, so two separately created `{}` objects are unequal even if their fields match. `NaN` is a number value representing an invalid numeric result and is unequal even to itself; `Number.isNaN` detects it without converting its argument.

A condition treats `false`, `0` (including `-0`), `0n`, `''`, `null`, `undefined`, and `NaN` as falsy; empty arrays and objects are truthy. That makes `value || fallback` inappropriate when zero, false, or empty text are meaningful. `value ?? fallback` substitutes only for null and undefined:

```js
const attempts = 0;
console.log(attempts || 3); // 3, discards a meaningful zero
console.log(attempts ?? 3); // 0
```

Order matters in `describe`: detect null, arrays, and NaN before returning the general `typeof` label. After passing, predict `describe(new Number(3))` and verify it in a small terminal probe. Explain why that wrapper differs from the primitive `3`.

## Practice

Implement describe(value), same(a,b), and label(value). describe returns null, array, nan, or the typeof label; same uses strict equality; label defaults only null and undefined to anonymous.

Choose **Open practice** in the exercise card to create or reopen this lesson’s files. Edit the implementation, save your changes, then choose **Check work**. Your practice folder is reused when you return; opening it again keeps your edits. No source checkout or Python is needed.

You can also run this command in the practice folder’s terminal:

```bash
node check.js
```

The starter intentionally fails. Leave `check.js` unchanged: it calls your functions with several inputs and reports the first failed assertion.

Read that failure, inspect the relevant input, and rerun after one focused edit.

## Hint before a solution

Check special cases before the general typeof result. Which defaulting operator preserves false, zero, and an empty string?

Try one focused change and check again. Before comparing with a reference solution, explain the failing case in your own words. A passing check covers the supplied examples; also try a new input and explain why your implementation handles it.
