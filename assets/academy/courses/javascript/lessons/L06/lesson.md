+++
title = "Errors You Can Handle"
concepts = ["throw and catch", "custom Error classes", "validation boundaries", "rethrow unexpected errors"]
[[exercise]]
prompt = "Implement parsePlayer and importPlayers in players.js. parsePlayer accepts a JSON string representing a non-array object with a nonblank name and nonnegative safe-integer score. Return only name and score, retaining the name as provided. Throw ValidationError(\"invalid JSON\") for malformed JSON and ValidationError(\"invalid player\") for invalid shape. importPlayers processes every string, collecting valid players and validation messages in order; rethrow unexpected errors."
[exercise.check]
command = ["node", "check.js"]
expected = "L06 passed\n"
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
path = "players.js"
starter = '''
export class ValidationError extends Error {
  constructor(message) { super(message); this.name = "ValidationError"; }
}
export function parsePlayer(text) { throw new Error("TODO: parse and validate"); }
export function importPlayers(texts) { return {players: [], errors: []}; }
'''
solution = '''
export class ValidationError extends Error {
  constructor(message) { super(message); this.name = "ValidationError"; }
}
export function parsePlayer(text) {
  let value;
  try { value = JSON.parse(text); }
  catch (error) {
    if (error instanceof SyntaxError) throw new ValidationError("invalid JSON");
    throw error;
  }
  if (typeof value !== "object" || value === null || Array.isArray(value)
    || typeof value.name !== "string" || value.name.trim().length === 0
    || !Number.isSafeInteger(value.score) || value.score < 0) {
    throw new ValidationError("invalid player");
  }
  return {name: value.name, score: value.score};
}
export function importPlayers(texts) {
  const players = [];
  const errors = [];
  for (const text of texts) {
    try { players.push(parsePlayer(text)); }
    catch (error) {
      if (error instanceof ValidationError) errors.push(error.message);
      else throw error;
    }
  }
  return {players, errors};
}
'''

[[exercise.files]]
path = "check.js"
starter = '''
import assert from "node:assert/strict";
import { ValidationError, parsePlayer, importPlayers } from "./players.js";
assert.deepEqual(parsePlayer('{"name":" Ada ","score":0,"extra":true}'), {name:" Ada ",score:0});
for (const value of [null, [], {}, {name:" ",score:1}, {name:"A",score:-1}, {name:"A",score:1.5}, {name:"A",score:"1"}, {name:"A",score:Number.MAX_SAFE_INTEGER+1}]) {
  assert.throws(() => parsePlayer(JSON.stringify(value)), error => error instanceof ValidationError && error.message === "invalid player");
}
assert.throws(() => parsePlayer("{"), error => error instanceof ValidationError && error.message === "invalid JSON");
assert.deepEqual(importPlayers(['{"name":"A","score":2}', '{', '{"name":"B","score":3}', 'null']), {players:[{name:"A",score:2},{name:"B",score:3}],errors:["invalid JSON","invalid player"]});
assert.deepEqual(importPlayers([]), {players:[],errors:[]});
const unexpected = new Error("unexpected");
assert.throws(() => importPlayers([{toString() {throw unexpected;}}]), error => error === unexpected);
console.log("L06 passed");
'''
solution = '''
import assert from "node:assert/strict";
import { ValidationError, parsePlayer, importPlayers } from "./players.js";
assert.deepEqual(parsePlayer('{"name":" Ada ","score":0,"extra":true}'), {name:" Ada ",score:0});
for (const value of [null, [], {}, {name:" ",score:1}, {name:"A",score:-1}, {name:"A",score:1.5}, {name:"A",score:"1"}, {name:"A",score:Number.MAX_SAFE_INTEGER+1}]) {
  assert.throws(() => parsePlayer(JSON.stringify(value)), error => error instanceof ValidationError && error.message === "invalid player");
}
assert.throws(() => parsePlayer("{"), error => error instanceof ValidationError && error.message === "invalid JSON");
assert.deepEqual(importPlayers(['{"name":"A","score":2}', '{', '{"name":"B","score":3}', 'null']), {players:[{name:"A",score:2},{name:"B",score:3}],errors:["invalid JSON","invalid player"]});
assert.deepEqual(importPlayers([]), {players:[],errors:[]});
const unexpected = new Error("unexpected");
assert.throws(() => importPlayers([{toString() {throw unexpected;}}]), error => error === unexpected);
console.log("L06 passed");
'''
+++
# Errors You Can Handle

An exception interrupts normal control flow and travels up the call stack until a catch block handles it. Use exceptions when a function cannot honor its promise. A parser promising a valid player must not quietly return a partial object after validation fails.

Extend Error to create an expected failure category. Setting `this.name` gives the error a useful label; inheriting from Error preserves its message and stack. The message should describe the invalid input.

Catch errors at the layer that can decide what to do next: parsePlayer translates malformed JSON into a domain error, while importPlayers can skip invalid rows and continue importing.

Keep the try around JSON.parse narrow. Catching the entire parser and relabeling every exception as invalid input could hide your own programming mistakes.

The batch importer should collect ValidationError messages and rethrow anything else. A stack trace is a route to the source of a failure, so inspect the first frame in your code rather than copying the message into a workaround.

A different domain illustrates the pattern:

```js
class EmptyTitleError extends Error {}
function requireTitle(text) {
  if (text.trim() === "") throw new EmptyTitleError("title is empty");
  return text;
}
try { requireTitle(" "); }
catch (error) {
  if (error instanceof EmptyTitleError) console.log(error.message);
  else throw error;
}
```

Validate the object before reading its fields: null and arrays are not player records. Check Number.isSafeInteger as well as nonnegativity; zero is valid.

Do not mutate the caller's raw strings or discard accepted whitespace from names. After passing, call parsePlayer directly with malformed text in a scratch script and compare its thrown error with the batch import's collected message.

Choose **Open practice** in the exercise card to create or reopen this lesson’s files. Edit the implementation, save your changes, then choose **Check work**. Your practice folder is reused when you return; opening it again keeps your edits. No source checkout or Python is needed.

You can also run this command in the practice folder’s terminal:

```bash
node check.js
```

Edit the implementation files and keep the supplied assertions intact. Read an assertion failure as a concrete example of behavior to repair.

No exercise check downloads packages or accesses the network.

## Hint before a solution

Write down the difference between malformed JSON and a parsed value with invalid fields. Catch only the errors your layer knows how to handle.

Try one focused change and check again. Before comparing with a reference solution, explain the failing case in your own words. A passing check covers the supplied examples; also try a new input and explain why your implementation handles it.
