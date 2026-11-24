import type { Edit, LineRange, Source } from "../source/index.ts";
import { type Change, compare } from "./compare.ts";
import type { Hunk, HunkLine, LineKind } from "./hunk.ts";
import { compareWords, type PlacedLine, type WordChange } from "./word.ts";

/** The unchanged lines on either side of a change in a hunk. */
const CONTEXT_LINES = 3;

/** The space at the end of a line. */
const TRAILING_SPACE = /\s+$/u;

/** The spaces anywhere in a line. */
const SPACE = /\s+/gu;

/** How a difference compares lines. */
export interface DifferenceOptions {
    /** Whether space counts, all of it by default. */
    readonly whitespace?: "exact" | "trailing" | "ignored";
}

/** What changed between two sources, line by line. */
export class Difference {
    /** The old source. */
    readonly before: Source;
    /** The new source. */
    readonly after: Source;
    /** The lines of the new source in place of lines of the old one, in order. */
    readonly changes: readonly Change[];

    /** Keep the changes between two sources. */
    constructor(before: Source, after: Source, changes: readonly Change[]) {
        this.before = before;
        this.after = after;
        this.changes = changes;
    }

    /** Compare two sources. */
    static of(
        before: Source,
        after: Source,
        { whitespace = "exact" }: DifferenceOptions = {},
    ): Difference {
        // number each distinct line
        const numbers = new Map<string, number>();
        const number = (source: Source): Int32Array => {
            const list = new Int32Array(source.lines);
            for (let line = 0; line < list.length; line += 1) {
                const written = source.slice({ start: line, end: line + 1 });
                const key =
                    whitespace === "exact"
                        ? written
                        : whitespace === "trailing"
                          ? written.replace(TRAILING_SPACE, "")
                          : written.replace(SPACE, "");
                const known = numbers.get(key) ?? numbers.size;
                numbers.set(key, known);
                list[line] = known;
            }

            return list;
        };
        const [olds, news] = [number(before), number(after)];
        const changes = compare(olds, news);

        return new Difference(before, after, slide(changes, { before, after, olds, news }));
    }

    /** The count of lines added. */
    get added(): number {
        return this.changes.reduce((sum, { after }) => sum + after.end - after.start, 0);
    }

    /** The count of lines removed. */
    get removed(): number {
        return this.changes.reduce((sum, { before }) => sum + before.end - before.start, 0);
    }

    /** Write the changes as replacements that make the new text of the old one. */
    edits(): Edit[] {
        return this.changes.map(({ before, after }) => ({
            from: this.before.start(before.start),
            to: this.before.start(before.end),
            insert: this.after.slice(after),
        }));
    }

    /** Gather the changes into hunks with some unchanged lines around them. */
    hunks(context: number = CONTEXT_LINES): Hunk[] {
        // join changes whose context meets
        const groups: Change[][] = [];
        for (const change of this.changes) {
            const last = groups.at(-1)?.at(-1);
            if (last !== undefined && change.before.start - last.before.end <= context * 2) {
                groups.at(-1)?.push(change);
            } else {
                groups.push([change]);
            }
        }

        return groups.map((group) => this.#hunk(group, context));
    }

    /** Compare the words of the lines of a change that are alike, none for lines that are not. */
    words(change: Change): WordChange[] {
        return compareWords(linesIn(this.before, change.before), linesIn(this.after, change.after));
    }

    /** Gather some changes into a hunk. */
    #hunk(changes: readonly Change[], context: number): Hunk {
        // start some lines before the first change
        const lines: HunkLine[] = [];
        const [first, last] = [changes[0], changes.at(-1)];
        if (first === undefined || last === undefined) {
            throw new TypeError("a hunk needs a change");
        }
        const lead = Math.min(context, first.before.start);
        const tail = Math.min(context, this.before.lines - last.before.end);
        const write = (kind: LineKind, source: Source, { start, end }: LineRange): void => {
            for (let line = start; line < end; line += 1) {
                const isLast = line === source.lines - 1;
                const hasLineBreak = !isLast || source.isClosed;
                lines.push({ kind, text: source.line(line), hasLineBreak });
            }
        };

        // write each change after the lines before it
        let at = first.before.start - lead;
        for (const { before, after } of changes) {
            write("same", this.before, { start: at, end: before.start });
            write("removed", this.before, before);
            write("added", this.after, after);
            at = before.end;
        }
        write("same", this.before, { start: at, end: last.before.end + tail });

        return {
            before: { start: first.before.start - lead, end: last.before.end + tail },
            after: { start: first.after.start - lead, end: last.after.end + tail },
            lines,
        };
    }
}

/** List some lines of a source with where each starts. */
function linesIn(source: Source, { start, end }: LineRange): PlacedLine[] {
    return Array.from({ length: end - start }, (_, index) => ({
        from: source.start(start + index),
        text: source.line(start + index),
    }));
}

/** The two sources with a number for each line. */
interface Sides {
    /** The old source. */
    readonly before: Source;
    /** The new source. */
    readonly after: Source;
    /** The numbers of the old lines. */
    readonly olds: Int32Array;
    /** The numbers of the new lines. */
    readonly news: Int32Array;
}

/** Move each addition or removal alone to where its lines start a block, as a reader sees it. */
function slide(changes: readonly Change[], sides: Sides): Change[] {
    const slid: Change[] = [];
    for (const [place, change] of changes.entries()) {
        // leave replacements, which have both sides in place
        const isAddition = change.before.start === change.before.end;
        const isRemoval = change.after.start === change.after.end;
        if (isAddition === isRemoval) {
            slid.push(change);
            continue;
        }

        // find how far it moves each way without meeting another change
        const [source, list] = isAddition ? [sides.after, sides.news] : [sides.before, sides.olds];
        const range = isAddition ? change.after : change.before;
        const other = isAddition ? change.before : change.after;
        const [last, next] = [slid.at(-1), changes[place + 1]];
        const floor = last === undefined ? 0 : (isAddition ? last.after : last.before).end + 1;
        const otherFloor = last === undefined ? 0 : (isAddition ? last.before : last.after).end + 1;
        const ceiling =
            next === undefined ? list.length : (isAddition ? next.after : next.before).start - 1;
        let up = 0;
        while (
            range.start - up - 1 >= floor &&
            other.start - up - 1 >= otherFloor &&
            list[range.start - up - 1] === list[range.end - up - 1]
        ) {
            up += 1;
        }
        let down = 0;
        while (range.end + down < ceiling && list[range.start + down] === list[range.end + down]) {
            down += 1;
        }

        // take the place whose first line is indented the least, after an empty line, lowest
        let best = 0;
        let least = Infinity;
        for (let shift = -up; shift <= down; shift += 1) {
            const start = range.start + shift;
            const isAfterEmpty = start === 0 || source.line(start - 1).trim() === "";
            const cost = indentOf(source.line(start)) + (isAfterEmpty ? 0 : 0.5);
            if (cost <= least) {
                least = cost;
                best = shift;
            }
        }
        const move = ({ start, end }: LineRange): LineRange => ({
            start: start + best,
            end: end + best,
        });
        slid.push({ before: move(change.before), after: move(change.after) });
    }

    return slid;
}

/** Count the spaces a line starts with, more than any for an empty line. */
function indentOf(line: string): number {
    const text = line.trimStart();

    return text === "" ? Number.MAX_SAFE_INTEGER : line.length - text.length;
}
