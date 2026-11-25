import { nth, present } from "@destack/schema";
import type { TextChange, TextEdit } from "./edit.ts";
import type { TextElement, TextRun } from "./element.ts";
import type { TextMark } from "./mark.ts";
import type { TextMarkType } from "./schema.ts";
import { TextSequence } from "./sequence.ts";

/** The most characters per chunk: a keystroke syncs one ~1 KiB row, 1 MB is ~2,000 rows. */
export const CHUNK_CHARACTERS = 512;

/** The last whitespace of a text, where a chunk may end between words. */
const LAST_SPACE = /\s(?=\S*$)/u;

/** The last character of a text in no word, where a chunk of a script without spaces may end. */
const LAST_GAP = /[^\p{L}\p{N}\p{M}](?=[\p{L}\p{N}\p{M}]*$)/u;

/** The characters left in each chunk after a split: three quarters, with room to type. */
const FILL_CHARACTERS = (CHUNK_CHARACTERS * 3) / 4;

/** A text as a list of small sequences, its chunks. */
export class TextRope {
    /** The chunks in text order. */
    readonly chunks: readonly TextSequence[];
    /** The visible elements of each chunk. */
    readonly lengths: readonly number[];
    /** The visible elements of the text. */
    readonly length: number;

    /** Store chunks in text order with their visible lengths. */
    constructor(
        chunks: readonly TextSequence[] = [],
        lengths: readonly number[] = chunks.map((chunk) => chunk.length),
    ) {
        this.chunks = chunks;
        this.lengths = lengths;
        this.length = lengths.reduce((sum, length) => sum + length, 0);
    }

    /** Join the chunks into one sequence. */
    read(): TextSequence {
        return new TextSequence(
            this.chunks.flatMap((chunk) => chunk.runs),
            this.chunks.flatMap((chunk) => chunk.marks),
        );
    }

    /** Translate a text change at offsets into edits, inserting at a site's next element. */
    change(change: TextChange, next: TextElement): TextEdit[] {
        // delete the visible pieces of each chunk the range covers
        const deleted = this.#covered(change.from, change.to).flatMap(({ index, from, to }) =>
            nth(this.chunks, index).slice(from, to),
        );
        const edits: TextEdit[] = deleted.length === 0 ? [] : [{ delete: deleted }];
        if (change.insert.length === 0) {
            return edits;
        }

        // insert after the element before the offset, at the start without one
        const after = change.from === 0 ? undefined : this.element(change.from - 1);
        edits.push({
            insert: change.insert,
            run: next.run,
            start: next.offset,
            ...(after === undefined ? {} : { after }),
        });

        return edits;
    }

    /** Apply one edit to the span of chunks it names, returning the chunks it makes. */
    apply(edit: TextEdit): TextRope {
        // find the chunks holding the named elements
        const holders = TextSequence.elements(edit).map((element) => this.#holder(element));
        if (holders.length === 0 || TextSequence.isAtStart(edit)) {
            holders.push(0);
        }
        const first = Math.min(...holders);
        const last = Math.max(...holders);

        // join the span, apply the edit and lay it out
        const span = this.chunks.slice(first, last + 1);
        const joined = new TextSequence(
            span.flatMap((chunk) => chunk.runs),
            span.flatMap((chunk) => chunk.marks),
        );
        const laid = TextRope.layout(
            joined.apply(edit),
            span.slice(1).map((chunk) => TextRope.start(chunk)),
        ).flat();

        return new TextRope(
            this.chunks.toSpliced(first, span.length, ...laid),
            this.lengths.toSpliced(first, span.length, ...laid.map((chunk) => chunk.length)),
        );
    }

    /** Anchor a mark of a type over a visible range, joining only the chunks around it. */
    range(from: number, to: number, type: TextMarkType): Pick<TextMark, "start" | "end"> {
        // refuse an empty range or one past the end
        if (from >= to || to > this.length) {
            throw new RangeError(
                `range from '${from}' to '${to}' is invalid: the text has ${this.length} visible elements`,
            );
        }

        // join the chunks from the element before the range to the one after it
        const covered = this.#covered(Math.max(0, from - 1), to + 1);
        const first = nth(covered, 0).index;
        const last = nth(covered, covered.length - 1).index;
        const span = this.chunks.slice(first, last + 1);
        const before = this.lengths.slice(0, first).reduce((sum, length) => sum + length, 0);
        const joined = new TextSequence(span.flatMap((chunk) => chunk.runs));

        return joined.range(from - before, to - before, type);
    }

    /** Find the element at a visible offset, absent past the end. */
    element(offset: number): TextElement | undefined {
        const located = this.#locate(offset);

        return located === undefined
            ? undefined
            : nth(this.chunks, located.index).element(located.offset);
    }

    /** Find the chunk holding a visible offset, and the offset within it. */
    #locate(offset: number): { readonly index: number; readonly offset: number } | undefined {
        // walk the lengths to the chunk the offset falls in
        let start = 0;
        for (const [index, length] of this.lengths.entries()) {
            if (offset < start + length) {
                return { index, offset: offset - start };
            }
            start += length;
        }

        return undefined;
    }

    /** List the chunks a visible range covers, each with its part of the range. */
    #covered(from: number, to: number) {
        // collect each chunk's share of the range
        const covered: { readonly index: number; readonly from: number; readonly to: number }[] =
            [];
        let start = 0;
        for (const [index, length] of this.lengths.entries()) {
            const end = start + length;
            if (from < end && start < to) {
                covered.push({
                    index,
                    from: Math.max(from, start) - start,
                    to: Math.min(to, end) - start,
                });
            }
            start = end;
        }

        return covered;
    }

    /** Find the chunk holding an element, refusing one no chunk holds. */
    #holder(element: TextElement): number {
        // scan the pieces in one tight pass
        for (const [index, chunk] of this.chunks.entries()) {
            for (const piece of chunk.runs) {
                if (TextSequence.contains(piece, element)) {
                    return index;
                }
            }
        }

        throw new RangeError(
            `element '${element.run}' at offset ${element.offset} is invalid: no chunk has it`,
        );
    }

    /** Lay a text out in chunks, starting a section at each start element. */
    static layout(
        sequence: TextSequence,
        starts: readonly TextElement[],
    ): [TextSequence, ...TextSequence[]][] {
        return TextRope.#cut(sequence.runs, starts).map((runs, section) => {
            // fill the section and place each mark in its start's chunk
            const [first, ...rest] = TextRope.#fill(runs);

            return [
                TextRope.#chunk(sequence, first, section === 0),
                ...rest.map((group) => TextRope.#chunk(sequence, group, false)),
            ];
        });
    }

    /** Build one chunk of a laid out text with the marks starting in it. */
    static #chunk(sequence: TextSequence, runs: TextRun[], isFirst: boolean): TextSequence {
        const marks = sequence.marks.filter(({ start }) =>
            "edge" in start
                ? isFirst
                : runs.some((piece) => TextSequence.contains(piece, start.element)),
        );

        return new TextSequence(runs, marks);
    }

    /** Read the element a chunk starts at. */
    static start(chunk: TextSequence): TextElement {
        const first = nth(chunk.runs, 0);

        return { run: first.run, offset: first.start };
    }

    /** Split runs into sections starting at each cut element. */
    static #cut(runs: readonly TextRun[], cuts: readonly TextElement[]): TextRun[][] {
        // walk the pieces toward the next cut
        let section: TextRun[] = [];
        const sections = [section];
        let next = 0;
        for (let piece of runs) {
            // start a section at each cut in the piece
            for (
                let cut = cuts[next];
                cut !== undefined && TextSequence.contains(piece, cut);
                cut = cuts[next]
            ) {
                const [head, tail] = TextSequence.split(piece, cut.offset - piece.start);
                if (tail !== undefined) {
                    section.push(head);
                    piece = tail;
                }
                section = [];
                sections.push(section);
                next += 1;
            }
            section.push(piece);
        }

        return sections;
    }

    /** Split a section's runs into even groups of at most a chunk's characters. */
    static #fill(runs: readonly TextRun[]): [TextRun[], ...TextRun[][]] {
        // split a text longer than a chunk evenly into partly full chunks
        const total = runs.reduce((sum, piece) => sum + TextRope.#characters(piece), 0);
        const count = total <= CHUNK_CHARACTERS ? 1 : Math.ceil(total / FILL_CHARACTERS);
        const capacity = Math.ceil(total / count);

        // split the pieces at each group's end
        const ends = TextRope.#ends(runs, capacity);
        let group: TextRun[] = [];
        const groups: [TextRun[], ...TextRun[][]] = [group];
        let filled = 0;
        let next = 0;
        for (let piece of runs) {
            for (
                let end = ends[next];
                end !== undefined && filled + TextRope.#characters(piece) > end;
                end = ends[next]
            ) {
                // keep the piece's head in the group, and start the next one with its tail
                if (end > filled) {
                    const [head, tail] = TextSequence.split(piece, end - filled);
                    group.push(head);
                    piece = present(tail, "the tail of a piece split before its end");
                }
                group = [];
                groups.push(group);
                filled = end;
                next += 1;
            }
            group.push(piece);
            filled += TextRope.#characters(piece);
        }

        return groups;
    }

    /** List where groups end among the characters of pieces, between words where they can. */
    static #ends(runs: readonly TextRun[], capacity: number): number[] {
        // read the characters, and where nodes are among them
        let text = "";
        const nodes: number[] = [];
        for (const piece of runs) {
            if ("node" in piece) {
                nodes.push(text.length);
            } else if (typeof piece.text === "string") {
                text += piece.text;
            }
        }

        // end each group after a space of its second half, else between words, else where it is full
        const ends: number[] = [];
        let start = 0;
        while (text.length - start > capacity) {
            const half = start + Math.ceil(capacity / 2);
            const full = start + capacity;
            const second = text.slice(half, full);
            const space = second.search(LAST_SPACE);
            const gap = second.search(LAST_GAP);
            const node = nodes.findLast((place) => place > half && place <= full) ?? -1;
            const between =
                space === -1 ? Math.max(gap === -1 ? -1 : half + gap + 1, node) : half + space + 1;
            start = between === -1 ? full : between;
            ends.push(start);
        }

        return ends;
    }

    /** Count the characters a piece stores. */
    static #characters(piece: TextRun): number {
        return "text" in piece && typeof piece.text === "string" ? piece.text.length : 0;
    }
}
