/** A run of a mark's characters, which opens or closes it. */
const MARKER = /(\\.)|(`+)|(\*\*\*|\*\*|\*|~~)|(?<![\p{L}\p{N}])(__|_)|(__|_)(?![\p{L}\p{N}])/gu;

/** The lines that open and close a block of code. */
const FENCE = /^\s*(?:`{3,}|~{3,})/u;

/** Close the marks a block has left open, as it will most likely go on. */
export function closed(text: string): string {
    // leave code as it is, which needs no end, and what comes before finished code
    const lines = text.split("\n");
    const fences = lines.flatMap((line, index) => (FENCE.test(line) ? [index] : []));
    const after = (fences.at(-1) ?? -1) + 1;
    if (fences.length % 2 === 1 || /^(?: {4}|\t)/u.test(text)) {
        return text;
    } else if (after > 0) {
        const before = lines.slice(0, after).join("\n");

        return after === lines.length
            ? before
            : `${before}\n${closed(lines.slice(after).join("\n"))}`;
    }

    // leave out a table before the line under its header
    if (/^ {0,3}\|/u.test(text) && !/^[^\n]*\n[ |:-]*-[ |:-]*\n?/u.test(text)) {
        return "";
    }

    // leave out a last line that may turn out to underline a heading, and a lone mark
    const last = lines.at(-1) ?? "";
    const isUnderline = lines.length > 1 && /^ {0,3}(?:-{1,2}|=+)$/u.test(last);
    const isLone = /^\s*(?:[-*+]|\d{1,9}[.)]|#{1,6}|>)?\s*$/u.test(last);
    const kept =
        isUnderline || (isLone && last.trim() !== "") ? lines.slice(0, -1).join("\n") : text;

    return closeMarks(closeLink(kept));
}

/** Show the words of a link that is not finished, and nothing of an image. */
function closeLink(text: string): string {
    // find the last bracket that nothing closes
    const image = /!\[[^\]]*(?:\](?:\([^)]*)?)?$/u.exec(text);
    if (image !== null && !/\]\([^)]*\)$/u.test(image[0])) {
        return text.slice(0, image.index);
    }
    const link = /\[([^\][]*)(?:\]\([^)]*)?$/u.exec(text);
    if (link === null || text.slice(0, link.index).endsWith("`")) {
        return text;
    }

    return text.slice(0, link.index) + (link[1] ?? "");
}

/** Close the marks that are open at the end, the innermost first. */
function closeMarks(text: string): string {
    // open and close each mark in turn, code hiding the others
    const open: { readonly mark: string; readonly end: number }[] = [];
    for (const match of text.matchAll(MARKER)) {
        const [mark, escaped, code] = match;
        const isInCode = open.at(-1)?.mark.startsWith("`") === true;
        if (escaped !== undefined || (isInCode && code === undefined)) {
            continue;
        }
        const at = open.findLastIndex((each) => each.mark === mark);
        const after = text[match.index + mark.length];
        const isOpening = code !== undefined || (after !== undefined && !/\s/u.test(after));
        if (at !== -1 && (code !== undefined || !/\s/u.test(text[match.index - 1] ?? " "))) {
            open.length = at;
        } else if (isOpening || after === undefined) {
            open.push({ mark, end: match.index + mark.length });
        }
    }

    // leave out marks at the very end, which mark nothing yet
    let shown = text.replace(/(?<!~)~$/u, "");
    while (open.at(-1)?.end === shown.length) {
        shown = shown.slice(0, -(open.pop()?.mark.length ?? 0));
    }

    return (
        shown.trimEnd() +
        open
            .toReversed()
            .map(({ mark }) => mark)
            .join("")
    );
}
