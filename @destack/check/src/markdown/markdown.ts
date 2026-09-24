import type { Diagnostic } from "../inspect/diagnostic.ts";

/** Words whose trailing period does not end a sentence. */
const ABBREVIATIONS = /(?:^|[\s(])(?:e\.g|i\.e|etc|vs|cf|approx|incl|Mr|Ms|Dr|St|No)\.$/;

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
}

/** A Markdown source line and whether it holds prose. */
interface SourceLine {
    /** The line text. */
    readonly text: string;
    /** Whether the line is inside front matter, a fence or an HTML comment. */
    readonly isVerbatim: boolean;
}

/** Check Markdown against the Destack prose rules, returning diagnostics with UTF-8 spans. */
export function checkMarkdown(filename: string, text: string): Diagnostic[] {
    // run each rule over the classified lines
    const lines = classifyLines(text);
    const findings = [
        ...checkSentences(lines),
        ...checkHeadings(lines, /^---\n(?:.*\n)*?title:/.test(text)),
        ...checkFences(lines),
    ];

    // convert character positions to UTF-8 byte offsets
    const starts = lineStarts(text);

    return findings
        .sort((left, right) => left.line - right.line || left.column - right.column)
        .map((finding) => {
            const prefix = text.slice(0, starts[finding.line] + finding.column);

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
            const breaks = line.isVerbatim ? [] : sentenceBoundaries(line.text);
            if (!breaks.length) {
                return line.text;
            }

            // continue each sentence at the indentation of the line's content
            const marker = /^(\s*(?:[-*+]|\d+\.)\s+|\s*>\s*|\s*)/.exec(line.text)![0];
            const indent = /^\s*>/.test(marker) ? marker : " ".repeat(marker.length);
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
    for (const [index, line] of text.split("\n").entries()) {
        // track the region each line belongs to
        const trimmed = line.trim();
        const opensFence = /^(```|~~~)/.exec(trimmed)?.[1];
        const isVerbatim =
            isFrontMatter ||
            fence !== undefined ||
            isComment ||
            opensFence !== undefined ||
            trimmed.startsWith("<!--");
        lines.push({ text: line, isVerbatim });

        // leave the region at its closing line
        if (isFrontMatter && index > 0 && trimmed === "---") {
            isFrontMatter = false;
        } else if (fence !== undefined && trimmed.startsWith(fence)) {
            fence = undefined;
        } else if (fence === undefined && opensFence && !isFrontMatter && !isComment) {
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
        line.isVerbatim
            ? []
            : sentenceBoundaries(line.text)
                  .slice(0, 1)
                  .map((column) => ({
                      code: "one-sentence-per-line",
                      message: "[WD10] start each sentence on its own line",
                      line: index,
                      column,
                  })),
    );
}

/** Find where sentences after the first start on a prose line. */
function sentenceBoundaries(line: string): number[] {
    // skip headings, tables and link definitions
    if (/^\s*(?:#|\||\[[^\]]+\]:)/.test(line)) {
        return [];
    }

    // blank inline code and link targets so their punctuation is ignored
    const masked = line
        .replace(/`[^`]*`/g, (code) => "x".repeat(code.length))
        .replace(/\]\([^)]*\)/g, (target) => "x".repeat(target.length));

    // start a sentence after terminal punctuation followed by a capital or code
    const breaks: number[] = [];
    for (const match of masked.matchAll(/[.?!]["')\]*_]*\s+(?=[*_"([]*[A-Z`])/g)) {
        const end = match.index + match[0].length;
        const before = masked.slice(0, match.index + 1);
        const isListMarker = /^\s*\d+\.$/.test(before);
        if (!isListMarker && !ABBREVIATIONS.test(before) && !/(?:^|\s)[A-Z]\.$/.test(before)) {
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
        const heading = line.isVerbatim ? undefined : /^(#{1,6})\s/.exec(line.text);
        if (!heading) {
            continue;
        }
        const level = heading[1].length;

        // allow one title and one level deeper at a time
        titles += level === 1 ? 1 : 0;
        if (level === 1 && titles > 1 && !hasTitle) {
            findings.push({
                code: "single-title",
                message: "use one top-level heading per file",
                line: index,
                column: 0,
            });
        } else if (previous && level > previous + 1) {
            findings.push({
                code: "heading-increment",
                message: `use heading level ${previous + 1} here`,
                line: index,
                column: 0,
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
        if (!/^(```|~~~)/.test(trimmed)) {
            continue;
        }
        if (!isOpen && /^(```|~~~)\s*$/.test(trimmed)) {
            findings.push({
                code: "fenced-code-language",
                message: "name the language of the code block",
                line: index,
                column: 0,
            });
        }
        isOpen = !isOpen;
    }

    return findings;
}

/** Return the character index where each line starts. */
function lineStarts(text: string): number[] {
    const starts = [0];
    for (let index = 0; index < text.length; index++) {
        if (text[index] === "\n") {
            starts.push(index + 1);
        }
    }

    return starts;
}
