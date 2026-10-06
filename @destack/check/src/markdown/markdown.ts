import type { Diagnostic } from "../inspect/diagnostic.ts";

/** Words whose trailing period does not end a sentence. */
const ABBREVIATIONS = /(?:^|[\s(])(?:e\.g|i\.e|etc|vs|cf|approx|incl|Mr|Ms|Dr|St|No)\.$/u;

/** A Markdown rule failure before it is placed in its file. */
interface Finding {
    /** The rule identifier. */
    readonly code: string;
    /** The user-facing explanation. */
    readonly message: string;
    /** The zero-based line index. */
    readonly line: number;
    /** The zero-based character column. */
    readonly column: number;
    /** The zero-based character offset in the file. */
    readonly offset: number;
}

/** The region of a Markdown file a line belongs to. */
type Region = "prose" | "front matter" | "fence" | "comment";

/** What a line of a package README section holds. */
type Block = "prose" | "listing" | "table";

/** A Markdown source line and the region holding it. */
interface SourceLine {
    /** The line text. */
    readonly text: string;
    /** The character offset where the line starts. */
    readonly start: number;
    /** The region holding the line. */
    readonly region: Region;
}

/** Check Markdown against the Destack prose rules, returning diagnostics with UTF-8 spans. */
export function checkMarkdown(filename: string, text: string): Diagnostic[] {
    // run each rule over the classified lines
    const lines = classifyLines(text);
    const findings = [
        ...checkSentences(lines),
        ...checkHeadings(lines, /^---\n(?:.*\n)*?title:/u.test(text)),
        ...checkFences(lines),
        ...(/(?:^|\/)README\.md$/u.test(filename) ? checkReadme(lines) : []),
    ];

    // convert character positions to UTF-8 byte offsets
    return findings
        .toSorted((left, right) => left.offset - right.offset)
        .map((finding) => {
            const prefix = text.slice(0, finding.offset);

            return {
                code: `markdown(${finding.code})`,
                message: finding.message,
                severity: "error" as const,
                filename,
                labels: [
                    {
                        span: {
                            offset: Buffer.byteLength(prefix),
                            length: 1,
                            line: finding.line + 1,
                            column: finding.column + 1,
                        },
                    },
                ],
            };
        });
}

/** Split prose lines at sentence breaks, keeping indentation and list continuation. */
export function fixMarkdown(text: string): string {
    const lines = classifyLines(text);

    return lines
        .map((line) => {
            // leave verbatim content and single-sentence lines unchanged
            const breaks = line.region === "prose" ? sentenceBoundaries(line.text) : [];
            if (!breaks.length) {
                return line.text;
            }

            // continue each sentence at the indentation of the line's content
            const match = /^(\s*(?:[-*+]|\d+\.)\s+|\s*>\s*|\s*)/u.exec(line.text);
            if (match === null) {
                throw new TypeError("the line marker pattern matches every line");
            }
            const [marker] = match;
            const indent = /^\s*>/u.test(marker) ? marker : " ".repeat(marker.length);
            const sentences: string[] = [];
            let start = 0;
            for (const position of breaks) {
                sentences.push(line.text.slice(start, position).trimEnd());
                start = position;
            }
            sentences.push(line.text.slice(start));

            return sentences
                .map((sentence, index) => (index ? indent + sentence.trimStart() : sentence))
                .join("\n");
        })
        .join("\n");
}

/** Mark front matter, fenced code and HTML comment lines as verbatim. */
function classifyLines(text: string): SourceLine[] {
    // start outside any region, or inside front matter when the file opens with it
    const lines: SourceLine[] = [];
    let fence: string | undefined;
    let isFrontMatter = text.startsWith("---\n");
    let isComment = false;
    let start = 0;
    for (const [index, line] of text.split("\n").entries()) {
        // track the region each line belongs to
        const trimmed = line.trim();
        const opensFence = /^(```|~~~)/u.exec(trimmed)?.[1];
        let region: Region = "prose";
        if (isFrontMatter) {
            region = "front matter";
        } else if (isComment) {
            region = "comment";
        } else if (fence !== undefined || opensFence !== undefined) {
            region = "fence";
        } else if (trimmed.startsWith("<!--")) {
            region = "comment";
        }
        lines.push({ text: line, start, region });
        start += line.length + 1;

        // leave the region at its closing line
        if (isFrontMatter && index > 0 && trimmed === "---") {
            isFrontMatter = false;
        } else if (fence !== undefined && trimmed.startsWith(fence)) {
            fence = undefined;
        } else if (
            fence === undefined &&
            opensFence !== undefined &&
            !isFrontMatter &&
            !isComment
        ) {
            fence = opensFence;
        }
        if (trimmed.startsWith("<!--") && !trimmed.includes("-->")) {
            isComment = true;
        } else if (isComment && trimmed.includes("-->")) {
            isComment = false;
        }
    }

    return lines;
}

/** Report prose lines that hold more than one sentence. */
function checkSentences(lines: readonly SourceLine[]): Finding[] {
    return lines.flatMap((line, index) =>
        line.region === "prose"
            ? sentenceBoundaries(line.text)
                  .slice(0, 1)
                  .map((column) => ({
                      code: "one-sentence-per-line",
                      message: "[WD10] start each sentence on its own line",
                      line: index,
                      column,
                      offset: line.start + column,
                  }))
            : [],
    );
}

/** Find where sentences after the first start on a prose line. */
function sentenceBoundaries(line: string): number[] {
    // skip headings, tables and link definitions
    if (/^\s*(?:#|\||\[[^\]]+\]:)/u.test(line)) {
        return [];
    }

    // blank inline code and link targets so their punctuation is ignored
    const masked = line
        .replace(/`[^`]*`/gu, (code) => "x".repeat(code.length))
        .replace(/\]\([^)]*\)/gu, (target) => "x".repeat(target.length));

    // start a sentence after terminal punctuation followed by a capital or code
    const breaks: number[] = [];
    for (const match of masked.matchAll(/[.?!]["')\]*_]*\s+(?=[*_"([]*[A-Z`])/gu)) {
        const end = match.index + match[0].length;
        const before = masked.slice(0, match.index + 1);
        const isListMarker = /^\s*\d+\.$/u.test(before);
        if (!isListMarker && !ABBREVIATIONS.test(before) && !/(?:^|\s)[A-Z]\.$/u.test(before)) {
            breaks.push(end);
        }
    }

    return breaks;
}

/** Report skipped heading levels, and more than one title when front matter names none. */
function checkHeadings(lines: readonly SourceLine[], hasTitle: boolean): Finding[] {
    // track the previous level and the number of titles
    const findings: Finding[] = [];
    let previous = 0;
    let titles = 0;
    for (const [index, line] of lines.entries()) {
        // read ATX heading levels outside verbatim regions
        const heading = line.region === "prose" ? /^#{1,6}(?=\s)/u.exec(line.text) : undefined;
        if (!heading) {
            continue;
        }
        const level = heading[0].length;

        // allow one title and one level deeper at a time
        titles += level === 1 ? 1 : 0;
        if (level === 1 && titles > 1 && !hasTitle) {
            findings.push({
                code: "single-title",
                message: "use one top-level heading per file",
                line: index,
                column: 0,
                offset: line.start,
            });
        } else if (previous && level > previous + 1) {
            findings.push({
                code: "heading-increment",
                message: `use heading level ${previous + 1} here`,
                line: index,
                column: 0,
                offset: line.start,
            });
        }
        previous = level;
    }

    return findings;
}

/** Report fenced code blocks without a language. */
function checkFences(lines: readonly SourceLine[]): Finding[] {
    // track whether the next fence opens or closes a block
    const findings: Finding[] = [];
    let isOpen = false;
    for (const [index, line] of lines.entries()) {
        // alternate between opening and closing fences
        const trimmed = line.text.trim();
        if (!/^(```|~~~)/u.test(trimmed)) {
            continue;
        }
        if (!isOpen && /^(```|~~~)\s*$/u.test(trimmed)) {
            findings.push({
                code: "fenced-code-language",
                message: "name the language of the code block",
                line: index,
                column: 0,
                offset: line.start,
            });
        }
        isOpen = !isOpen;
    }

    return findings;
}

/** Report package README sections other than one prose line followed by listings, and tables. */
function checkReadme(lines: readonly SourceLine[]): Finding[] {
    // track what the current section holds, starting with the opening line under the title
    const findings: Finding[] = [];
    let section: Block | undefined;
    let previous: Block | undefined;
    for (const [index, line] of lines.entries()) {
        // start a section at each heading
        if (line.region === "prose" && /^#{1,6}\s/u.test(line.text)) {
            section = undefined;
            previous = undefined;
            continue;
        }

        // skip blank lines and comments
        const block = readmeBlock(line, previous);
        if (block === undefined) {
            continue;
        }

        // report a table once at its first row
        const position = { line: index, column: 0, offset: line.start };
        if (block === "table" && previous !== "table") {
            const message = "replace the table with a listing";
            findings.push({ code: "readme-table", message, ...position });
        }
        // report a second prose line and prose after a listing
        else if (block === "prose" && section !== undefined) {
            const message =
                section === "prose"
                    ? "open the section with one prose line, then a listing"
                    : "move the prose after the listing into its own section";
            findings.push({ code: "readme-section", message, ...position });
        }

        // keep the section's first block until a listing follows its prose
        section = section === undefined || block !== "prose" ? block : section;
        previous = block;
    }

    return findings;
}

/** Classify a package README line as prose, a listing or a table, absent for blank lines and comments. */
function readmeBlock(line: SourceLine, previous: Block | undefined): Block | undefined {
    // read code blocks as listings and skip front matter, comments and blank lines
    if (line.region === "fence") {
        return "listing";
    } else if (line.region !== "prose" || !line.text.trim()) {
        return undefined;
    }

    // read list items and their indented continuations as listings
    if (
        /^\s*(?:[-*+]|\d+\.)\s/u.test(line.text) ||
        (previous === "listing" && /^\s/u.test(line.text))
    ) {
        return "listing";
    }

    return /^\s*\|/u.test(line.text) ? "table" : "prose";
}
