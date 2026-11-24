import {
    Difference,
    type DifferenceOptions,
    type Hunk,
    type HunkLine,
} from "../difference/index.ts";
import type { LineKind } from "../difference/index.ts";
import { type LineRange, Source } from "../source/index.ts";

/** The mark before a line of each kind. */
const MARKS: Readonly<Record<LineKind, string>> = { same: " ", added: "+", removed: "-" };

/** The kind each kind of line has in the other direction. */
const OPPOSITES: Readonly<Record<LineKind, LineKind>> = {
    same: "same",
    added: "removed",
    removed: "added",
};

/** What git writes under a line its text ends with. */
const OPEN_END = "\\ No newline at end of file";

/** What a patch does to a file. */
export type FileStatus = "added" | "removed" | "renamed" | "copied" | "changed";

/** What a patch says of a file. */
export interface FilePatchOptions {
    /** What happens to the file. */
    readonly status: FileStatus;
    /** The path before, absent for an added file. */
    readonly before: string | undefined;
    /** The path after, absent for a removed file. */
    readonly after: string | undefined;
    /** The mode before, such as `100644`, absent where it stays or is not known. */
    readonly beforeMode?: string | undefined;
    /** The mode after, absent where it stays or is not known. */
    readonly afterMode?: string | undefined;
    /** Whether the file is no text, whose content the patch leaves out. */
    readonly isBinary?: boolean;
    /** The changed lines. */
    readonly hunks: readonly Hunk[];
}

/** A source with the hunks that did not fit it. */
export interface PatchResult {
    /** The source with every hunk that fit. */
    readonly source: Source;
    /** The hunks that did not fit. */
    readonly rejected: readonly Hunk[];
}

/** The changes of one file. */
export class FilePatch {
    /** What happens to the file. */
    readonly status: FileStatus;
    /** The path before, absent for an added file. */
    readonly before: string | undefined;
    /** The path after, absent for a removed file. */
    readonly after: string | undefined;
    /** The mode before, absent where it stays or is not known. */
    readonly beforeMode: string | undefined;
    /** The mode after, absent where it stays or is not known. */
    readonly afterMode: string | undefined;
    /** Whether the file is no text, whose content the patch leaves out. */
    readonly isBinary: boolean;
    /** The changed lines. */
    readonly hunks: readonly Hunk[];

    /** Keep the changes of a file. */
    constructor(options: FilePatchOptions) {
        // take what is said of it
        this.status = options.status;
        this.before = options.before;
        this.after = options.after;
        this.beforeMode = options.beforeMode;
        this.afterMode = options.afterMode;
        this.isBinary = options.isBinary ?? false;
        this.hunks = options.hunks;
    }

    /** Write the changes between two sources of a file, an absent source for no file. */
    static between(
        path: string,
        before: Source | undefined,
        after: Source | undefined,
        options: DifferenceOptions = {},
    ): FilePatch {
        const empty = Source.of("");

        return new FilePatch({
            status: before === undefined ? "added" : after === undefined ? "removed" : "changed",
            before: before === undefined ? undefined : path,
            after: after === undefined ? undefined : path,
            hunks: Difference.of(before ?? empty, after ?? empty, options).hunks(),
        });
    }

    /** The path people know the file by: the one after, else the one before. */
    get path(): string {
        return this.after ?? this.before ?? "";
    }

    /** The count of lines added. */
    get added(): number {
        return countOf(this.hunks, "added");
    }

    /** The count of lines removed. */
    get removed(): number {
        return countOf(this.hunks, "removed");
    }

    /** Apply the hunks to a source, each where its lines are, nearest to where they were. */
    apply(source: Source): PatchResult {
        // keep each line with its line break
        const lines = linesOf(source);
        const rejected: Hunk[] = [];
        let shift = 0;
        for (const hunk of this.hunks) {
            // find the old lines, and put the new ones in their place
            const old = sideOf(hunk, "removed");
            const next = sideOf(hunk, "added");
            const place = placeOf(lines, old, hunk.before.start + shift);
            if (place === undefined) {
                rejected.push(hunk);
            } else {
                lines.splice(place, old.length, ...next);
                shift = place - hunk.before.start + next.length - old.length;
            }
        }

        return { source: Source.of(lines.join("")), rejected };
    }

    /** Leave a hunk out. */
    without(hunk: Hunk): FilePatch {
        const { status, before, after, beforeMode, afterMode, isBinary } = this;
        const hunks = this.hunks.filter((other) => other !== hunk);

        return new FilePatch({ status, before, after, beforeMode, afterMode, isBinary, hunks });
    }

    /** Turn the changes around, which takes them back. */
    invert(): FilePatch {
        const { status } = this;

        return new FilePatch({
            status: status === "added" ? "removed" : status === "removed" ? "added" : status,
            before: this.after,
            after: this.before,
            beforeMode: this.afterMode,
            afterMode: this.beforeMode,
            isBinary: this.isBinary,
            hunks: this.hunks.map(invertHunk),
        });
    }

    /** Write the changes in git's format. */
    toString(): string {
        return writeFile(this);
    }
}

/** Turn a hunk around, removed lines staying before added ones. */
function invertHunk(hunk: Hunk): Hunk {
    // gather the lines between two unchanged ones, removed ones first
    const lines: HunkLine[] = [];
    let added: HunkLine[] = [];
    for (const line of hunk.lines) {
        const turned = { ...line, kind: OPPOSITES[line.kind] };
        if (turned.kind === "added") {
            added.push(turned);
        } else if (turned.kind === "same") {
            lines.push(...added, turned);
            added = [];
        } else {
            lines.push(turned);
        }
    }

    return { ...hunk, before: hunk.after, after: hunk.before, lines: [...lines, ...added] };
}

/** Write a file's changes in git's format. */
function writeFile(file: FilePatch): string {
    // say what happens to the file
    const before = file.before ?? file.after ?? "";
    const after = file.after ?? file.before ?? "";
    const lines = [`diff --git ${quote(`a/${before}`)} ${quote(`b/${after}`)}`];
    if (file.status === "added") {
        lines.push(`new file mode ${file.afterMode ?? "100644"}`);
    } else if (file.status === "removed") {
        lines.push(`deleted file mode ${file.beforeMode ?? "100644"}`);
    } else if (file.beforeMode !== undefined && file.afterMode !== undefined) {
        lines.push(`old mode ${file.beforeMode}`, `new mode ${file.afterMode}`);
    }
    if (file.status === "renamed") {
        lines.push(`rename from ${quote(before)}`, `rename to ${quote(after)}`);
    } else if (file.status === "copied") {
        lines.push(`copy from ${quote(before)}`, `copy to ${quote(after)}`);
    }

    // write its hunks
    const [old, next] = [
        file.before === undefined ? "/dev/null" : quote(`a/${file.before}`),
        file.after === undefined ? "/dev/null" : quote(`b/${file.after}`),
    ];
    if (file.isBinary) {
        lines.push(`Binary files ${old} and ${next} differ`);
    } else if (file.hunks.length > 0) {
        lines.push(`--- ${old}`, `+++ ${next}`, ...file.hunks.flatMap(writeHunk));
    }

    return `${lines.join("\n")}\n`;
}

/** Write a hunk's lines in git's format. */
function writeHunk(hunk: Hunk): string[] {
    const heading = hunk.heading === undefined ? "" : ` ${hunk.heading}`;

    return [
        `@@ -${writeRange(hunk.before)} +${writeRange(hunk.after)} @@${heading}`,
        ...hunk.lines.flatMap((line) => [
            `${MARKS[line.kind]}${line.text}`,
            ...(line.hasLineBreak ? [] : [OPEN_END]),
        ]),
    ];
}

/** Write a range of lines as git does: the first line from one, and how many where not one. */
function writeRange({ start, end }: LineRange): string {
    const count = end - start;

    return count === 1
        ? String(start + 1)
        : `${String(count === 0 ? start : start + 1)},${String(count)}`;
}

/** Quote a path with marks git escapes. */
function quote(path: string): string {
    return /["\\\n\t]/u.test(path) ? JSON.stringify(path) : path;
}

/** List the lines of a source, each with its line break. */
function linesOf(source: Source): string[] {
    return Array.from({ length: source.lines }, (_, line) =>
        source.slice({ start: line, end: line + 1 }),
    );
}

/** Count the lines of a kind in some hunks. */
function countOf(hunks: readonly Hunk[], kind: LineKind): number {
    return hunks.reduce(
        (sum, hunk) => sum + hunk.lines.filter((line) => line.kind === kind).length,
        0,
    );
}

/** List the lines of one side of a hunk, each with its line break. */
function sideOf(hunk: Hunk, side: "added" | "removed"): string[] {
    return hunk.lines
        .filter((line) => line.kind === "same" || line.kind === side)
        .map((line) => (line.hasLineBreak ? `${line.text}\n` : line.text));
}

/** Find where some lines are, nearest to a place. */
function placeOf(
    lines: readonly string[],
    searched: readonly string[],
    near: number,
): number | undefined {
    // look ever further both ways
    const last = lines.length - searched.length;
    const isAt = (place: number): boolean =>
        place >= 0 &&
        place <= last &&
        searched.every((line, index) => lines[place + index] === line);
    for (let away = 0; away <= lines.length; away += 1) {
        if (isAt(near - away)) {
            return near - away;
        } else if (isAt(near + away)) {
            return near + away;
        }
    }

    return undefined;
}
