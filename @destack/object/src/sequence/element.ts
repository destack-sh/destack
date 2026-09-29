import { schema } from "@destack/schema";

/** A stable element of a sequence: a run and an offset within it. */
export interface Element {
    /** The run the element was inserted in, `site.counter` of the inserting party. */
    readonly run: string;
    /** The element's offset within its run. */
    readonly offset: number;
}

/** Consecutive elements of one run in sequence order. */
export interface Run {
    /** The run the elements were inserted in. */
    readonly run: string;
    /** The offset of the piece's first element within its run. */
    readonly start: number;
    /** The piece's characters, or its length once deleted. */
    readonly text: string | number;
}

/** A stable element of a sequence. */
export const Element: schema.Schema<Element> = schema.object({
    /** The run the element was inserted in. */
    run: schema.string().min(1),
    /** The element's offset within its run. */
    offset: schema.number().int().nonnegative(),
});

/** Consecutive elements of one run in sequence order. */
export const Run: schema.Schema<Run> = schema.object({
    /** The run the elements were inserted in. */
    run: schema.string().min(1),
    /** The offset of the piece's first element within its run. */
    start: schema.number().int().nonnegative(),
    /** The piece's characters, or its length once deleted. */
    text: schema.union([schema.string().min(1), schema.number().int().positive()]),
});
