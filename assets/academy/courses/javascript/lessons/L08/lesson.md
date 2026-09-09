+++
title = "Promises, Await, and Parallel Work"
concepts = ["async functions", "await", "Promise.all", "rejection handling"]
[[exercise]]
prompt = "Implement sequential(tasks), parallel(tasks), and outcome(task) in tasks.js. The first two accept zero-argument promise-returning functions and resolve to values in input order; sequential starts the next task after the previous resolves, parallel starts every task before waiting. outcome returns {ok:true,value} or {ok:false,error:message}, normalizing non-Error rejections with String."
[exercise.check]
command = ["node", "check.js"]
expected = "L08 passed\n"
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
path = "tasks.js"
starter = '''
export async function sequential(tasks) { return []; }
export async function parallel(tasks) { return []; }
export async function outcome(task) { return {ok:true,value:undefined}; }
'''
solution = '''
export async function sequential(tasks) {
  const values = [];
  for (const task of tasks) values.push(await task());
  return values;
}
export async function parallel(tasks) {
  return await Promise.all(tasks.map(task => task()));
}
export async function outcome(task) {
  try { return {ok:true,value:await task()}; }
  catch (error) { return {ok:false,error:error instanceof Error ? error.message : String(error)}; }
}
'''

[[exercise.files]]
path = "check.js"
starter = '''
import assert from "node:assert/strict";
import { sequential, parallel, outcome } from "./tasks.js";
let release;
const gate = new Promise(resolve => {release = resolve;});
const starts = [];
const sequentialResult = sequential([async () => {starts.push("first"); await gate; return 1;}, async () => {starts.push("second"); return 2;}]);
await Promise.resolve();
assert.deepEqual(starts,["first"],"second task waits for first");
release();
assert.deepEqual(await sequentialResult,[1,2]);
assert.deepEqual(starts,["first","second"]);
let releaseFirst;
const firstGate = new Promise(resolve => {releaseFirst = resolve;});
const parallelStarts = [];
const parallelResult = parallel([async () => {parallelStarts.push("first"); await firstGate; return "slow";}, async () => {parallelStarts.push("second"); return "fast";}]);
await Promise.resolve();
assert.deepEqual(parallelStarts,["first","second"],"both tasks start before first resolves");
releaseFirst();
assert.deepEqual(await parallelResult,["slow","fast"],"input order retained");
assert.deepEqual(await sequential([]),[]);
assert.deepEqual(await parallel([]),[]);
assert.deepEqual(await outcome(async () => 0),{ok:true,value:0});
assert.deepEqual(await outcome(async () => {throw new Error("boom");}),{ok:false,error:"boom"});
assert.deepEqual(await outcome(async () => {throw "offline";}),{ok:false,error:"offline"});
await assert.rejects(parallel([async () => {throw new Error("stop");}]), /stop/);
console.log("L08 passed");
'''
solution = '''
import assert from "node:assert/strict";
import { sequential, parallel, outcome } from "./tasks.js";
let release;
const gate = new Promise(resolve => {release = resolve;});
const starts = [];
const sequentialResult = sequential([async () => {starts.push("first"); await gate; return 1;}, async () => {starts.push("second"); return 2;}]);
await Promise.resolve();
assert.deepEqual(starts,["first"],"second task waits for first");
release();
assert.deepEqual(await sequentialResult,[1,2]);
assert.deepEqual(starts,["first","second"]);
let releaseFirst;
const firstGate = new Promise(resolve => {releaseFirst = resolve;});
const parallelStarts = [];
const parallelResult = parallel([async () => {parallelStarts.push("first"); await firstGate; return "slow";}, async () => {parallelStarts.push("second"); return "fast";}]);
await Promise.resolve();
assert.deepEqual(parallelStarts,["first","second"],"both tasks start before first resolves");
releaseFirst();
assert.deepEqual(await parallelResult,["slow","fast"],"input order retained");
assert.deepEqual(await sequential([]),[]);
assert.deepEqual(await parallel([]),[]);
assert.deepEqual(await outcome(async () => 0),{ok:true,value:0});
assert.deepEqual(await outcome(async () => {throw new Error("boom");}),{ok:false,error:"boom"});
assert.deepEqual(await outcome(async () => {throw "offline";}),{ok:false,error:"offline"});
await assert.rejects(parallel([async () => {throw new Error("stop");}]), /stop/);
console.log("L08 passed");
'''
+++
# Promises, Await, and Parallel Work

A promise represents an eventual result. Calling an async function starts its execution; it returns a promise, and await suspends that function until the awaited promise settles. An async function's return value becomes its promise's fulfillment value. A throw becomes rejection.

Await inside a loop when each operation depends on the previous one or when you intentionally limit concurrency. For independent operations, call each task to obtain its promise, then pass the promises to Promise.all. Promise.all waits for them and preserves input order in its result. It rejects when an input rejects; it does not cancel other work already started.

This example illustrates aggregation without timers or a network:

```js
async function square(value) { return value * value; }
const pending = [square(3), square(2)];
const values = await Promise.all(pending);
console.log(values); // [9, 4]
```

Do not mistake `items.forEach(async item => ...)` for a way to await a batch: forEach discards callback return values. Choose a for-of loop with await, or map to promises and await Promise.all. A floating promise can fail outside your intended error handling.

Wrap the await itself in try/catch when converting failure into a result object. Returning a promise directly from inside a try block does not catch its later rejection unless you await it there. Error messages are meaningful only after narrowing with instanceof Error; JavaScript permits rejecting with a string too.

The checks use manually released promises to prove start order and overlap. No elapsed time threshold determines success. After passing, explain why parallel output order differs from completion order, and why a failure does not automatically undo successful neighboring tasks.

From the repository root, export this lesson with `python3 scripts/check_academy_courses.py --export javascript L08 /tmp/llnzy-javascript-L08` (Python 3.11+, destination must be new). Open the exported directory in the editor and run `node check.js` from that directory with Node.js 22 or newer. Edit the implementation files and keep the supplied assertions intact. Read an assertion failure as a concrete example of behavior to repair. No exercise check downloads packages or accesses the network.
