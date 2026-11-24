import { type Parser, type Tree, TreeFragment } from "@lezer/common";
import { highlightTree } from "@lezer/highlight";
import type { Language } from "../language/index.ts";
import { Edit, type Range, type Source } from "../source/index.ts";
import { TokenBuilder } from "../token/builder.ts";
import { HIGHLIGHTER, kindOf } from "../token/highlight.ts";
import type { Tokens } from "../token/index.ts";
import type { Grammar, Syntax } from "./grammar.ts";

/** A grammar Lezer parses. */
export class LezerGrammar implements Grammar {
    /** The language. */
    readonly language: Language;
    /** The parser. */
    readonly #parser: Parser;

    /** Read a language with a parser. */
    constructor(language: Language, parser: Parser) {
        this.language = language;
        this.#parser = parser;
    }

    /** Parse a source, up to an offset for the part shown first. */
    parse(source: Source, to: number = source.text.length): Syntax {
        return new LezerSyntax(this.#parser, source, parseTo(this.#parser, source, [], to), false);
    }
}

/** The code of a source as a Lezer tree. */
class LezerSyntax implements Syntax {
    /** The source. */
    readonly source: Source;
    /** The offset it is parsed up to, the text's end once it is whole. */
    readonly parsed: number;
    /** The parser. */
    readonly #parser: Parser;
    /** The tree, of the part that is parsed. */
    readonly #tree: Tree;
    /** Whether an edit reused parts of a tree, which broken code may leave out of place. */
    readonly #isReused: boolean;

    /** Keep the tree of a source. */
    constructor(parser: Parser, source: Source, tree: Tree, isReused: boolean) {
        // the tree tells how far it goes
        this.#parser = parser;
        this.source = source;
        this.#tree = tree;
        this.#isReused = isReused;
        this.parsed = tree.length;
    }

    /** Read the tokens of a range, plain past what is parsed. */
    tokens({ from, to }: Range = { from: 0, to: this.source.text.length }): Tokens {
        // fill what lies between the tokens, and cut those that reach past the range
        const builder = new TokenBuilder(this.source, from);
        const write = (start: number, end: number, classes: string): void => {
            builder.add(Math.max(from, start), "plain");
            builder.add(Math.min(to, end), kindOf(classes));
        };
        highlightTree(this.#tree, HIGHLIGHTER, write, from, to);

        return builder.build(to);
    }

    /** Parse on up to an offset, the text's end by default, afresh after edits. */
    parse(to: number = this.source.text.length): Syntax {
        // leave what is parsed already
        const isFar = this.parsed >= Math.min(to, this.source.text.length);
        if (isFar && !this.#isReused) {
            return this;
        }

        // go on from the tree, unless an edit reused parts of one
        const fragments = this.#isReused ? [] : TreeFragment.addTree(this.#tree, [], true);
        const tree = parseTo(this.#parser, this.source, fragments, to);

        return new LezerSyntax(this.#parser, this.source, tree, false);
    }

    /** Replace some text, parsing again only what that changes. */
    edit(edits: Edit | readonly Edit[]): Syntax {
        // keep the parts of the tree the replacements leave
        const source = this.source.edit(edits);
        const isWhole = this.parsed === this.source.text.length;
        let shift = 0;
        const changes = Edit.list(edits).map(({ from, to, insert }) => {
            const change = {
                fromA: from,
                toA: to,
                fromB: from + shift,
                toB: from + shift + insert.length,
            };
            shift += insert.length - (to - from);

            return change;
        });
        const fragments = TreeFragment.applyChanges(
            TreeFragment.addTree(this.#tree, [], !isWhole),
            changes,
        );

        // parse as far as before
        const end = isWhole ? source.text.length : Math.max(0, this.parsed + shift);
        const tree = parseTo(this.#parser, source, fragments, end);

        return new LezerSyntax(this.#parser, source, tree, true);
    }
}

/** Parse a source up to an offset, reusing fragments of a tree. */
function parseTo(
    parser: Parser,
    source: Source,
    fragments: readonly TreeFragment[],
    to: number,
): Tree {
    // stop past the offset, then advance until the tree is there
    const parse = parser.startParse(source.text, fragments);
    if (to < source.text.length) {
        parse.stopAt(to);
    }
    let tree = parse.advance();
    while (tree === null) {
        tree = parse.advance();
    }

    return tree;
}
