+++
title = "Transform Collections with map, filter & reduce"
concepts = ["map", "filter", "reduce", "integer cents", "empty collections", "non-mutating sorting"]

[[exercise]]
prompt = "Implement summarize(entries,tag). Select matching tags (or all entries when tag is undefined); return labels, totalCents, and largestCents containing up to three amounts sorted descending. Use filter, map, and reduce, keep integer cents, and leave inputs unchanged."
[exercise.check]
command = ["node", "check.js"]
expected = "L05 passed\n"
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
path = "pipeline.js"
starter = '''
export function summarize(entries, tag) {
  return { labels: [], totalCents: 0, largestCents: [] };
}
'''
solution = '''
export function summarize(entries, tag) {
  const selected = entries.filter(entry => tag === undefined || entry.tag === tag);
  const amounts = selected.map(entry => entry.cents);
  return {
    labels: selected.map(entry => entry.label),
    totalCents: amounts.reduce((total, cents) => total + cents, 0),
    largestCents: [...amounts].sort((a, b) => b - a).slice(0, 3),
  };
}
'''

[[exercise.files]]
path = "check.js"
starter = '''
import assert from 'node:assert/strict';
import { summarize } from './pipeline.js';
const entries = Object.freeze([
  Object.freeze({ label: 'tea', cents: 250, tag: 'food' }),
  Object.freeze({ label: 'train', cents: 1200, tag: 'travel' }),
  Object.freeze({ label: 'bread', cents: 400, tag: 'food' }),
  Object.freeze({ label: 'fruit', cents: 90, tag: 'food' }),
  Object.freeze({ label: 'water', cents: 0, tag: 'food' }),
]);
assert.deepEqual(summarize(entries, 'food'), { labels: ['tea', 'bread', 'fruit', 'water'], totalCents: 740, largestCents: [400, 250, 90] });
assert.deepEqual(summarize(entries), { labels: ['tea', 'train', 'bread', 'fruit', 'water'], totalCents: 1940, largestCents: [1200, 400, 250] });
assert.deepEqual(summarize(entries, 'missing'), { labels: [], totalCents: 0, largestCents: [] });
assert.deepEqual(summarize([]), { labels: [], totalCents: 0, largestCents: [] });
assert.deepEqual(summarize([{ label: 'free', cents: 0, tag: '' }, { label: 'tea', cents: 250, tag: 'food' }], ''), { labels: ['free'], totalCents: 0, largestCents: [0] });
assert.deepEqual(entries.map(entry => entry.cents), [250, 1200, 400, 90, 0]);
console.log('L05 passed');
'''
solution = '''
import assert from 'node:assert/strict';
import { summarize } from './pipeline.js';
const entries = Object.freeze([
  Object.freeze({ label: 'tea', cents: 250, tag: 'food' }),
  Object.freeze({ label: 'train', cents: 1200, tag: 'travel' }),
  Object.freeze({ label: 'bread', cents: 400, tag: 'food' }),
  Object.freeze({ label: 'fruit', cents: 90, tag: 'food' }),
  Object.freeze({ label: 'water', cents: 0, tag: 'food' }),
]);
assert.deepEqual(summarize(entries, 'food'), { labels: ['tea', 'bread', 'fruit', 'water'], totalCents: 740, largestCents: [400, 250, 90] });
assert.deepEqual(summarize(entries), { labels: ['tea', 'train', 'bread', 'fruit', 'water'], totalCents: 1940, largestCents: [1200, 400, 250] });
assert.deepEqual(summarize(entries, 'missing'), { labels: [], totalCents: 0, largestCents: [] });
assert.deepEqual(summarize([]), { labels: [], totalCents: 0, largestCents: [] });
assert.deepEqual(summarize([{ label: 'free', cents: 0, tag: '' }, { label: 'tea', cents: 250, tag: 'food' }], ''), { labels: ['free'], totalCents: 0, largestCents: [0] });
assert.deepEqual(entries.map(entry => entry.cents), [250, 1200, 400, 90, 0]);
console.log('L05 passed');
'''
+++
# Transform Collections with map, filter & reduce

Collection methods express three different questions. `filter` asks which items to keep, returning a new array of zero or more original items. `map` asks what each item should become, returning one result per input.

`reduce` combines the items into one accumulator. These methods create arrays or values, but their callbacks can still mutate referenced objects; keep callbacks pure here.

```js
const measurements = [2, -1, 4];
const positiveSquares = measurements
  .filter(value => value > 0)
  .map(value => value * value);
const sum = positiveSquares.reduce((total, value) => total + value, 0);
console.log(positiveSquares, sum); // [4, 16] 20
```

Supply an initial accumulator such as zero. Without it, reducing an empty array throws, and the first element takes on a different role from the others.

A numeric total, an array, or an object can be an accumulator; choose the shape that fits the answer rather than putting every operation into one complicated reduction.

The exercise starts a small expense domain used later in the course. Store money as integer cents: `250` means 2.50 currency units.

Binary floating-point numbers do not represent every decimal fraction exactly, so repeated arithmetic on fractional currency can introduce rounding surprises. Integer cents keep these small, validated exercise amounts exact; JavaScript numbers still have a finite safe integer range.

`sort` mutates its receiver. Copy before sorting data you need to preserve, and provide a numeric comparator:

```js
const costs = [9, 100, 20];
const ascending = [...costs].sort((a, b) => a - b);
console.log(ascending); // [9, 20, 100]
console.log(costs);     // [9, 100, 20]
```

Without that comparator, ordinary `sort()` compares string representations, so `100` comes before `9`. Use the opposite subtraction for descending order and `slice(0, 3)` for the largest three. Preserve label order from the original input; ranking amounts should not reorder the selected entries.

The optional `tag` filter deliberately distinguishes `undefined` from an empty string. An empty string is still a requested tag. After passing, explain which stage changes the number of items, which changes their representation, and which produces a single value.

## Practice

Implement summarize(entries,tag). Select matching tags (or all entries when tag is undefined); return labels, totalCents, and largestCents containing up to three amounts sorted descending. Use filter, map, and reduce, keep integer cents, and leave inputs unchanged.

Export a fresh workspace from the repository root. The exporter needs Python 3.11 or newer:

```bash
python3 scripts/check_academy_courses.py --export javascript L05 /tmp/llnzy-js-l05
```

The destination must not already exist, so choose a new path for a retake.

Open those files in LLNZY, then run the check from the exported directory:

```bash
node check.js
```

The starter intentionally fails. Leave `check.js` unchanged: it calls your functions with several inputs and reports the first failed assertion.

Read that failure, inspect the relevant input, and rerun after one focused edit.