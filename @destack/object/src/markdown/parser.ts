import { type SyntaxNode, type Tree, TreeFragment } from "@lezer/common";
import { GFM, parser } from "@lezer/markdown";
import { canonicalize, nth } from "@destack/schema";
import { Block } from "../sequence/block.ts";
import { closed } from "./close.ts";
import { Inline } from "./inline.ts";

/** The parser of Markdown with GitHub's tables, task lists, strikethrough and addresses. */
const PARSER = parser.configure([GFM]);

/** Markdown read as it is written, finished blocks staying the ones they are. */
export class MarkdownParser {
    /** The text so far. */
    #text = "";
    /** The parts of the last tree the next one takes over. */
    #fragments: readonly TreeFragment[] = [];
    /** The parts at the root, such as a list, with the blocks of each. */
    #sections: readonly Section[] = [];
    /** The blocks read last. */
    #blocks: readonly Block[] = [];
    /** The addresses the text defines for its links, by their labels. */
    #references: ReadonlyMap<string, string> = new Map();

    /** Read the text so far. */
    get text(): string {
        return this.#text;
    }

    /** Add text to the end. */
    append(text: string): void {
        // parse on from what the text does not change
        const from = this.#text.length;
        this.#text += text;
        const change = { fromA: from, toA: from, fromB: from, toB: this.#text.length };
        const kept = TreeFragment.applyChanges(this.#fragments, [change]);
        const tree = PARSER.parse(this.#text, kept);
        this.#fragments = TreeFragment.addTree(tree, kept);

        // read the sections anew that the text may change: the last, and all once an address is defined
        const references = referencesOf(tree, this.#text);
        const isSame = canonicalize([...references]) === canonicalize([...this.#references]);
        this.#references = references;
        const finished = new Map(
            isSame ? this.#sections.slice(0, -1).map((section) => [section.from, section]) : [],
        );
        const inline = new Inline(this.#text, references);
        this.#sections = sectionsOf(tree, inline, finished);
    }

    /** Read the blocks, the last ones with their unfinished marks closed while they are written. */
    blocks(isWriting: boolean): readonly Block[] {
        // read every section as it is
        const last = this.#sections.at(-1);
        if (!isWriting || last === undefined) {
            this.#blocks = Block.kept(
                this.#sections.flatMap(({ blocks }) => blocks),
                this.#blocks,
            );

            return this.#blocks;
        }

        // else the last from its closed text, its places moved back into the whole
        const text = closed(this.#text.slice(last.from));
        const inline = new Inline(text, this.#references);
        const reread = sectionsOf(PARSER.parse(text), inline, new Map(), last.from);
        this.#blocks = Block.kept(
            [...this.#sections.slice(0, -1), ...reread].flatMap(({ blocks }) => blocks),
            this.#blocks,
        );

        return this.#blocks;
    }
}

/** A part at the root of some Markdown, such as a list. */
interface Section {
    /** Where it starts. */
    readonly from: number;
    /** Where it ends. */
    readonly to: number;
    /** Its blocks. */
    readonly blocks: readonly Block[];
}

/** Read the sections of a tree, those finished before as they were. */
function sectionsOf(
    tree: Tree,
    inline: Inline,
    finished: ReadonlyMap<number, Section>,
    offset = 0,
): Section[] {
    const sections: Section[] = [];
    for (let node = tree.topNode.firstChild; node !== null; node = node.nextSibling) {
        const { from, to } = node;
        const before = finished.get(from);
        sections.push(
            before?.to === to ? before : { from, to, blocks: blocksOf(node, inline, offset) },
        );
    }

    return sections;
}

/** Read the addresses a text defines for its links. */
function referencesOf(tree: Tree, text: string): Map<string, string> {
    const references = new Map<string, string>();
    for (let node = tree.topNode.firstChild; node !== null; node = node.nextSibling) {
        const label = node.name === "LinkReference" ? node.getChild("LinkLabel") : null;
        const url = label === null ? null : node.getChild("URL");
        if (label !== null && url !== null) {
            references.set(
                Inline.label(text.slice(label.from, label.to)),
                text.slice(url.from, url.to),
            );
        }
    }

    return references;
}

/** Read the blocks of a node, none for one that shows nothing. */
function blocksOf(node: SyntaxNode, inline: Inline, offset: number): Block[] {
    // read a heading by its level
    const key = String(node.from + offset);
    const block = (type: string, value: Block["value"], rest: Partial<Block> = {}): Block[] => [
        { key, type, value, spans: [], blocks: [], ...rest },
    ];
    const heading = /^(?:ATX|Setext)Heading([1-6])$/u.exec(node.name);
    if (heading !== null) {
        const level = Number(nth(heading, 1));

        return block("heading", { level }, { spans: inline.spans(node) });
    }

    // read each other type, none for what shows nothing
    switch (node.name) {
        case "Paragraph":
        case "HTMLBlock": {
            const image = inline.image(node);

            return image === undefined
                ? block("paragraph", null, { spans: inline.spans(node) })
                : block("image", image);
        }
        case "HorizontalRule":
            return block("divider", null);
        case "FencedCode":
        case "CodeBlock":
            return block("codeBlock", inline.language(node), { spans: inline.code(node) });
        case "Blockquote":
            return block("quote", null, { blocks: childrenOf(node, inline, offset) });
        case "BulletList":
        case "OrderedList":
            return itemsOf(node, inline, offset);
        case "Table":
            return block("table", inline.alignments(node), {
                blocks: rowsOf(node, inline, offset),
            });
        default:
            return [];
    }
}

/** Read the blocks in a node. */
function childrenOf(node: SyntaxNode, inline: Inline, offset: number): Block[] {
    const blocks: Block[] = [];
    for (let child = node.firstChild; child !== null; child = child.nextSibling) {
        blocks.push(...blocksOf(child, inline, offset));
    }

    return blocks;
}

/** Read the items of a list, each with its first line's text and the blocks after it. */
function itemsOf(list: SyntaxNode, inline: Inline, offset: number): Block[] {
    const isNumbered = list.name === "OrderedList";
    const start = isNumbered ? inline.start(list) : 1;

    return list.getChildren("ListItem").map((item): Block => {
        // take the text from a paragraph or task beside the mark
        const first = leading(item);
        const isTask = first?.name === "Task";
        const isText = first !== null && (isTask || first.name === "Paragraph");
        const blocks: Block[] = [];
        for (let child = item.firstChild; child !== null; child = child.nextSibling) {
            if (!(isText && child.from === first.from)) {
                blocks.push(...blocksOf(child, inline, offset));
            }
        }
        const spans = isText ? inline.spans(first) : [];
        const style = isNumbered ? "number" : "bullet";

        return {
            key: String(item.from + offset),
            type: isTask ? "todo" : "list",
            value: isTask
                ? { isChecked: inline.isChecked(first) }
                : { style, ...(start === 1 ? {} : { start }) },
            spans,
            blocks,
        };
    });
}

/** Find the first node of an item beside its mark, null for an empty one. */
function leading(item: SyntaxNode): SyntaxNode | null {
    let child = item.firstChild;
    while (child !== null && child.name === "ListMark") {
        child = child.nextSibling;
    }

    return child;
}

/** Read the rows of a table, the header first, each with a cell in every column. */
function rowsOf(table: SyntaxNode, inline: Inline, offset: number): Block[] {
    const rows: Block[] = [];
    for (let row = table.firstChild; row !== null; row = row.nextSibling) {
        if (row.name === "TableHeader" || row.name === "TableRow") {
            const blocks = cellsOf(row, inline, offset);
            const columns = rows[0]?.blocks.length ?? blocks.length;
            while (blocks.length < columns) {
                blocks.push(cellOf(`${row.from + offset}:${blocks.length}`));
            }
            rows.push({
                key: String(row.from + offset),
                type: "row",
                value: { isHeader: row.name === "TableHeader" },
                spans: [],
                blocks: blocks.slice(0, columns),
            });
        }
    }

    return rows;
}

/** Read the cells of a row in their columns, an empty one where it has none. */
function cellsOf(row: SyntaxNode, inline: Inline, offset: number): Block[] {
    // start a column at each mark but one before every cell
    const cells: Block[] = [];
    let isOpen = false;
    for (let child = row.firstChild; child !== null; child = child.nextSibling) {
        const key = `${row.from + offset}:${cells.length}`;
        if (child.name === "TableCell") {
            cells.push(cellOf(key, inline.spans(child)));
            isOpen = false;
        } else if (child.name === "TableDelimiter") {
            if (isOpen) {
                cells.push(cellOf(key));
            }
            isOpen = true;
        }
    }

    return cells;
}

/** Write a table's cell. */
function cellOf(key: string, spans: Block["spans"] = []): Block {
    return { key, type: "cell", value: null, spans, blocks: [] };
}
