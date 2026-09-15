+++
title = "Immutable Record Updates"
concepts = ["objects and arrays", "destructuring", "spread", "reference identity", "shallow copies"]

[[exercise]]
prompt = "Implement renamePlayer(player,name) returning a new player with a new nested profile containing name, preserving other fields. Implement appendScore(player,score) returning a new player and scores array. Neither function may mutate its input; preserve unrelated fields."
[exercise.check]
command = ["node", "check.js"]
expected = "L04 passed\n"
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
path = "records.js"
starter = '''
export function renamePlayer(player, name) {
  player.profile.name = name;
  return player;
}
export function appendScore(player, score) {
  player.scores.push(score);
  return player;
}
'''
solution = '''
export function renamePlayer(player, name) {
  const { profile } = player;
  return { ...player, profile: { ...profile, name } };
}
export function appendScore(player, score) {
  const { scores } = player;
  return { ...player, scores: [...scores, score] };
}
'''

[[exercise.files]]
path = "check.js"
starter = '''
import assert from 'node:assert/strict';
import { renamePlayer, appendScore } from './records.js';
const player = Object.freeze({
  id: 7,
  profile: Object.freeze({ name: 'Ada', team: 'blue' }),
  scores: Object.freeze([3, 8]),
  active: true,
});
const renamed = renamePlayer(player, 'Grace');
assert.deepEqual(renamed, { id: 7, profile: { name: 'Grace', team: 'blue' }, scores: [3, 8], active: true });
assert.notEqual(renamed, player);
assert.notEqual(renamed.profile, player.profile);
assert.equal(player.profile.name, 'Ada');
const scored = appendScore(player, 5);
assert.deepEqual(scored.scores, [3, 8, 5]);
assert.notEqual(scored, player);
assert.notEqual(scored.scores, player.scores);
assert.deepEqual(player.scores, [3, 8]);
assert.equal(scored.active, true);
const other = { id: 9, profile: { name: 'Lin', team: 'gold', rank: 2 }, scores: [] };
assert.equal(renamePlayer(other, 'Sam').profile.rank, 2);
assert.deepEqual(appendScore(other, 0).scores, [0]);
console.log('L04 passed');
'''
solution = '''
import assert from 'node:assert/strict';
import { renamePlayer, appendScore } from './records.js';
const player = Object.freeze({
  id: 7,
  profile: Object.freeze({ name: 'Ada', team: 'blue' }),
  scores: Object.freeze([3, 8]),
  active: true,
});
const renamed = renamePlayer(player, 'Grace');
assert.deepEqual(renamed, { id: 7, profile: { name: 'Grace', team: 'blue' }, scores: [3, 8], active: true });
assert.notEqual(renamed, player);
assert.notEqual(renamed.profile, player.profile);
assert.equal(player.profile.name, 'Ada');
const scored = appendScore(player, 5);
assert.deepEqual(scored.scores, [3, 8, 5]);
assert.notEqual(scored, player);
assert.notEqual(scored.scores, player.scores);
assert.deepEqual(player.scores, [3, 8]);
assert.equal(scored.active, true);
const other = { id: 9, profile: { name: 'Lin', team: 'gold', rank: 2 }, scores: [] };
assert.equal(renamePlayer(other, 'Sam').profile.rank, 2);
assert.deepEqual(appendScore(other, 0).scores, [0]);
console.log('L04 passed');
'''
+++
# Immutable Record Updates

An object stores named properties; an array stores an ordered sequence. The properties can themselves refer to objects or arrays.

When two variables refer to the same object, a mutation through either variable is visible through the other. `const` prevents reassignment of the variable, not modification of the referenced object.

Spread builds a new container from existing fields. Later fields override earlier ones:

```js
const item = { label: 'tea', cents: 250, tag: 'food' };
const updated = { ...item, cents: 300 };
console.log(item.cents, updated.cents); // 250 300
```

Destructuring extracts properties into bindings: `const { label, cents } = item`. Rest collects the remaining fields: `const { label, ...details } = item`. These operations help describe transformations without manually naming every unchanged property.

Object and array spread are shallow. Copying the outer object leaves nested references shared:

```js
const before = { options: { color: 'blue', size: 'M' } };
const after = { ...before, options: { ...before.options, color: 'red' } };
console.log(before.options.color); // blue
```

The second spread copies the nested object on the path you are changing. Unchanged branches can remain shared. For arrays, `[...scores, nextScore]` appends into a new array, whereas `scores.push(nextScore)` mutates the existing array and returns its new length.

The starter is deliberately plausible but wrong: it changes the caller's data. The harness freezes the provided object and nested containers, making such writes fail immediately instead of silently corrupting a later assertion. `Object.freeze` itself is shallow; the harness explicitly freezes each relevant level.

First decide which containers need copying, then preserve every field not named in the requested change.

After passing, explain why `{ ...player, profile: { name } }` loses information and why `{ ...player }` alone is insufficient for renaming a nested profile. Use hover to inspect the shape of `renamed` in a separate example.

## Practice

Implement renamePlayer(player,name) returning a new player with a new nested profile containing name, preserving other fields. Implement appendScore(player,score) returning a new player and scores array. Neither function may mutate its input; preserve unrelated fields.

Choose **Open practice** in the exercise card to create or reopen this lesson’s files. Edit the implementation, save your changes, then choose **Check work**. Your practice folder is reused when you return; opening it again keeps your edits. No source checkout or Python is needed.

You can also run this command in the practice folder’s terminal:

```bash
node check.js
```

The starter intentionally fails. Leave `check.js` unchanged: it calls your functions with several inputs and reports the first failed assertion.

Read that failure, inspect the relevant input, and rerun after one focused edit.

## Hint before a solution

A shallow copy still shares nested objects and arrays. List which containers must be new and which unrelated fields must survive.

Try one focused change and check again. Before comparing with a reference solution, explain the failing case in your own words. A passing check covers the supplied examples; also try a new input and explain why your implementation handles it.
