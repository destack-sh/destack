import type { LineRange } from "../source/index.ts";

/** Whether a line is in both texts, or in the new or the old one alone. */
export type LineKind = "same" | "added" | "removed";

/** A line of a hunk. */
export interface HunkLine {
    /** Which texts have the line. */
    readonly kind: LineKind;
    /** The line without its line break. */
    readonly text: string;
    /** Whether a line break follows it, as after every line but a text's last. */
    readonly hasLineBreak: boolean;
}

/** Changed lines with some unchanged ones around them. */
export interface Hunk {
    /** The lines of the old text. */
    readonly before: LineRange;
    /** The lines of the new text. */
    readonly after: LineRange;
    /** The line of code the hunk is inside, absent when not known. */
    readonly heading?: string;
    /** The lines, removed ones before the added ones in their place. */
    readonly lines: readonly HunkLine[];
}
