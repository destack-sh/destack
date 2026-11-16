import { schema } from "@destack/schema";
import { TextElement, type TextRun, TextNode } from "./element.ts";
import { TextMark } from "./mark.ts";

/** One change to a text: an insertion, a deletion or restoration, a node change, or a mark. */
export type TextEdit =
    | {
          /** Insert a run's characters, or one node, right after an element or at the start. */
          readonly insert: string | TextNode;
          /** The run's identifier. */
          readonly run: string;
          /** The offset of the first inserted element in its run. */
          readonly start: number;
          /** The element the run follows, absent at the start. */
          readonly after?: TextElement;
      }
    | {
          /** Delete the pieces of runs the editor saw, each as the tombstone length it leaves. */
          readonly delete: readonly TextRun[];
      }
    | {
          /** Restore deleted pieces with their characters or nodes. */
          readonly restore: readonly TextRun[];
      }
    | {
          /** Replace the node at an element. */
          readonly set: TextElement;
          /** The node it becomes. */
          readonly node: TextNode;
      }
    | {
          /** Add a mark for the store to order. */
          readonly mark: TextMark;
      }
    | {
          /** Remove marks. */
          readonly unmark: readonly TextMark[];
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

/** A piece of one run restored: its first element and its characters or node. */
const RestoredPiece = schema.union([
    schema.object({
        /** The run the elements were inserted in. */
        run: schema.string().min(1),
        /** The offset of the piece's first element within its run. */
        start: schema.number().int().nonnegative(),
        /** The piece's characters. */
        text: schema.string().min(1),
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

/** One change to a text. */
export const TextEdit: schema.Schema<TextEdit> = schema.union([
    schema.object({
        /** Insert a run's characters, or one node, right after an element or at the start. */
        insert: schema.union([schema.string().min(1), TextNode]),
        /** The run's identifier. */
        run: schema.string().min(1),
        /** The offset the inserted elements start at in their run. */
        start: schema.number().int().nonnegative(),
        /** The element the run follows, absent at the start. */
        after: TextElement.exactOptional(),
    }),
    schema.object({
        /** Delete the pieces of runs the editor saw, each as the tombstone length it leaves. */
        delete: schema.array(DeletedPiece).min(1),
    }),
    schema.object({
        /** Restore deleted pieces with their characters or nodes. */
        restore: schema.array(RestoredPiece).min(1),
    }),
    schema.object({
        /** Replace the node at an element. */
        set: TextElement,
        /** The node it becomes. */
        node: TextNode,
    }),
    schema.object({
        /** Add a mark. */
        mark: TextMark,
    }),
    schema.object({
        /** Remove marks. */
        unmark: schema.array(TextMark).min(1),
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
