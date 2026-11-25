import { canonicalize, type JsonValue, nth } from "@destack/schema";
import { expect, test } from "@destack/test";
import type { Block, TextSpan } from "../sequence/index.ts";
import { Markdown, MarkdownParser } from "./index.ts";

/** A document with every construct. */
const DOCUMENT = [
    "# Plan **now**",
    "",
    "Ship _it_ ~~soon~~ with `code`, a [link](https://example.com) and",
    "a second line.\\",
    "After a break &amp; <br> <b>bold</b> \\*stars\\* https://destack.sh",
    "",
    "- one",
    "- two",
    "  - nested",
    "",
    "    with a paragraph",
    "- [x] done",
    "- [ ] open",
    "",
    "3. three",
    "4. four",
    "",
    "> quoted",
    ">",
    "> - in a list",
    "",
    "```ts",
    "const x = `1`;",
    "",
    "run();",
    "```",
    "",
    "| Name | Price |",
    "|:-----|------:|",
    "| **A** | 1 |",
    "| b \\| c |  |",
    "|  | x<br>y |",
    "",
    "---",
    "",
    "![A harbour](https://example.com/a.png)",
    "",
    "See [the notes][notes] and [a place](<https://example.com/a(b)>).",
    "",
    "[notes]: https://example.com/notes",
].join("\n");

/** A block without its key, its empty parts left out. */
interface Outline {
    /** The type. */
    readonly type: string;
    /** The value, absent for none. */
    readonly value?: JsonValue;
    /** The spans, absent for none. */
    readonly spans?: Block["spans"];
    /** The blocks, absent for none. */
    readonly blocks?: readonly Outline[];
}

/** Read blocks without their keys and empty parts. */
function outlineOf(blocks: readonly Block[]): Outline[] {
    return blocks.map(({ type, value, spans, blocks: inner }) => ({
        type,
        ...(value === null ? {} : { value }),
        ...(spans.length === 0 ? {} : { spans }),
        ...(inner.length === 0 ? {} : { blocks: outlineOf(inner) }),
    }));
}

/** Read Markdown in pieces of a size. */
function streamed(text: string, size: number): MarkdownParser {
    const parser = new MarkdownParser();
    for (let start = 0; start < text.length; start += size) {
        parser.append(text.slice(start, start + size));
    }

    return parser;
}

/** Read the last block of Markdown being written. */
function last(text: string): Outline | undefined {
    return outlineOf(streamed(text, 5).blocks(true)).at(-1);
}

/** Outline a paragraph. */
function paragraph(...spans: Block["spans"]): Outline {
    return { type: "paragraph", spans };
}

/** Mark a text. */
function marked(text: string, mark: string): Block["spans"][number] {
    return { text, marks: { [mark]: true } };
}

test("read every construct as blocks", () => {
    const bold = { bold: true };
    expect(outlineOf(Markdown.parse(DOCUMENT))).toEqual([
        {
            type: "heading",
            value: { level: 1 },
            spans: [{ text: "Plan " }, { text: "now", marks: bold }],
        },
        {
            type: "paragraph",
            spans: [
                { text: "Ship " },
                { text: "it", marks: { italic: true } },
                { text: " " },
                { text: "soon", marks: { strike: true } },
                { text: " with " },
                { text: "code", marks: { code: true } },
                { text: ", a " },
                { text: "link", marks: { link: { href: "https://example.com" } } },
                { text: " and a second line.\nAfter a break & \n bold *stars* " },
                {
                    text: "https://destack.sh",
                    marks: { link: { href: "https://destack.sh" } },
                },
            ],
        },
        { type: "list", value: { style: "bullet" }, spans: [{ text: "one" }] },
        {
            type: "list",
            value: { style: "bullet" },
            spans: [{ text: "two" }],
            blocks: [
                {
                    type: "list",
                    value: { style: "bullet" },
                    spans: [{ text: "nested" }],
                    blocks: [{ type: "paragraph", spans: [{ text: "with a paragraph" }] }],
                },
            ],
        },
        { type: "todo", value: { isChecked: true }, spans: [{ text: "done" }] },
        { type: "todo", value: { isChecked: false }, spans: [{ text: "open" }] },
        { type: "list", value: { style: "number", start: 3 }, spans: [{ text: "three" }] },
        { type: "list", value: { style: "number", start: 3 }, spans: [{ text: "four" }] },
        {
            type: "quote",
            blocks: [
                { type: "paragraph", spans: [{ text: "quoted" }] },
                { type: "list", value: { style: "bullet" }, spans: [{ text: "in a list" }] },
            ],
        },
        {
            type: "codeBlock",
            value: { language: "ts" },
            spans: [{ text: "const x = `1`;\n\nrun();" }],
        },
        {
            type: "table",
            value: { alignments: ["start", "end"] },
            blocks: [
                {
                    type: "row",
                    value: { isHeader: true },
                    blocks: [
                        { type: "cell", spans: [{ text: "Name" }] },
                        { type: "cell", spans: [{ text: "Price" }] },
                    ],
                },
                {
                    type: "row",
                    value: { isHeader: false },
                    blocks: [
                        { type: "cell", spans: [{ text: "A", marks: bold }] },
                        { type: "cell", spans: [{ text: "1" }] },
                    ],
                },
                {
                    type: "row",
                    value: { isHeader: false },
                    blocks: [{ type: "cell", spans: [{ text: "b | c" }] }, { type: "cell" }],
                },
                {
                    type: "row",
                    value: { isHeader: false },
                    blocks: [{ type: "cell" }, { type: "cell", spans: [{ text: "x\ny" }] }],
                },
            ],
        },
        { type: "divider" },
        { type: "image", value: { source: "https://example.com/a.png", alternative: "A harbour" } },
        {
            type: "paragraph",
            spans: [
                { text: "See " },
                { text: "the notes", marks: { link: { href: "https://example.com/notes" } } },
                { text: " and " },
                { text: "a place", marks: { link: { href: "https://example.com/a(b)" } } },
                { text: "." },
            ],
        },
    ]);
});

test("write blocks as Markdown that reads as the same blocks", () => {
    const blocks = Markdown.parse(DOCUMENT);
    const written = Markdown.stringify(blocks);

    expect(outlineOf(Markdown.parse(written))).toEqual(outlineOf(blocks));
    expect(Markdown.stringify(Markdown.parse(written))).toBe(written);
});

test("write any blocks as Markdown that reads as the same, in pieces too", () => {
    // draw documents from text that looks like marks, links, lists and tables
    const random = new Random(1);
    const changed: string[] = [];
    for (let draw = 0; draw < 60; draw += 1) {
        const blocks = random.blocks(0);
        const written = Markdown.stringify(blocks);
        const read = Markdown.parse(written);
        if (JSON.stringify(shownOf(read)) !== JSON.stringify(shownOf(blocks))) {
            changed.push(written);
        }

        // read it in pieces as whole
        const parser = new MarkdownParser();
        while (parser.text.length < written.length) {
            parser.append(written.slice(parser.text.length, parser.text.length + random.below(30)));
            parser.blocks(true);
        }
        if (JSON.stringify(parser.blocks(false)) !== JSON.stringify(read)) {
            changed.push(`in pieces: ${written}`);
        }
    }

    expect(changed).toEqual([]);
});

test("read a todo with a paragraph under it", () => {
    expect(outlineOf(Markdown.parse("- [ ] Send\n\n  the proof"))).toEqual([
        {
            type: "todo",
            value: { isChecked: false },
            spans: [{ text: "Send" }],
            blocks: [{ type: "paragraph", spans: [{ text: "the proof" }] }],
        },
    ]);
});

test.for([1, 3, 7, 40])("read text in pieces of %i as whole, finished blocks the same", (size) => {
    // collect the blocks after each piece
    const parser = new MarkdownParser();
    let redrawn = 0;
    let before: readonly Block[] = [];
    for (let start = 0; start < DOCUMENT.length; start += size) {
        parser.append(DOCUMENT.slice(start, start + size));
        const blocks = parser.blocks(false);
        for (const [index, block] of before.slice(0, -1).entries()) {
            redrawn +=
                block === blocks[index] || JSON.stringify(block) !== JSON.stringify(blocks[index])
                    ? 0
                    : 1;
        }
        before = blocks;
    }

    expect(parser.blocks(false)).toEqual(Markdown.parse(DOCUMENT));
    expect(redrawn).toBe(0);
});

test("close the marks of the last block while it is written", () => {
    expect([
        last("It is **bol"),
        last("It is *ital"),
        last("It is `cod"),
        last("It is ~~gon"),
        last("It is **bold and *both"),
        last("It is **"),
        last("It is `a ** b` and 2 * 3"),
        last("See [the not"),
        last("See [the notes](https://exa"),
        last("See ![a har"),
        last("Title\n-"),
        last("One\n\n-"),
        last("```ts\nconst x = **"),
        last("snake_case_name"),
        last("```ts\na * b **\n```\nIt is **bol"),
    ]).toEqual([
        paragraph({ text: "It is " }, marked("bol", "bold")),
        paragraph({ text: "It is " }, marked("ital", "italic")),
        paragraph({ text: "It is " }, marked("cod", "code")),
        paragraph({ text: "It is " }, marked("gon", "strike")),
        paragraph({ text: "It is " }, marked("bold and ", "bold"), {
            text: "both",
            marks: { bold: true, italic: true },
        }),
        paragraph({ text: "It is" }),
        paragraph({ text: "It is " }, marked("a ** b", "code"), { text: " and 2 * 3" }),
        paragraph({ text: "See the not" }),
        paragraph({ text: "See the notes" }),
        paragraph({ text: "See" }),
        paragraph({ text: "Title" }),
        paragraph({ text: "One" }),
        { type: "codeBlock", value: { language: "ts" }, spans: [{ text: "const x = **" }] },
        paragraph({ text: "snake_case_name" }),
        paragraph({ text: "It is " }, marked("bol", "bold")),
    ]);
});

test("show no mark of an unfinished block at any point", () => {
    // read a sentence character by character
    const text = "Use **bold**, *italics*, `code`, ~~gone~~ and [a link](https://example.com).";
    const shown: string[] = [];
    for (let end = 1; end <= text.length; end += 1) {
        const parser = streamed(text.slice(0, end), 1);
        shown.push(
            parser
                .blocks(true)
                .map(({ spans }) => spans.map((span) => ("text" in span ? span.text : "")).join(""))
                .join(),
        );
    }

    expect(shown.filter((line) => /[*`~[\]()]/u.test(line))).toEqual([]);
});

/** What random text is made of, some of it what Markdown reads as more. */
const WORDS = [
    ..."one x 5 * ! a*b snake_case _a a_ #no [b] <t> a|b ~~s a~b \\e &amp; `q > +".split(" "),
    ..."1. - --- === www.e.com a@b.c https://e.com".split(" "),
];

/** Random blocks, the same every run. */
class Random {
    /** The state the next number comes from. */
    #state: number;

    /** Start from a seed. */
    constructor(seed: number) {
        this.#state = seed;
    }

    /** Draw a number from zero up to another. */
    below(end: number): number {
        this.#state = (this.#state * 1_103_515_245 + 12_345) % 2_147_483_648;

        return Math.floor((this.#state / 2_147_483_648) * end);
    }

    /** Draw one of some. */
    one<Item>(items: readonly Item[]): Item {
        return nth(items, this.below(items.length));
    }

    /** Draw as many as a number from one up to another. */
    many<Item>(end: number, draw: (index: number) => Item): Item[] {
        return Array.from({ length: 1 + this.below(end) }, (_, index) => draw(index));
    }

    /** Draw the blocks at a depth. */
    blocks(depth: number): Block[] {
        return this.many(3, () => this.block(depth));
    }

    /** Draw a block, with others in it but for deep ones. */
    block(depth: number): Block {
        // draw one with text alone
        const inner = depth < 3 && this.below(2) === 0 ? this.blocks(depth + 1) : [];
        switch (this.below(depth < 3 ? 8 : 4)) {
            case 0:
                return blockOf("heading", { level: 1 + this.below(6) }, this.spans());
            case 1:
                return blockOf("divider", null);
            case 2: {
                const text = this.one(["a", "``` x", "a\n\n  b"]);

                return blockOf("codeBlock", { language: "ts" }, [{ text }]);
            }
            // draw one with others in it
            case 4:
                return blockOf("list", { style: "bullet" }, this.spans(), inner);
            case 5:
                return blockOf("list", { style: "number" }, this.spans(), inner);
            case 6:
                return blockOf("todo", { isChecked: this.below(2) === 0 }, this.spans(), inner);
            case 7:
                return this.below(2) === 0
                    ? blockOf("quote", null, [], this.blocks(depth + 1))
                    : this.table();
            default:
                return blockOf("paragraph", null, this.spans());
        }
    }

    /** Draw a table, some of its cells empty. */
    table(): Block {
        const alignments = this.many(3, () => this.one(["start", "center", "end"]));
        const cell = () => blockOf("cell", null, this.below(5) === 0 ? [] : this.spans());
        const rows = this.many(3, (row) =>
            blockOf("row", { isHeader: row === 0 }, [], alignments.map(cell)),
        );

        return blockOf("table", { alignments }, [], rows);
    }

    /** Draw spans, each with some marks. */
    spans(): TextSpan[] {
        return this.many(3, (index) => {
            const words = this.many(3, () => this.one(WORDS)).join(" ");
            const marks = {
                ...(this.below(3) === 0 ? { bold: true } : {}),
                ...(this.below(3) === 0 ? { italic: true } : {}),
                ...(this.below(5) === 0 ? { strike: true } : {}),
                ...(this.below(6) === 0 ? { code: true } : {}),
                ...(this.below(6) === 0
                    ? { link: { href: this.one(["https://e.com/a", "/b(c)", "/d e"]) } }
                    : {}),
            };

            return { text: (index > 0 && this.below(2) === 0 ? " " : "") + words, marks };
        });
    }
}

/** Write a block. */
function blockOf(
    type: string,
    value: Block["value"],
    spans: readonly TextSpan[] = [],
    blocks: readonly Block[] = [],
): Block {
    return { key: "", type, value, spans, blocks };
}

/** Write blocks as they show: each word and code with its marks, the space between without. */
function shownOf(blocks: readonly Block[]): unknown[] {
    return blocks.map((block) => {
        // put a same mark's stretches together
        const stretches: [marks: string, text: string][] = [];
        for (const span of block.spans) {
            const text = "text" in span ? span.text : "";
            const marks = "text" in span ? (span.marks ?? {}) : {};
            const pieces = marks["code"] === true ? [text] : (text.match(/\s+|\S+/gu) ?? []);
            for (const piece of pieces) {
                const written = marks["code"] !== true && piece.trim() === "" ? {} : marks;
                const key = canonicalize(written);
                const before = stretches.at(-1);
                if (before?.[0] === key) {
                    before[1] += piece;
                } else {
                    stretches.push([key, piece]);
                }
            }
        }

        return [block.type, block.value, stretches, shownOf(block.blocks)];
    });
}
