import { type StreamParser, StringStream } from "@codemirror/streamparser";
import { type Tag, tags } from "@lezer/highlight";
import type { Language } from "../language/index.ts";
import { Edit, type Range, type Source } from "../source/index.ts";
import { TokenBuilder } from "../token/builder.ts";
import { HIGHLIGHTER, kindOf } from "../token/highlight.ts";
import type { TokenKind, Tokens } from "../token/index.ts";
import type { Grammar, Syntax } from "./grammar.ts";

/** The columns a tab stands for, and those a block is indented by. */
const TAB_SIZE = 4;

/** The lines between two kept states, about what a screen shows. */
const CHECKPOINT_LINES = 64;

/** The tags of the names stream parsers give tokens besides the tags' own. */
const NAMES: Readonly<Record<string, Tag>> = {
    attribute: tags.attributeName,
    builtin: tags.standard(tags.variableName),
    def: tags.definition(tags.variableName),
    error: tags.invalid,
    header: tags.heading,
    property: tags.propertyName,
    qualifier: tags.modifier,
    tag: tags.tagName,
    type: tags.typeName,
    variable: tags.variableName,
    "string-2": tags.special(tags.string),
    "variable-2": tags.special(tags.variableName),
    "variable-3": tags.typeName,
};

/** The tags and modifiers by name. */
const TAGS: ReadonlyMap<string, unknown> = new Map<string, unknown>(Object.entries(tags));

/** A grammar that reads code line by line. */
export class StreamGrammar implements Grammar {
    /** The language. */
    readonly language: Language;
    /** The parser. */
    readonly #parser: StreamParser<unknown>;
    /** The kinds of the names the parser has given so far. */
    readonly #kinds = new Map<string, TokenKind>();

    /** Read a language with a stream parser. */
    constructor(language: Language, parser: StreamParser<unknown>) {
        this.language = language;
        this.#parser = parser;
    }

    /** Parse a source, whose lines are read as their tokens are. */
    parse(source: Source): Syntax {
        return new StreamSyntax(this, source, [this.start()]);
    }

    /** Make the state a source starts in. */
    start(): unknown {
        return this.#parser.startState?.(TAB_SIZE) ?? {};
    }

    /** Copy a state, which reading a line changes. */
    copy(state: unknown): unknown {
        if (this.#parser.copyState !== undefined) {
            return this.#parser.copyState(state);
        }

        return typeof state === "object" && state !== null
            ? Object.fromEntries(
                  Object.entries(state).map(([key, value]) => [
                      key,
                      Array.isArray(value) ? value.slice() : value,
                  ]),
              )
            : state;
    }

    /** Read a line, changing the state and telling where each token ends. */
    read(line: string, state: unknown, tell?: (end: number, kind: TokenKind) => void): void {
        // tell the parser of an empty line
        if (line === "") {
            this.#parser.blankLine?.(state, TAB_SIZE);
            return;
        }

        // read a token after another, once more where one changed the state alone
        const stream = new StringStream(line, TAB_SIZE, TAB_SIZE);
        while (!stream.eol()) {
            stream.start = stream.pos;
            let style = this.#parser.token(stream, state);
            if (stream.pos === stream.start) {
                style = this.#parser.token(stream, state);
            }
            if (stream.pos === stream.start) {
                throw new Error(`grammar of '${this.language.name}' is stuck at '${line}'`);
            }
            tell?.(stream.pos, this.#kindOf(style));
        }
    }

    /** Read the kind of a token's name, such as `keyword` or `variableName.constant`. */
    #kindOf(style: string | null | undefined): TokenKind {
        // look it up, or read it once
        if (style === null || style === undefined || style === "") {
            return "plain";
        }
        const known = this.#kinds.get(style);
        const kind = known ?? kindOf(HIGHLIGHTER.style(tagsOf(style)));
        this.#kinds.set(style, kind);

        return kind;
    }
}

/** The code of a source read line by line, from the nearest kept state. */
class StreamSyntax implements Syntax {
    /** The source. */
    readonly source: Source;
    /** The offset it is parsed up to, always its end, as lines are read when shown. */
    readonly parsed: number;
    /** The grammar. */
    readonly #grammar: StreamGrammar;
    /** The state every so many lines start in, as far as lines were read. */
    readonly #states: unknown[];

    /** Keep a source and the states known of it. */
    constructor(grammar: StreamGrammar, source: Source, states: unknown[]) {
        // lines are read as they are shown
        this.#grammar = grammar;
        this.source = source;
        this.#states = states;
        this.parsed = source.text.length;
    }

    /** Read the tokens of a range. */
    tokens({ from, to }: Range = { from: 0, to: this.source.text.length }): Tokens {
        // read on to the first line shown
        const builder = new TokenBuilder(this.source, from);
        const first = this.source.lineAt(from);
        const last = this.source.lineAt(to);
        const state = this.#stateAt(first);

        // build the tokens of the lines shown
        for (let line = first; line <= last; line += 1) {
            this.#keep(line, state);
            const range = this.source.range(line);
            this.#grammar.read(this.source.line(line), state, (end, kind) => {
                builder.add(Math.min(to, Math.max(from, range.from + end)), kind);
            });
            builder.add(Math.min(to, this.source.start(line + 1)), "plain");
        }

        return builder.build(to);
    }

    /** Parse on, which a source read as it is shown never needs. */
    parse(): Syntax {
        return this;
    }

    /** Replace some text, keeping the states before it. */
    edit(edits: Edit | readonly Edit[]): Syntax {
        // forget the states the first replacement may change
        const [first] = Edit.list(edits);
        const known =
            first === undefined
                ? this.#states.length
                : Math.floor(this.source.lineAt(first.from) / CHECKPOINT_LINES) + 1;

        return new StreamSyntax(
            this.#grammar,
            this.source.edit(edits),
            this.#states.slice(0, known),
        );
    }

    /** Read on to a line from the nearest kept state, and copy the state it starts in. */
    #stateAt(line: number): unknown {
        // start at the nearest kept one
        const place = Math.min(Math.floor(line / CHECKPOINT_LINES), this.#states.length - 1);
        const state = this.#grammar.copy(this.#states[place]);

        // read the lines between
        for (let at = place * CHECKPOINT_LINES; at < line; at += 1) {
            this.#keep(at, state);
            this.#grammar.read(this.source.line(at), state);
        }

        return state;
    }

    /** Keep the state a line starts in, if it is the next of those kept. */
    #keep(line: number, state: unknown): void {
        if (line === this.#states.length * CHECKPOINT_LINES) {
            this.#states.push(this.#grammar.copy(state));
        }
    }
}

/** Read the tags of a token's name, each a tag's own name with its modifiers. */
function tagsOf(style: string): Tag[] {
    return style.split(" ").flatMap((each) => {
        // apply each modifier to the tag
        const [name = "", ...modifiers] = each.split(".");
        const tag = NAMES[name] ?? TAGS.get(name);
        if (!isTag(tag)) {
            return [];
        }

        return modifiers.reduce((modified: Tag, modifier) => {
            const modify = TAGS.get(modifier);

            return isModifier(modify) ? modify(modified) : modified;
        }, tag);
    });
}

/** Check that a value is a tag. */
function isTag(value: unknown): value is Tag {
    return typeof value === "object" && value !== null && "set" in value;
}

/** Check that a value modifies tags. */
function isModifier(value: unknown): value is (tag: Tag) => Tag {
    return typeof value === "function";
}
