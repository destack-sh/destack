import { schema } from "@destack/schema";
import { TextElement } from "./element.ts";

/** An edge of a mark or selection: a side of an element, or the start or end of the whole text. */
export type TextAnchor =
    | {
          /** The element the edge binds to. */
          readonly element: TextElement;
          /** The side of the element the edge sits on. */
          readonly side: "before" | "after";
      }
    | {
          /** The text's start or end. */
          readonly edge: "start" | "end";
      };

/** An edge of a mark or selection. */
export const TextAnchor: schema.Schema<TextAnchor> = schema.union([
    schema.object({
        /** The element the edge binds to. */
        element: TextElement,
        /** The side of the element the edge sits on. */
        side: schema.enum(["before", "after"]),
    }),
    schema.object({
        /** The text's start or end. */
        edge: schema.enum(["start", "end"]),
    }),
]);

/** A selection in a text field. */
export type TextSelection = {
    /** The text field. */
    readonly field: string;
    /** The edge the selection started at. */
    readonly anchor: TextAnchor;
    /** The edge the selection ends at, the cursor. */
    readonly head: TextAnchor;
};

/** A selection in a text field. */
export const TextSelection: schema.Schema<TextSelection> = schema.object({
    /** The text field. */
    field: schema.string().min(1),
    /** The edge the selection started at. */
    anchor: TextAnchor,
    /** The edge the selection ends at, the cursor. */
    head: TextAnchor,
});
