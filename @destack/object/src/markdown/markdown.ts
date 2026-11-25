import { schema } from "@destack/schema";
import type { Block } from "../sequence/block.ts";
import type { TextSpan } from "../sequence/index.ts";
import { MarkdownParser } from "./parser.ts";
import { addressOf, Writer } from "./writer.ts";

/** The value of a heading. */
const Heading = schema.looseObject({ level: schema.number() });

/** The value of a list's item. */
const Item = schema.looseObject({ style: schema.string(), start: schema.number().exactOptional() });

/** The value of a todo. */
const Todo = schema.looseObject({ isChecked: schema.boolean() });

/** The value of a block of code. */
const Code = schema.looseObject({ language: schema.string().exactOptional() });

/** The value of an image. */
const Image = schema.looseObject({ source: schema.string(), alternative: schema.string() });

/** The value of a table. */
const Table = schema.looseObject({ alignments: schema.array(schema.string()) });

/** Text as Markdown writes it, with GitHub's tables, task lists and strikethrough. */
export const Markdown = {
    /** Read the blocks of Markdown. */
    parse(text: string): readonly Block[] {
        const parser = new MarkdownParser();
        parser.append(text);

        return parser.blocks(false);
    },

    /** Write blocks as Markdown, what it has no writing for as its text. */
    stringify(blocks: readonly Block[]): string {
        return linesOf(blocks).join("\n");
    },
};

/** Write blocks as lines, with an empty one between but for a list's items. */
function linesOf(blocks: readonly Block[]): string[] {
    let number = 0;

    return blocks.flatMap((block, index) => {
        // count on from the number a list starts at
        const before = blocks[index - 1];
        const isFollowing = before !== undefined && isItem(block) && isSameList(block, before);
        const start = Item.safeParse(block.value).data?.start ?? 1;
        number = isFollowing ? number + 1 : start;

        return [...(index === 0 || isFollowing ? [] : [""]), ...blockLines(block, number)];
    });
}

/** Write a block as lines. */
function blockLines(block: Block, number: number): string[] {
    const text = Writer.write(block.spans);

    // write each type as Markdown does, and any other as a paragraph
    switch (block.type) {
        case "heading":
            return [
                `${"#".repeat(Heading.safeParse(block.value).data?.level ?? 1)} ${text.replace(/\n/gu, " ")}`,
            ];
        case "list": {
            const isNumbered = Item.safeParse(block.value).data?.style === "number";

            return itemLines(isNumbered ? `${number}. ` : "- ", text, block.blocks);
        }
        case "todo": {
            const isChecked = Todo.safeParse(block.value).data?.isChecked === true;

            return itemLines(`- [${isChecked ? "x" : " "}] `, text, block.blocks, 2);
        }
        case "quote": {
            const own = text === "" ? [] : text.split("\n");
            const inner = linesOf(block.blocks);
            const gap = own.length > 0 && inner.length > 0 ? [""] : [];

            return [...own, ...gap, ...inner].map((line) => (line === "" ? ">" : `> ${line}`));
        }
        case "codeBlock": {
            const code = plainOf(block.spans);
            const longest = Math.max(
                2,
                ...Array.from(code.matchAll(/`+/gu), ([run]) => run.length),
            );
            const fence = "`".repeat(longest + 1);

            return [
                `${fence}${Code.safeParse(block.value).data?.language ?? ""}`,
                ...code.split("\n"),
                fence,
            ];
        }
        case "divider":
            return ["---"];
        case "image": {
            const image = Image.safeParse(block.value).data;

            return [`![${image?.alternative ?? ""}](${addressOf(image?.source ?? "")})`];
        }
        case "table":
            return tableLines(block);
        default:
            return [
                ...text
                    .split("\n")
                    .map((line, index, lines) => line + (index < lines.length - 1 ? "\\" : "")),
                ...(block.blocks.length === 0 ? [] : ["", ...linesOf(block.blocks)]),
            ];
    }
}

/** Write an item as lines, those after the first under its text. */
function itemLines(
    mark: string,
    text: string,
    blocks: readonly Block[],
    indent: number = mark.length,
): string[] {
    // write the blocks in it after an empty line, but a list right below
    const isList = blocks[0] !== undefined && isItem(blocks[0]);
    const inner = blocks.length === 0 ? [] : [...(isList ? [] : [""]), ...linesOf(blocks)];
    const [first = "", ...rest] = text.split("\n");

    return [
        mark + first,
        ...[...rest, ...inner].map((line) => (line === "" ? "" : " ".repeat(indent) + line)),
    ];
}

/** Write a table as lines: its header, how its columns align, and its rows. */
function tableLines(table: Block): string[] {
    // write each row, and how the columns align after the first
    const alignments = Table.safeParse(table.value).data?.alignments ?? [];
    const rows = table.blocks.map(
        ({ blocks }) =>
            `| ${blocks.map(({ spans }) => Writer.write(spans).replace(/\|/gu, "\\|").replace(/\n/gu, "<br>")).join(" | ")} |`,
    );
    const columns = table.blocks[0]?.blocks.length ?? 0;
    const delimiters = Array.from({ length: columns }, (_, column) => {
        const alignment = alignments[column];

        return alignment === "center" ? ":-:" : alignment === "end" ? "--:" : "---";
    });
    const [header, ...body] = rows;

    return header === undefined ? [] : [header, `| ${delimiters.join(" | ")} |`, ...body];
}

/** Read the text of spans alone. */
function plainOf(spans: readonly TextSpan[]): string {
    return spans.map((span) => ("text" in span ? span.text : "")).join("");
}

/** Report whether a block is an item of a list. */
function isItem({ type }: Block): boolean {
    return type === "list" || type === "todo";
}

/** Report whether two items stand in one list. */
function isSameList(one: Block, other: Block): boolean {
    return isItem(one) && isItem(other) && styleOf(one) === styleOf(other);
}

/** Read the style of an item, a bullet's for a todo. */
function styleOf({ value }: Block): string {
    return Item.safeParse(value).data?.style ?? "bullet";
}
