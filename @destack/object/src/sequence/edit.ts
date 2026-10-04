import { schema } from "@destack/schema";
import { Element } from "./element.ts";

/** One change to a sequence: an insertion, deletion or restoration. */
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
          /** Delete the elements from one element through another, in sequence order. */
          readonly delete: { readonly from: Element; readonly to: Element };
      }
    | {
          /** Restore the deleted elements from one element through another with their characters. */
          readonly restore: {
              readonly from: Element;
              readonly to: Element;
              readonly text: string;
          };
      };

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
        /** Delete the elements from one element through another, in sequence order. */
        delete: schema.object({ from: Element, to: Element }),
    }),
    schema.object({
        /** Restore the deleted elements from one element through another with their characters. */
        restore: schema.object({ from: Element, to: Element, text: schema.string().min(1) }),
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
    /** Find the one replacement that turns a text into another outside their common start and end. */
    between(before: string, after: string): TextChange {
        // keep the common start
        let start = 0;
        const shorter = Math.min(before.length, after.length);
        while (start < shorter && before[start] === after[start]) {
            start += 1;
        }

        // keep the common end after the start
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
