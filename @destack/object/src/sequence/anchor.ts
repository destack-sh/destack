import { schema } from "@destack/schema";
import { Element } from "./element.ts";

/** An edge of an annotation: the side of an element it binds to. */
export type Anchor = {
    /** The element the edge binds to. */
    readonly element: Element;
    /** The side of the element the edge sits on. */
    readonly side: "before" | "after";
};

/** An edge of an annotation: the side of an element it binds to. */
export const Anchor: schema.Schema<Anchor> = schema.object({
    /** The element the edge binds to. */
    element: Element,
    /** The side of the element the edge sits on. */
    side: schema.enum(["before", "after"]),
});

/** A selection in a text field. */
export type Selection = {
    /** The text field. */
    readonly field: string;
    /** The edge the selection started at. */
    readonly anchor: Anchor;
    /** The edge the selection ends at, the cursor. */
    readonly head: Anchor;
};

/** A selection in a text field. */
export const Selection: schema.Schema<Selection> = schema.object({
    /** The text field. */
    field: schema.string().min(1),
    /** The edge the selection started at. */
    anchor: Anchor,
    /** The edge the selection ends at, the cursor. */
    head: Anchor,
});
