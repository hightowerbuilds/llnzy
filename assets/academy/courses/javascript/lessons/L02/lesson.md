+++
title = "Decisions & Repetition"
concepts = ["if and else", "switch", "for of", "continue", "early returns"]

[[exercise]]
prompt = "Implement classify(n): positive, negative, or zero. Implement totalPositive(numbers) ignoring zero and negatives. Implement commandAction(command) mapping add to write, list to read, and everything else to unknown using switch."
[exercise.check]
command = ["node", "check.js"]
expected = "L02 passed\n"
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
path = "control.js"
starter = '''
export function classify(n) { return 'TODO'; }
export function totalPositive(numbers) { return 0; }
export function commandAction(command) { return 'TODO'; }
'''
solution = '''
export function classify(n) {
  if (n > 0) return 'positive';
  if (n < 0) return 'negative';
  return 'zero';
}
export function totalPositive(numbers) {
  let total = 0;
  for (const n of numbers) {
    if (n <= 0) continue;
    total += n;
  }
  return total;
}
export function commandAction(command) {
  switch (command) {
    case 'add': return 'write';
    case 'list': return 'read';
    default: return 'unknown';
  }
}
'''

[[exercise.files]]
path = "check.js"
starter = '''
import assert from 'node:assert/strict';
import { classify, totalPositive, commandAction } from './control.js';
assert.equal(classify(-12), 'negative');
assert.equal(classify(0), 'zero');
assert.equal(classify(0.5), 'positive');
assert.equal(totalPositive([]), 0);
assert.equal(totalPositive([-8, -2, 0]), 0);
assert.equal(totalPositive([4, -5, 0, 7, 2]), 13);
assert.equal(totalPositive([1, 1, 1]), 3);
assert.equal(commandAction('add'), 'write');
assert.equal(commandAction('list'), 'read');
assert.equal(commandAction('delete'), 'unknown');
assert.equal(commandAction('ADD'), 'unknown');
console.log('L02 passed');
'''
solution = '''
import assert from 'node:assert/strict';
import { classify, totalPositive, commandAction } from './control.js';
assert.equal(classify(-12), 'negative');
assert.equal(classify(0), 'zero');
assert.equal(classify(0.5), 'positive');
assert.equal(totalPositive([]), 0);
assert.equal(totalPositive([-8, -2, 0]), 0);
assert.equal(totalPositive([4, -5, 0, 7, 2]), 13);
assert.equal(totalPositive([1, 1, 1]), 3);
assert.equal(commandAction('add'), 'write');
assert.equal(commandAction('list'), 'read');
assert.equal(commandAction('delete'), 'unknown');
assert.equal(commandAction('ADD'), 'unknown');
console.log('L02 passed');
'''
+++
# Decisions & Repetition

A condition chooses the next operation; a loop repeats operations over a collection. Keep the input contract visible: this lesson's numeric functions receive finite numbers, so you can concentrate on decisions. Validation of untrusted input arrives in L06.

An early return finishes the whole function immediately. That can keep a successful path close to the left margin:

```js
function shippingCost(itemCount) {
  if (itemCount === 0) return 0;
  if (itemCount >= 5) return 3;
  return 6;
}
```

Unlike a standalone `if`, an `else` runs only when its paired condition is false. In this example, an explicit `else` is unnecessary because each successful branch already returns. Order broad and narrow conditions carefully: putting `itemCount > 0` first would hide the `>= 5` case if both branches returned.

Use `for...of` to read values from an array. `continue` skips the rest of the current iteration; `break` ends the loop; `return` exits the surrounding function. They have different scopes:

```js
let letters = '';
for (const word of ['sun', '', 'moon']) {
  if (word === '') continue;
  letters += word[0];
}
console.log(letters); // sm
```

A `switch` compares one value against cases using strict equality. End each ordinary case with `break`, or return directly from a function, to avoid accidentally continuing into the next case. A `default` handles values not listed explicitly.

Write the empty-array result down before implementing the total: starting an accumulator at zero makes that case work without a special branch.

After passing, explain why replacing `continue` with `return total` would incorrectly stop at the first negative value. Add your own mixed input to a separate probe file and trace the accumulator by hand.

## Practice

Implement classify(n): positive, negative, or zero. Implement totalPositive(numbers) ignoring zero and negatives. Implement commandAction(command) mapping add to write, list to read, and everything else to unknown using switch.

Export a fresh workspace from the repository root. The exporter needs Python 3.11 or newer:

```bash
python3 scripts/check_academy_courses.py --export javascript L02 /tmp/llnzy-js-l02
```

The destination must not already exist, so choose a new path for a retake.

Open those files in LLNZY, then run the check from the exported directory:

```bash
node check.js
```

The starter intentionally fails. Leave `check.js` unchanged: it calls your functions with several inputs and reports the first failed assertion.

Read that failure, inspect the relevant input, and rerun after one focused edit.