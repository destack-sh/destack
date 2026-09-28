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
        after: Element.optional(),
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

/** A text as ordered runs of stable elements, deleted ones kept as tombstone lengths. */
export class Sequence {
    /** The runs in sequence order. */
    readonly runs: readonly Run[];

    /** Hold runs in sequence order. */
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
        // replace a range's characters by tombstones, or tombstones by characters
        else if ("delete" in edit) {
            return this.#mark(edit.delete.from, edit.delete.to, undefined);
        } else {
            return this.#mark(edit.restore.from, edit.restore.to, edit.restore.text);
        }
    }

    /** Derive the edits undoing one edit applied to a sequence. */
    static inverse(edit: SequenceEdit, before: Sequence): SequenceEdit[] {
        // delete what an insertion added
        if ("insert" in edit) {
            const last = { run: edit.run, offset: edit.insert.length - 1 };

            return [{ delete: { from: { run: edit.run, offset: 0 }, to: last } }];
        }

        // invert each piece the edit changed
        const range = "delete" in edit ? edit.delete : edit.restore;
        const cut = before.#cut(range.from, "before").#cut(range.to, "after");
        const pieces = cut.runs.slice(cut.#find(range.from), cut.#find(range.to) + 1);

        return pieces.flatMap((piece): SequenceEdit[] => {
            // restore deleted characters, or delete restored ones
            const from = { run: piece.run, offset: piece.start };
            const to = { run: piece.run, offset: piece.start + Sequence.length(piece) - 1 };
            if ("delete" in edit) {
                return typeof piece.text === "string"
                    ? [{ restore: { from, to, text: piece.text } }]
                    : [];
            }

            return typeof piece.text === "number" ? [{ delete: { from, to } }] : [];
        });
    }

    /** Translate a text change at offsets into edits naming elements. */
    change(change: TextChange, run: string): SequenceEdit[] {
        // delete, then insert
        const deleted: SequenceEdit[] =
            change.to > change.from
                ? [
                      {
                          delete: {
                              from: this.element(change.from)!,
                              to: this.element(change.to - 1)!,
                          },
                      },
                  ]
                : [];
        const after = change.from === 0 ? undefined : this.element(change.from - 1);
        const inserted: SequenceEdit[] =
            change.insert.length > 0
                ? [{ insert: change.insert, run, ...(after === undefined ? {} : { after }) }]
                : [];

        return [...deleted, ...inserted];
    }

    /** Translate an edit into text changes at offsets. */
    changesOf(edit: SequenceEdit): TextChange[] {
        // insert at the offset after the anchor
        if ("insert" in edit) {
            const from = edit.after === undefined ? 0 : this.#after(edit.after);

            return [{ from, to: from, insert: edit.insert }];
        }

        // walk the range's pieces
        const range = "delete" in edit ? edit.delete : edit.restore;
        const cut = this.#cut(range.from, "before").#cut(range.to, "after");
        const pieces = cut.runs.slice(cut.#find(range.from), cut.#find(range.to) + 1);
        const from = this.offset(range.from);
        const changes: TextChange[] = [];
        let cursor = from;
        let restored = "restore" in edit ? edit.restore.text : "";
        for (const piece of pieces) {
            // insert restored characters at each tombstone, and step over visible ones
            const length = Sequence.length(piece);
            if ("restore" in edit && typeof piece.text === "number") {
                changes.push({ from: cursor, to: cursor, insert: restored.slice(0, length) });
                restored = restored.slice(length);
                cursor += length;
            } else if (typeof piece.text === "string") {
                cursor += length;
            }
        }

        return "delete" in edit
            ? cursor > from
                ? [{ from, to: cursor, insert: "" }]
                : []
            : changes;
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

        // place each boundary
        const place = (anchor: Anchor) => {
            const at = order.get(`${anchor.element.run}:${anchor.element.offset}`);
            if (at === undefined) {
                throw new RangeError(
                    `sequence holds no element ${anchor.element.run}:${anchor.element.offset}`,
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
                const position = 2 * order.get(`${piece.run}:${piece.start + index}`)! + 1;
                const covering = ranges
                    .filter((range) => range.start < position && position < range.end)
                    .map((range) => range.annotation);
                const last = spans.at(-1);
                if (last !== undefined && Sequence.#same(last.annotations, covering)) {
                    last.text += text[index];
                } else {
                    spans.push({ text: text[index]!, annotations: covering });
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
        const piece = this.runs[index]!;

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
        throw new RangeError(`sequence holds no element ${element.run}:${element.offset}`);
    }

    /** Insert a run's characters right after an element, splitting the piece holding it. */
    #insert(text: string, run: string, after: Element | undefined): Sequence {
        // insert at the start
        const inserted: Run = { run, start: 0, text };
        if (after === undefined) {
            return new Sequence([inserted, ...this.runs]);
        }

        // split after the anchor and insert between
        const index = this.#find(after);
        const piece = this.runs[index]!;
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

    /** Delete a range of elements, or restore its tombstones with the given characters. */
    #mark(from: Element, to: Element, restored: string | undefined): Sequence {
        // cut at the ends
        const cut = this.#cut(from, "before").#cut(to, "after");
        const first = cut.#find(from);
        const last = cut.#find(to);
        if (last < first) {
            throw new RangeError(
                `sequence range ends before it starts: ${from.run}:${from.offset}`,
            );
        }

        // require as many restored characters as tombstones
        const inside = cut.runs.slice(first, last + 1);
        const deleted = inside.reduce(
            (sum, piece) => sum + (typeof piece.text === "number" ? piece.text : 0),
            0,
        );
        if (restored !== undefined && restored.length !== deleted) {
            throw new RangeError(
                `sequence restores ${restored.length} characters into ${deleted} deleted ones`,
            );
        }

        // replace characters by lengths, or lengths by restored characters
        let remaining = restored ?? "";
        const marked = cut.runs.map((piece, index): Run => {
            // change only the pieces in the range
            const length = Sequence.length(piece);
            if (index < first || index > last) {
                return piece;
            } else if (restored === undefined) {
                return { run: piece.run, start: piece.start, text: length };
            } else if (typeof piece.text === "string") {
                return piece;
            }
            const text = remaining.slice(0, length);
            remaining = remaining.slice(length);

            return { run: piece.run, start: piece.start, text };
        });

        return new Sequence(Sequence.#merge(marked));
    }

    /** Split the piece holding an element at one of its edges. */
    #cut(element: Element, edge: "before" | "after"): Sequence {
        // cut the piece before the element, or after it
        const index = this.#find(element);
        const piece = this.runs[index]!;
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

    /** Find the index of the piece holding an element. */
    #find(element: Element): number {
        const index = this.runs.findIndex(
            (piece) =>
                piece.run === element.run &&
                element.offset >= piece.start &&
                element.offset < piece.start + Sequence.length(piece),
        );
        if (index === -1) {
            throw new RangeError(`sequence holds no element ${element.run}:${element.offset}`);
        }

        return index;
    }

    /** Split a piece before a count of its elements. */
    static split(piece: Run, cut: number): readonly [Run, Run | undefined] {
        // keep the piece whole when the cut reaches either end
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
                previous.start + Sequence.length(previous) === piece.start &&
                typeof previous.text === typeof piece.text;
            if (isContinued) {
                merged[merged.length - 1] = {
                    ...previous,
                    text:
                        typeof previous.text === "number"
                            ? previous.text + (piece.text as number)
                            : previous.text + (piece.text as string),
                };
            } else {
                merged.push(piece);
            }
        }

        return merged;
    }

    /** Decide whether two lists name the same annotations in the same order. */
    static #same(
        left: readonly Annotation<unknown>[],
        right: readonly Annotation<unknown>[],
    ): boolean {
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
