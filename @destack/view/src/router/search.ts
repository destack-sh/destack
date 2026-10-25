import { useLocation, useSearchParams } from "@solidjs/router";
import { mapValues, type schema } from "@destack/schema";
import { type Accessor, createEffect, createMemo, untrack } from "solid-js";
import { ViewError } from "../error/error.ts";

/** How setting URL state moves through history, scrolls and treats defaults. */
export interface Options {
    /** Add a history entry, or replace the current one, replace by default. */
    readonly history?: "push" | "replace";
    /** Scroll to the top after the change, false by default. */
    readonly scroll?: boolean;
    /** Leave a value equal to its default out of the URL, true by default. */
    readonly clearOnDefault?: boolean;
}

/** Read and write one value of the URL's query as text. */
export interface Parser<Value> {
    /** Read a value from its text, null for text without one. */
    readonly parse: (text: string) => Value | null;
    /** Write a value as its text. */
    serialize(value: Value): string;
    /** Report whether two values are the same, which decides whether a value is its default. */
    eq(left: Value, right: Value): boolean;
}

/** A parser with the options its value moves through the URL with. */
export class ParserBuilder<Value> implements Parser<Value> {
    /** How the value reads and writes as text. */
    readonly parser: Parser<Value>;
    /** The options setting this value applies. */
    readonly options: Options;

    /** Hold a parser with its options. */
    constructor(parser: Parser<Value>, options: Options = {}) {
        this.parser = parser;
        this.options = options;
    }

    /** Read a value from its text, null for text without one. */
    parse(text: string): Value | null {
        return this.parser.parse(text);
    }

    /** Write a value as its text. */
    serialize(value: Value): string {
        return this.parser.serialize(value);
    }

    /** Report whether two values are the same, which decides whether a value is its default. */
    eq(left: Value, right: Value): boolean {
        return this.parser.eq(left, right);
    }

    /** Read a query text's value, null for none or for text the parser refuses. */
    read(text: string | undefined): Value | null {
        return text === undefined ? null : this.parse(text);
    }

    /** Write a value as the text the query has for it, none for null. */
    write(value: Value | null | undefined, _options: Options): string | undefined {
        return value === null || value === undefined ? undefined : this.serialize(value);
    }

    /** Give the value a default, which reading answers in place of a missing or invalid value. */
    withDefault(value: Value): ParserWithDefault<Value> {
        return new ParserWithDefault(this.parser, this.options, value);
    }

    /** Give the value the options setting it applies. */
    withOptions(options: Options): ParserBuilder<Value> {
        return new ParserBuilder(this.parser, { ...this.options, ...options });
    }
}

/** A parser of a value that has a default. */
export class ParserWithDefault<Value> extends ParserBuilder<Value> {
    /** The value reading answers in place of a missing or invalid one. */
    readonly defaultValue: Value;

    /** Hold a parser with its options and default. */
    constructor(parser: Parser<Value>, options: Options, defaultValue: Value) {
        super(parser, options);
        this.defaultValue = defaultValue;
    }

    /** Read a query text's value, the default for none or for text the parser refuses. */
    override read(text: string | undefined): Value {
        return super.read(text) ?? this.defaultValue;
    }

    /** Write a value as its query text, none for null or a default left out. */
    override write(value: Value | null | undefined, options: Options): string | undefined {
        // leave out a value at its default unless told to keep defaults
        const isCleared = this.options.clearOnDefault ?? options.clearOnDefault ?? true;
        if (
            isCleared &&
            value !== null &&
            value !== undefined &&
            this.eq(value, this.defaultValue)
        ) {
            return undefined;
        }

        return super.write(value, options);
    }

    /** Give the value the options setting it applies, keeping its default. */
    override withOptions(options: Options): ParserWithDefault<Value> {
        return new ParserWithDefault(
            this.parser,
            { ...this.options, ...options },
            this.defaultValue,
        );
    }
}

/** The parsers of a page's URL state, by key. */
export type Parsers = Readonly<Record<string, ParserBuilder<unknown>>>;

/** The value one parser reads: its value, or null without a default. */
export type ValueOf<Built> =
    Built extends ParserWithDefault<infer Value>
        ? Value
        : Built extends ParserBuilder<infer Value>
          ? Value | null
          : never;

/** The values a page's parsers read, by key. */
export type Values<Built extends Parsers> = { readonly [Key in keyof Built]: ValueOf<Built[Key]> };

/** The values to set, by key: a value, or null to remove it. */
export type Updates<Built extends Parsers> = {
    readonly [Key in keyof Built]?: ValueOf<Built[Key]> | null;
};

/** Set values: some of them, a function of the current ones, or null to remove them all. */
export type SetValues<Built extends Parsers> = (
    updates: Updates<Built> | ((old: Values<Built>) => Updates<Built> | null) | null,
    options?: Options,
) => Promise<URLSearchParams>;

/** Set one value: a value, a function of the current one, or null to remove it. */
export type SetValue<Value> = (
    value: Value | null | ((old: Value) => Value | null),
    options?: Options,
) => Promise<URLSearchParams>;

/** Write values as a query string for links, over a base's query when given. */
export interface Serializer<Built extends Parsers> {
    /** Write values as a query string, null removing them all. */
    (values: Updates<Built> | null): string;
    /** Write values over a base's query: a path, an address or a query, null removing them all. */
    (base: string | URL | URLSearchParams, values: Updates<Built> | null): string;
}

/** How several values map to the URL: the query key of each value, its own key by default. */
export interface QueryStatesOptions<Built extends Parsers> extends Options {
    /** The query key of each value, such as `q` for `query`. */
    readonly urlKeys?: Readonly<Partial<Record<keyof Built, string>>>;
}

/** Make a parser from how a value reads and writes as text. */
export function createParser<Value>(
    parser: Pick<Parser<Value>, "parse" | "serialize"> & Partial<Pick<Parser<Value>, "eq">>,
): ParserBuilder<Value> {
    return new ParserBuilder({
        eq: (left: Value, right: Value) => Object.is(left, right),
        ...parser,
    });
}

/** Read text as itself. */
export const parseAsString: ParserBuilder<string> = createParser({
    parse: (text) => text,
    serialize: (value) => value,
});

/** Read a whole number, refusing other text, and refuse to write any other number. */
export const parseAsInteger: ParserBuilder<number> = createParser({
    parse: (text) => (/^-?\d+$/u.test(text) ? Number(text) : null),
    serialize: (value) => {
        if (!Number.isInteger(value)) {
            throw ViewError.of("QUERY_VALUE_INVALID", `${value} is not a whole number`);
        }

        return String(value);
    },
});

/** Read a number, refusing other text. */
export const parseAsFloat: ParserBuilder<number> = createParser({
    parse: (text) => {
        const value = Number(text);

        return text.trim() === "" || Number.isNaN(value) ? null : value;
    },
    serialize: (value) => String(value),
});

/** Read a whole number written in hexadecimal, refusing other text. */
export const parseAsHex: ParserBuilder<number> = createParser({
    parse: (text) => (/^[0-9a-f]+$/iu.test(text) ? Number.parseInt(text, 16) : null),
    serialize: (value) => {
        if (!Number.isInteger(value)) {
            throw ViewError.of("QUERY_VALUE_INVALID", `${value} is not a whole number`);
        }
        const hex = value.toString(16);

        return hex.padStart(hex.length + (hex.length % 2), "0");
    },
});

/** Read `true` or `false`, refusing other text. */
export const parseAsBoolean: ParserBuilder<boolean> = createParser({
    parse: (text) => (text === "true" ? true : text === "false" ? false : null),
    serialize: (value) => String(value),
});

/** Read a date as its calendar day, `2026-10-08`, refusing other text. */
export const parseAsIsoDate: ParserBuilder<Date> = createParser({
    parse: (text) => parseDate(/^\d{4}-\d{2}-\d{2}$/u.test(text) ? text : ""),
    serialize: (value) => value.toISOString().slice(0, 10),
    eq: (left, right) => left.getTime() === right.getTime(),
});

/** Read an instant as its ISO 8601 date and time, refusing other text. */
export const parseAsIsoDateTime: ParserBuilder<Date> = createParser({
    parse: (text) => parseDate(text),
    serialize: (value) => value.toISOString(),
    eq: (left, right) => left.getTime() === right.getTime(),
});

/** Read an instant as its milliseconds since the epoch, refusing other text. */
export const parseAsTimestamp: ParserBuilder<Date> = createParser({
    parse: (text) => (/^-?\d+$/u.test(text) ? new Date(Number(text)) : null),
    serialize: (value) => String(value.getTime()),
    eq: (left, right) => left.getTime() === right.getTime(),
});

/** Read one of an enum's string values, refusing any other. */
export function parseAsStringEnum<Enum extends string>(
    values: readonly Enum[],
): ParserBuilder<Enum> {
    return parseAsStringLiteral(values);
}

/** Read one of some literal numbers, refusing any other. */
export function parseAsNumberLiteral<const Literal extends number>(
    literals: readonly Literal[],
): ParserBuilder<Literal> {
    return createParser({
        parse: (text) => literals.find((literal) => String(literal) === text) ?? null,
        serialize: (value) => String(value),
    });
}

/** Read one of some literal texts, refusing any other. */
export function parseAsStringLiteral<const Literal extends string>(
    literals: readonly Literal[],
): ParserBuilder<Literal> {
    return createParser({
        parse: (text) => literals.find((literal) => literal === text) ?? null,
        serialize: (value) => value,
    });
}

/** Read a list of items joined by a separator. */
export function parseAsArrayOf<Item>(
    item: Parser<Item>,
    separator = ",",
): ParserBuilder<readonly Item[]> {
    return createParser({
        parse: (text) => {
            // read each item, refusing the list for one refused item
            const items = text === "" ? [] : text.split(separator).map((part) => item.parse(part));

            return items.every((parsed) => parsed !== null) ? items : null;
        },
        serialize: (value) => value.map((entry) => item.serialize(entry)).join(separator),
        eq: (left, right) =>
            left.length === right.length &&
            left.every((entry, index) => {
                const other = right[index];

                return other !== undefined && item.eq(entry, other);
            }),
    });
}

/** Read a value as JSON a schema checks. */
export function parseAsJson<Value>(checked: schema.Schema<Value>): ParserBuilder<Value> {
    return createParser({
        parse: (text) => {
            // read the JSON and check it
            const parsed = checked.safeParse(parseJson(text));

            return parsed.success ? parsed.data : null;
        },
        serialize: (value) => JSON.stringify(value),
        eq: (left, right) => JSON.stringify(left) === JSON.stringify(right),
    });
}

/** Make a function that writes values as a query string for links to a page's URL state. */
export function createSerializer<Built extends Parsers>(
    parsers: Built,
    options: Pick<QueryStatesOptions<Built>, "urlKeys" | "clearOnDefault"> = {},
): Serializer<Built> {
    return (
        ...args: [Updates<Built> | null] | [string | URL | URLSearchParams, Updates<Built> | null]
    ) => {
        // merge each written value over the base's query
        const [base, values] = args.length === 1 ? ["", args[0]] : args;
        const query = new URLSearchParams(readQuery(base));
        for (const [key, text] of Object.entries(
            writeUpdates(parsers, values ?? cleared(parsers), options),
        )) {
            if (text === undefined) {
                query.delete(key);
            } else {
                query.set(key, text);
            }
        }
        const search = query.size === 0 ? "" : `?${query.toString()}`;

        // write the base's path and fragment around the query
        if (base instanceof URL) {
            return `${base.origin}${base.pathname}${search}${base.hash}`;
        } else if (base instanceof URLSearchParams) {
            return search;
        }
        const [path = ""] = base.split(/[?#]/u, 1);
        const hashAt = base.indexOf("#");

        return `${path}${search}${hashAt === -1 ? "" : base.slice(hashAt)}`;
    };
}

/** Make a function that reads a page's URL state from an address or a query. */
export function createLoader<Built extends Parsers>(
    parsers: Built,
    options: Pick<QueryStatesOptions<Built>, "urlKeys"> = {},
): (input: string | URL | URLSearchParams | Request) => Values<Built> {
    return (input) => {
        // read each key's texts from the address or query
        const query = new URLSearchParams(
            readQuery(input instanceof Request ? new URL(input.url) : input),
        );
        const texts = Object.fromEntries(
            [...new Set(query.keys())].map((key) => [key, query.getAll(key)] as const),
        );

        return readValues(parsers, texts, options);
    };
}

/** Follow several values of the URL's query. */
export function useQueryStates<Built extends Parsers>(
    parsers: Built,
    options: QueryStatesOptions<Built> = {},
): [Accessor<Values<Built>>, SetValues<Built>] {
    // read each value from its query key
    const location = useLocation();
    const [, setSearchParams] = useSearchParams();
    const values = createMemo(() => readValues(parsers, location.query, options));

    // replace a query with an invalid value, or a value set to its default, by its canonical form once hydrated
    createEffect(
        () => canonicalChanges(parsers, location.query, values(), options),
        (canonical) => {
            if (Object.keys(canonical).length > 0) {
                setSearchParams(canonical, { replace: true, scroll: false });
            }
        },
        { ssrSource: "client" },
    );

    return [
        values,
        async (updates, overrides = {}) => {
            // resolve the updates against the current values, null removing every value
            const resolved = typeof updates === "function" ? updates(untrack(values)) : updates;
            const next = resolved ?? cleared(parsers);

            // write the values, pushing when the call or a changed value's parser asks to
            const texts = writeUpdates(parsers, next, options);
            const isPush = (overrides.history ?? historyMode(parsers, next, options)) === "push";
            setSearchParams(texts, {
                replace: !isPush,
                scroll: overrides.scroll ?? options.scroll ?? false,
            });

            // resolve to the query as written
            const written = new URLSearchParams(location.search);
            for (const [key, text] of Object.entries(texts)) {
                if (text === undefined) {
                    written.delete(key);
                } else {
                    written.set(key, text);
                }
            }

            return written;
        },
    ];
}

/** Follow one value of the URL's query under a key as text, null where missing. */
export function useQueryState(
    key: string,
    options?: Options,
): [Accessor<string | null>, SetValue<string>];
/** Follow one value of the URL's query under a key, its default where missing or invalid. */
export function useQueryState<Value>(
    key: string,
    parser: ParserWithDefault<Value>,
): [Accessor<Value>, SetValue<Value>];
/** Follow one value of the URL's query under a key, null where missing or invalid. */
export function useQueryState<Value>(
    key: string,
    parser: ParserBuilder<Value>,
): [Accessor<Value | null>, SetValue<Value>];
/**
 * Follow one value of the URL's query under a key.
 *
 * NOTE #Architecture: the stated overloads of the value, as nuqs' useQueryState.
 */
export function useQueryState(
    key: string,
    parserOrOptions: ParserBuilder<unknown> | Options = {},
): [Accessor<unknown>, (value: never, options?: Options) => Promise<URLSearchParams>] {
    // read text with the options when given no parser
    const parser =
        parserOrOptions instanceof ParserBuilder
            ? parserOrOptions
            : parseAsString.withOptions(parserOrOptions);
    const [values, set] = useQueryStates({ [key]: parser });
    const value = () => values()[key] ?? null;

    return [
        value,
        async (next: unknown, options?: Options) =>
            set({ [key]: isUpdater(next) ? next(untrack(value)) : next }, options),
    ];
}

/** Read each parser's value from the query, its default or null where missing or invalid. */
function readValues<Built extends Parsers>(
    parsers: Built,
    query: Readonly<Record<string, string | string[] | undefined>>,
    options: QueryStatesOptions<Built>,
): Values<Built> {
    return mapValues<Built, Values<Built>>(parsers, (parser, key) =>
        parser.read(firstText(query[urlKey(key, options)])),
    );
}

/** Find the query changes that make the query canonical. */
function canonicalChanges<Built extends Parsers>(
    parsers: Built,
    query: Readonly<Record<string, string | string[] | undefined>>,
    values: Values<Built>,
    options: QueryStatesOptions<Built>,
): Record<string, string | undefined> {
    const changes: Record<string, string | undefined> = {};
    for (const [key, parser] of Object.entries(parsers)) {
        // write the value as the canonical text the query should have
        const queryKey = urlKey(key, options);
        const text = firstText(query[queryKey]);
        const canonical = parser.write(values[key], options);
        if (text !== undefined && text !== canonical) {
            changes[queryKey] = canonical;
        }
    }

    return changes;
}

/** Write each update as its query text, removing nulls and defaults left out. */
function writeUpdates<Built extends Parsers>(
    parsers: Built,
    updates: Updates<Built>,
    options: QueryStatesOptions<Built>,
): Record<string, string | undefined> {
    return Object.fromEntries(
        Object.entries(updates).flatMap(([key, value]) => {
            const parser = parsers[key];
            if (parser === undefined) {
                throw new TypeError(`no parser for query value ${key}`);
            }

            return [[urlKey(key, options), parser.write(value, options)]];
        }),
    );
}

/** Read whether any update's parser pushes a history entry. */
function historyMode<Built extends Parsers>(
    parsers: Built,
    updates: Updates<Built>,
    options: Options,
): "push" | "replace" {
    const pushes = Object.keys(updates).some((key) => parsers[key]?.options.history === "push");

    return pushes ? "push" : (options.history ?? "replace");
}

/** Read the query of a base: an address's or a path's, or the query itself. */
function readQuery(base: string | URL | URLSearchParams): URLSearchParams {
    if (base instanceof URLSearchParams) {
        return base;
    }

    return (base instanceof URL ? base : new URL(base, "http://query.invalid")).searchParams;
}

/** Write null for every value, removing them all. */
function cleared<Built extends Parsers>(parsers: Built): Updates<Built> {
    return mapValues<Built, Updates<Built>>(parsers, () => null);
}

/** Check whether a set value is a function of the current one. */
function isUpdater(value: unknown): value is (old: unknown) => unknown {
    return typeof value === "function";
}

/** Read the first text of a query value, which repeats a key as a list. */
function firstText(value: string | string[] | undefined): string | undefined {
    return Array.isArray(value) ? value[0] : value;
}

/** Read the query key of a value, its own key by default. */
function urlKey<Built extends Parsers>(key: string, options: QueryStatesOptions<Built>): string {
    return options.urlKeys?.[key] ?? key;
}

/** Read a date from its text, null for an invalid one. */
function parseDate(text: string): Date | null {
    const date = new Date(text);

    return text === "" || Number.isNaN(date.getTime()) ? null : date;
}

/** Read JSON, undefined for text that is not JSON. */
function parseJson(text: string): unknown {
    try {
        return JSON.parse(text);
    } catch {
        return undefined;
    }
}
