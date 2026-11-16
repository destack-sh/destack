import { type JsonValue, schema } from "@destack/schema";

/** A stable element of a sequence: a run and an offset within it. */
export type TextElement = {
    /** The run the element was inserted in, the inserting party's site. */
    readonly run: string;
    /** The element's offset within its run, the site's clock when it inserted the element. */
    readonly offset: number;
};

/** A value in place of a character, such as a heading marker or a mention. */
export type TextNode = {
    /** The node's type, a block or an inline node. */
    readonly type: string;
    /** The node's value, such as a heading's level or a mentioned subject. */
    readonly value: JsonValue;
};

/** Consecutive elements of one run in sequence order: characters, a tombstone length or a node. */
export type TextRun =
    | {
          /** The run the elements were inserted in. */
          readonly run: string;
          /** The offset of the piece's first element within its run. */
          readonly start: number;
          /** The piece's characters, or its length once deleted. */
          readonly text: string | number;
      }
    | {
          /** The run the node was inserted in. */
          readonly run: string;
          /** The node's offset within its run. */
          readonly start: number;
          /** The node, one element long. */
          readonly node: TextNode;
      };

/** A stable element of a sequence. */
export const TextElement: schema.Schema<TextElement> = schema.object({
    /** The run the element was inserted in. */
    run: schema.string().min(1),
    /** The element's offset within its run. */
    offset: schema.number().int().nonnegative(),
});

/** A value in place of a character. */
export const TextNode: schema.Schema<TextNode> = schema.object({
    /** The node's type. */
    type: schema.string().min(1),
    /** The node's value. */
    value: schema.json(),
});

/** Consecutive elements of one run in sequence order. */
export const TextRun: schema.Schema<TextRun> = schema.union([
    schema.object({
        /** The run the elements were inserted in. */
        run: schema.string().min(1),
        /** The offset of the piece's first element within its run. */
        start: schema.number().int().nonnegative(),
        /** The piece's characters, or its length once deleted. */
        text: schema.union([schema.string().min(1), schema.number().int().positive()]),
    }),
    schema.object({
        /** The run the node was inserted in. */
        run: schema.string().min(1),
        /** The node's offset within its run. */
        start: schema.number().int().nonnegative(),
        /** The node. */
        node: TextNode,
    }),
]);
