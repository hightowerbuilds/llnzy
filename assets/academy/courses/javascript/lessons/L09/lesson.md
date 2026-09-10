+++
title = "Consume Text as an Async Stream"
concepts = ["async iteration", "readline", "chunk boundaries", "pure formatting"]
[[exercise]]
prompt = "Implement summarizeLines(readable) in lines.js using readline and for-await-of. Trim each line, skip blank lines, uppercase text, and count whitespace-separated words. Return {lines:[{text,words}],total}. Implement formatSummary to render each line as \"TEXT | words\" followed by \"total: N\"; an empty input renders only \"total: 0\"."
[exercise.check]
command = ["node", "check.js"]
expected = "L09 passed\n"
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
path = "lines.js"
starter = '''
import { createInterface } from "node:readline";
export async function summarizeLines(readable) { return {lines:[],total:0}; }
export function formatSummary(summary) { return "TODO"; }
'''
solution = '''
import { createInterface } from "node:readline";
export async function summarizeLines(readable) {
  const reader = createInterface({input: readable, crlfDelay: Infinity});
  const lines = [];
  let total = 0;
  try {
    for await (const raw of reader) {
      const text = raw.trim();
      if (text === "") continue;
      const words = text.split(/\s+/).length;
      lines.push({text:text.toUpperCase(),words});
      total += words;
    }
  } finally { reader.close(); }
  return {lines,total};
}
export function formatSummary(summary) {
  return [...summary.lines.map(line => `${line.text} | ${line.words}`), `total: ${summary.total}`].join("\n");
}
'''

[[exercise.files]]
path = "check.js"
starter = '''
import assert from "node:assert/strict";
import { Readable } from "node:stream";
import { summarizeLines, formatSummary } from "./lines.js";
const expected = {lines:[{text:"HELLO WORLD",words:2},{text:"JAVASCRIPT   STREAMS",words:2},{text:"FINAL",words:1}],total:5};
assert.deepEqual(await summarizeLines(Readable.from([" hello wo","rld\r","\n \njava","script   streams\nfinal"])), expected);
assert.equal(formatSummary(expected),"HELLO WORLD | 2\nJAVASCRIPT   STREAMS | 2\nFINAL | 1\ntotal: 5");
const whole = " hello world\r\n \njavascript   streams\nfinal";
assert.deepEqual(await summarizeLines(Readable.from([whole])),expected,"chunk independent");
assert.deepEqual(await summarizeLines(Readable.from([])),{lines:[],total:0});
assert.equal(formatSummary(await summarizeLines(Readable.from([" \n\t\r\n"]))),"total: 0");
assert.deepEqual(await summarizeLines(Readable.from(["one\ttwo"])),{lines:[{text:"ONE\tTWO",words:2}],total:2});
console.log("L09 passed");
'''
solution = '''
import assert from "node:assert/strict";
import { Readable } from "node:stream";
import { summarizeLines, formatSummary } from "./lines.js";
const expected = {lines:[{text:"HELLO WORLD",words:2},{text:"JAVASCRIPT   STREAMS",words:2},{text:"FINAL",words:1}],total:5};
assert.deepEqual(await summarizeLines(Readable.from([" hello wo","rld\r","\n \njava","script   streams\nfinal"])), expected);
assert.equal(formatSummary(expected),"HELLO WORLD | 2\nJAVASCRIPT   STREAMS | 2\nFINAL | 1\ntotal: 5");
const whole = " hello world\r\n \njavascript   streams\nfinal";
assert.deepEqual(await summarizeLines(Readable.from([whole])),expected,"chunk independent");
assert.deepEqual(await summarizeLines(Readable.from([])),{lines:[],total:0});
assert.equal(formatSummary(await summarizeLines(Readable.from([" \n\t\r\n"]))),"total: 0");
assert.deepEqual(await summarizeLines(Readable.from(["one\ttwo"])),{lines:[{text:"ONE\tTWO",words:2}],total:2});
console.log("L09 passed");
'''
+++
# Consume Text as an Async Stream

A stream delivers chunks as they become available. A chunk is not necessarily a complete line: it might contain half a word, several lines, or a carriage return separated from its following newline. Splitting each raw chunk independently would make the answer depend on how the data arrived.

Node's readline interface handles line framing. Construct it with `{ input: readable, crlfDelay: Infinity }` to treat CRLF as one line ending even across delayed chunks.

Iterate with for-await-of; each iteration receives one framed line. Trim the line, skip an empty result, and split the remainder on one or more whitespace characters to count words.

Here is a separate example that reads two rows from arbitrary chunks:

```js
import { Readable } from "node:stream";
import { createInterface } from "node:readline";
const source = Readable.from(["north\nso", "uth\n"]);
const reader = createInterface({input: source, crlfDelay: Infinity});
for await (const row of reader) console.log(row);
```

Use try/finally to close your readline interface when iteration finishes or your processing throws. Closing an interface and destroying a caller-owned input stream are different operations; this function does not take ownership of destroying the stream.

This exercise accepts finite, non-failing streams. Readline async iteration does not forward input-stream errors; try/finally provides cleanup, not recovery from a failing source. These fixtures use finite in-memory sources, so there are no network resources or file handles to manage.

Keep formatting separate from input consumption. A pure formatSummary is easy to test and reuse. Count words before uppercasing; preserve internal spaces in displayed text while using flexible whitespace matching for the count.

The exercise stores line results, so its memory usage grows with input size even though it consumes a stream. A huge-file version would emit each summary or aggregate only totals rather than retaining every row.

From the repository root, export this lesson. Python 3.11+ is required, and the destination must be new:

```bash
python3 scripts/check_academy_courses.py --export javascript L09 /tmp/llnzy-javascript-L09
```

Open the exported directory in the editor, then run the check from that directory with Node.js 22 or newer:

```bash
node check.js
```

Edit the implementation files and keep the supplied assertions intact. Read an assertion failure as a concrete example of behavior to repair.

No exercise check downloads packages or accesses the network.

Reference: [Node.js readline async iteration](https://nodejs.org/api/readline.html#rlsymbolasynciterator).
