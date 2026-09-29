import { schema } from "@destack/schema";
import { Element } from "./element.ts";

/** A boundary of an annotation: the side of an element it binds to. */
export interface Anchor {
    /** The element the boundary binds to. */
    readonly element: Element;
    /** The side of the element the boundary sits on. */
    readonly side: "before" | "after";
}

/** A boundary of an annotation: the side of an element it binds to. */
export const Anchor: schema.Schema<Anchor> = schema.object({
    /** The element the boundary binds to. */
    element: Element,
    /** The side of the element the boundary sits on. */
    side: schema.enum(["before", "after"]),
});

/** A selection in a text field. */
export interface Selection {
    /** The text field. */
    readonly field: string;
    /** The boundary the selection started at. */
    readonly anchor: Anchor;
    /** The boundary the selection ends at, the cursor. */
    readonly head: Anchor;
}

/** A selection in a text field. */
export const Selection: schema.Schema<Selection> = schema.object({
    /** The text field. */
    field: schema.string().min(1),
    /** The boundary the selection started at. */
    anchor: Anchor,
    /** The boundary the selection ends at, the cursor. */
    head: Anchor,
});
