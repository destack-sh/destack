import { canonicalize, type JsonValue, schema } from "@destack/schema";
import { TextSpan } from "./mark.ts";

/** A block of a text, such as a heading, with the blocks in it. */
export type Block = {
    /** What tells it from its siblings as the text changes: an element, an object or a place. */
    readonly key: string;
    /** The type, such as heading. */
    readonly type: string;
    /** The type's value, such as a heading's level. */
    readonly value: JsonValue;
    /** The text, with its marks and inline nodes. */
    readonly spans: readonly TextSpan[];
    /** The blocks in it, such as a list item's. */
    readonly blocks: readonly Block[];
};

/** A block of a text, such as a heading, with the blocks in it. */
const BlockSchema: schema.Schema<Block> = schema.lazy(() =>
    schema.object({
        /** What tells it from its siblings. */
        key: schema.string(),
        /** The type. */
        type: schema.string().min(1),
        /** The type's value. */
        value: schema.json(),
        /** The text. */
        spans: schema.array(TextSpan),
        /** The blocks in it. */
        blocks: schema.array(BlockSchema),
    }),
);

/** How deep a block's value indents it, none for a value without an indent. */
const Indented = schema.looseObject({ indent: schema.number().int().nonnegative() });

/** Blocks of a text, and how they read. */
export const Block = Object.assign(BlockSchema, {
    /** Nest blocks in those before them that their values indent less, leaving out the indent. */
    nested(blocks: readonly Block[]): Block[] {
        // keep the open blocks by their indents, the innermost last
        const roots: Block[] = [];
        const open: { readonly indent: number; readonly into: Block[] }[] = [];
        for (const block of blocks) {
            // place a block without an indent at the root
            const indented = Indented.safeParse(block.value).data;
            if (indented === undefined) {
                open.length = 0;
                roots.push(block);
                continue;
            }

            // place an indented one in the nearest that is indented less
            const { indent, ...value } = indented;
            while ((open.at(-1)?.indent ?? -1) >= indent) {
                open.pop();
            }
            const into: Block[] = [...block.blocks];
            const nested = { ...block, value: schema.jsonObject().parse(value), blocks: into };
            (open.at(-1)?.into ?? roots).push(nested);
            open.push({ indent, into });
        }

        return roots;
    },

    /** Keep the blocks of before that are the same, so what draws them leaves them. */
    kept(blocks: readonly Block[], before: readonly Block[]): Block[] {
        // find each by its key
        const byKey = new Map(before.map((block) => [block.key, block]));

        return blocks.map((block) => {
            // take a same block as it was, else one with the same spans and blocks in it
            const last = byKey.get(block.key);
            if (last === undefined || last === block) {
                return block;
            }
            const inner = Block.kept(block.blocks, last.blocks);
            const spans = block.spans.map((span, index) => {
                const known = last.spans[index];

                return known !== undefined && canonicalize(known) === canonicalize(span)
                    ? known
                    : span;
            });
            const isSame =
                block.type === last.type &&
                canonicalize(block.value) === canonicalize(last.value) &&
                spans.length === last.spans.length &&
                spans.every((span, index) => span === last.spans[index]) &&
                inner.length === last.blocks.length &&
                inner.every((each, index) => each === last.blocks[index]);

            return isSame ? last : { ...block, spans, blocks: inner };
        });
    },

    /** Read blocks as plain text, a line for each, a table's cells apart by tabs. */
    plain(blocks: readonly Block[]): string {
        return blocks.map((block) => lineOf(block)).join("");
    },

    /** Cut blocks where their plain text has a length, whole ones staying as they are. */
    cut(blocks: readonly Block[], length: number): readonly Block[] {
        return cutBlocks(blocks, { left: length });
    },
});

/** Write a block's plain text, the blocks in it under its own. */
function lineOf(block: Block): string {
    const own = TextSpan.plain(block.spans);
    const gap = own !== "" && block.blocks.length > 0 ? "\n" : "";

    return own + gap + Block.plain(block.blocks) + (block.type === "cell" ? "\t" : "\n");
}

/** Cut blocks, taking off the characters left what they have. */
function cutBlocks(blocks: readonly Block[], rest: { left: number }): readonly Block[] {
    const kept: Block[] = [];
    for (const block of blocks) {
        // stop at the end, and keep a block that fits as it is
        if (rest.left <= 0) {
            break;
        }
        const { length } = lineOf(block);
        if (length <= rest.left) {
            rest.left -= length;
            kept.push(block);
        }
        // else cut its text, and the blocks in it
        else {
            const spans = cutSpans(block.spans, rest);
            const gap = spans.length > 0 && block.blocks.length > 0 ? 1 : 0;
            rest.left -= gap;
            kept.push({ ...block, spans, blocks: cutBlocks(block.blocks, rest) });
            rest.left = 0;
        }
    }

    return kept;
}

/** Cut spans, taking off the characters left what they have. */
function cutSpans(spans: readonly TextSpan[], rest: { left: number }): readonly TextSpan[] {
    const kept: TextSpan[] = [];
    for (const span of spans) {
        if (rest.left <= 0) {
            break;
        }
        const { length } = TextSpan.plain([span]);
        kept.push(
            "text" in span && length > rest.left
                ? { ...span, text: span.text.slice(0, rest.left) }
                : span,
        );
        rest.left -= length;
    }

    return kept;
}
