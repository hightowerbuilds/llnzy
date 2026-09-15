+++
title = "Build a Program from ES Modules"
concepts = ["named imports and exports", "relative module paths", "single responsibility", "refactoring behavior"]
[[exercise]]
prompt = "Implement stats.js and report.js. activeNames selects active members and sorts their names without changing the input. pointsByTeam totals points for every member and returns sorted [team, total] pairs. formatReport imports both functions and returns two lines: \"active: names\" and \"teams: team=total, ...\". Empty collections use \"none\"."
[exercise.check]
command = ["node", "check.js"]
expected = "L07 passed\n"
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
path = "stats.js"
starter = '''
export function activeNames(members) { return []; }
export function pointsByTeam(members) { return []; }
'''
solution = '''
export function activeNames(members) {
  return members.filter(member => member.active).map(member => member.name).sort();
}
export function pointsByTeam(members) {
  const totals = members.reduce((groups, member) => {
    groups.set(member.team, (groups.get(member.team) ?? 0) + member.points);
    return groups;
  }, new Map());
  return [...totals.keys()].sort().map(team => [team, totals.get(team)]);
}
'''

[[exercise.files]]
path = "report.js"
starter = '''
import { activeNames, pointsByTeam } from "./stats.js";
export function formatReport(members) { return "TODO"; }
'''
solution = '''
import { activeNames, pointsByTeam } from "./stats.js";
export function formatReport(members) {
  const names = activeNames(members).join(", ") || "none";
  const teams = pointsByTeam(members).map(([team, total]) => `${team}=${total}`).join(", ") || "none";
  return `active: ${names}\nteams: ${teams}`;
}
'''

[[exercise.files]]
path = "check.js"
starter = '''
import assert from "node:assert/strict";
import { activeNames, pointsByTeam } from "./stats.js";
import { formatReport } from "./report.js";
const members = Object.freeze([
  Object.freeze({name:"Zoe",team:"red",points:2,active:true}),
  Object.freeze({name:"Ada",team:"blue",points:3,active:true}),
  Object.freeze({name:"Lin",team:"red",points:5,active:false}),
]);
assert.deepEqual(activeNames(members), ["Ada","Zoe"]);
assert.deepEqual(pointsByTeam(members), [["blue",3],["red",7]]);
assert.equal(formatReport(members), "active: Ada, Zoe\nteams: blue=3, red=7");
assert.equal(formatReport([]), "active: none\nteams: none");
assert.deepEqual(pointsByTeam([{team:"__proto__",points:4}]), [["__proto__",4]]);
assert.deepEqual(members.map(member => member.name), ["Zoe","Ada","Lin"]);
console.log("L07 passed");
'''
solution = '''
import assert from "node:assert/strict";
import { activeNames, pointsByTeam } from "./stats.js";
import { formatReport } from "./report.js";
const members = Object.freeze([
  Object.freeze({name:"Zoe",team:"red",points:2,active:true}),
  Object.freeze({name:"Ada",team:"blue",points:3,active:true}),
  Object.freeze({name:"Lin",team:"red",points:5,active:false}),
]);
assert.deepEqual(activeNames(members), ["Ada","Zoe"]);
assert.deepEqual(pointsByTeam(members), [["blue",3],["red",7]]);
assert.equal(formatReport(members), "active: Ada, Zoe\nteams: blue=3, red=7");
assert.equal(formatReport([]), "active: none\nteams: none");
assert.deepEqual(pointsByTeam([{team:"__proto__",points:4}]), [["__proto__",4]]);
assert.deepEqual(members.map(member => member.name), ["Zoe","Ada","Lin"]);
console.log("L07 passed");
'''
+++
# Build a Program from ES Modules

Modules divide a program into pieces with explicit dependencies. A named export makes a binding available to other files; a named import states exactly what the consumer uses.

Each file has its own scope, so helper variables do not accidentally become globals. In this lesson package.json selects ES modules through `"type": "module"`.

Keep data transformations in stats.js and presentation in report.js. report.js should call the exported functions rather than copy their implementations. This separation lets one calculation serve a terminal report, a chart, or another module. Relative Node imports include the extension and begin with `./` or `../`.

For example, two small files can share one conversion:

```js
// units.js
export function minutes(hours) { return hours * 60; }
// schedule.js
import { minutes } from "./units.js";
console.log(minutes(2)); // 120
```

Refactoring is a structural change that keeps behavior stable. Before splitting a working script, capture its outputs and edge cases. Then move one responsibility at a time and rerun those checks.

Here the tests verify both lower-level calculations and the composed report so that a presentation fix cannot conceal a broken calculation.

Array.sort mutates its receiver. Sort the new array produced by map rather than the caller's original members. Use a Map for team totals, which avoids collisions with object property names such as `__proto__`.

Sort team names before formatting to make output stable. Use ordinary string comparison, not locale-dependent formatting, for these ASCII fixtures.

Choose **Open practice** in the exercise card to create or reopen this lesson’s files. Edit the implementation, save your changes, then choose **Check work**. Your practice folder is reused when you return; opening it again keeps your edits. No source checkout or Python is needed.

You can also run this command in the practice folder’s terminal:

```bash
node check.js
```

Edit the implementation files and keep the supplied assertions intact. Read an assertion failure as a concrete example of behavior to repair.

No exercise check downloads packages or accesses the network.

## Hint before a solution

Start by making the calculation helpers pass, then compose the report from their results. Which array can you safely sort without changing the input?

Try one focused change and check again. Before comparing with a reference solution, explain the failing case in your own words. A passing check covers the supplied examples; also try a new input and explain why your implementation handles it.
