import { type JsonValue, schema } from "@destack/schema";
import { TextAnchor } from "./anchor.ts";
import { TextNode } from "./element.ts";

/** A mark's type and value over a range of a text, such as bold, a link or a highlight. */
export type TextMark = {
    /** The mark's identifier. */
    readonly id: string;
    /** The mark's type. */
    readonly type: string;
    /** The mark's value, true for a plain style, null to remove the type over the range. */
    readonly value: JsonValue;
    /** The edge the range starts at. */
    readonly start: TextAnchor;
    /** The edge the range ends at. */
    readonly end: TextAnchor;
    /** The mark's place in the store's order, later winning, null while predicted. */
    readonly order: number | null;
};

/** A stretch of visible text with the marks covering it, or one node. */
export type TextSpan =
    | {
          /** The stretch's characters. */
          readonly text: string;
          /** The values of the marks covering it, by type. */
          readonly marks?: Readonly<Record<string, JsonValue>>;
      }
    | {
          /** The node. */
          readonly node: TextNode;
      };

/** A mark over a range of a text. */
export const TextMark: schema.Schema<TextMark> = schema.object({
    /** The mark's identifier. */
    id: schema.string().min(1),
    /** The mark's type. */
    type: schema.string().min(1),
    /** The mark's value, null to remove the type. */
    value: schema.json(),
    /** The edge the range starts at. */
    start: TextAnchor,
    /** The edge the range ends at. */
    end: TextAnchor,
    /** The mark's place among the text's marks, null while predicted. */
    order: schema.number().int().positive().nullable(),
});

/** A stretch of visible text with the marks covering it, or one node. */
const TextSpanSchema: schema.Schema<TextSpan> = schema.union([
    schema.object({
        /** The stretch's characters. */
        text: schema.string(),
        /** The values of the marks covering it by type. */
        marks: schema.record(schema.string(), schema.json()).exactOptional(),
    }),
    schema.object({
        /** The node. */
        node: TextNode,
    }),
]);

/** Stretches of visible text, and how they read as plain text. */
export const TextSpan = Object.assign(TextSpanSchema, {
    /** Read spans as plain text, with U+FFFC for each node. */
    plain(spans: readonly TextSpan[]): string {
        return spans.map((span) => ("text" in span ? span.text : "\uFFFC")).join("");
    },
});
