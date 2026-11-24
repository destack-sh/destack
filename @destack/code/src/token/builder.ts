import type { Source } from "../source/index.ts";
import { TOKEN_KINDS, type TokenKind, Tokens } from "./token.ts";

/** The tokens room is made for at first, about a screen of code. */
const CAPACITY = 512;

/** The place of each kind among the kinds. */
const PLACES: ReadonlyMap<TokenKind, number> = new Map(
    TOKEN_KINDS.map((kind, place) => [kind, place]),
);

/** Builds the tokens of a range of a source in order. */
export class TokenBuilder {
    /** The source. */
    readonly #source: Source;
    /** The offset the first token starts at. */
    readonly #from: number;
    /** The kinds so far, with room for more. */
    #kinds = new Uint8Array(CAPACITY);
    /** The ends so far, with room for more. */
    #ends = new Uint32Array(CAPACITY);
    /** The count of tokens so far. */
    #length = 0;
    /** The offset the last token ends before. */
    #end: number;

    /** Build tokens of a source from an offset. */
    constructor(source: Source, from: number) {
        this.#source = source;
        this.#from = from;
        this.#end = from;
    }

    /** Add a token up to an offset, joined to the one before where both are of one kind. */
    add(to: number, kind: TokenKind): void {
        // leave out an empty one
        if (to <= this.#end) {
            return;
        }
        this.#end = to;

        // join it to the one before
        const place = PLACES.get(kind) ?? 0;
        if (this.#length > 0 && this.#kinds[this.#length - 1] === place) {
            this.#ends[this.#length - 1] = to;
            return;
        }

        // make room, then add it
        if (this.#length === this.#ends.length) {
            const kinds = new Uint8Array(this.#length * 2);
            const ends = new Uint32Array(this.#length * 2);
            kinds.set(this.#kinds);
            ends.set(this.#ends);
            this.#kinds = kinds;
            this.#ends = ends;
        }
        this.#kinds[this.#length] = place;
        this.#ends[this.#length] = to;
        this.#length += 1;
    }

    /** Take the tokens, plain up to an offset. */
    build(to: number): Tokens {
        this.add(to, "plain");

        return new Tokens(
            this.#source,
            this.#from,
            this.#kinds.slice(0, this.#length),
            this.#ends.slice(0, this.#length),
        );
    }
}
