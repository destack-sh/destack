import assert from "node:assert/strict";

import { highlightCode } from "./highlight.mjs";

const destackSource = `const port: uint8 = 300;
console.log(port);`;
const expectedDestack = `<span data-k="keyword">const</span> <span data-k="name">port</span><span data-k="punct">:</span> <span data-k="type">uint8</span> <span data-k="punct">=</span> <span data-k="literal">300</span><span data-k="punct">;</span>
<span data-k="name">console</span><span data-k="punct">.</span><span data-k="function">log</span><span data-k="punct">(</span><span data-k="name">port</span><span data-k="punct">)</span><span data-k="punct">;</span>`;
const pattern = "$CALLEE($URL, $$$ARGUMENTS)";
const expectedPattern = `<span data-k="function">$CALLEE</span><span data-k="punct">(</span><span data-k="name">$URL</span><span data-k="punct">,</span> <span data-k="name">$$$ARGUMENTS</span><span data-k="punct">)</span>`;
const mirSource = `function @fib(v0: int64) -> int64 {
block0(v0: int64):
    v1: int64 = iconst 2i64
    return v1
}`;
const expectedMir = `<span data-k="keyword">function</span> <span data-k="name">@fib</span><span data-k="punct">(</span><span data-k="name">v0</span><span data-k="punct">:</span> <span data-k="type">int64</span><span data-k="punct">)</span> <span data-k="punct">-&gt;</span> <span data-k="type">int64</span> <span data-k="punct">{</span>
<span data-k="name">block0</span><span data-k="punct">(</span><span data-k="name">v0</span><span data-k="punct">:</span> <span data-k="type">int64</span><span data-k="punct">)</span><span data-k="punct">:</span>
    <span data-k="name">v1</span><span data-k="punct">:</span> <span data-k="type">int64</span> <span data-k="punct">=</span> iconst <span data-k="literal">2i64</span>
    <span data-k="keyword">return</span> <span data-k="name">v1</span>
<span data-k="punct">}</span>`;
const bytecodeSource = `// imported declaration
function imported

function dispatch {
b0:
    return r0:r1
b1:return r2
}`;
const expectedBytecode = `<span data-k="comment">// imported declaration</span>
<span data-k="keyword">function</span> <span data-k="function">imported</span>

<span data-k="keyword">function</span> <span data-k="function">dispatch</span> {
<span data-k="name">b0</span><span data-k="punct">:</span>
    <span data-k="function">return</span> <span data-k="name">r0</span><span data-k="punct">:</span><span data-k="name">r1</span>
<span data-k="name">b1</span><span data-k="punct">:</span><span data-k="function">return</span> <span data-k="name">r2</span>
}`;

// preserve the complete parser backed rendering used by homepage Destack examples
assert.equal(highlightCode(destackSource, "destack"), expectedDestack);

// preserve Destack metavariables in structural pattern specimens
assert.equal(highlightCode(pattern, "pattern"), expectedPattern);

// render MIR fences through the repository grammar under canonical and descriptive names
assert.equal(highlightCode(mirSource, "dsm"), expectedMir);
assert.equal(highlightCode(mirSource, "mir"), expectedMir);

// render bytecode labels, registers, and opcodes through the repository grammar
assert.equal(highlightCode(bytecodeSource, "dsa"), expectedBytecode);
assert.equal(highlightCode(bytecodeSource, "bytecode"), expectedBytecode);
