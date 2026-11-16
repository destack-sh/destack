import { nth, found } from "@destack/schema";
import type { Anchor } from "./anchor.ts";
import type { Annotation, Span } from "./annotation.ts";
import type { Element, Run } from "./element.ts";
import type { SequenceEdit, TextChange } from "./edit.ts";

/** A text as ordered runs of stable elements, deleted ones as tombstone lengths. */
export class Sequence {
    /** The runs in sequence order. */
    readonly runs: readonly Run[];

    /** Store runs in sequence order. */
    constructor(runs: readonly Run[] = []) {
        this.runs = runs;
    }

    /** Read the visible text. */
    text(): string {
        return this.runs
            .flatMap((piece) => (typeof piece.text === "string" ? [piece.text] : []))
            .join("");
    }

    /** Apply one edit, returning the sequence it makes. */
    apply(edit: SequenceEdit): Sequence {
        // insert a run after its anchor
        if ("insert" in edit) {
            return this.#insert(edit.insert, edit.run, edit.after);
        }
        // set exactly the named pieces to tombstones, or tombstones back to characters
        else if ("delete" in edit) {
            return this.#set(edit.delete);
        } else {
            return this.#set(edit.restore);
        }
    }

    /** Derive the edits undoing one edit applied to a sequence. */
    static inverse(edit: SequenceEdit, before: Sequence): SequenceEdit[] {
        // delete what an insertion added
        if ("insert" in edit) {
            return [{ delete: [{ run: edit.run, start: 0, text: edit.insert.length }] }];
        }

        // restore the named pieces a deletion found visible, or delete those a restoration found deleted
        const parts = before.#covered("delete" in edit ? edit.delete : edit.restore);
        if ("delete" in edit) {
            const visible = parts.filter((part) => typeof part.text === "string");

            return visible.length === 0 ? [] : [{ restore: visible }];
        }
        const deleted = parts.filter((part) => typeof part.text === "number");

        return deleted.length === 0 ? [] : [{ delete: deleted }];
    }

    /** Translate a text change at offsets into edits referring to elements. */
    change(change: TextChange, run: string): SequenceEdit[] {
        // delete the visible pieces between the offsets before inserting
        const pieces = this.#visible(change.from, change.to);
        const deleted: SequenceEdit[] = pieces.length === 0 ? [] : [{ delete: pieces }];
        const after = change.from === 0 ? undefined : this.element(change.from - 1);
        const inserted: SequenceEdit[] =
            change.insert.length > 0
                ? [{ insert: change.insert, run, ...(after === undefined ? {} : { after }) }]
                : [];

        return [...deleted, ...inserted];
    }

    /** Translate an edit into text changes at offsets, each applying after the ones before. */
    translate(edit: SequenceEdit): TextChange[] {
        // insert at the offset after the anchor
        if ("insert" in edit) {
            const from = edit.after === undefined ? 0 : this.#after(edit.after);

            return [{ from, to: from, insert: edit.insert }];
        }

        // walk the text, deleting named visible pieces and inserting named restored ones
        const named = "delete" in edit ? edit.delete : edit.restore;
        const changes: TextChange[] = [];
        let cursor = 0;
        for (const part of this.#cutAt(named).runs) {
            // remove a deleted visible part, insert a restored deleted one, and step over the rest
            const piece = Sequence.#coverer(named, part);
            const length = Sequence.length(part);
            if (piece !== undefined && "delete" in edit && typeof part.text === "string") {
                changes.push({ from: cursor, to: cursor + length, insert: "" });
            } else if (piece !== undefined && "restore" in edit && typeof part.text === "number") {
                changes.push({
                    from: cursor,
                    to: cursor,
                    insert: Sequence.#characters(piece, part),
                });
                cursor += length;
            } else if (typeof part.text === "string") {
                cursor += length;
            }
        }

        return changes;
    }

    /** Split the visible text into stretches each covered by one set of annotations. */
    spans<Value>(annotations: readonly Annotation<Value>[]): Span<Value>[] {
        // order every element, tombstones included
        const order = new Map<string, number>();
        let ordinal = 0;
        for (const piece of this.runs) {
            for (let index = 0; index < Sequence.length(piece); index++) {
                order.set(`${piece.run}:${piece.start + index}`, ordinal);
                ordinal += 1;
            }
        }

        // place each edge
        const place = (anchor: Anchor) => {
            const at = order.get(`${anchor.element.run}:${anchor.element.offset}`);
            if (at === undefined) {
                throw new RangeError(
                    `sequence has no element '${anchor.element.run}' at offset ${anchor.element.offset}`,
                );
            }

            return 2 * at + (anchor.side === "before" ? 0 : 2);
        };
        const ranges = annotations.map((annotation) => ({
            annotation,
            start: place(annotation.start),
            end: place(annotation.end),
        }));

        // group visible characters by covering annotations
        const spans: { text: string; annotations: Annotation<Value>[] }[] = [];
        for (const piece of this.runs) {
            if (typeof piece.text !== "string") {
                continue;
            }
            const text = piece.text;
            for (let index = 0; index < text.length; index++) {
                const position = 2 * found(order, `${piece.run}:${piece.start + index}`) + 1;
                const covering = ranges
                    .filter((range) => range.start < position && position < range.end)
                    .map((range) => range.annotation);
                const last = spans.at(-1);
                const character = text.charAt(index);
                if (last !== undefined && Sequence.#same(last.annotations, covering)) {
                    last.text += character;
                } else {
                    spans.push({ text: character, annotations: covering });
                }
            }
        }

        return spans;
    }

    /** Find the element at an offset of the visible text, absent at the text's end. */
    element(offset: number): Element | undefined {
        // walk the visible runs to the offset
        let remaining = offset;
        for (const piece of this.runs) {
            const length = Sequence.length(piece);
            if (typeof piece.text === "number") {
                continue;
            } else if (remaining < length) {
                return { run: piece.run, offset: piece.start + remaining };
            }
            remaining -= length;
        }

        return undefined;
    }

    /** Find the text offset right after an element. */
    #after(element: Element): number {
        const index = this.#find(element);
        const piece = nth(this.runs, index);

        return this.offset(element) + (typeof piece.text === "number" ? 0 : 1);
    }

    /** Find the text offset of an element. */
    offset(element: Element): number {
        // count the visible characters before the element
        let before = 0;
        for (const piece of this.runs) {
            const length = Sequence.length(piece);
            if (
                piece.run === element.run &&
                element.offset >= piece.start &&
                element.offset < piece.start + length
            ) {
                return before + (typeof piece.text === "number" ? 0 : element.offset - piece.start);
            } else if (typeof piece.text === "string") {
                before += length;
            }
        }
        throw new RangeError(
            `sequence has no element '${element.run}' at offset ${element.offset}`,
        );
    }

    /** Insert a run's characters right after an element, splitting the piece containing it. */
    #insert(text: string, run: string, after: Element | undefined): Sequence {
        // insert at the start
        const inserted: Run = { run, start: 0, text };
        if (after === undefined) {
            return new Sequence([inserted, ...this.runs]);
        }

        // split after the anchor and insert between
        const index = this.#find(after);
        const piece = nth(this.runs, index);
        const cut = after.offset - piece.start + 1;
        const [head, tail] = Sequence.split(piece, cut);

        return new Sequence([
            ...this.runs.slice(0, index),
            head,
            inserted,
            ...(tail === undefined ? [] : [tail]),
            ...this.runs.slice(index + 1),
        ]);
    }

    /** Set the named pieces of runs to tombstones or characters, leaving every other element. */
    #set(named: readonly Run[]): Sequence {
        // delete each named part, and restore each named deleted one with the piece's characters
        const set = this.#cutAt(named).runs.map((part): Run => {
            const piece = Sequence.#coverer(named, part);
            if (piece === undefined) {
                return part;
            } else if (typeof piece.text === "number") {
                return { run: part.run, start: part.start, text: Sequence.length(part) };
            } else if (typeof part.text === "string") {
                return part;
            }

            return { run: part.run, start: part.start, text: Sequence.#characters(piece, part) };
        });

        return new Sequence(Sequence.#merge(set));
    }

    /** Read the parts of the sequence the named pieces cover, in sequence order, as they stand. */
    #covered(named: readonly Run[]): Run[] {
        return this.#cutAt(named).runs.filter(
            (part) => Sequence.#coverer(named, part) !== undefined,
        );
    }

    /** Cut the sequence at the edges of the named pieces, refusing a piece whose end it lacks. */
    #cutAt(named: readonly Run[]): Sequence {
        // cut before each piece's first element and after its last, a run's elements between them
        return named.reduce((cut: Sequence, piece) => {
            const first = { run: piece.run, offset: piece.start };
            const last = { run: piece.run, offset: piece.start + Sequence.length(piece) - 1 };

            return cut.#cut(first, "before").#cut(last, "after");
        }, this);
    }

    /** Read the visible elements between two text offsets as pieces of runs, each as its length. */
    #visible(from: number, to: number): Run[] {
        // walk the visible parts overlapping the offsets
        const pieces: Run[] = [];
        let cursor = 0;
        for (const part of this.runs) {
            if (typeof part.text !== "string") {
                continue;
            }
            const length = part.text.length;
            const start = Math.max(from, cursor);
            const end = Math.min(to, cursor + length);
            if (start < end) {
                pieces.push({
                    run: part.run,
                    start: part.start + start - cursor,
                    text: end - start,
                });
            }
            cursor += length;
        }

        return Sequence.#merge(pieces);
    }

    /** Find the named piece covering a whole part of the sequence, absent for none. */
    static #coverer(named: readonly Run[], part: Run): Run | undefined {
        return named.find(
            (piece) =>
                piece.run === part.run &&
                part.start >= piece.start &&
                part.start < piece.start + Sequence.length(piece),
        );
    }

    /** Read the characters a restored piece gives one of its parts. */
    static #characters(piece: Run, part: Run): string {
        const offset = part.start - piece.start;

        return String(piece.text).slice(offset, offset + Sequence.length(part));
    }

    /** Split the piece with an element at one of its edges. */
    #cut(element: Element, edge: "before" | "after"): Sequence {
        // cut the piece before the element, or after it
        const index = this.#find(element);
        const piece = nth(this.runs, index);
        const count = element.offset - piece.start + (edge === "after" ? 1 : 0);
        const [head, tail] = Sequence.split(piece, count);

        return tail === undefined
            ? this
            : new Sequence([
                  ...this.runs.slice(0, index),
                  head,
                  tail,
                  ...this.runs.slice(index + 1),
              ]);
    }

    /** Find the index of the piece with an element. */
    #find(element: Element): number {
        const index = this.runs.findIndex(
            (piece) =>
                piece.run === element.run &&
                element.offset >= piece.start &&
                element.offset < piece.start + Sequence.length(piece),
        );
        if (index === -1) {
            throw new RangeError(
                `sequence has no element '${element.run}' at offset ${element.offset}`,
            );
        }

        return index;
    }

    /** Split a piece before a count of its elements. */
    static split(piece: Run, cut: number): readonly [Run, Run | undefined] {
        // take the piece whole when the cut is at either end
        const length = Sequence.length(piece);
        if (cut <= 0 || cut >= length) {
            return [piece, undefined];
        }

        // cut the characters, or the length of a tombstone
        const [head, tail] =
            typeof piece.text === "number"
                ? [cut, length - cut]
                : [piece.text.slice(0, cut), piece.text.slice(cut)];

        return [
            { run: piece.run, start: piece.start, text: head },
            { run: piece.run, start: piece.start + cut, text: tail },
        ];
    }

    /** Merge neighbouring pieces continuing one run in the same state. */
    static #merge(runs: readonly Run[]): Run[] {
        const merged: Run[] = [];
        for (const piece of runs) {
            // extend the previous piece when this one continues it
            const previous = merged.at(-1);
            const isContinued =
                previous !== undefined &&
                previous.run === piece.run &&
                previous.start + Sequence.length(previous) === piece.start;
            if (
                isContinued &&
                typeof previous.text === "number" &&
                typeof piece.text === "number"
            ) {
                merged[merged.length - 1] = { ...previous, text: previous.text + piece.text };
            } else if (
                isContinued &&
                typeof previous.text === "string" &&
                typeof piece.text === "string"
            ) {
                merged[merged.length - 1] = { ...previous, text: previous.text + piece.text };
            } else {
                merged.push(piece);
            }
        }

        return merged;
    }

    /** Decide whether two lists refer to the same annotations in the same order. */
    static #same(left: readonly Annotation[], right: readonly Annotation[]): boolean {
        return (
            left.length === right.length &&
            left.every((annotation, index) => annotation === right[index])
        );
    }

    /** Count a piece's elements. */
    static length(piece: Run): number {
        return typeof piece.text === "number" ? piece.text : piece.text.length;
    }
}
