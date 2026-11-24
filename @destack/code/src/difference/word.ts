import type { Range } from "../source/index.ts";
import { compare } from "./compare.ts";

/** A word, a run of space, or a single mark. */
const WORD = /[\p{L}\p{N}_]+|\s+|[^\p{L}\p{N}_\s]/gu;

/** What makes a word more than space or a mark. */
const LETTER = /[\p{L}\p{N}_]/u;

/** The most pairs of lines compared to find which belong together, about a screen by a screen. */
const MOST_PAIRS = 2500;

/** The least share of their words two lines have in common to belong together. */
const LEAST_SHARE = 0.4;

/** Characters of one text that stand in place of characters of another. */
export interface WordChange {
    /** The characters replaced, or where none are. */
    readonly before: Range;
    /** The characters in their place, or where none are. */
    readonly after: Range;
}

/** A line whose words are compared. */
interface WordLine {
    /** The offset it starts at. */
    readonly from: number;
    /** The line. */
    readonly text: string;
    /** The number of each word. */
    readonly words: Int32Array;
    /** The offset in the line of each word and of its end. */
    readonly starts: readonly number[];
    /** The words that are more than space. */
    readonly weight: number;
}

/** A line as it is read: where it starts and what it says. */
export interface PlacedLine {
    /** The offset it starts at. */
    readonly from: number;
    /** The line without its line break. */
    readonly text: string;
}

/** Compare the words of the lines that belong together among removed and added ones. */
export function compareWords(
    removed: readonly PlacedLine[],
    added: readonly PlacedLine[],
): WordChange[] {
    // number the words of every line
    const numbers = new Map<string, number>();
    const olds = removed.map((line) => readLine(line, numbers));
    const news = added.map((line) => readLine(line, numbers));

    return pairsOf(olds, news).flatMap(([old, next]) => lineChanges(old, next));
}

/** Read a line's words. */
function readLine({ from, text }: PlacedLine, numbers: Map<string, number>): WordLine {
    // number each distinct word
    const starts: number[] = [];
    const words: number[] = [];
    let weight = 0;
    for (const match of text.matchAll(WORD)) {
        const [word] = match;
        const number = numbers.get(word) ?? numbers.size;
        numbers.set(word, number);
        words.push(number);
        starts.push(match.index);
        weight += word.trim() === "" ? 0 : 1;
    }

    return { from, text, words: Int32Array.from(words), starts: [...starts, text.length], weight };
}

/** Pair the lines that are most alike, in order. */
function pairsOf(
    olds: readonly WordLine[],
    news: readonly WordLine[],
): (readonly [WordLine, WordLine])[] {
    // pair lines by their place where there are too many to compare
    if (olds.length * news.length > MOST_PAIRS) {
        return olds.length === news.length
            ? olds.flatMap((old, place) => {
                  const next = news[place];

                  return next === undefined || alike(old, next) === 0 ? [] : [[old, next] as const];
              })
            : [];
    }

    // find the likeness of the best pairs up to each two lines
    const width = news.length + 1;
    const best = new Float64Array((olds.length + 1) * width);
    const likeness = olds.map((old) => news.map((next) => alike(old, next)));
    for (let row = olds.length - 1; row >= 0; row -= 1) {
        for (let column = news.length - 1; column >= 0; column -= 1) {
            const share = likeness[row]?.[column] ?? 0;
            best[row * width + column] = Math.max(
                best[(row + 1) * width + column] ?? 0,
                best[row * width + column + 1] ?? 0,
                share === 0 ? 0 : share + (best[(row + 1) * width + column + 1] ?? 0),
            );
        }
    }

    // take them from the start
    const pairs: (readonly [WordLine, WordLine])[] = [];
    let [row, column] = [0, 0];
    while (row < olds.length && column < news.length) {
        const share = likeness[row]?.[column] ?? 0;
        const here = best[row * width + column] ?? 0;
        const [old, next] = [olds[row], news[column]];
        const isPair = share > 0 && here === share + (best[(row + 1) * width + column + 1] ?? 0);
        if (isPair && old !== undefined && next !== undefined) {
            pairs.push([old, next]);
            row += 1;
            column += 1;
        } else if (here === (best[(row + 1) * width + column] ?? 0)) {
            row += 1;
        } else {
            column += 1;
        }
    }

    return pairs;
}

/** Read the share of their words two lines have in common, zero for too little. */
function alike(old: WordLine, next: WordLine): number {
    // count the words of one, then those of the other among them
    const counts = new Map<number, number>();
    for (const [place, word] of old.words.entries()) {
        if (old.text.slice(old.starts[place], old.starts[place + 1]).trim() !== "") {
            counts.set(word, (counts.get(word) ?? 0) + 1);
        }
    }
    let shared = 0;
    for (const word of next.words) {
        const left = counts.get(word) ?? 0;
        if (left > 0) {
            counts.set(word, left - 1);
            shared += 1;
        }
    }
    const share = (2 * shared) / Math.max(1, old.weight + next.weight);

    return share >= LEAST_SHARE ? share : 0;
}

/** Compare the words of two lines, joining changes with nothing but a mark or space between. */
function lineChanges(old: WordLine, next: WordLine): WordChange[] {
    // leave out how far the lines are indented, unless nothing else changed
    const isIndent = old.text.trim() === next.text.trim();
    const least = {
        before: old.from + (isIndent ? 0 : indentOf(old.text)),
        after: next.from + (isIndent ? 0 : indentOf(next.text)),
    };

    // compare the words
    const changes: WordChange[] = [];
    for (const { before, after } of compare(old.words, next.words)) {
        // place the words in both texts
        const change = {
            before: rangeOf(old, before.start, before.end, least.before),
            after: rangeOf(next, after.start, after.end, least.after),
        };
        if (change.before.from === change.before.to && change.after.from === change.after.to) {
            continue;
        }

        // join it to the one before
        const last = changes.at(-1);
        const between = old.text.slice(
            (last?.before.to ?? 0) - old.from,
            change.before.from - old.from,
        );
        if (last !== undefined && between.length <= 2 && !LETTER.test(between)) {
            changes[changes.length - 1] = {
                before: { from: last.before.from, to: change.before.to },
                after: { from: last.after.from, to: change.after.to },
            };
        } else {
            changes.push(change);
        }
    }

    return changes;
}

/** Count the characters of space a line starts with. */
function indentOf(text: string): number {
    return text.length - text.trimStart().length;
}

/** Find the offsets of some words of a line, none before the least one. */
function rangeOf(line: WordLine, start: number, end: number, least: number): Range {
    return {
        from: Math.max(least, line.from + (line.starts[start] ?? line.text.length)),
        to: Math.max(least, line.from + (line.starts[end] ?? line.text.length)),
    };
}
