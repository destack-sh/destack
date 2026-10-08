import assert from "node:assert/strict";

import { highlightCode } from "./highlight.ts";

/** A TypeScript snippet. */
const typescriptSource = `const port: number = 300;`;

/** The highlighted TypeScript snippet, as highlight.js classes its tokens. */
const expectedTypescript = [
    {
        type: "element",
        tagName: "span",
        properties: { className: ["hljs-keyword"] },
        children: [{ type: "text", value: "const" }],
    },
    { type: "text", value: " " },
    {
        type: "element",
        tagName: "span",
        properties: { className: ["hljs-attr"] },
        children: [{ type: "text", value: "port" }],
    },
    { type: "text", value: ": " },
    {
        type: "element",
        tagName: "span",
        properties: { className: ["hljs-built_in"] },
        children: [{ type: "text", value: "number" }],
    },
    { type: "text", value: " = " },
    {
        type: "element",
        tagName: "span",
        properties: { className: ["hljs-number"] },
        children: [{ type: "text", value: "300" }],
    },
    { type: "text", value: ";" },
];

// highlight TypeScript fences under both language names
assert.deepEqual(highlightCode(typescriptSource, "ts"), expectedTypescript);
assert.deepEqual(highlightCode(typescriptSource, "typescript"), expectedTypescript);

// keep plain text as one text node, metacharacters and all
assert.deepEqual(highlightCode('a < b && "c"', "text"), [{ type: "text", value: 'a < b && "c"' }]);
process.stdout.write("highlighting tests passed\n");
