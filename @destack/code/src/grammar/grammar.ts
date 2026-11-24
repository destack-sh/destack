import type { Language } from "../language/index.ts";
import type { Edit, Range, Source } from "../source/index.ts";
import type { Tokens } from "../token/index.ts";

/** What reads the code of a language. */
export interface Grammar {
    /** The language. */
    readonly language: Language;
    /** Parse a source, up to an offset for the part shown first. */
    readonly parse: (source: Source, to?: number) => Syntax;
}

/** The code of a source as a grammar parsed it. */
export interface Syntax {
    /** The source. */
    readonly source: Source;
    /** The offset it is parsed up to, the text's end once it is whole. */
    readonly parsed: number;
    /** Read the tokens of a range, plain past what is parsed. */
    readonly tokens: (range?: Range) => Tokens;
    /** Parse on up to an offset, the text's end by default, afresh after edits. */
    readonly parse: (to?: number) => Syntax;
    /** Replace some text, parsing again only what that changes. */
    readonly edit: (edits: Edit | readonly Edit[]) => Syntax;
}
