import { type NumberFormat, NumberParser } from "../number/index.ts";
import { MessageFormat } from "messageformat";
import { type Direction, Locale, type LocaleTag } from "../locale/locale.ts";
import type { Catalog } from "../message/catalog.ts";
import type { Message } from "../message/message.ts";
import { type JsonValue, type PlainDate, TimeZone, type Week } from "@destack/schema";
import type { PackageId } from "@destack/package";

/** The smallest relative time unit, with its length in milliseconds. */
const SECOND: readonly [Intl.RelativeTimeFormatUnit, number] = ["second", 1000];

/** The relative time units, from the largest, with their length in milliseconds. */
const UNITS: readonly (readonly [Intl.RelativeTimeFormatUnit, number])[] = [
    ["year", 365 * 24 * 60 * 60 * 1000],
    ["month", 30 * 24 * 60 * 60 * 1000],
    ["week", 7 * 24 * 60 * 60 * 1000],
    ["day", 24 * 60 * 60 * 1000],
    ["hour", 60 * 60 * 1000],
    ["minute", 60 * 1000],
    SECOND,
];

/** How an instant is written relative to now. */
export interface RelativeFormat {
    /** How long the words are, long by default, such as 3 days ago, 3 days ago or 3d ago. */
    readonly style?: Intl.RelativeTimeFormatStyle;
    /** The smallest unit written, below which it is now, seconds by default. */
    readonly precision?: Intl.RelativeTimeFormatUnitSingular;
}

/** The arguments a message's reader supplies, none for a message without any. */
export type RenderArguments<Names extends string> = [Names] extends [never]
    ? []
    : string extends Names
      ? [arguments?: Readonly<Record<string, JsonValue>>]
      : [arguments: Readonly<Record<Names, JsonValue>>];

/** A person's locale bound to the packages' catalogs, rendering messages and formatting values. */
export interface Localization {
    /** The language and region the person reads. */
    readonly tag: LocaleTag;
    /** The direction text and controls run in, the locale's own unless a host sets another. */
    readonly direction: Direction;
    /** The week of the locale. */
    readonly week: Week;

    /** Render a message with its arguments in the nearest locale its catalogs translate it into. */
    readonly render: <Names extends string>(
        message: Message<Names>,
        ...rest: RenderArguments<Names>
    ) => string;

    /** Write a number. */
    readonly number: (value: number, options?: Intl.NumberFormatOptions) => string;
    /** Read a number as people of the locale write it, undefined for text that is none. */
    readonly parseNumber: (text: string, format?: NumberFormat) => number | undefined;
    /** Write an amount in its currency's minor unit, such as 1200 EUR cents as €12.00. */
    readonly money: (amount: number, currency: string) => string;
    /** Write an instant given in UTC epoch milliseconds. */
    readonly date: (time: number, options?: Intl.DateTimeFormatOptions) => string;
    /** Write the span between two UTC epoch millisecond instants, sharing their common parts. */
    readonly dateRange: (
        start: number,
        end: number,
        options?: Intl.DateTimeFormatOptions,
    ) => string;
    /** Write a calendar date in the locale. */
    readonly plainDate: (date: PlainDate, options?: Intl.DateTimeFormatOptions) => string;
    /** Write the span between two calendar dates. */
    readonly plainDateRange: (
        start: PlainDate,
        end: PlainDate,
        options?: Intl.DateTimeFormatOptions,
    ) => string;
    /** Write a list joined as the locale joins one, such as a, b and c. */
    readonly list: (items: readonly string[], options?: Intl.ListFormatOptions) => string;
    /** Write an instant relative to now in its largest whole unit, such as 3 days ago. */
    readonly relative: (time: number, now: number, format?: RelativeFormat) => string;
}

/** How a localization reads its messages and runs its text. */
export interface LocalizationOptions {
    /** The language messages are written in, English when absent. */
    readonly source?: LocaleTag;
    /** The direction text and controls run in, the locale's own when absent. */
    readonly direction?: Direction;
    /** Ask for the catalogs of a package whose message renders without any. */
    readonly request?: (packageId: PackageId) => void;
}

/** The language messages are written in when a localization names none. */
export const SOURCE_LOCALE: LocaleTag = "en";

/** Unicode's mark that opens text of its own direction, after its first letter. */
const FIRST_STRONG_ISOLATE = "\u2068";

/** Unicode's mark that closes text of its own direction. */
const POP_ISOLATE = "\u2069";

/** The localizations people read in. */
export const Localization = {
    /** Bind a locale to the catalogs of the packages whose messages it renders. */
    of(
        tag: LocaleTag,
        catalogs: readonly Catalog[],
        { source = SOURCE_LOCALE, direction, request }: LocalizationOptions = {},
    ): Localization {
        // index the translations by package and locale, and walk the fallback chain once
        const translations = new Map(
            catalogs.map((catalog) => [`${catalog.package}\u0000${catalog.locale}`, catalog]),
        );
        const packages = new Set(catalogs.map((catalog) => catalog.package));
        const chain = Locale.fallback(tag, source);
        const runs = direction ?? Locale.direction(tag);

        // build each formatter once per options and reuse it
        const numbers = cached(
            (options?: Intl.NumberFormatOptions) => new Intl.NumberFormat(tag, options),
        );
        const parsers = cached((options?: NumberFormat) => new NumberParser(tag, options));
        const dates = cached(
            (options?: Intl.DateTimeFormatOptions) => new Intl.DateTimeFormat(tag, options),
        );
        const lists = cached(
            (options?: Intl.ListFormatOptions) => new Intl.ListFormat(tag, options),
        );
        const relatives = cached(
            (options?: Intl.RelativeTimeFormatOptions) =>
                new Intl.RelativeTimeFormat(tag, { numeric: "auto", ...options }),
        );
        const compiled = new Map<string, MessageFormat>();
        const compile = (locale: LocaleTag, text: string) => {
            // compile each locale's text once
            const key = `${locale}\u0000${text}`;
            const found = compiled.get(key) ?? new MessageFormat(locale, text);
            compiled.set(key, found);

            return found;
        };

        return {
            tag,
            direction: runs,
            week: Locale.week(tag),
            render: (message, ...[supplied]) => {
                // ask for the catalogs of a package rendering without any
                if (!packages.has(message.package)) {
                    request?.(message.package);
                }

                const [text, written] = render(
                    { ...message, values: { ...message.values, ...supplied } },
                    chain,
                    source,
                    translations,
                    compile,
                );

                // keep text of a language that runs the other way in its own order
                return Locale.direction(written) === runs ? text : isolate(text);
            },
            number: (value, options) => numbers(options).format(value),
            parseNumber: (text, options) => parsers(options).parse(text),
            money: (amount, currency) => money(amount, currency, numbers),
            date: (time, options) => dates(options).format(time),
            dateRange: (start, end, options) => dates(options).formatRange(start, end),
            plainDate: (date, options) =>
                dates({ ...options, timeZone: "UTC" }).format(
                    TimeZone.instant("UTC", date, "00:00"),
                ),
            plainDateRange: (start, end, options) =>
                dates({ ...options, timeZone: "UTC" }).formatRange(
                    TimeZone.instant("UTC", start, "00:00"),
                    TimeZone.instant("UTC", end, "00:00"),
                ),
            list: (items, options) => lists(options).format(items),
            relative: (time, now, { style = "long", precision = "second" } = {}) =>
                relative(time, now, relatives({ style }), precision),
        };
    },
};

/** Render a message in the nearest translation of its package, with the locale that has it. */
function render(
    message: Message,
    chain: readonly LocaleTag[],
    source: LocaleTag,
    translations: ReadonlyMap<string, Catalog>,
    compile: (locale: LocaleTag, text: string) => MessageFormat,
): readonly [text: string, locale: LocaleTag] {
    // take the nearest translation its own package's catalogs hold
    for (const tag of chain) {
        const translated = translations.get(`${message.package}\u0000${tag}`)?.messages[message.id];
        if (translated !== undefined) {
            return [format(compile(tag, translated), message.values), tag];
        }
    }

    // read the source text in the source language
    return [format(compile(source, message.source), message.values), source];
}

/** Isolate text from the direction of the text around it, its first letter setting its own. */
function isolate(text: string): string {
    return `${FIRST_STRONG_ISOLATE}${text}${POP_ISOLATE}`;
}

/** Format a compiled message, refusing one that fails, such as one missing an argument. */
function format(compiled: MessageFormat, values: Readonly<Record<string, unknown>>): string {
    // collect the formatter's errors, which it would otherwise log and render around
    const errors: unknown[] = [];
    const text = compiled.format(values, (error) => errors.push(error));
    const [first] = errors;
    if (first !== undefined) {
        throw new TypeError(
            `could not format message: '${first instanceof Error ? first.message : JSON.stringify(first)}'`,
        );
    }

    return text;
}

/** Write an amount in its currency's minor unit, a custom x- currency by its code. */
function money(
    amount: number,
    currency: string,
    numbers: (options?: Intl.NumberFormatOptions) => Intl.NumberFormat,
): string {
    // write a custom currency's amount beside its code
    if (currency.startsWith("x-")) {
        return `${numbers().format(amount)} ${currency.slice(2)}`;
    }

    // write the amount shifted by the currency's minor unit digits
    const formatter = numbers({ style: "currency", currency });
    const digits = formatter.resolvedOptions().maximumFractionDigits ?? 2;

    return formatter.format(amount / 10 ** digits);
}

/** Write an instant relative to now in its largest whole unit, now below the smallest. */
function relative(
    time: number,
    now: number,
    relatives: Intl.RelativeTimeFormat,
    precision: Intl.RelativeTimeFormatUnitSingular,
): string {
    // take the largest unit the distance covers once, seconds below that
    const distance = time - now;
    const least = UNITS.find(([unit]) => unit === precision)?.[1] ?? SECOND[1];
    if (Math.abs(distance) < least) {
        return relatives.format(0, SECOND[0]);
    }

    // round in that unit, then take the unit the rounded distance covers: 59.9 minutes are an hour
    const [, size] = unitOf(distance);
    const rounded = Math.round(distance / size) * size;
    const [unit, length] = unitOf(rounded);

    return relatives.format(Math.round(rounded / length), unit);
}

/** Find the largest unit a distance covers once, seconds below a minute. */
function unitOf(distance: number): (typeof UNITS)[number] {
    return UNITS.find(([, size]) => Math.abs(distance) >= size) ?? SECOND;
}

/** Return a factory that builds a formatter once per options and returns the same one after. */
function cached<Options, Formatter>(
    create: (options?: Options) => Formatter,
): (options?: Options) => Formatter {
    const formatters = new Map<string, Formatter>();

    return (options) => {
        // reuse the formatter of equal options
        const key = JSON.stringify(options ?? {});
        const known = formatters.get(key);
        if (known !== undefined) {
            return known;
        }

        // build and keep the first one
        const formatter = create(options);
        formatters.set(key, formatter);

        return formatter;
    };
}
