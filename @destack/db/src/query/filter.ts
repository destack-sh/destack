import { DatabaseError } from "../error/error.ts";
import { type Comparable, Condition, type JsonCondition } from "./condition.ts";
import { Predicate } from "./predicate.ts";
import type { Row } from "../table/row.ts";

/** The comparison operators of a restriction, longest first so `<=` reads before `<`. */
const COMPARATORS = ["<=", ">=", "!=", "=", "<", ">", ":"] as const;

/** The words a filter reserves for combining restrictions. */
const KEYWORDS = new Set(["AND", "OR", "NOT"]);

/** A comparison operator of a restriction. */
type Comparator = (typeof COMPARATORS)[number];

/** The condition key of each ordering comparison. */
const ORDERINGS = { "<": "lt", "<=": "lte", ">": "gt", ">=": "gte" } as const;

/** A token of a filter: a word, a quoted text, an operator or a parenthesis. */
type Token =
    | { readonly kind: "word"; readonly text: string; readonly start: number }
    | { readonly kind: "text"; readonly text: string; readonly start: number }
    | { readonly kind: "comparator"; readonly text: Comparator; readonly start: number }
    | { readonly kind: "open" | "close" | "minus"; readonly start: number };

/** How a filter reads its fields and values. */
export interface FilterOptions {
    /** The fields a filter may name, any when absent. */
    readonly fields?: ReadonlySet<string>;
    /** The values `@name` stands for, such as `@me` for the person. */
    readonly values?: Readonly<Record<string, Comparable>>;
    /** Convert a field's literal, such as a date for a time field, keeping it when absent. */
    readonly convert?: (path: readonly string[], literal: Comparable) => Comparable;
    /** Whether a dotted name walks relations, true by default, or names one field such as an attribute key. */
    readonly isRelational?: boolean;
}

/** Filters in AIP-160's syntax with SQL's precedence, read into conditions. */
export const Filter = {
    /**
     * Read a filter into a condition.
     *
     * Restrictions compare a field with `=`, `!=`, `<`, `<=`, `>`, `>=` or `:` (contains, ignoring case).
     * `AND` and adjacency bind tighter than `OR`, as in SQL, `NOT` or `-` negates, and keywords are uppercase.
     */
    parse(filter: string, options: FilterOptions = {}): JsonCondition {
        return new FilterReader(tokenize(filter), filter, options).read();
    },

    /** Read a filter over plain rows, such as attribute maps, into a match deciding as SQL decides. */
    compile(
        filter: string,
        options: Omit<FilterOptions, "isRelational"> = {},
    ): (row: Row) => boolean {
        // name every restriction's whole name as a field
        const condition = Filter.parse(filter, { ...options, isRelational: false });
        const match = Predicate.compile(Condition.resolve(condition, namesOf(condition)));

        return (row) => Predicate.matches(match, row);
    },
};

/** A reader of one filter's tokens, binding NOT, then AND, then OR. */
class FilterReader {
    /** The filter's tokens. */
    readonly #tokens: readonly Token[];
    /** The filter's text, for refusals. */
    readonly #filter: string;
    /** How the filter reads fields and values. */
    readonly #options: FilterOptions;
    /** The next token's position. */
    #position = 0;

    /** Read a filter's tokens. */
    constructor(tokens: readonly Token[], filter: string, options: FilterOptions) {
        this.#tokens = tokens;
        this.#filter = filter;
        this.#options = options;
    }

    /** Read the whole filter, refusing tokens after its expression. */
    read(): JsonCondition {
        // read the expression and require the end
        const condition = this.#expression();
        const rest = this.#tokens[this.#position];
        if (rest !== undefined) {
            throw this.#refuse("expected AND, OR or the end", rest.start);
        }

        return condition;
    }

    /** Read conjunctions joined by `OR`. */
    #expression(): JsonCondition {
        // read each conjunction after an OR
        const conjunctions = [this.#conjunction()];
        while (this.#isWord("OR")) {
            this.#position++;
            conjunctions.push(this.#conjunction());
        }

        // keep a single conjunction as it is
        const [only] = conjunctions;

        return conjunctions.length === 1 && only !== undefined ? only : { OR: conjunctions };
    }

    /** Read terms joined by `AND` or adjacent, which all hold. */
    #conjunction(): JsonCondition {
        // read each term after an AND or beside the previous one
        const terms = [this.#term()];
        while (this.#isWord("AND") || this.#startsTerm()) {
            if (this.#isWord("AND")) {
                this.#position++;
            }
            terms.push(this.#term());
        }

        // keep a single term as it is
        const [only] = terms;

        return terms.length === 1 && only !== undefined ? only : { AND: terms };
    }

    /** Read a negated term, a parenthesized expression or a restriction. */
    #term(): JsonCondition {
        // negate the next term
        const token = this.#tokens[this.#position];
        if (token?.kind === "minus" || this.#isWord("NOT")) {
            this.#position++;

            return { NOT: this.#term() };
        }

        // read a parenthesized expression
        if (token?.kind === "open") {
            this.#position++;
            const inner = this.#expression();
            if (this.#tokens[this.#position]?.kind !== "close") {
                throw this.#refuse("expected )", this.#offset());
            }
            this.#position++;

            return inner;
        }

        return this.#restriction();
    }

    /** Read a restriction: a field, its comparator and a value. */
    #restriction(): JsonCondition {
        // read the field's path
        const field = this.#tokens[this.#position];
        if (field?.kind !== "word" || KEYWORDS.has(field.text) || field.text.startsWith("@")) {
            throw this.#refuse("expected a field", field?.start ?? this.#filter.length);
        }
        const path = this.#options.isRelational === false ? [field.text] : field.text.split(".");
        const [name] = path;
        if (
            name === undefined ||
            path.some((part) => part === "") ||
            (this.#options.fields !== undefined && !this.#options.fields.has(name))
        ) {
            throw this.#refuse(`unknown field ${field.text}`, field.start);
        }
        this.#position++;

        // read the comparator and the value
        const comparator = this.#tokens[this.#position];
        if (comparator?.kind !== "comparator") {
            throw this.#refuse(`expected a comparison after ${field.text}`, this.#offset());
        }
        this.#position++;
        const value = this.#value(path);
        if (value === null && comparator.text !== "=" && comparator.text !== "!=") {
            throw this.#refuse("null compares only with = or !=", comparator.start);
        }

        return nest(path, compare(comparator.text, value));
    }

    /** Read a restriction's value as a quoted text, a named value, null, a boolean, a number or a word. */
    #value(path: readonly string[]): Comparable | null {
        // read the literal
        const token = this.#tokens[this.#position];
        if (token?.kind !== "word" && token?.kind !== "text") {
            throw this.#refuse("expected a value", token?.start ?? this.#filter.length);
        }
        this.#position++;
        let literal: Comparable | null;
        if (token.kind === "text") {
            literal = token.text;
        } else if (token.text.startsWith("@")) {
            const named = this.#options.values?.[token.text.slice(1)];
            if (named === undefined) {
                throw this.#refuse(`unknown value ${token.text}`, token.start);
            }
            literal = named;
        } else if (token.text === "null") {
            literal = null;
        } else if (token.text === "true" || token.text === "false") {
            literal = token.text === "true";
        } else if (/^-?\d+(\.\d+)?$/u.test(token.text)) {
            literal = Number(token.text);
        } else {
            literal = token.text;
        }

        // convert it for its field
        return literal === null || this.#options.convert === undefined
            ? literal
            : this.#options.convert(path, literal);
    }

    /** Report whether the next token is a keyword. */
    #isWord(keyword: string): boolean {
        const token = this.#tokens[this.#position];

        return token?.kind === "word" && token.text === keyword;
    }

    /** Report whether the next token starts another term of a conjunction. */
    #startsTerm(): boolean {
        const token = this.#tokens[this.#position];

        return (
            token !== undefined &&
            (token.kind === "open" ||
                token.kind === "minus" ||
                (token.kind === "word" && (token.text === "NOT" || !KEYWORDS.has(token.text))))
        );
    }

    /** Read the next token's offset, the filter's end past its last. */
    #offset(): number {
        return this.#tokens[this.#position]?.start ?? this.#filter.length;
    }

    /** Refuse a filter at an offset. */
    #refuse(reason: string, offset: number): DatabaseError {
        return new DatabaseError(
            "INVALID_QUERY",
            `${reason} at ${offset} in filter ${this.#filter}`,
        );
    }
}

/** Split a filter into tokens, refusing an unclosed text. */
function tokenize(filter: string): Token[] {
    // read one token at a time
    const tokens: Token[] = [];
    let index = 0;
    while (index < filter.length) {
        const character = filter.charAt(index);
        const comparator = comparatorAt(filter, index);

        // skip whitespace
        if (/\s/u.test(character)) {
            index++;
        }
        // read parentheses and a leading minus
        else if (character === "(" || character === ")") {
            tokens.push({ kind: character === "(" ? "open" : "close", start: index });
            index++;
        } else if (character === "-" && !/\d/u.test(filter[index + 1] ?? "")) {
            tokens.push({ kind: "minus", start: index });
            index++;
        }
        // read a quoted text with backslash escapes
        else if (character === '"') {
            const start = index;
            let text = "";
            index++;
            while (index < filter.length && filter[index] !== '"') {
                text += filter[index] === "\\" ? (filter[++index] ?? "") : filter[index];
                index++;
            }
            if (index >= filter.length) {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    `unclosed text at ${start} in filter ${filter}`,
                );
            }
            tokens.push({ kind: "text", text, start });
            index++;
        }
        // read a comparator
        else if (comparator !== undefined) {
            tokens.push({ kind: "comparator", text: comparator, start: index });
            index += comparator.length;
        }
        // read a word up to whitespace, a parenthesis, a quote or a comparator
        else {
            const start = index;
            while (
                index < filter.length &&
                !/[\s()"]/u.test(filter.charAt(index)) &&
                comparatorAt(filter, index) === undefined
            ) {
                index++;
            }
            tokens.push({ kind: "word", text: filter.slice(start, index), start });
        }
    }

    return tokens;
}

/** Compare a field with a value by a comparator. */
function compare(comparator: Comparator, value: Comparable | null): Comparable | JsonCondition {
    // match a missing value, or a present one
    if (value === null) {
        return comparator === "=" ? { isNull: true } : { isNotNull: true };
    }

    // match text containing the value, ignoring case
    if (comparator === ":") {
        return { ilike: `%${escape(String(value))}%` };
    }

    // match a pattern for an equality with wildcards
    if (
        (comparator === "=" || comparator === "!=") &&
        typeof value === "string" &&
        value.includes("*")
    ) {
        const pattern = value.split("*").map(escape).join("%");

        return comparator === "=" ? { like: pattern } : { notLike: pattern };
    }

    // compare by equality or order
    if (comparator === "=") {
        return value;
    } else if (comparator === "!=") {
        return { ne: value };
    }

    return { [ORDERINGS[comparator]]: value };
}

/** Escape a pattern's `%`, `_` and `\`. */
function escape(text: string): string {
    return text.replaceAll(/[\\%_]/gu, (character) => `\\${character}`);
}

/** Nest a field's comparison under the relations its path names. */
function nest(path: readonly string[], entry: Comparable | JsonCondition): JsonCondition {
    const [name, ...rest] = path;
    if (name === undefined) {
        throw new DatabaseError("INVALID_QUERY", "a filter restriction names no field");
    }

    return { [name]: rest.length === 0 ? entry : nest(rest, entry) };
}

/** Collect the field names a condition compares, inside its combinations. */
function namesOf(condition: JsonCondition): Set<string> {
    const names = new Set<string>();
    for (const [name, entry] of Object.entries(condition)) {
        // descend into combinations and keep the other names
        if (name === "AND" || name === "OR" || name === "NOT") {
            const combined = Array.isArray(entry) ? entry : [entry];
            for (const each of combined.filter(isCondition)) {
                namesOf(each).forEach((found) => names.add(found));
            }
        } else {
            names.add(name);
        }
    }

    return names;
}

/** Report whether a condition's entry is itself a condition, as combinations hold. */
function isCondition(entry: unknown): entry is JsonCondition {
    return typeof entry === "object" && entry !== null && !Array.isArray(entry);
}

/** Read the comparator starting at an offset, absent for none. */
function comparatorAt(filter: string, index: number): Comparator | undefined {
    return COMPARATORS.find((comparator) => filter.startsWith(comparator, index));
}
