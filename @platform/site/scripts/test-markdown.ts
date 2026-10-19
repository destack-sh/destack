import assert from "node:assert/strict";

import { renderMarkdown } from "./markdown.ts";

/** A page with a fence, an alert, a quotation, a footnote, a table, a figure and an abstract. */
const page = `## Setup

\`\`\`ts title="install.ts"
const port = 1;
\`\`\`

> [!NOTE]
> Installs in one step.

> Programs must be written for people to read.
>
> — Harold Abelson

Read the notes.[^1]

| Command | Time |
| --- | --- |
| build | 3 s |

![A bull in a few lines](https://example.com/bull.jpg){width=600}
The Bull, 1945

> [!ABSTRACT]
> One stack for personal software.
>
> [Skip to Setup](#setup)

[^1]: One database per space.
`;

/** The page's tree, as remark and rehype build it and the site shapes it. */
const tree = renderMarkdown(page, { assets: [], kind: "blog", ownHeadings: new Set(["setup"]) });

/** Return a top-level element of the tree by its position, skipping the newlines between. */
function element(index: number) {
    const found = tree.children.filter((node) => node.type === "element")[index];
    if (found === undefined) {
        throw new Error(`missing element ${index}`);
    }

    return found;
}

// name the heading after its text
assert.deepEqual(element(0), {
    type: "element",
    tagName: "h2",
    properties: { id: "setup" },
    children: [{ type: "text", value: "Setup" }],
});

// carry the fence's title, format and language on the listing, its code highlighted one span per line
assert.deepEqual(element(1), {
    type: "element",
    tagName: "pre",
    properties: { dataLanguage: "ts", dataTitle: "install.ts", dataFormat: ".ts" },
    children: [
        {
            type: "element",
            tagName: "code",
            properties: { className: ["language-ts"] },
            children: [
                {
                    type: "element",
                    tagName: "span",
                    properties: {},
                    children: [
                        {
                            type: "element",
                            tagName: "span",
                            properties: { className: ["hljs-keyword"] },
                            children: [{ type: "text", value: "const" }],
                        },
                        { type: "text", value: " port = " },
                        {
                            type: "element",
                            tagName: "span",
                            properties: { className: ["hljs-number"] },
                            children: [{ type: "text", value: "1" }],
                        },
                        { type: "text", value: ";" },
                    ],
                },
            ],
        },
    ],
});

// write the alert as GitHub does, titled by its kind
assert.deepEqual(element(2).properties, { className: ["markdown-alert", "markdown-alert-note"] });

// set the credit outside the quoted words, as the figure's caption
assert.deepEqual(
    [
        element(3).tagName,
        element(3).properties,
        element(3)
            .children.filter((node) => node.type === "element")
            .map((node) => node.tagName),
    ],
    ["figure", { dataKind: "quote" }, ["blockquote", "figcaption"]],
);

// label each body cell with its column's header
assert.deepEqual(JSON.stringify(element(5)).match(/"dataLabel":"[^"]*"/gu), [
    '"dataLabel":"Command"',
    '"dataLabel":"Time"',
]);

// carry the figure's image, width and number, the line under it as the caption
assert.deepEqual(element(6), {
    type: "element",
    tagName: "figure",
    properties: {
        dataKind: "image",
        dataWidth: "600",
        dataSrc: "https://example.com/bull.jpg",
        dataAlt: "A bull in a few lines",
        dataLabel: "Figure 1",
    },
    children: [
        {
            type: "element",
            tagName: "figcaption",
            properties: {},
            children: [{ type: "text", value: "The Bull, 1945" }],
        },
    ],
});

// set the abstract aside, its closing link to a heading as the skip link
assert.deepEqual(
    [
        element(7).tagName,
        element(7).properties,
        element(7)
            .children.filter((node) => node.type === "element")
            .map((node) => node.tagName),
    ],
    ["aside", { dataKind: "abstract", dataTo: "#setup", dataSkip: "Skip to Setup" }, ["p"]],
);

// list the footnote under its visible heading, as GitHub does
assert.deepEqual(
    [element(8).tagName, element(8).properties],
    ["section", { dataFootnotes: true, className: ["footnotes"] }],
);
process.stdout.write("markdown tests passed\n");
