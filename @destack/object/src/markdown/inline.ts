import type { SyntaxNode } from "@lezer/common";
import { type JsonObject, JsonValue } from "@destack/schema";
import type { TextSpan } from "../sequence/index.ts";
import { Mention } from "./mention.ts";

/** The characters Markdown names by entities, as writers use them. */
const ENTITIES: Readonly<Record<string, string>> = {
    amp: "&",
    lt: "<",
    gt: ">",
    quot: '"',
    apos: "'",
    nbsp: " ",
    copy: "©",
    hellip: "…",
    mdash: "—",
    ndash: "–",
};

/** The marks nodes set on the text in them. */
const MARKS: Readonly<Record<string, string>> = {
    StrongEmphasis: "bold",
    Emphasis: "italic",
    Strikethrough: "strike",
    InlineCode: "code",
};

/** The nodes that mark text and show nothing themselves. */
const HIDDEN: ReadonlySet<string> = new Set([
    "EmphasisMark",
    "StrikethroughMark",
    "CodeMark",
    "LinkMark",
    "LinkTitle",
    "LinkLabel",
    "HeaderMark",
    "QuoteMark",
    "ListMark",
    "TaskMarker",
    "TableDelimiter",
    "Comment",
    "CommentBlock",
]);

/** The marks on a stretch of text. */
type Marks = Readonly<Record<string, JsonValue>>;

/** Reads the text within the blocks of some Markdown. */
export class Inline {
    /** The Markdown. */
    readonly #text: string;
    /** The addresses it defines for links, by their labels. */
    readonly #references: ReadonlyMap<string, string>;

    /** Read the text of some Markdown. */
    constructor(text: string, references: ReadonlyMap<string, string>) {
        this.#text = text;
        this.#references = references;
    }

    /** Read a label as links compare it, without its brackets. */
    static label(text: string): string {
        return text.slice(1, -1).trim().replace(/\s+/gu, " ").toLowerCase();
    }

    /** Read the text of a node with its marks. */
    spans(node: SyntaxNode): TextSpan[] {
        // drop the space around the text and after a task's mark
        const spans: { text: string; marks?: Marks }[] = [];
        this.#walk(node, {}, spans);
        const first = spans[0];
        const last = spans.at(-1);
        if (first !== undefined && last !== undefined) {
            first.text = first.text.trimStart();
            last.text = last.text.trimEnd();
        }

        // read a link to a subject from its at sign as a mention
        return spans
            .filter(({ text }) => text !== "")
            .map((span): TextSpan => {
                // take the subject of the span's link address
                const link = span.marks?.["link"];
                const href =
                    link !== undefined && JsonValue.isObject(link) ? link["href"] : undefined;
                const subject = typeof href === "string" ? Mention.subject(href) : undefined;

                return subject !== undefined && span.text.startsWith("@")
                    ? { node: Mention.node(subject) }
                    : span;
            });
    }

    /** Read the code of a block, line by line as it stands in the block. */
    code(node: SyntaxNode): TextSpan[] {
        const text = node
            .getChildren("CodeText")
            .map((each) => this.#text.slice(each.from, each.to))
            .join("");

        return text === "" ? [] : [{ text }];
    }

    /** Read the language of a block of code, none for plain text. */
    language(node: SyntaxNode): JsonObject {
        const written = node.getChild("CodeInfo");
        const language =
            written === null
                ? ""
                : (this.#text.slice(written.from, written.to).split(/\s/u)[0] ?? "");

        return language === "" ? {} : { language };
    }

    /** Read the number a list starts at. */
    start(list: SyntaxNode): number {
        const mark = list.getChild("ListItem")?.getChild("ListMark") ?? null;

        return mark === null ? 1 : Number.parseInt(this.#text.slice(mark.from, mark.to), 10);
    }

    /** Report whether a task is checked. */
    isChecked(task: SyntaxNode): boolean {
        const mark = task.getChild("TaskMarker");

        return mark !== null && /x/iu.test(this.#text.slice(mark.from, mark.to));
    }

    /** Read how the columns of a table align. */
    alignments(table: SyntaxNode): JsonObject {
        // take the line under the header
        const delimiter = table.getChild("TableDelimiter");
        const written = delimiter === null ? "" : this.#text.slice(delimiter.from, delimiter.to);

        // read each column's colons
        const alignments = written
            .replace(/^\s*\||\|\s*$/gu, "")
            .split("|")
            .map((each) => {
                const column = each.trim();
                const isEnd = column.endsWith(":");

                return column.startsWith(":") && isEnd ? "center" : isEnd ? "end" : "start";
            });

        return { alignments };
    }

    /** Read a paragraph that is one image, undefined for any other. */
    image(paragraph: SyntaxNode): JsonObject | undefined {
        const { firstChild } = paragraph;
        const isAlone =
            firstChild !== null &&
            firstChild.name === "Image" &&
            firstChild.from === paragraph.from &&
            firstChild.to === paragraph.to;

        return isAlone ? this.#imageOf(firstChild) : undefined;
    }

    /** Add a node's text to spans, with the marks around it. */
    #walk(node: SyntaxNode, marks: Marks, spans: { text: string; marks?: Marks }[]): void {
        // add the text between the node's children, and theirs
        let from = node.from;
        for (let child = node.firstChild; child !== null; child = child.nextSibling) {
            this.#add(this.#plain(from, child.from), marks, spans);
            this.#child(child, marks, spans);
            from = child.to;
        }
        this.#add(this.#plain(from, node.to), marks, spans);
    }

    /** Add a child's text to spans. */
    #child(child: SyntaxNode, marks: Marks, spans: { text: string; marks?: Marks }[]): void {
        const written = this.#text.slice(child.from, child.to);
        const mark = MARKS[child.name];
        if (HIDDEN.has(child.name)) {
            return;
        }
        // mark the text in it
        else if (child.name === "InlineCode") {
            this.#add(this.#codeOf(child), { ...marks, code: true }, spans);
        } else if (mark !== undefined) {
            this.#walk(child, { ...marks, [mark]: true }, spans);
        }
        // link its text to its address
        else if (child.name === "Link") {
            this.#link(child, marks, spans);
        } else if (child.name === "Autolink" || child.name === "URL") {
            const text = written.replace(/^<|>$/gu, "");
            const href = /^[a-z][a-z0-9+.-]*:/iu.test(text)
                ? text
                : text.includes("@")
                  ? `mailto:${text}`
                  : `https://${text}`;
            this.#add(text, { ...marks, link: { href } }, spans);
        }
        // show an image within text by its words
        else if (child.name === "Image") {
            this.#add(this.#imageOf(child).alternative, marks, spans);
        }
        // break the line
        else if (child.name === "HardBreak") {
            this.#add("\n", marks, spans);
        }
        // read an escaped character and an entity as what they stand for
        else if (child.name === "Escape") {
            this.#add(written.slice(1), marks, spans);
        } else if (child.name === "Entity") {
            this.#add(entityOf(written), marks, spans);
        }
        // leave out HTML but for its line breaks
        else if (child.name === "HTMLTag") {
            this.#add(/^<br\s*\/?>$/iu.test(written) ? "\n" : "", marks, spans);
        } else {
            this.#walk(child, marks, spans);
        }
    }

    /** Add a link's words to spans, linked to its address when it has one. */
    #link(link: SyntaxNode, marks: Marks, spans: { text: string; marks?: Marks }[]): void {
        // take the words between its first two marks, an address among them as words
        const href = this.#addressOf(link);
        const own = href === undefined ? marks : { ...marks, link: { href } };
        const [open, close] = link.getChildren("LinkMark");
        let from = open?.to ?? link.from;
        for (
            let child = open?.nextSibling ?? null;
            child !== null && child.from < (close?.from ?? 0);
            child = child.nextSibling
        ) {
            this.#add(this.#plain(from, child.from), own, spans);
            if (child.name === "URL") {
                this.#add(this.#text.slice(child.from, child.to), own, spans);
            } else {
                this.#child(child, own, spans);
            }
            from = child.to;
        }
        this.#add(this.#plain(from, close?.from ?? from), own, spans);
    }

    /** Add text to the last span with the same marks, else to a new one. */
    #add(text: string, marks: Marks, spans: { text: string; marks?: Marks }[]): void {
        const last = spans.at(-1);
        if (text === "") {
            return;
        } else if (
            last !== undefined &&
            JSON.stringify(last.marks ?? {}) === JSON.stringify(marks)
        ) {
            last.text += text;
        } else {
            spans.push(Object.keys(marks).length === 0 ? { text } : { text, marks });
        }
    }

    /** Read code within text, its padding and the escapes of a table off. */
    #codeOf(code: SyntaxNode): string {
        // take the backticks off, and the pipes' escapes in a cell
        const written = this.#text.slice(code.from, code.to);
        const fence = /^`+/u.exec(written)?.[0].length ?? 0;
        let text = written.slice(fence, -fence).replace(/\n/gu, " ");
        for (let node = code.parent; node !== null; node = node.parent) {
            if (node.name === "TableCell") {
                text = text.replaceAll("\\|", "|");
            }
        }

        return /^ [\s\S]*[^ ][\s\S]* $/u.test(text) ? text.slice(1, -1) : text;
    }

    /** Read text between nodes, a line's end and the next line's indent as one space. */
    #plain(from: number, to: number): string {
        return this.#text.slice(from, to).replace(/[ \t]*\n[ \t]*/gu, " ");
    }

    /** Read the address a link or an image has, or its label is defined for. */
    #addressOf(node: SyntaxNode): string | undefined {
        // take its own, after the words
        const close = node.getChildren("LinkMark")[1];
        const url = node.getChildren("URL").find(({ from }) => from > (close?.from ?? 0));
        if (url !== undefined) {
            return this.#text.slice(url.from, url.to).replace(/^<|>$/gu, "");
        }

        // else look up its label, or its text in brackets
        const label = node.getChild("LinkLabel");
        const [open] = node.getChildren("LinkMark");
        const written =
            label !== null && label.to - label.from > 2
                ? this.#text.slice(label.from, label.to)
                : open === undefined || close === undefined
                  ? ""
                  : `[${this.#text.slice(open.to, close.from)}]`;

        return this.#references.get(Inline.label(written));
    }

    /** Read an image: its address, and the words that stand for it. */
    #imageOf(node: SyntaxNode): { readonly source: string; readonly alternative: string } {
        const [open, close] = node.getChildren("LinkMark");
        const alternative =
            open === undefined || close === undefined ? "" : this.#text.slice(open.to, close.from);

        return { source: this.#addressOf(node) ?? "", alternative };
    }
}

/** Read the character an entity stands for, the entity itself where it is unknown. */
function entityOf(written: string): string {
    // read a name, else a number
    const name = written.slice(1, -1);
    if (!name.startsWith("#")) {
        return ENTITIES[name] ?? written;
    }
    const isHexadecimal = /^#x/iu.test(name);
    const point = Number.parseInt(name.slice(isHexadecimal ? 2 : 1), isHexadecimal ? 16 : 10);

    return point > 0 && point <= 0x10ffff ? String.fromCodePoint(point) : "\ufffd";
}
