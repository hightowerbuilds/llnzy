+++
title = "Capstone: A JSON Expense Report CLI"
concepts = ["command-line arguments", "JSON file boundaries", "integer cents", "module composition", "exit codes"]
[[exercise]]
prompt = "Complete ledger.js and cli.js. The command is node cli.js FILE [TAG]. Read the JSON array asynchronously, validate each expense, optionally filter by exact tag, then output sorted \"tag: cents\" lines and a final \"total: cents\" line. Validate the complete input before filtering. Errors go to stderr with prefix \"error: \" and exit code 1. See the lesson for exact validation messages."
[exercise.check]
command = ["node", "check.js"]
expected = "L10 passed\n"
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
path = "ledger.js"
starter = '''
export function parseEntries(raw) { return []; }
export function formatLedger(entries, tag) { return "total: 0"; }
'''
solution = '''
export function parseEntries(raw) {
  if (!Array.isArray(raw)) throw new Error("entries must be an array");
  return raw.map(value => {
    if (typeof value !== "object" || value === null || Array.isArray(value)
      || typeof value.label !== "string" || value.label.trim().length === 0
      || typeof value.tag !== "string" || value.tag.trim().length === 0
      || !Number.isSafeInteger(value.cents) || value.cents < 0) throw new Error("invalid entry");
    return {label:value.label,tag:value.tag,cents:value.cents};
  });
}
export function formatLedger(entries, tag) {
  const selected = entries.filter(entry => tag === undefined || entry.tag === tag);
  const totals = selected.reduce((groups, entry) => {
    const sum = (groups.get(entry.tag) ?? 0) + entry.cents;
    if (!Number.isSafeInteger(sum)) throw new Error("total exceeds safe integer range");
    groups.set(entry.tag, sum);
    return groups;
  }, new Map());
  const total = [...totals.values()].reduce((sum, value) => {
    const next = sum + value;
    if (!Number.isSafeInteger(next)) throw new Error("total exceeds safe integer range");
    return next;
  }, 0);
  return [...[...totals.keys()].sort().map(key => `${key}: ${totals.get(key)}`), `total: ${total}`].join("\n");
}
'''

[[exercise.files]]
path = "cli.js"
starter = '''
import { readFile } from "node:fs/promises";
import { parseEntries, formatLedger } from "./ledger.js";
console.log("TODO: read and report");
'''
solution = '''
import { readFile } from "node:fs/promises";
import { parseEntries, formatLedger } from "./ledger.js";
try {
  const args = process.argv.slice(2);
  if (args.length < 1 || args.length > 2) throw new Error("usage: node cli.js FILE [TAG]");
  const text = await readFile(args[0], "utf8");
  const entries = parseEntries(JSON.parse(text));
  console.log(formatLedger(entries, args[1]));
} catch (error) {
  console.error(`error: ${error instanceof Error ? error.message : String(error)}`);
  process.exitCode = 1;
}
'''

[[exercise.files]]
path = "check.js"
starter = '''
import assert from "node:assert/strict";
import { parseEntries, formatLedger } from "./ledger.js";
import { mkdtemp, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
const raw = [{label:"train",tag:"travel",cents:450,extra:true},{label:"tea",tag:"food",cents:125},{label:"rice",tag:"food",cents:275}];
const entries = parseEntries(raw);
assert.deepEqual(entries[0],{label:"train",tag:"travel",cents:450});
assert.notEqual(entries[0],raw[0]);
assert.equal(formatLedger(entries),"food: 400\ntravel: 450\ntotal: 850");
assert.equal(formatLedger(entries,"food"),"food: 400\ntotal: 400");
assert.equal(formatLedger(entries,""),"total: 0");
assert.equal(formatLedger([]),"total: 0");
for (const value of [null,[],{}, {label:" ",tag:"x",cents:1},{label:"x",tag:"",cents:1},{label:"x",tag:"x",cents:-1},{label:"x",tag:"x",cents:1.2},{label:"x",tag:"x",cents:Number.MAX_SAFE_INTEGER+1}]) {
  assert.throws(() => parseEntries([value]), /invalid entry/);
}
assert.throws(() => parseEntries({}), /entries must be an array/);
assert.equal(formatLedger(parseEntries([{label:"free",tag:"__proto__",cents:0}])),"__proto__: 0\ntotal: 0");
for (const secondTag of ["a","b"]) {
  assert.throws(() => formatLedger(parseEntries([{label:"a",tag:"a",cents:Number.MAX_SAFE_INTEGER},{label:"b",tag:secondTag,cents:1}])), /total exceeds safe integer range/);
}
const directory = await mkdtemp(join(tmpdir(),"llnzy-ledger-"));
try {
  const file = join(directory,"expenses.json");
  await writeFile(file,JSON.stringify(raw));
  function run(args) {
    const result = spawnSync(process.execPath,["cli.js",...args],{encoding:"utf8",timeout:3000});
    assert.equal(result.error,undefined,"CLI starts and finishes");
    return result;
  }
  const all = run([file]);
  assert.equal(all.status,0);
  assert.equal(all.stdout,"food: 400\ntravel: 450\ntotal: 850\n");
  assert.equal(all.stderr,"");
  const filtered = run([file,"food"]);
  assert.equal(filtered.status,0);
  assert.equal(filtered.stdout,"food: 400\ntotal: 400\n");
  for (const args of [[],[file,"food","extra"]]) {
    const result = run(args);
    assert.equal(result.status,1);
    assert.equal(result.stdout,"");
    assert.equal(result.stderr,"error: usage: node cli.js FILE [TAG]\n");
  }
  await writeFile(file,JSON.stringify([...raw,{label:"bad",tag:"other",cents:-1}]));
  const bad = run([file,"food"]);
  assert.equal(bad.status,1,"validate before filtering");
  assert.equal(bad.stdout,"");
  assert.equal(bad.stderr,"error: invalid entry\n");
  await writeFile(file,"{");
  const malformed = run([file]);
  assert.equal(malformed.status,1);
  assert.equal(malformed.stdout,"");
  assert.match(malformed.stderr,/^error: /);
  const absent = run([join(directory,"missing.json")]);
  assert.equal(absent.status,1);
  assert.equal(absent.stdout,"");
  assert.match(absent.stderr,/^error: /);
} finally {
  await rm(directory,{recursive:true,force:true});
}
console.log("L10 passed");
'''
solution = '''
import assert from "node:assert/strict";
import { parseEntries, formatLedger } from "./ledger.js";
import { mkdtemp, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
const raw = [{label:"train",tag:"travel",cents:450,extra:true},{label:"tea",tag:"food",cents:125},{label:"rice",tag:"food",cents:275}];
const entries = parseEntries(raw);
assert.deepEqual(entries[0],{label:"train",tag:"travel",cents:450});
assert.notEqual(entries[0],raw[0]);
assert.equal(formatLedger(entries),"food: 400\ntravel: 450\ntotal: 850");
assert.equal(formatLedger(entries,"food"),"food: 400\ntotal: 400");
assert.equal(formatLedger(entries,""),"total: 0");
assert.equal(formatLedger([]),"total: 0");
for (const value of [null,[],{}, {label:" ",tag:"x",cents:1},{label:"x",tag:"",cents:1},{label:"x",tag:"x",cents:-1},{label:"x",tag:"x",cents:1.2},{label:"x",tag:"x",cents:Number.MAX_SAFE_INTEGER+1}]) {
  assert.throws(() => parseEntries([value]), /invalid entry/);
}
assert.throws(() => parseEntries({}), /entries must be an array/);
assert.equal(formatLedger(parseEntries([{label:"free",tag:"__proto__",cents:0}])),"__proto__: 0\ntotal: 0");
for (const secondTag of ["a","b"]) {
  assert.throws(() => formatLedger(parseEntries([{label:"a",tag:"a",cents:Number.MAX_SAFE_INTEGER},{label:"b",tag:secondTag,cents:1}])), /total exceeds safe integer range/);
}
const directory = await mkdtemp(join(tmpdir(),"llnzy-ledger-"));
try {
  const file = join(directory,"expenses.json");
  await writeFile(file,JSON.stringify(raw));
  function run(args) {
    const result = spawnSync(process.execPath,["cli.js",...args],{encoding:"utf8",timeout:3000});
    assert.equal(result.error,undefined,"CLI starts and finishes");
    return result;
  }
  const all = run([file]);
  assert.equal(all.status,0);
  assert.equal(all.stdout,"food: 400\ntravel: 450\ntotal: 850\n");
  assert.equal(all.stderr,"");
  const filtered = run([file,"food"]);
  assert.equal(filtered.status,0);
  assert.equal(filtered.stdout,"food: 400\ntotal: 400\n");
  for (const args of [[],[file,"food","extra"]]) {
    const result = run(args);
    assert.equal(result.status,1);
    assert.equal(result.stdout,"");
    assert.equal(result.stderr,"error: usage: node cli.js FILE [TAG]\n");
  }
  await writeFile(file,JSON.stringify([...raw,{label:"bad",tag:"other",cents:-1}]));
  const bad = run([file,"food"]);
  assert.equal(bad.status,1,"validate before filtering");
  assert.equal(bad.stdout,"");
  assert.equal(bad.stderr,"error: invalid entry\n");
  await writeFile(file,"{");
  const malformed = run([file]);
  assert.equal(malformed.status,1);
  assert.equal(malformed.stdout,"");
  assert.match(malformed.stderr,/^error: /);
  const absent = run([join(directory,"missing.json")]);
  assert.equal(absent.status,1);
  assert.equal(absent.stdout,"");
  assert.match(absent.stderr,/^error: /);
} finally {
  await rm(directory,{recursive:true,force:true});
}
console.log("L10 passed");
'''
+++
# Capstone: A JSON Expense Report CLI

Build a small command-line program whose responsibilities are visible in separate modules. ledger.js owns validation and reporting; cli.js owns argument parsing, file reading, stdout, stderr, and exit status.

The check harness creates temporary JSON files and runs the actual CLI as a child process, so success requires the pieces to work together.

Run `node cli.js expenses.json` to report every tag, or `node cli.js expenses.json food` to select food. Exactly one file argument and at most one tag are allowed.

Reject other argument counts with `usage: node cli.js FILE [TAG]`. An explicit empty tag matches nothing. A successful report always ends with a total line, including `total: 0` for no matching records.

The input must be an array; otherwise throw `entries must be an array`. Each expense must be a non-null, non-array object with a nonblank string label, a nonblank string tag, and nonnegative safe-integer cents; otherwise throw `invalid entry`.

Return fresh records containing only label, tag, and cents. Preserve accepted strings. Use integer cents throughout, and reject either a per-tag sum or the grand total outside the safe-integer range with `total exceeds safe integer range`.

The following file is a useful manual fixture, different from the automated checks:

```json
[
  {"label":"notebook","tag":"office","cents":350},
  {"label":"pencil","tag":"office","cents":75}
]
```

It should print `office: 425` and then `total: 425`. JSON requires double-quoted keys and strings; it does not allow comments or trailing commas.

The file reader returns text, JSON.parse constructs JavaScript values, and validation decides whether those values meet your contract. Validate every expense before filtering so a tag filter cannot hide malformed records.

Use `process.argv.slice(2)` for user arguments. The first two entries describe the runtime and script. Read with `readFile(path, "utf8")` from node:fs/promises and await the result.

Catch at the CLI boundary, print `error: ` plus the error message with console.error, and set `process.exitCode = 1`; this allows normal stream flushing. Do not match platform-dependent filesystem error wording in tests, but preserve useful messages for a person debugging their own command.

Build summaries with filter, reduce, and map, keeping formatting deterministic by sorting tags. A Map handles arbitrary tag strings safely.

As a final review, explain why validation precedes reporting, why cents stay integers, why report generation does not write files, and why a failed CLI writes to stderr rather than stdout. This project reports an existing journal; persistent editing is a possible follow-up extension.

Choose **Open practice** in the exercise card to create or reopen this lesson’s files. Edit the implementation, save your changes, then choose **Check work**. Your practice folder is reused when you return; opening it again keeps your edits. No source checkout or Python is needed.

You can also run this command in the practice folder’s terminal:

```bash
node check.js
```

Edit the implementation files and keep the supplied assertions intact. Read an assertion failure as a concrete example of behavior to repair.

No exercise check downloads packages or accesses the network.

## Hint before a solution

Make validation and pure reporting work before connecting file input. Exercise invalid data and no matching tags as carefully as a successful report.

Try one focused change and check again. Before comparing with a reference solution, explain the failing case in your own words. A passing check covers the supplied examples; also try a new input and explain why your implementation handles it.
