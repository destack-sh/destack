import { schema } from "@destack/schema";
import { Element, type Run } from "./element.ts";

/** One change to a sequence: an insertion, or the deletion or restoration of exact pieces. */
export type SequenceEdit =
    | {
          /** Insert a new run's characters right after an element, or at the start. */
          readonly insert: string;
          /** The new run's identifier. */
          readonly run: string;
          /** The element the run follows, absent at the start. */
          readonly after?: Element;
      }
    | {
          /** Delete the pieces of runs the editor saw, each as the tombstone length it leaves. */
          readonly delete: readonly Run[];
      }
    | {
          /** Restore deleted pieces of runs, each with its characters. */
          readonly restore: readonly Run[];
      };

/** A piece of one run deleted: its first element and its length. */
const DeletedPiece = schema.object({
    /** The run the elements were inserted in. */
    run: schema.string().min(1),
    /** The offset of the piece's first element within its run. */
    start: schema.number().int().nonnegative(),
    /** The piece's length. */
    text: schema.number().int().positive(),
});

/** A piece of one run restored: its first element and its characters. */
const RestoredPiece = schema.object({
    /** The run the elements were inserted in. */
    run: schema.string().min(1),
    /** The offset of the piece's first element within its run. */
    start: schema.number().int().nonnegative(),
    /** The piece's characters. */
    text: schema.string().min(1),
});

/** One change to a sequence. */
export const SequenceEdit: schema.Schema<SequenceEdit> = schema.union([
    schema.object({
        /** Insert a new run's characters right after an element, or at the start. */
        insert: schema.string().min(1),
        /** The new run's identifier. */
        run: schema.string().min(1),
        /** The element the run follows, absent at the start. */
        after: Element.exactOptional(),
    }),
    schema.object({
        /** Delete the pieces of runs the editor saw, each as the tombstone length it leaves. */
        delete: schema.array(DeletedPiece).min(1),
    }),
    schema.object({
        /** Restore deleted pieces of runs, each with its characters. */
        restore: schema.array(RestoredPiece).min(1),
    }),
]);

/** A replacement of visible text between two offsets. */
export interface TextChange {
    /** The offset the change starts at. */
    readonly from: number;
    /** The offset the replaced characters end before. */
    readonly to: number;
    /** The characters inserted at the start. */
    readonly insert: string;
}

/** Replacements of visible text. */
export const TextChange = {
    /** Find the one replacement turning a text into another outside their common ends. */
    between(before: string, after: string): TextChange {
        // skip the common start
        let start = 0;
        const shorter = Math.min(before.length, after.length);
        while (start < shorter && before[start] === after[start]) {
            start += 1;
        }

        // skip the common end after the start
        let end = 0;
        while (
            end < shorter - start &&
            before[before.length - 1 - end] === after[after.length - 1 - end]
        ) {
            end += 1;
        }

        return {
            from: start,
            to: before.length - end,
            insert: after.slice(start, after.length - end),
        };
    },
};
