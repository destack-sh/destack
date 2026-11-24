import type { Hunk, HunkLine, LineKind } from "../difference/index.ts";
import { FilePatch, type FileStatus } from "./file.ts";

/** The line a file's changes start with. */
const FILE = /^diff --git (?<paths>.+)$/u;

/** The line a hunk starts with. */
const HUNK =
    /^@@ -(?<beforeFrom>\d+)(?:,(?<beforeCount>\d+))? \+(?<afterFrom>\d+)(?:,(?<afterCount>\d+))? @@ ?(?<heading>.*)$/u;

/** The kind of line each mark stands for. */
const KINDS: ReadonlyMap<string, LineKind> = new Map<string, LineKind>([
    [" ", "same"],
    ["+", "added"],
    ["-", "removed"],
]);

/** What a file's header says so far. */
interface Header {
    /** What happens to the file. */
    status: FileStatus;
    /** The path before. */
    before: string | undefined;
    /** The path after. */
    after: string | undefined;
    /** The mode before. */
    beforeMode: string | undefined;
    /** The mode after. */
    afterMode: string | undefined;
    /** Whether the file is no text. */
    isBinary: boolean;
}

/** Reads a patch line by line. */
export class PatchReader {
    /** The lines. */
    readonly #lines: readonly string[];
    /** The line read next. */
    #at = 0;

    /** Read a patch. */
    constructor(text: string) {
        this.#lines = text.replace(/\n$/u, "").split("\n");
    }

    /** Read the changes of every file, leaving out what stands around them. */
    files(): FilePatch[] {
        const files: FilePatch[] = [];
        while (this.#at < this.#lines.length) {
            // start at git's line, or at the paths of a plain patch
            const line = this.#lines[this.#at] ?? "";
            const isPlain =
                line.startsWith("--- ") && this.#lines[this.#at + 1]?.startsWith("+++ ") === true;
            if (FILE.test(line) || isPlain) {
                files.push(this.#file());
            } else {
                this.#at += 1;
            }
        }

        return files;
    }

    /** Read the changes of a file. */
    #file(): FilePatch {
        // take the paths of git's line, which the lines after it make precise
        const header: Header = {
            status: "changed",
            before: undefined,
            after: undefined,
            beforeMode: undefined,
            afterMode: undefined,
            isBinary: false,
        };
        const paths = FILE.exec(this.#lines[this.#at] ?? "")?.groups?.["paths"];
        if (paths !== undefined) {
            const [before, after] = splitPaths(paths);
            header.before = before;
            header.after = after;
            this.#at += 1;
        }
        while (this.#at < this.#lines.length && this.#head(this.#lines[this.#at] ?? "", header)) {
            this.#at += 1;
        }

        // read its hunks
        const hunks: Hunk[] = [];
        while (HUNK.test(this.#lines[this.#at] ?? "")) {
            hunks.push(this.#hunk());
        }

        return new FilePatch({
            ...header,
            before: header.status === "added" ? undefined : header.before,
            after: header.status === "removed" ? undefined : header.after,
            hunks,
        });
    }

    /** Take a line of a file's header, reporting whether it is one. */
    #head(line: string, header: Header): boolean {
        // read each line git writes between the paths and the hunks
        const rest = (start: string): string => line.slice(start.length);
        if (line.startsWith("new file mode ")) {
            header.status = "added";
            header.afterMode = rest("new file mode ");
        } else if (line.startsWith("deleted file mode ")) {
            header.status = "removed";
            header.beforeMode = rest("deleted file mode ");
        } else if (line.startsWith("old mode ")) {
            header.beforeMode = rest("old mode ");
        } else if (line.startsWith("new mode ")) {
            header.afterMode = rest("new mode ");
        } else if (line.startsWith("rename from ")) {
            header.status = "renamed";
            header.before = unquote(rest("rename from "));
        } else if (line.startsWith("rename to ")) {
            header.after = unquote(rest("rename to "));
        } else if (line.startsWith("copy from ")) {
            header.status = "copied";
            header.before = unquote(rest("copy from "));
        } else if (line.startsWith("copy to ")) {
            header.after = unquote(rest("copy to "));
        } else if (line.startsWith("Binary files ") || line === "GIT binary patch") {
            header.isBinary = true;
        } else if (line.startsWith("--- ")) {
            this.#side(header, "before", rest("--- "));
        } else if (line.startsWith("+++ ")) {
            this.#side(header, "after", rest("+++ "));
        } else {
            return /^(?:index|similarity index|dissimilarity index) /u.test(line);
        }

        return true;
    }

    /** Take the path of one side, none of which says the file was added or removed. */
    #side(header: Header, side: "before" | "after", written: string): void {
        const path = unquote(written.split("\t", 1)[0] ?? "");
        if (path === "/dev/null") {
            header.status = side === "before" ? "added" : "removed";
        } else if (header.status !== "renamed" && header.status !== "copied") {
            header[side] = path.replace(/^[ab]\//u, "");
        }
    }

    /** Read a hunk, refusing one whose lines do not add up. */
    #hunk(): Hunk {
        // read how many lines each side has
        const { groups = {} } = HUNK.exec(this.#lines[this.#at] ?? "") ?? {};
        const counts = {
            before: Number(groups["beforeCount"] ?? 1),
            after: Number(groups["afterCount"] ?? 1),
        };
        this.#at += 1;

        // read as many
        const lines: HunkLine[] = [];
        let left = { ...counts };
        while (left.before > 0 || left.after > 0) {
            const line = this.#lines[this.#at];
            const kind = line === "" ? "same" : KINDS.get(line?.[0] ?? "");
            if (line === undefined || kind === undefined) {
                throw new TypeError(
                    `patch is invalid: line '${String(this.#at + 1)}' is outside its hunk`,
                );
            }
            lines.push({ kind, text: line.slice(1), hasLineBreak: true });
            left = {
                before: left.before - (kind === "added" ? 0 : 1),
                after: left.after - (kind === "removed" ? 0 : 1),
            };
            this.#at += 1;

            // take what follows a line its text ends with
            if (this.#lines[this.#at]?.startsWith("\\ ") === true) {
                lines[lines.length - 1] = { kind, text: line.slice(1), hasLineBreak: false };
                this.#at += 1;
            }
        }
        const [before, after] = [
            firstLine(groups["beforeFrom"], counts.before),
            firstLine(groups["afterFrom"], counts.after),
        ];
        const heading = groups["heading"];

        return {
            before: { start: before, end: before + counts.before },
            after: { start: after, end: after + counts.after },
            ...(heading === undefined || heading === "" ? {} : { heading }),
            lines,
        };
    }
}

/** Read the first line of a range, which git counts from one, and one less for an empty range. */
function firstLine(written: string | undefined, count: number): number {
    return Number(written ?? 0) - (count === 0 ? 0 : 1);
}

/** Split the two paths of git's line, the same one twice unless a later line says otherwise. */
function splitPaths(written: string): readonly [string, string] {
    // read quoted paths, else the half of the line each takes
    const quoted = /^(?<before>"(?:[^"\\]|\\.)*"|\S+) (?<after>"(?:[^"\\]|\\.)*"|\S+)$/u.exec(
        written,
    );
    const half = (written.length - 1) / 2;
    const [before, after] =
        quoted === null
            ? [written.slice(0, half), written.slice(half + 1)]
            : [quoted.groups?.["before"] ?? "", quoted.groups?.["after"] ?? ""];

    return [unquote(before).replace(/^a\//u, ""), unquote(after).replace(/^b\//u, "")];
}

/** Read a path git quoted, its bytes written in octal. */
function unquote(path: string): string {
    // leave a plain path
    if (!path.startsWith('"') || !path.endsWith('"')) {
        return path;
    }

    // read each byte
    const escapes = new Map([
        ["n", "\n"],
        ["t", "\t"],
        ["r", "\r"],
    ]);
    const bytes: number[] = [];
    const encoder = new TextEncoder();
    for (const match of path
        .slice(1, -1)
        .matchAll(/\\(?<octal>[0-7]{3})|\\(?<mark>.)|(?<text>[^\\]+)/gu)) {
        const { octal, mark, text } = match.groups ?? {};
        if (octal !== undefined) {
            bytes.push(Number.parseInt(octal, 8));
        } else {
            bytes.push(...encoder.encode(text ?? escapes.get(mark ?? "") ?? mark ?? ""));
        }
    }

    return new TextDecoder().decode(Uint8Array.from(bytes));
}
