import { MessageFormat } from "messageformat";
import { type Direction, Locale, type LocaleTag } from "../locale/locale.ts";
import type { Catalog } from "../message/catalog.ts";
import type { Message } from "../message/message.ts";

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

/** The compiled formatters, by locale and source, compiled once each. */
const COMPILED = new Map<string, MessageFormat>();

/** A person's locale bound to the packages' catalogs, rendering messages and formatting values in it. */
export interface Localization {
    /** The language and region the person reads. */
    readonly tag: LocaleTag;
    /** The direction text and controls run in, the locale's own unless a host sets another. */
    readonly direction: Direction;

    /** Render a message in the nearest locale its package's catalogs translate it into, else in its source language. */
    render(message: Message): string;

    /** Write a number. */
    number(value: number, options?: Intl.NumberFormatOptions): string;
    /** Write an amount in its currency's minor unit, such as 1200 EUR cents as €12.00, a custom x- currency by its code. */
    money(amount: number, currency: string): string;
    /** Write an instant given in UTC epoch milliseconds. */
    date(time: number, options?: Intl.DateTimeFormatOptions): string;
    /** Write the span between two instants given in UTC epoch milliseconds, sharing the parts they have in common. */
    dateRange(start: number, end: number, options?: Intl.DateTimeFormatOptions): string;
    /** Write a list joined as the locale joins one, such as a, b and c. */
    list(items: readonly string[], options?: Intl.ListFormatOptions): string;
    /** Write an instant relative to now in its largest whole unit, such as 3 days ago. */
    relative(time: number, now: number): string;
}

/** How a localization reads its messages and runs its text. */
export interface LocalizationOptions {
    /** The language messages are written in, English when absent. */
    readonly source?: LocaleTag;
    /** The direction text and controls run in, the locale's own when absent. */
    readonly direction?: Direction;
}

/** The language messages are written in when a localization names none. */
export const SOURCE_LOCALE: LocaleTag = "en";

/** The localizations people read in. */
export const Localization = {
    /** Bind a locale to the catalogs of the packages whose messages it renders, messages written in the source language. */
    of(
        tag: LocaleTag,
        catalogs: readonly Catalog[],
        { source = SOURCE_LOCALE, direction }: LocalizationOptions = {},
    ): Localization {
        // index the translations by package and locale, and walk the fallback chain once
        const translations = new Map(
            catalogs.map((catalog) => [`${catalog.package}\u0000${catalog.locale}`, catalog]),
        );
        const chain = Locale.fallback(tag, source);

        // build each formatter once per options and reuse it
        const numbers = cached(
            (options?: Intl.NumberFormatOptions) => new Intl.NumberFormat(tag, options),
        );
        const dates = cached(
            (options?: Intl.DateTimeFormatOptions) => new Intl.DateTimeFormat(tag, options),
        );
        const lists = cached(
            (options?: Intl.ListFormatOptions) => new Intl.ListFormat(tag, options),
        );
        const relatives = new Intl.RelativeTimeFormat(tag, { numeric: "auto" });

        return {
            tag,
            direction: direction ?? Locale.direction(tag),
            render: (message) => render(message, chain, source, translations),
            number: (value, options) => numbers(options).format(value),
            money: (amount, currency) => money(amount, currency, numbers),
            date: (time, options) => dates(options).format(time),
            dateRange: (start, end, options) => dates(options).formatRange(start, end),
            list: (items, options) => lists(options).format(items),
            relative: (time, now) => relative(time, now, relatives),
        };
    },
};

/** Render a message in the nearest translation of its package along a fallback chain, else in its source text. */
function render(
    message: Message,
    chain: readonly LocaleTag[],
    source: LocaleTag,
    translations: ReadonlyMap<string, Catalog>,
): string {
    // take the nearest translation its own package's catalogs hold
    for (const tag of chain) {
        const translated = translations.get(`${message.package}\u0000${tag}`)?.messages[message.id];
        if (translated !== undefined) {
            return compile(tag, translated).format(message.values);
        }
    }

    // read the source text in the source language
    return compile(source, message.source).format(message.values);
}

/** Compile a MessageFormat 2 source for a locale once. */
function compile(locale: LocaleTag, source: string): MessageFormat {
    // reuse the formatter compiled for the locale and source
    const key = `${locale}\u0000${source}`;
    let compiled = COMPILED.get(key);
    if (compiled === undefined) {
        compiled = new MessageFormat(locale, source);
        COMPILED.set(key, compiled);
    }

    return compiled;
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

/** Write an instant relative to now in its largest whole unit, seconds below a second. */
function relative(time: number, now: number, relatives: Intl.RelativeTimeFormat): string {
    // take the largest unit the distance covers once, seconds below that
    const distance = time - now;
    const [unit, length] = UNITS.find(([, size]) => Math.abs(distance) >= size) ?? SECOND;

    return relatives.format(Math.round(distance / length), unit);
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
