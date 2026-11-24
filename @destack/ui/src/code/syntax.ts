import { type Grammar, type Source, type Syntax, Tokens } from "@destack/code";
import { type Accessor, createMemo, createSignal, type Setter, untrack } from "@destack/view";
import { isRenderedAtOnce } from "./near.ts";
import { Timeout } from "@destack/view/primitives/timer";

/** The characters parsed at once, about 8 ms of work at 8 MB a second. */
const PARSED_CHARACTERS = 64_000;

/** The lines parsed past one that shows, so lines next to each other parse once. */
const PARSED_LINES = 200;

/** A source parsed as far as its lines show. */
export class CodeSyntax {
    /** The source. */
    readonly #source: Accessor<Source>;
    /** Whether every line is parsed at once, for markup a server renders. */
    readonly #isAtOnce: boolean;
    /** The source before any of it is parsed, absent without a grammar. */
    readonly #first: Accessor<Syntax | undefined>;
    /** What the next slice waits on, until nothing shows the source. */
    readonly #timer = new Timeout();
    /** How often slices reached their end. */
    readonly #revision: Accessor<number>;
    /** Replace how often slices reached their end. */
    readonly #setRevision: Setter<number>;
    /** The source as far as it is parsed, absent before any of it is. */
    #parsed: Syntax | undefined;
    /** The offset the slices parse on to, absent while none do. */
    #end: number | undefined;

    /** Follow a source and the grammar that colors it. */
    constructor(source: Accessor<Source>, grammar: Accessor<Grammar | undefined>) {
        // parse nothing until a line shows
        const [revision, setRevision] = createSignal(0, { ownedWrite: true });
        this.#isAtOnce = isRenderedAtOnce();
        this.#source = source;
        this.#revision = revision;
        this.#setRevision = setRevision;
        this.#first = createMemo(() => {
            this.#parsed = undefined;

            return grammar()?.parse(source(), 0);
        });
    }

    /** Read the tokens of a line, plain without a grammar and while it lies too far ahead. */
    tokens(line: number): Tokens {
        const range = this.#source().range(line);

        return this.#reach(line)?.tokens(range) ?? Tokens.plain(this.#source(), range);
    }

    /** Parse past a line, at once where it is near and in slices where it is far. */
    #reach(line: number): Syntax | undefined {
        // read what was parsed before
        const first = this.#first();
        const syntax = this.#parsed ?? first;
        const source = this.#source();
        if (first === undefined || syntax === undefined || syntax.parsed >= source.range(line).to) {
            return syntax;
        }

        // parse on to a line nearby at once, or to any where every line is
        const end = source.start(line + PARSED_LINES);
        if (this.#isAtOnce || end - syntax.parsed <= PARSED_CHARACTERS) {
            this.#parsed = syntax.parse(end);

            return this.#parsed;
        }

        // parse on to one far ahead in slices, and color it then
        this.#revision();
        const isRunning = this.#end !== undefined;
        this.#end = Math.max(this.#end ?? 0, end);
        if (!isRunning) {
            this.#timer.set(() => this.#slice(first), 0);
        }

        return undefined;
    }

    /** Parse a slice, and tell the lines that wait once they are there. */
    #slice(first: Syntax): void {
        // go on until the end, unless the source is another
        const syntax = this.#parsed ?? first;
        const end = this.#end ?? 0;
        if (untrack(this.#first) === first && syntax.parsed < end) {
            const parsed = syntax.parse(Math.min(end, syntax.parsed + PARSED_CHARACTERS));
            this.#parsed = parsed;
            if (parsed.parsed > syntax.parsed) {
                this.#timer.set(() => this.#slice(first), 0);

                return;
            }
        }
        this.#end = undefined;
        this.#setRevision((count) => count + 1);
    }
}
