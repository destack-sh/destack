import { type Change, Difference } from "../difference/index.ts";
import { type LineRange, Source } from "../source/index.ts";

/** A run of a merge: what both have, what one side changed, or what both changed differently. */
export interface Chunk {
    /** What became of it. */
    readonly kind: "same" | "ours" | "theirs" | "conflict";
    /** The text both sides started from. */
    readonly base: string;
    /** The text on our side. */
    readonly ours: string;
    /** The text on their side. */
    readonly theirs: string;
}

/** Which side a conflict is resolved to, or the text that takes its place. */
export type Resolution = "base" | "ours" | "theirs" | "both" | { readonly text: string };

/** The names a conflict's marks have, such as those of the branches. */
export interface MergeLabels {
    /** What our side is called. */
    readonly ours?: string;
    /** What both started from is called. */
    readonly base?: string;
    /** What their side is called. */
    readonly theirs?: string;
}

/** Two texts changed from a third, put together chunk by chunk. */
export class Merge {
    /** The chunks, in order. */
    readonly chunks: readonly Chunk[];

    /** Keep the chunks of a merge. */
    constructor(chunks: readonly Chunk[]) {
        this.chunks = chunks;
    }

    /** Merge two sources changed from a base, as diff3 does. */
    static of(base: Source, ours: Source, theirs: Source): Merge {
        // compare both sides with the base
        const sources = { base, ours, theirs };
        const sides = {
            ours: new Side(Difference.of(sources.base, sources.ours).changes),
            theirs: new Side(Difference.of(sources.base, sources.theirs).changes),
        };
        const chunks: Chunk[] = [];

        // take the next change of either side, with every change of the other it touches
        let at = 0;
        for (let from = nextOf(sides); from !== undefined; from = nextOf(sides)) {
            let to = from;
            const taken: { ours: Change[]; theirs: Change[] } = { ours: [], theirs: [] };
            for (let isGrowing = true; isGrowing;) {
                const more = {
                    ours: sides.ours.take(from, to),
                    theirs: sides.theirs.take(from, to),
                };
                taken.ours.push(...more.ours);
                taken.theirs.push(...more.theirs);
                const ends = [...more.ours, ...more.theirs].map(({ before }) => before.end);
                isGrowing = ends.length > 0;
                to = Math.max(to, ...ends);
            }

            // write what is unchanged before it, then what each side has there
            const old = sources.base.slice({ start: at, end: from });
            chunks.push({ kind: "same", base: old, ours: old, theirs: old });
            const chunk = {
                base: sources.base.slice({ start: from, end: to }),
                ours: sideText(sources, "ours", taken.ours, { start: from, end: to }),
                theirs: sideText(sources, "theirs", taken.theirs, { start: from, end: to }),
            };
            chunks.push(...settle(chunk));
            at = to;
        }
        const rest = sources.base.slice({ start: at, end: sources.base.lines });
        chunks.push({ kind: "same", base: rest, ours: rest, theirs: rest });

        return new Merge(chunks.filter((chunk) => chunk.base + chunk.ours + chunk.theirs !== ""));
    }

    /** Read a source with the marks of conflicts, which git leaves in files. */
    static read(source: Source): Merge {
        return new Merge(readChunks(source));
    }

    /** The chunks both sides changed differently. */
    get conflicts(): readonly Chunk[] {
        return this.chunks.filter((chunk) => chunk.kind === "conflict");
    }

    /** Resolve a conflict to a side, or to a text. */
    resolve(conflict: Chunk, resolution: Resolution): Merge {
        const text =
            typeof resolution === "object"
                ? resolution.text
                : resolution === "both"
                  ? conflict.ours + conflict.theirs
                  : conflict[resolution];

        return new Merge(
            this.chunks.map((chunk) =>
                chunk === conflict ? { kind: "same", base: text, ours: text, theirs: text } : chunk,
            ),
        );
    }

    /** Write the text, conflicts between marks as git writes them. */
    toString(labels: MergeLabels = {}): string {
        return this.chunks
            .map((chunk) =>
                chunk.kind === "conflict"
                    ? writeConflict(chunk, labels)
                    : chunk.kind === "ours"
                      ? chunk.ours
                      : chunk.kind === "theirs"
                        ? chunk.theirs
                        : chunk.base,
            )
            .join("");
    }
}

/** The changes of one side, taken in order. */
class Side {
    /** The changes left. */
    readonly #changes: Change[];

    /** Keep the changes of a side. */
    constructor(changes: readonly Change[]) {
        this.#changes = [...changes];
    }

    /** The base line the next change starts at, absent for none. */
    get next(): number | undefined {
        return this.#changes[0]?.before.start;
    }

    /** Take the changes that touch some base lines. */
    take(from: number, to: number): Change[] {
        const taken: Change[] = [];
        while (this.next !== undefined && this.next >= from && this.next <= to) {
            taken.push(...this.#changes.splice(0, 1));
        }

        return taken;
    }
}

/** Find the base line the next change of either side starts at. */
function nextOf(sides: { readonly ours: Side; readonly theirs: Side }): number | undefined {
    const starts = [sides.ours.next, sides.theirs.next].filter((start) => start !== undefined);

    return starts.length === 0 ? undefined : Math.min(...starts);
}

/** Read what a side has in place of some base lines. */
function sideText(
    sources: Readonly<Record<"base" | "ours" | "theirs", Source>>,
    side: "ours" | "theirs",
    changes: readonly Change[],
    { start, end }: LineRange,
): string {
    // write the changes with the base lines between them
    const { base } = sources;
    const source = sources[side];
    let at = start;
    let written = "";
    for (const { before, after } of changes) {
        written += base.slice({ start: at, end: before.start }) + source.slice(after);
        at = before.end;
    }

    return written + base.slice({ start: at, end });
}

/** Decide a chunk: one side changed it, both did the same, or they conflict in its middle. */
function settle(chunk: Omit<Chunk, "kind">): Chunk[] {
    // take what one side changed, or both the same way
    const { base, ours, theirs } = chunk;
    if (ours === theirs || base === theirs) {
        return [{ ...chunk, kind: "ours" }];
    } else if (base === ours) {
        return [{ ...chunk, kind: "theirs" }];
    }

    // leave the lines both sides start or end with out of the conflict
    const [left, right] = [Source.of(ours), Source.of(theirs)];
    const shorter = Math.min(left.lines, right.lines);
    let start = 0;
    while (start < shorter && lineOf(left, start) === lineOf(right, start)) {
        start += 1;
    }
    let end = 0;
    while (
        end < shorter - start &&
        lineOf(left, left.lines - 1 - end) === lineOf(right, right.lines - 1 - end)
    ) {
        end += 1;
    }
    const [leftEnd, rightEnd] = [left.lines - end, right.lines - end];

    return [
        sharedChunk(ours.slice(0, left.start(start))),
        {
            kind: "conflict",
            base,
            ours: ours.slice(left.start(start), left.start(leftEnd)),
            theirs: theirs.slice(right.start(start), right.start(rightEnd)),
        },
        sharedChunk(ours.slice(left.start(leftEnd))),
    ];
}

/** Write a conflict between git's marks. */
function writeConflict(chunk: Chunk, labels: MergeLabels): string {
    return [
        writeMark("<", labels.ours),
        close(chunk.ours),
        writeMark("|", labels.base),
        close(chunk.base),
        writeMark("=", undefined),
        close(chunk.theirs),
        writeMark(">", labels.theirs),
    ].join("");
}

/** Read the chunks of a text with marks, refusing a conflict that does not end. */
function readChunks(source: Source): Chunk[] {
    // follow which side each line is of
    const chunks: Chunk[] = [];
    let same = "";
    let conflict: { base: string; ours: string; theirs: string } | undefined;
    let side: "base" | "ours" | "theirs" = "ours";
    for (let line = 0; line < source.lines; line += 1) {
        const written = source.slice({ start: line, end: line + 1 });
        if (/^<{7}(?: |\n)/u.test(written) && conflict === undefined) {
            chunks.push({ kind: "same", base: same, ours: same, theirs: same });
            same = "";
            conflict = { base: "", ours: "", theirs: "" };
            side = "ours";
        } else if (conflict !== undefined && /^\|{7}(?: |\n)/u.test(written)) {
            side = "base";
        } else if (conflict !== undefined && /^={7}\n/u.test(written)) {
            side = "theirs";
        } else if (conflict !== undefined && /^>{7}(?: |\n|$)/u.test(written)) {
            chunks.push({ kind: "conflict", ...conflict });
            conflict = undefined;
        } else if (conflict === undefined) {
            same += written;
        } else {
            conflict[side] += written;
        }
    }
    if (conflict !== undefined) {
        throw new TypeError("merged text is invalid: its last conflict does not end");
    }
    chunks.push({ kind: "same", base: same, ours: same, theirs: same });

    return chunks.filter((chunk) => chunk.kind === "conflict" || chunk.base !== "");
}

/** Read a line of a source with its line break. */
function lineOf(source: Source, line: number): string {
    return source.slice({ start: line, end: line + 1 });
}

/** Make a chunk both sides have alike. */
function sharedChunk(text: string): Chunk {
    return { kind: "ours", base: "", ours: text, theirs: text };
}

/** End the last line of a text, which a mark follows. */
function close(text: string): string {
    return text === "" || text.endsWith("\n") ? text : `${text}\n`;
}

/** Write a mark of a conflict with its label. */
function writeMark(sign: string, label: string | undefined): string {
    return `${sign.repeat(7)}${label === undefined ? "" : ` ${label}`}\n`;
}
