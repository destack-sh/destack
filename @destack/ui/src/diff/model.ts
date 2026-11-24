import {
    type Change,
    Difference,
    type FilePatch,
    type LineKind,
    type LineRange,
    type Range,
    Source,
} from "@destack/code";

/** The fewest unchanged lines worth folding away, fewer than the fold's own row costs to read. */
const LEAST_FOLDED = 4;

/** A line that code stands in, as git finds it: one that starts at the edge. */
const HEADING = /^[\p{L}_$]/u;

/** Which of a diff's texts something is in. */
export type DiffSide = "before" | "after";

/** A line of a diff, with its place in each text that has it. */
export interface DiffLine {
    /** Which texts have the line. */
    readonly kind: LineKind;
    /** Its place in the old text, absent for an added line. */
    readonly before: number | undefined;
    /** Its place in the new text, absent for a removed line. */
    readonly after: number | undefined;
    /** The change it is of, absent for an unchanged line. */
    readonly change: Change | undefined;
    /** The place of its change among the changes, absent for an unchanged line. */
    readonly place: number | undefined;
}

/** Unchanged lines folded away. */
export interface DiffFold {
    /** The first old line of the unchanged lines around, which what is revealed is kept by. */
    readonly key: number;
    /** The lines of the old text. */
    readonly before: LineRange;
    /** The lines of the new text. */
    readonly after: LineRange;
    /** The line of code the next change is inside, absent for none. */
    readonly heading: string | undefined;
    /** Where it lies: before the first change, between two, or after the last. */
    readonly position: "first" | "between" | "last";
    /** Whether a patch leaves the lines out, so nothing can reveal them. */
    readonly isMissing: boolean;
}

/** A row of a diff: a line, or lines folded away. */
export type DiffRow =
    | { readonly kind: "line"; readonly line: DiffLine }
    | { readonly kind: "fold"; readonly fold: DiffFold };

/** A row of two columns: the lines that stand beside each other, or lines folded away. */
export type DiffPair =
    | {
          readonly kind: "pair";
          readonly old: DiffLine | undefined;
          readonly next: DiffLine | undefined;
      }
    | { readonly kind: "fold"; readonly fold: DiffFold };

/** The unchanged lines shown of a fold, by the first line it folds. */
export type Revealed = ReadonlyMap<number, { readonly above: number; readonly below: number }>;

/** Old lines a patch leaves out, with the heading of the hunk after them. */
interface Missing extends LineRange {
    /** The line of code the hunk is inside, absent for none. */
    readonly heading: string | undefined;
}

/** What a diff shows: two texts and what changed between them. */
export class DiffModel {
    /** The old text. */
    readonly before: Source;
    /** The new text. */
    readonly after: Source;
    /** What changed. */
    readonly difference: Difference;
    /** The old lines a patch leaves out, with the heading of the hunk after them. */
    readonly #missing: readonly Missing[];
    /** The changed words on each side of a change. */
    readonly #words = new Map<Change, ReadonlyMap<DiffSide, readonly Range[]>>();
    /** The line of code each line of the new text is at or inside, once one is asked for. */
    #headings: Int32Array | undefined;
    /** The rows of each change. */
    readonly #changed = new Map<Change, readonly DiffRow[]>();
    /** The rows of unchanged lines, by old line. */
    readonly #same = new Map<number, DiffRow>();

    /** Show the changes between two texts, of which a patch may leave out lines. */
    constructor(difference: Difference, missing: readonly Missing[] = []) {
        // take the texts of the difference
        this.before = difference.before;
        this.after = difference.after;
        this.difference = difference;
        this.#missing = missing;
    }

    /** Read a file's changes, whose texts are the lines the patch has. */
    static ofPatch(file: FilePatch): DiffModel {
        // put the lines where the patch says, with empty ones for those it leaves out
        const texts = { before: "", after: "" };
        const missing: Missing[] = [];
        const changes: Change[] = [];
        let end = { before: 0, after: 0 };
        for (const { before, after, heading, lines } of file.hunks) {
            texts.before += "\n".repeat(before.start - end.before);
            texts.after += "\n".repeat(after.start - end.after);
            if (before.start > end.before) {
                missing.push({ start: end.before, end: before.start, heading });
            }

            // read the lines, those changed next to each other as one change
            const at = { before: before.start, after: after.start };
            let start = { ...at };
            for (const [index, { kind, text, hasLineBreak }] of lines.entries()) {
                // write the line in the texts that have it
                const line = hasLineBreak ? `${text}\n` : text;
                if (kind !== "added") {
                    texts.before += line;
                    at.before += 1;
                }
                if (kind !== "removed") {
                    texts.after += line;
                    at.after += 1;
                }

                // end the change at its last line
                const isLast = (lines[index + 1]?.kind ?? "same") === "same";
                if (kind !== "same" && isLast) {
                    changes.push({
                        before: { start: start.before, end: at.before },
                        after: { start: start.after, end: at.after },
                    });
                }
                if (isLast || kind === "same") {
                    start = { ...at };
                }
            }
            end = { before: before.end, after: after.end };
        }
        const sources = [Source.of(texts.before), Source.of(texts.after)] as const;

        return new DiffModel(new Difference(...sources, changes), missing);
    }

    /** Report whether a line is the last of its text and, unlike the other text, has no break. */
    isOpen(side: DiffSide, line: number): boolean {
        // compare how the texts end
        const { isClosed, lines } = this[side];
        const other = this[side === "before" ? "after" : "before"];

        return !isClosed && other.isClosed && line === lines - 1;
    }

    /** Read the changed words of a line of a change, as offsets. */
    words(line: DiffLine): readonly Range[] {
        // compare the words of a change once, as its first line shows
        const { change } = line;
        if (change === undefined) {
            return [];
        }
        let words = this.#words.get(change);
        if (words === undefined) {
            const found = this.difference.words(change);
            words = new Map<DiffSide, readonly Range[]>([
                ["before", found.map(({ before }) => before)],
                ["after", found.map(({ after }) => after)],
            ]);
            this.#words.set(change, words);
        }

        // take those of the line
        const side: DiffSide = line.after === undefined ? "before" : "after";
        const { from, to } = this[side].range(line[side] ?? 0);

        return (words.get(side) ?? []).filter(
            (word) => word.to > word.from && word.from < to && word.to > from,
        );
    }

    /** List the rows in one column, unchanged lines away from changes folded. */
    rows(context: number, revealed: Revealed): DiffRow[] {
        // list each change after what is unchanged before it
        const rows: DiffRow[] = [];
        const changes = this.difference.changes;
        let at = { before: 0, after: 0 };
        for (const [place, change] of changes.entries()) {
            const same = { start: at.before, end: change.before.start };
            const position = place === 0 ? "first" : "between";
            rows.push(...this.#unchanged(same, at.after, position, context, revealed));
            rows.push(...this.#changedRows(change, place));
            at = { before: change.before.end, after: change.after.end };
        }

        // list what is unchanged after them
        const same = { start: at.before, end: this.before.lines };
        const position = changes.length === 0 ? "first" : "last";
        rows.push(...this.#unchanged(same, at.after, position, context, revealed));

        return rows;
    }

    /** List the lines a change removes and adds, the same rows every time. */
    #changedRows(change: Change, place: number): readonly DiffRow[] {
        // read them once
        const known = this.#changed.get(change);
        if (known !== undefined) {
            return known;
        }
        const { before, after } = change;
        const rows = [
            ...Array.from({ length: before.end - before.start }, (_, index): DiffRow => {
                const line = { before: before.start + index, after: undefined };

                return { kind: "line", line: { kind: "removed", ...line, change, place } };
            }),
            ...Array.from({ length: after.end - after.start }, (_, index): DiffRow => {
                const line = { before: undefined, after: after.start + index };

                return { kind: "line", line: { kind: "added", ...line, change, place } };
            }),
        ];
        this.#changed.set(change, rows);

        return rows;
    }

    /** Read the row of an unchanged line, the same row every time. */
    #sameRow(before: number, after: number): DiffRow {
        // read it once
        let row = this.#same.get(before);
        if (row === undefined) {
            const line = { before, after, change: undefined, place: undefined };
            row = { kind: "line", line: { kind: "same", ...line } };
            this.#same.set(before, row);
        }

        return row;
    }

    /** List unchanged lines, those away from a change folded unless they were revealed. */
    #unchanged(
        { start, end }: LineRange,
        after: number,
        position: DiffFold["position"],
        context: number,
        revealed: Revealed,
    ): DiffRow[] {
        // fold the lines a patch leaves out
        const list = (from: number, to: number): DiffRow[] =>
            Array.from({ length: Math.max(0, to - from) }, (_, index) =>
                this.#sameRow(from + index, after + from + index - start),
            );
        const fold = (
            hidden: LineRange,
            heading: string | undefined,
            isMissing: boolean,
        ): DiffRow => {
            // find the lines in the new text
            const offset = after - start;
            const other = { start: hidden.start + offset, end: hidden.end + offset };
            const ranges = { before: hidden, after: other };

            return { kind: "fold", fold: { key: start, ...ranges, heading, position, isMissing } };
        };
        const missing = this.#missing.find((range) => range.start >= start && range.end <= end);
        if (missing !== undefined) {
            return [
                ...list(start, missing.start),
                fold(missing, missing.heading, true),
                ...list(missing.end, end),
            ];
        }

        // show some lines next to each change, and those revealed
        const shown = revealed.get(start) ?? { above: 0, below: 0 };
        const above = (position === "first" ? 0 : context) + shown.above;
        const below = (position === "last" ? 0 : context) + shown.below;
        if (end - start - above - below < LEAST_FOLDED) {
            return list(start, end);
        }

        // fold the rest
        const hidden = { start: start + above, end: end - below };
        const isInside = position !== "last";
        const heading = isInside ? this.#heading(after + hidden.end - start) : undefined;

        return [
            ...list(start, hidden.start),
            fold(hidden, heading, false),
            ...list(hidden.end, end),
        ];
    }

    /** Find the line of code a line of the new text is inside, absent for none. */
    #heading(line: number): string | undefined {
        // find the one before every line, once
        this.#headings ??= this.#findHeadings();
        const place = this.#headings[line - 1] ?? -1;

        return place < 0 ? undefined : this.after.line(place).trim();
    }

    /** Find the line of code each line of the new text is at or inside, negative for none. */
    #findHeadings(): Int32Array {
        // remember the last that starts at the edge
        const headings = new Int32Array(this.after.lines);
        let last = -1;
        for (let line = 0; line < headings.length; line += 1) {
            last = HEADING.test(this.after.line(line)) ? line : last;
            headings[line] = last;
        }

        return headings;
    }
}

/** Put removed lines beside the added ones in their place. */
export function pairsOf(rows: readonly DiffRow[]): DiffPair[] {
    // gather the lines of a change, and pair them once it ends
    const pairs: DiffPair[] = [];
    let removed: DiffLine[] = [];
    let added: DiffLine[] = [];
    const pair = (): void => {
        // put each beside the one in its place
        const count = Math.max(removed.length, added.length);
        for (let index = 0; index < count; index += 1) {
            pairs.push({ kind: "pair", old: removed[index], next: added[index] });
        }
        removed = [];
        added = [];
    };

    // list the rows, starting anew at a change next to another
    for (const row of rows) {
        const line = row.kind === "line" ? row.line : undefined;
        const last = added.at(-1) ?? removed.at(-1);
        if (last !== undefined && line?.change !== last.change) {
            pair();
        }
        if (row.kind === "fold") {
            pairs.push(row);
        } else if (row.line.kind === "removed") {
            removed.push(row.line);
        } else if (row.line.kind === "added") {
            added.push(row.line);
        } else {
            pairs.push({ kind: "pair", old: row.line, next: row.line });
        }
    }
    pair();

    return pairs;
}
