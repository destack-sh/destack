import { DatabaseError } from "../error/error.ts";
import { type FoldedText, Tokenizer } from "./tokenizer.ts";

/** The most words someone searches for, a long sentence. */
const MAX_WORDS = 32;

/** The characters of an excerpt, two lines of a list. */
const EXCERPT_LENGTH = 132;

/** A stretch of a line an excerpt is cut from, to a space where there is one. */
const STRETCH = /.{1,400}(?:\s|$)|.{1,400}/gu;

/** What ends a line of a text: a line break, or the character that stands for a block or node. */
const LINE_END = /[\n\ufffc]+/u;

/** How well texts match, from a text that is the search to words somewhere inside them. */
export const SEARCH_SCORE = {
    /** The whole title. */
    whole: 10,
    /** The title's opening. */
    opening: 8,
    /** Words of the title. */
    words: 6,
    /** The starts of the title's words. */
    starts: 4,
    /** The starts of words across the texts. */
    across: 2,
    /** Characters inside the texts. */
    inside: 1,
} as const;

/** How much less a match of another text than the title scores. */
export const SEARCH_LATER = 1;

/** A match's place in a text: its first character, and the one after its last. */
export type SearchRange = readonly [start: number, end: number];

/** What someone searches for, matched alike in SQLite, PostgreSQL and memory. */
export class Search {
    /** The words, folded. */
    readonly words: readonly string[];
    /** The tokens each match needs, a word's last character left out after its pairs. */
    readonly tokens: readonly string[];

    /** Search for some folded words. */
    constructor(words: readonly string[]) {
        this.words = words;
        this.tokens = words.flatMap((word) => {
            const pairs = Tokenizer.pairs(word);

            return pairs.length > 1 ? pairs.slice(0, -1) : pairs;
        });
    }

    /** Read a search from what someone typed: its first words, none for blank text. */
    static of(text: string): Search {
        return new Search(Tokenizer.words(text).slice(0, MAX_WORDS));
    }

    /** Read a condition's search, refusing one of no words or of too many. */
    static parse(text: string): Search {
        // count its words
        const words = Tokenizer.words(text);
        const { length } = words;
        if (length === 0 || length > MAX_WORDS) {
            throw DatabaseError.of(
                "QUERY_INVALID",
                `search '${text}' is invalid: it has ${length} words, not 1 to ${MAX_WORDS}`,
            );
        }

        return new Search(words);
    }

    /** Whether it has no words, so every text matches. */
    get isEmpty(): boolean {
        return this.words.length === 0;
    }

    /** Report whether each token starts a word of some texts, as a row matches. */
    isIn(texts: readonly string[]): boolean {
        const line = texts.map((text) => Tokenizer.line(text)).join("");

        return this.tokens.every((token) => line.includes(startOf(token)));
    }

    /** Score how well texts match, the title first, undefined where a word is nowhere in them. */
    score(texts: readonly string[]): number | undefined {
        // leave out texts without the tokens anywhere, before taking their words apart
        const folded = texts.map((text) => Tokenizer.fold(text));
        if (!this.tokens.every((token) => folded.some((text) => text.includes(token)))) {
            return undefined;
        }

        // match the starts of words, else the words inside the texts
        if (this.isIn(texts)) {
            return this.relevance(texts);
        }
        const isInside = this.words.every((word) => folded.some((text) => text.includes(word)));

        return isInside ? SEARCH_SCORE.inside : undefined;
    }

    /** Score how well the best of the texts of a row that matches does, the title first. */
    relevance(texts: readonly string[]): number {
        const scores = texts.map((text, index) =>
            Math.max(0, this.#scoreOf(text) - (index === 0 ? 0 : SEARCH_LATER)),
        );

        return Math.max(SEARCH_SCORE.across, ...scores);
    }

    /** List the items whose texts match, the best first, equal ones in their order. */
    rank<Item>(items: readonly Item[], textsOf: (item: Item) => readonly string[]): Item[] {
        // leave them in order without words
        if (this.isEmpty) {
            return [...items];
        }

        // put the best first
        const matched = items.flatMap((item) => {
            const score = this.score(textsOf(item));

            return score === undefined ? [] : [{ item, score }];
        });

        return matched.toSorted((one, other) => other.score - one.score).map(({ item }) => item);
    }

    /** List the places of a text's words that a word starts, else those all words are inside. */
    ranges(text: string): readonly SearchRange[] {
        // find every word at the start of one, or a character of a script without spaces
        const located = Tokenizer.located(text);
        const starts = Array.from(located.text.matchAll(START), ({ index }) => index);
        const found = this.words.flatMap((word) =>
            starts
                .filter((start) => located.text.startsWith(word, start))
                .map((start) => rangeOf(located, start, word.length)),
        );
        if (found.length > 0) {
            return joined(found);
        }

        // else find them all inside
        const inside = this.words.map((word) => [located.text.indexOf(word), word.length] as const);

        return inside.some(([start]) => start === -1)
            ? []
            : joined(inside.map(([start, length]) => rangeOf(located, start, length)));
    }

    /** Cut the passage around the first match of a text, undefined for none. */
    excerpt(text: string, length: number = EXCERPT_LENGTH): string | undefined {
        // take the first of the stretches with the most tokens, each quick to fold
        const stretches = text
            .split(LINE_END)
            .flatMap((line) => line.replace(/\s+/gu, " ").trim().match(STRETCH) ?? []);
        const counts = stretches.map((stretch) => {
            const folded = Tokenizer.line(stretch);

            return this.tokens.filter((token) => folded.includes(startOf(token))).length;
        });
        const most = Math.max(0, ...counts);
        const line = most === 0 ? undefined : stretches[counts.indexOf(most)]?.trim();
        const [first] = line === undefined ? [] : this.ranges(line);
        if (line === undefined || first === undefined) {
            return undefined;
        }

        // start some words before the match, and end at a space
        const from = Math.max(0, first[0] - Math.floor(length / 3));
        const space = line.slice(from, first[0]).indexOf(" ");
        const start = from === 0 || space === -1 ? from : from + space + 1;
        const full = start + length;
        const last = line.slice(first[1], full).lastIndexOf(" ");
        const end = full >= line.length || last === -1 ? full : first[1] + last;

        return `${start === 0 ? "" : "…"}${line.slice(start, end)}${end >= line.length ? "" : "…"}`;
    }

    /** Score how well one text matches alone, 0 where a token starts none of its words. */
    #scoreOf(text: string): number {
        // compare the words in order, then each token with any
        const line = Tokenizer.line(text);
        const opening = ` ${this.words.join(" ")}`;
        if (line === `${opening} `) {
            return SEARCH_SCORE.whole;
        } else if (line.startsWith(opening)) {
            return SEARCH_SCORE.opening;
        } else if (this.tokens.every((token) => line.includes(wholeOf(token)))) {
            return SEARCH_SCORE.words;
        }

        return this.tokens.every((token) => line.includes(startOf(token)))
            ? SEARCH_SCORE.starts
            : 0;
    }
}

/** Write a token as it starts a word of a line, a pair as it is in one. */
export function startOf(token: string): string {
    return Tokenizer.isUnspaced(token) ? token : ` ${token}`;
}

/** Write a token as it is a word of a line, a pair as it is in one. */
export function wholeOf(token: string): string {
    return Tokenizer.isUnspaced(token) ? token : ` ${token} `;
}

/** The starts of words in a folded text, and each character of a script without spaces. */
const START = new RegExp(
    `[${Tokenizer.unspaced}]|(?<![${Tokenizer.letter}])[${Tokenizer.letter}]|(?<=[${Tokenizer.unspaced}])[${Tokenizer.letter}]`,
    "gu",
);

/** Read the place of some folded characters in their text, with the accents after them. */
function rangeOf({ places }: FoldedText, start: number, length: number): SearchRange {
    // end before the next character of the text folded to others
    const last = places[start + length - 1];
    const end = places.slice(start + length).find((place) => place !== last);

    return [places[start] ?? 0, end ?? places.at(-1) ?? 0];
}

/** Join ranges that overlap, in order. */
function joined(ranges: readonly SearchRange[]): SearchRange[] {
    const apart: SearchRange[] = [];
    for (const [start, end] of ranges.toSorted(([one], [other]) => one - other)) {
        // grow the last range, or begin another
        const last = apart.at(-1);
        if (last !== undefined && start < last[1]) {
            apart[apart.length - 1] = [last[0], Math.max(last[1], end)];
        } else {
            apart.push([start, end]);
        }
    }

    return apart;
}
