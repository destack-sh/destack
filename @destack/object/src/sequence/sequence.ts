import { canonicalize, type JsonValue, nth } from "@destack/schema";
import type { TextAnchor } from "./anchor.ts";
import type { TextElement, TextRun, TextNode } from "./element.ts";
import type { TextEdit, TextChange } from "./edit.ts";
import { Block } from "./block.ts";
import type { TextMark, TextSpan } from "./mark.ts";
import type { TextMarkType, TextSchema } from "./schema.ts";

/** The object replacement character, U+FFFC, shown for each node. */
export const NODE_CHARACTER = "￼";

/** Where a piece of a run starts among all elements, tombstones included. */
interface PieceOrdinal {
    /** The offset of the piece's first element within its run. */
    readonly start: number;
    /** The piece's elements. */
    readonly length: number;
    /** The ordinal of the piece's first element in the sequence. */
    readonly ordinal: number;
}

/** A mark placed on the sequence: its edges as positions between and on elements. */
interface PlacedMark {
    /** The mark. */
    readonly mark: TextMark;
    /** The position its range starts after. */
    readonly start: number;
    /** The position its range ends before. */
    readonly end: number;
}

/** A text as ordered pieces of runs, with tombstones and marks. */
export class TextSequence {
    /** The runs in sequence order. */
    readonly runs: readonly TextRun[];
    /** The marks, in the order the store applied them, predicted ones last. */
    readonly marks: readonly TextMark[];

    /** The visible elements, a node counting as one. */
    get length(): number {
        let length = 0;
        for (const piece of this.runs) {
            length += TextSequence.isVisible(piece) ? TextSequence.length(piece) : 0;
        }

        return length;
    }

    /** Store runs in sequence order and the marks over them. */
    constructor(runs: readonly TextRun[] = [], marks: readonly TextMark[] = []) {
        this.runs = runs;
        this.marks = marks;
    }

    /** Read the visible text, with U+FFFC for each node. */
    text(): string {
        return this.runs
            .flatMap((piece) =>
                "node" in piece
                    ? [NODE_CHARACTER]
                    : typeof piece.text === "string"
                      ? [piece.text]
                      : [],
            )
            .join("");
    }

    /** Apply one edit, returning the sequence it makes. */
    apply(edit: TextEdit): TextSequence {
        // insert a run after its anchor
        if ("insert" in edit) {
            return this.#insert(edit.insert, edit.run, edit.start, edit.after);
        }
        // delete or restore the named pieces
        else if ("delete" in edit) {
            return this.#set(edit.delete);
        } else if ("restore" in edit) {
            return this.#set(edit.restore);
        }
        // replace a node
        else if ("set" in edit) {
            return this.#replace(edit.set, edit.node);
        }
        // add a mark, replacing one of the same identifier
        else if ("mark" in edit) {
            return new TextSequence(this.runs, [
                ...this.marks.filter((mark) => mark.id !== edit.mark.id),
                edit.mark,
            ]);
        }
        // remove the named marks
        else {
            const removed = new Set(edit.unmark.map((mark) => mark.id));

            return new TextSequence(
                this.runs,
                this.marks.filter((mark) => !removed.has(mark.id)),
            );
        }
    }

    /** Derive the edits undoing one edit applied to a sequence. */
    static inverse(edit: TextEdit, before: TextSequence): TextEdit[] {
        // delete what an insertion added
        if ("insert" in edit) {
            const length = typeof edit.insert === "string" ? edit.insert.length : 1;

            return [{ delete: [{ run: edit.run, start: edit.start, text: length }] }];
        }
        // set a replaced node back
        else if ("set" in edit) {
            const piece = nth(before.runs, before.#find(edit.set));

            return "node" in piece ? [{ set: edit.set, node: piece.node }] : [];
        }
        // remove an added mark, or put back the one it replaced
        else if ("mark" in edit) {
            const replaced = before.marks.find((mark) => mark.id === edit.mark.id);

            return [replaced === undefined ? { unmark: [edit.mark] } : { mark: replaced }];
        }
        // add the removed marks back
        else if ("unmark" in edit) {
            const named = new Set(edit.unmark.map((mark) => mark.id));

            return before.marks.filter((mark) => named.has(mark.id)).map((mark) => ({ mark }));
        }

        // restore the named pieces a deletion found visible, or delete those a restoration found deleted
        const parts = before.#covered("delete" in edit ? edit.delete : edit.restore);
        if ("delete" in edit) {
            const visible = parts.filter((part) => TextSequence.isVisible(part));

            return visible.length === 0 ? [] : [{ restore: visible }];
        }
        const deleted = parts
            .filter((part) => !TextSequence.isVisible(part))
            .map((part) => ({ run: part.run, start: part.start, text: TextSequence.length(part) }));

        return deleted.length === 0 ? [] : [{ delete: deleted }];
    }

    /** Join consecutive deletions or restorations, and the touching pieces of each run. */
    static compact(edits: readonly TextEdit[]): TextEdit[] {
        const compacted: TextEdit[] = [];
        for (const edit of edits) {
            // join a deletion into the deletion before it
            const last = compacted.at(-1);
            if (last !== undefined && "delete" in last && "delete" in edit) {
                compacted.splice(-1, 1, {
                    delete: TextSequence.#join([...last.delete, ...edit.delete]),
                });
            }
            // join a restoration into the restoration before it
            else if (last !== undefined && "restore" in last && "restore" in edit) {
                compacted.splice(-1, 1, {
                    restore: TextSequence.#join([...last.restore, ...edit.restore]),
                });
            }
            // append any other edit
            else {
                compacted.push(edit);
            }
        }

        return compacted;
    }

    /** Join the pieces of a run that touch, in run and offset order. */
    static #join(pieces: readonly TextRun[]): TextRun[] {
        // order the pieces within each run
        const ordered = pieces.toSorted((left, right) =>
            left.run === right.run ? left.start - right.start : left.run < right.run ? -1 : 1,
        );

        // join each piece onto the piece of its run it continues
        const joined: TextRun[] = [];
        for (const piece of ordered) {
            const last = joined.at(-1);
            const text = last === undefined ? undefined : TextSequence.#continued(last, piece);
            if (last !== undefined && text !== undefined) {
                joined.splice(-1, 1, { run: last.run, start: last.start, text });
            }
            // start a new piece
            else {
                joined.push(piece);
            }
        }

        return joined;
    }

    /** Join a piece's characters or length onto the piece it continues, absent when it does not. */
    static #continued(last: TextRun, piece: TextRun): string | number | undefined {
        // join only text pieces of one run that touch
        const isTouching =
            last.run === piece.run && last.start + TextSequence.length(last) === piece.start;
        if (!isTouching || !("text" in last) || !("text" in piece)) {
            return undefined;
        }

        // join characters
        if (typeof last.text === "string" && typeof piece.text === "string") {
            return last.text + piece.text;
        }
        // join tombstone lengths
        else if (typeof last.text === "number" && typeof piece.text === "number") {
            return last.text + piece.text;
        }
        // leave characters and lengths apart
        else {
            return undefined;
        }
    }

    /** Translate a text change at offsets into edits, inserting at a site's next element. */
    change(change: TextChange, next: TextElement): TextEdit[] {
        // delete the visible pieces between the offsets before inserting
        const pieces = this.slice(change.from, change.to);
        const deleted: TextEdit[] = pieces.length === 0 ? [] : [{ delete: pieces }];
        if (change.insert.length === 0) {
            return deleted;
        }

        // insert after the element before the offset, at the start without one
        const after = change.from === 0 ? undefined : this.element(change.from - 1);
        const inserted: TextEdit = {
            insert: change.insert,
            run: next.run,
            start: next.offset,
            ...(after === undefined ? {} : { after }),
        };

        return [...deleted, inserted];
    }

    /** Decide whether an insertion names elements its run already has here. */
    isRepeating(insert: Extract<TextEdit, { readonly insert: unknown }>): boolean {
        const end = insert.start + (typeof insert.insert === "string" ? insert.insert.length : 1);

        return this.runs.some(
            (piece) =>
                piece.run === insert.run &&
                piece.start < end &&
                insert.start < piece.start + TextSequence.length(piece),
        );
    }

    /** Decide whether an insertion starts its run or continues it where one of its pieces ends. */
    isContinuing(insert: Extract<TextEdit, { readonly insert: unknown }>): boolean {
        return (
            insert.start === 0 ||
            this.runs.some(
                (piece) =>
                    piece.run === insert.run &&
                    piece.start + TextSequence.length(piece) === insert.start,
            )
        );
    }

    /** Translate an edit into text changes at offsets, each applying after the ones before. */
    translate(edit: TextEdit): TextChange[] {
        // insert at the offset after the anchor
        if ("insert" in edit) {
            const from = edit.after === undefined ? 0 : this.#after(edit.after);
            const insert = typeof edit.insert === "string" ? edit.insert : NODE_CHARACTER;

            return [{ from, to: from, insert }];
        }
        // change no characters for a node or mark change
        else if (!("delete" in edit) && !("restore" in edit)) {
            return [];
        }

        // walk the text, deleting named visible pieces and inserting named restored ones
        const named = "delete" in edit ? edit.delete : edit.restore;
        const changes: TextChange[] = [];
        let cursor = 0;
        for (const part of this.#cutAt(named).runs) {
            // remove a deleted visible part, insert a restored deleted one, and step over the rest
            const piece = TextSequence.#coverer(named, part);
            const length = TextSequence.length(part);
            const isVisible = TextSequence.isVisible(part);
            if (piece !== undefined && "delete" in edit && isVisible) {
                changes.push({ from: cursor, to: cursor + length, insert: "" });
            } else if (piece !== undefined && "restore" in edit && !isVisible) {
                const restored = TextSequence.#restored(piece, part);
                const insert = "node" in restored ? NODE_CHARACTER : String(restored.text);
                changes.push({ from: cursor, to: cursor, insert });
                cursor += length;
            } else if (isVisible) {
                cursor += length;
            }
        }

        return changes;
    }

    /** Split the visible text into spans of equal marks, and nodes. */
    spans(schema: TextSchema): TextSpan[] {
        return this.#spans(schema, new Map());
    }

    /** Split the visible text into blocks, each from its node on, the first a paragraph. */
    blocks(schema: TextSchema): Block[] {
        // start a block at each node that starts one
        const elements = new Map<TextSpan, TextElement>();
        const blocks: (Block & { readonly spans: TextSpan[] })[] = [];
        for (const span of this.#spans(schema, elements)) {
            const node = "node" in span ? span.node : undefined;
            const type = node === undefined ? undefined : schema.nodes.get(node.type);
            const element = elements.get(span);
            if (node !== undefined && type?.isBlock === true && element !== undefined) {
                const key = `${element.run}:${element.offset}`;
                blocks.push({ key, type: node.type, value: node.value, spans: [], blocks: [] });
            }
            // add any other to the block it is in
            else {
                let block = blocks.at(-1);
                if (block === undefined) {
                    block = { key: "", type: "paragraph", value: null, spans: [], blocks: [] };
                    blocks.push(block);
                }
                block.spans.push(span);
            }
        }

        return Block.nested(blocks);
    }

    /** Split the visible text into spans, collecting the element of each node. */
    #spans(schema: TextSchema, elements: Map<TextSpan, TextElement>): TextSpan[] {
        // place every mark's edges, element i at 2i + 1
        const pieces = this.#pieces();
        const placed = this.marks.flatMap((mark): PlacedMark[] => {
            const start = TextSequence.#place(pieces, mark.start);
            const end = TextSequence.#place(pieces, mark.end);

            return start === undefined || end === undefined || end <= start
                ? []
                : [{ mark, start, end }];
        });
        const opening = placed.toSorted((left, right) => left.start - right.start);
        const closing = placed.toSorted((left, right) => left.end - right.end);

        // sweep the pieces in segments between mark edges
        const active = new Set<PlacedMark>();
        const spans: TextSpan[] = [];
        let opened = 0;
        let closed = 0;
        let resolved: Readonly<Record<string, JsonValue>> | undefined;
        let ordinal = 0;
        for (const piece of this.runs) {
            const end = ordinal + TextSequence.length(piece);
            for (let cursor = ordinal; cursor < end && TextSequence.isVisible(piece);) {
                // open and close the marks around the element
                const position = 2 * cursor + 1;
                for (
                    ;
                    opened < opening.length && nth(opening, opened).start < position;
                    opened += 1
                ) {
                    active.add(nth(opening, opened));
                    resolved = undefined;
                }
                for (
                    ;
                    closed < closing.length && nth(closing, closed).end < position;
                    closed += 1
                ) {
                    active.delete(nth(closing, closed));
                    resolved = undefined;
                }

                // add the segment up to the next mark edge
                const next = Math.min(
                    opening[opened]?.start ?? Number.POSITIVE_INFINITY,
                    closing[closed]?.end ?? Number.POSITIVE_INFINITY,
                );
                const stop = Number.isFinite(next) ? Math.min(end, next / 2) : end;
                if ("node" in piece) {
                    const span = { node: piece.node };
                    spans.push(span);
                    elements.set(span, { run: piece.run, offset: piece.start });
                } else if (typeof piece.text === "string") {
                    resolved ??= TextSequence.#resolve([...active], schema);
                    const characters = piece.text.slice(cursor - ordinal, stop - ordinal);
                    TextSequence.#append(spans, characters, resolved);
                }
                cursor = stop;
            }
            ordinal = end;
        }

        return spans;
    }

    /** Choose a mark's edges over visible offsets. */
    range(from: number, to: number, type: TextMarkType): Pick<TextMark, "start" | "end"> {
        // refuse an empty or out-of-range selection
        const first = this.element(from);
        const last = to > from ? this.element(to - 1) : undefined;
        if (first === undefined || last === undefined) {
            const [start, end] = [from, to];
            throw new RangeError(`text has no visible range from ${start} to ${end}`);
        }

        // anchor the edges outside the range where the type grows
        const previous = from > 0 ? this.element(from - 1) : undefined;
        const following = this.element(to);
        const isGrowingBefore = type.expand === "before" || type.expand === "both";
        const isGrowingAfter = type.expand === "after" || type.expand === "both";
        const start: TextAnchor = !isGrowingBefore
            ? { element: first, side: "before" }
            : previous === undefined
              ? { edge: "start" }
              : { element: previous, side: "after" };
        const end: TextAnchor = !isGrowingAfter
            ? { element: last, side: "after" }
            : following === undefined
              ? { edge: "end" }
              : { element: following, side: "before" };

        return { start, end };
    }

    /** Find the element at an offset of the visible text, absent at the text's end. */
    element(offset: number): TextElement | undefined {
        // walk the visible runs to the offset
        let remaining = offset;
        for (const piece of this.runs) {
            const length = TextSequence.length(piece);
            if (!TextSequence.isVisible(piece)) {
                continue;
            } else if (remaining < length) {
                return { run: piece.run, offset: piece.start + remaining };
            }
            remaining -= length;
        }

        return undefined;
    }

    /** Find the text offset of an element. */
    offset(element: TextElement): number {
        // count the visible elements before the element
        let before = 0;
        for (const piece of this.runs) {
            const length = TextSequence.length(piece);
            const isVisible = TextSequence.isVisible(piece);
            if (
                piece.run === element.run &&
                element.offset >= piece.start &&
                element.offset < piece.start + length
            ) {
                return before + (isVisible ? element.offset - piece.start : 0);
            } else if (isVisible) {
                before += length;
            }
        }
        throw new RangeError(
            `sequence has no element '${element.run}' at offset ${element.offset}`,
        );
    }

    /** Find the text offset right after an element. */
    #after(element: TextElement): number {
        const piece = nth(this.runs, this.#find(element));

        return this.offset(element) + (TextSequence.isVisible(piece) ? 1 : 0);
    }

    /** Insert a run's characters or one node right after an element, splitting the piece there. */
    #insert(
        insert: string | TextNode,
        run: string,
        start: number,
        after: TextElement | undefined,
    ): TextSequence {
        // insert at the start
        const inserted: TextRun =
            typeof insert === "string"
                ? { run, start, text: insert }
                : { run, start, node: insert };
        if (after === undefined) {
            return new TextSequence([inserted, ...this.runs], this.marks);
        }

        // split after the anchor and join a continued piece
        const index = this.#find(after);
        const piece = nth(this.runs, index);
        const cut = after.offset - piece.start + 1;
        const [head, tail] = TextSequence.split(piece, cut);
        const joined =
            !("node" in head) &&
            !("node" in inserted) &&
            typeof head.text === "string" &&
            typeof inserted.text === "string" &&
            head.run === run &&
            head.start + head.text.length === start
                ? [{ run, start: head.start, text: head.text + inserted.text }]
                : [head, inserted];

        return new TextSequence(
            [
                ...this.runs.slice(0, index),
                ...joined,
                ...(tail === undefined ? [] : [tail]),
                ...this.runs.slice(index + 1),
            ],
            this.marks,
        );
    }

    /** Replace a node, refusing a character. */
    #replace(element: TextElement, node: TextNode): TextSequence {
        // refuse a character
        const index = this.#find(element);
        const piece = nth(this.runs, index);
        if (!("node" in piece)) {
            if (typeof piece.text === "number") {
                return this;
            }
            throw new RangeError(`element '${element.run}' at ${element.offset} is no node`);
        }

        return new TextSequence(
            this.runs.map((each, at) => (at === index ? { ...piece, node } : each)),
            this.marks,
        );
    }

    /** Set the named pieces of runs to tombstones or back to characters and nodes. */
    #set(named: readonly TextRun[]): TextSequence {
        // delete or restore each named piece
        const set = this.#cutAt(named).runs.map((part): TextRun => {
            const piece = TextSequence.#coverer(named, part);
            if (piece === undefined) {
                return part;
            } else if (!("node" in piece) && typeof piece.text === "number") {
                return { run: part.run, start: part.start, text: TextSequence.length(part) };
            } else if (TextSequence.isVisible(part)) {
                return part;
            }

            return TextSequence.#restored(piece, part);
        });

        return new TextSequence(TextSequence.#merge(set), this.marks);
    }

    /** Read the parts of the sequence the named pieces cover, in sequence order, as they stand. */
    #covered(named: readonly TextRun[]): TextRun[] {
        return this.#cutAt(named).runs.filter(
            (part) => TextSequence.#coverer(named, part) !== undefined,
        );
    }

    /** Cut the sequence at the edges of the named pieces, refusing a piece whose end it lacks. */
    #cutAt(named: readonly TextRun[]): TextSequence {
        // cut before each piece's first element and after its last, a run's elements between them
        return named.reduce((cut: TextSequence, piece) => {
            const first = { run: piece.run, offset: piece.start };
            const last = { run: piece.run, offset: piece.start + TextSequence.length(piece) - 1 };

            return cut.#cut(first, "before").#cut(last, "after");
        }, this);
    }

    /** Read the pieces between two visible offsets as lengths. */
    slice(from: number, to: number): TextRun[] {
        // walk the visible parts overlapping the offsets
        const pieces: TextRun[] = [];
        let cursor = 0;
        for (const part of this.runs) {
            if (!TextSequence.isVisible(part)) {
                continue;
            }
            const length = TextSequence.length(part);
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

        return TextSequence.#merge(pieces);
    }

    /** Index each run's pieces by where they start among all elements, tombstones included. */
    #pieces(): Map<string, PieceOrdinal[]> {
        // list each run's pieces with their ordinals
        const pieces = new Map<string, PieceOrdinal[]>();
        let ordinal = 0;
        for (const piece of this.runs) {
            const length = TextSequence.length(piece);
            const listed = pieces.get(piece.run) ?? [];
            listed.push({ start: piece.start, length, ordinal });
            pieces.set(piece.run, listed);
            ordinal += length;
        }

        // order each run's pieces by start for the binary search
        for (const listed of pieces.values()) {
            listed.sort((left, right) => left.start - right.start);
        }

        return pieces;
    }

    /** Place an anchor between elements, absent for an element the sequence lacks. */
    static #place(
        pieces: ReadonlyMap<string, readonly PieceOrdinal[]>,
        anchor: TextAnchor,
    ): number | undefined {
        // place the text's edges outside every element
        if ("edge" in anchor) {
            return anchor.edge === "start" ? Number.NEGATIVE_INFINITY : Number.POSITIVE_INFINITY;
        }

        // find the run's last piece starting at or before the element
        const { run, offset } = anchor.element;
        const listed = pieces.get(run) ?? [];
        let low = 0;
        let high = listed.length;
        while (low < high) {
            const middle = (low + high) >>> 1;
            if (nth(listed, middle).start <= offset) {
                low = middle + 1;
            } else {
                high = middle;
            }
        }

        // place the element by its piece's ordinal when the piece holds it
        const candidate = listed[low - 1];
        const piece =
            candidate !== undefined && offset < candidate.start + candidate.length
                ? candidate
                : undefined;

        return piece === undefined
            ? undefined
            : 2 * (piece.ordinal + offset - piece.start) + (anchor.side === "before" ? 0 : 2);
    }

    /** Resolve the marks covering a segment into its mark values. */
    static #resolve(
        covering: readonly PlacedMark[],
        schema: TextSchema,
    ): Readonly<Record<string, JsonValue>> {
        // take each type's latest mark
        const latest = covering
            .map((each) => each.mark)
            .toSorted((left, right) => TextSequence.#compare(left, right));
        const values: Record<string, JsonValue> = {};
        const listed = new Map<string, JsonValue[]>();
        for (const mark of latest) {
            const type = schema.marks.get(mark.type);
            if (type === undefined) {
                continue;
            } else if (type.isMultiple) {
                const each = listed.get(mark.type) ?? [];
                each.push(mark.value);
                listed.set(mark.type, each);
            } else if (mark.value === null) {
                delete values[mark.type];
            } else {
                values[mark.type] = mark.value;
            }
        }

        // add the overlapping types' values
        for (const [type, list] of listed) {
            values[type] = list;
        }
        for (const name of Object.keys(values)) {
            for (const excluded of schema.marks.get(name)?.excludes ?? []) {
                delete values[excluded];
            }
        }

        return values;
    }

    /** Order marks as the store applied them, predicted ones last, by identifier among equals. */
    static #compare(left: TextMark, right: TextMark): number {
        // order by the store's order, a predicted mark last
        const leftOrder = left.order ?? Number.POSITIVE_INFINITY;
        const rightOrder = right.order ?? Number.POSITIVE_INFINITY;
        if (leftOrder !== rightOrder) {
            return leftOrder - rightOrder;
        }

        return left.id < right.id ? -1 : left.id > right.id ? 1 : 0;
    }

    /** Append characters to the last span when its marks match, else start a span. */
    static #append(
        spans: TextSpan[],
        characters: string,
        marks: Readonly<Record<string, JsonValue>>,
    ): void {
        const last = spans.at(-1);
        const isMarked = Object.keys(marks).length > 0;
        if (last !== undefined && "text" in last && TextSequence.#same(last.marks ?? {}, marks)) {
            spans[spans.length - 1] = { ...last, text: last.text + characters };
        } else {
            spans.push(isMarked ? { text: characters, marks } : { text: characters });
        }
    }

    /** Decide whether two resolutions have the same mark values. */
    static #same(
        left: Readonly<Record<string, JsonValue>>,
        right: Readonly<Record<string, JsonValue>>,
    ): boolean {
        // compare by identity, then key by key
        if (left === right) {
            return true;
        }
        const keys = Object.keys(left);
        if (keys.length !== Object.keys(right).length) {
            return false;
        }

        return keys.every((key) => {
            const [one, other] = [left[key], right[key]];

            return (
                one === other ||
                (typeof one === "object" &&
                    typeof other === "object" &&
                    canonicalize(one) === canonicalize(other))
            );
        });
    }

    /** Find the named piece covering a whole part of the sequence, absent for none. */
    static #coverer(named: readonly TextRun[], part: TextRun): TextRun | undefined {
        return named.find(
            (piece) =>
                piece.run === part.run &&
                part.start >= piece.start &&
                part.start < piece.start + TextSequence.length(piece),
        );
    }

    /** Read what a restored piece gives back: characters or a node. */
    static #restored(piece: TextRun, part: TextRun): TextRun {
        if ("node" in piece) {
            return { run: part.run, start: part.start, node: piece.node };
        }
        const offset = part.start - piece.start;
        const text = String(piece.text).slice(offset, offset + TextSequence.length(part));

        return { run: part.run, start: part.start, text };
    }

    /** Split the piece with an element at one of its edges. */
    #cut(element: TextElement, edge: "before" | "after"): TextSequence {
        // cut the piece before the element, or after it
        const index = this.#find(element);
        const piece = nth(this.runs, index);
        const count = element.offset - piece.start + (edge === "after" ? 1 : 0);
        const [head, tail] = TextSequence.split(piece, count);

        return tail === undefined
            ? this
            : new TextSequence(
                  [...this.runs.slice(0, index), head, tail, ...this.runs.slice(index + 1)],
                  this.marks,
              );
    }

    /** Find the index of the piece with an element. */
    #find(element: TextElement): number {
        const index = this.runs.findIndex(
            (piece) =>
                piece.run === element.run &&
                element.offset >= piece.start &&
                element.offset < piece.start + TextSequence.length(piece),
        );
        if (index === -1) {
            throw new RangeError(
                `sequence has no element '${element.run}' at offset ${element.offset}`,
            );
        }

        return index;
    }

    /** Split a piece at an offset within it. */
    static split(piece: TextRun, cut: number): readonly [TextRun, TextRun | undefined] {
        // take the piece whole when the cut is at either end
        const length = TextSequence.length(piece);
        if ("node" in piece || cut <= 0 || cut >= length) {
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

    /** Merge neighbouring pieces of one run in the same state. */
    static #merge(runs: readonly TextRun[]): TextRun[] {
        const merged: TextRun[] = [];
        for (const piece of runs) {
            // extend the previous piece
            const previous = merged.at(-1);
            const isContinued =
                previous !== undefined &&
                !("node" in previous) &&
                !("node" in piece) &&
                previous.run === piece.run &&
                previous.start + TextSequence.length(previous) === piece.start &&
                typeof previous.text === typeof piece.text;
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

    /** Decide whether a piece is visible. */
    static isVisible(piece: TextRun): boolean {
        return "node" in piece || typeof piece.text === "string";
    }

    /** Count a piece's elements, a node being one. */
    static length(piece: TextRun): number {
        return "node" in piece
            ? 1
            : typeof piece.text === "number"
              ? piece.text
              : piece.text.length;
    }

    /** Decide whether a piece contains an element. */
    static contains(piece: TextRun, element: TextElement): boolean {
        return (
            piece.run === element.run &&
            element.offset >= piece.start &&
            element.offset < piece.start + TextSequence.length(piece)
        );
    }

    /** Decide whether an edit acts at the text's start. */
    static isAtStart(edit: TextEdit): boolean {
        if ("insert" in edit) {
            return edit.after === undefined;
        } else if ("mark" in edit || "unmark" in edit) {
            return ("mark" in edit ? [edit.mark] : edit.unmark).some(
                (mark) => "edge" in mark.start,
            );
        }

        return false;
    }

    /** List the elements an edit names: an anchor, a node, a mark's start, or piece ends. */
    static elements(edit: TextEdit): TextElement[] {
        // take an insertion's anchor, else each piece's first and last element
        if ("insert" in edit) {
            return edit.after === undefined ? [] : [edit.after];
        }
        // take a set node or a mark's start element
        else if ("set" in edit) {
            return [edit.set];
        } else if ("mark" in edit || "unmark" in edit) {
            return ("mark" in edit ? [edit.mark] : edit.unmark).flatMap((mark) =>
                "element" in mark.start ? [mark.start.element] : [],
            );
        }

        return ("delete" in edit ? edit.delete : edit.restore).flatMap((piece) => [
            { run: piece.run, offset: piece.start },
            { run: piece.run, offset: piece.start + TextSequence.length(piece) - 1 },
        ]);
    }
}
