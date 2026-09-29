import type { Anchor } from "./anchor.ts";

/** A kind and value over a range of a sequence. */
export interface Annotation<Value = unknown> {
    /** The annotation's identifier. */
    readonly id: string;
    /** What the annotation means, such as `bold`, `link` or `comment`. */
    readonly kind: string;
    /** The annotation's value, such as a link's target. */
    readonly value: Value;
    /** The boundary the range starts at. */
    readonly start: Anchor;
    /** The boundary the range ends at. */
    readonly end: Anchor;
}

/** A stretch of visible text and the annotations covering all of it. */
export interface Span<Value = unknown> {
    /** The stretch's characters. */
    readonly text: string;
    /** The annotations covering the stretch. */
    readonly annotations: readonly Annotation<Value>[];
}
