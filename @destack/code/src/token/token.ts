import type { Range, Source } from "../source/index.ts";

/** The kinds of tokens, each of which a theme may color. */
export const TOKEN_KINDS = [
    // text
    "plain",
    "comment",
    "punctuation",
    "operator",

    // values
    "keyword",
    "constant",
    "number",
    "string",

    // names
    "variable",
    "property",
    "function",
    "type",
    "tag",
    "attribute",

    // prose
    "heading",
    "link",
    "strong",
    "emphasis",

    // changes
    "inserted",
    "deleted",
] as const;

/** What a token is, such as a `keyword`. */
export type TokenKind = (typeof TOKEN_KINDS)[number];

/** A run of code of one kind. */
export interface Token extends Range {
    /** The kind. */
    readonly kind: TokenKind;
    /** The code. */
    readonly text: string;
}

/** The tokens of a range of a source, one after another without gaps. */
export class Tokens implements Iterable<Token> {
    /** The source. */
    readonly source: Source;
    /** The offset the first token starts at. */
    readonly from: number;
    /** The place of each token's kind among the kinds. */
    readonly kinds: Uint8Array;
    /** The offset each token ends before. */
    readonly ends: Uint32Array;

    /** Keep tokens by their kinds and ends. */
    constructor(source: Source, from: number, kinds: Uint8Array, ends: Uint32Array) {
        // both lists are as long
        this.source = source;
        this.from = from;
        this.kinds = kinds;
        this.ends = ends;
    }

    /** Read a range of a source as one plain token, as in a language that is not known. */
    static plain(source: Source, { from, to }: Range): Tokens {
        return new Tokens(source, from, new Uint8Array(1), Uint32Array.of(to));
    }

    /** The count of tokens. */
    get length(): number {
        return this.ends.length;
    }

    /** Read the tokens in order. */
    *[Symbol.iterator](): Generator<Token> {
        yield* this.within({ from: this.from, to: this.ends.at(-1) ?? this.from });
    }

    /** Read the tokens of a smaller range, cut where they reach past it. */
    *within({ from, to }: Range): Generator<Token> {
        // start at the first token that ends inside it
        for (let at = this.#at(from); at < this.length; at += 1) {
            const start = Math.max(from, this.#end(at - 1));
            const end = Math.min(to, this.#end(at));
            if (start >= to) {
                return;
            }
            if (end > start) {
                yield {
                    kind: TOKEN_KINDS[this.kinds[at] ?? 0] ?? "plain",
                    text: this.source.text.slice(start, end),
                    from: start,
                    to: end,
                };
            }
        }
    }

    /** Read the tokens of a line. */
    line(line: number): Token[] {
        return [...this.within(this.source.range(line))];
    }

    /** Read the offset a token ends before, the start of all before the first. */
    #end(at: number): number {
        return at < 0 ? this.from : (this.ends[at] ?? this.from);
    }

    /** Find the first token that ends past an offset. */
    #at(offset: number): number {
        // halve the tokens until one is left
        let low = 0;
        let high = this.length;
        while (low < high) {
            const middle = (low + high) >>> 1;
            if (this.#end(middle) <= offset) {
                low = middle + 1;
            } else {
                high = middle;
            }
        }

        return low;
    }
}
