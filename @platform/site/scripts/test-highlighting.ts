import assert from "node:assert/strict";

import { highlightCode } from "./highlight.ts";

const typescriptSource = `const port: number = 300;
console.log("port");`;
const expectedTypescript = `<span class="hljs-keyword">const</span> <span class="hljs-attr">port</span>: <span class="hljs-built_in">number</span> = <span class="hljs-number">300</span>;
<span class="hljs-variable language_">console</span>.<span class="hljs-title function_">log</span>(<span class="hljs-string">&quot;port&quot;</span>);`;
const textSource = 'a < b && "c"';
const expectedText = "a &lt; b &amp;&amp; &quot;c&quot;";

// render TypeScript fences under both language names
assert.equal(highlightCode(typescriptSource, "ts"), expectedTypescript);
assert.equal(highlightCode(typescriptSource, "typescript"), expectedTypescript);

// escape plain text without highlighting it
assert.equal(highlightCode(textSource, "text"), expectedText);
