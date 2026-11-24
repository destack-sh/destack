import type { LocaleTag } from "../locale/index.ts";
import type { NumberFormat } from "./format.ts";

/** The marks that set a text's direction, which numbers of some languages have inside. */
const DIRECTION_MARKS = /[\u061c\u200e\u200f\u202a-\u202e\u2066-\u2069]/gu;

/** The spaces that group digits or stand before a unit. */
const SPACES = /[\s\u00a0\u2009\u202f]/gu;

/** The signs written for a minus. */
const MINUSES = /[\u2212\u2010\u2011\u2012\u2013\ufe63\uff0d]/gu;

/** The apostrophes that group digits. */
const APOSTROPHES = /['\u2019]/gu;

/** A number once its digits, its separator and its sign are read. */
const NUMBER = /^[+-]?(?:\d+\.?\d*|\.\d+)$/u;

/** A separator with other than three digits after it, which groups none. */
const SINGLE = /^([+-]?\d+)\.(\d{1,2}|\d{4,})$/u;

/** How many decimals a share keeps past those it is written with, so 12.3% reads 0.123. */
const PERCENT_DIGITS = 2;

/** The numbers whose parts tell the words a language writes around one. */
const SAMPLES = [-1234567.89, 1234567.89, 0, 1, 2, 5, 11];

/** Reads numbers as people of a language write them. */
export class NumberParser {
    /** The language's own digits, each by the character written for it. */
    readonly #digits: ReadonlyMap<string, string>;
    /** What separates the decimals. */
    readonly #decimal: string;
    /** What groups the digits. */
    readonly #group: string;
    /** The words and signs a format writes around a number, the longest first. */
    readonly #words: readonly string[];
    /** Whether numbers are shares, written a hundred times larger. */
    readonly #isPercent: boolean;

    /** Read numbers of a language written in a format. */
    constructor(tag: LocaleTag, format: NumberFormat = {}) {
        // read what the language writes for digits, groups and decimals
        const plain = new Intl.NumberFormat(tag, { useGrouping: false });
        const digits = Array.from({ length: 10 }, (_digit, value) => String(value));
        this.#digits = new Map(digits.map((value) => [plain.format(Number(value)), value]));
        const parts = new Intl.NumberFormat(tag, { maximumFractionDigits: 2 }).formatToParts(
            -1234567.89,
        );
        this.#decimal = parts.find(({ type }) => type === "decimal")?.value ?? ".";
        this.#group = parts.find(({ type }) => type === "group")?.value ?? ",";

        // read what the format writes around them
        const written = new Intl.NumberFormat(tag, format);
        const words = SAMPLES.flatMap((sample) => written.formatToParts(sample)).flatMap(
            ({ type, value }) =>
                ["literal", "currency", "unit", "percentSign"].includes(type) ? [value.trim()] : [],
        );
        this.#words = [...new Set(words)]
            .filter((word) => word !== "")
            .toSorted((one, other) => other.length - one.length);
        this.#isPercent = format.style === "percent";
    }

    /** Read a number from text, undefined for text that is none. */
    parse(text: string): number | undefined {
        // read parentheses as a minus, as accounts write one, and a minus after the number
        const plain = this.#plain(text);
        const isNegative = /^\(.+\)$/u.test(plain) || /^[^-].*-$/u.test(plain);
        let rest = isNegative ? `-${plain.replace(/[()-]/gu, "")}` : plain;

        // read a lone separator by the digits after it, where it groups none
        const group = this.#group.replace(SPACES, "").replace(APOSTROPHES, "'");
        const single = group === "." ? SINGLE.exec(rest) : null;
        if (single === null) {
            rest = group === "" ? rest : rest.replaceAll(group, "");
            rest = rest.replace(this.#decimal, ".");
        }
        if (!NUMBER.test(rest)) {
            return undefined;
        }
        const value = Number(rest);

        return this.#isPercent ? shareOf(value, rest) : value;
    }

    /** Leave out direction, words and spaces, and write every sign and digit alike. */
    #plain(text: string): string {
        // leave out the words, the longest first
        let rest = text.replace(DIRECTION_MARKS, "");
        for (const word of this.#words) {
            rest = rest.replaceAll(word, "");
        }
        rest = rest.replace(SPACES, "").replace(MINUSES, "-").replace(APOSTROPHES, "'");

        return Array.from(rest, (each) => this.#digits.get(each) ?? each).join("");
    }
}

/** Read a share written a hundred times larger, without the noise of binary fractions. */
function shareOf(value: number, text: string): number {
    const decimals = text.split(".")[1]?.length ?? 0;

    return Number((value / 100).toFixed(decimals + PERCENT_DIGITS));
}
