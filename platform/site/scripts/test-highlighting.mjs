import assert from "node:assert/strict";

import { highlightCode } from "./highlight.mjs";

const source = `const port: uint8 = 300;
console.log(port);`;
const expected = `<span data-k="keyword">const</span> <span data-k="name">port</span><span data-k="punct">:</span> <span data-k="type">uint8</span> <span data-k="punct">=</span> <span data-k="literal">300</span><span data-k="punct">;</span>
<span data-k="name">console</span><span data-k="punct">.</span><span data-k="function">log</span><span data-k="punct">(</span><span data-k="name">port</span><span data-k="punct">)</span><span data-k="punct">;</span>`;

// preserve the complete parser backed rendering used by homepage Destack examples
assert.equal(highlightCode(source, "destack"), expected);
