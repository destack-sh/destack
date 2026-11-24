import { expect, test } from "@destack/test";
import type { LocaleTag } from "../locale/index.ts";
import type { NumberFormat } from "./format.ts";
import { NumberParser } from "./parser.ts";

/** Read texts in a language and a format. */
function read(tag: LocaleTag, texts: readonly string[], format?: NumberFormat) {
    const parser = new NumberParser(tag, format);

    return texts.map((text) => parser.parse(text));
}

test("read numbers as languages write them", () => {
    expect({
        en: read("en", ["1,234.5", "-3", "\u22123", "+4", ".5", "(12)", "12-"]),
        de: read("de", ["1.234,5", "1.5", "1.234", "1234,50"]),
        ch: read("de-CH", ["1\u2019234.5", "1'234.5"]),
        fr: read("fr", ["1\u202f234,5", "1 234,5"]),
        ar: read("ar-EG", ["\u0661\u0662\u0663\u066b\u0665", "123"]),
        hi: read("hi-IN-u-nu-deva", ["\u0967\u0968,\u0969\u096a\u096b"]),
    }).toEqual({
        en: [1234.5, -3, -3, 4, 0.5, -12, -12],
        de: [1234.5, 1.5, 1234, 1234.5],
        ch: [1234.5, 1234.5],
        fr: [1234.5, 1234.5],
        ar: [123.5, 123],
        hi: [12345],
    });
});

test("read what a format writes around a number", () => {
    expect({
        euro: read("de", ["1.234,50 €", "€ 12", "12"], { style: "currency", currency: "EUR" }),
        dollar: read("en", ["$1,234.50", "-$5", "USD 3"], { style: "currency", currency: "USD" }),
        percent: read("en", ["12.3%", "50", "7 %"], { style: "percent" }),
        unit: read("en", ["3 km", "1,200 km"], { style: "unit", unit: "kilometer" }),
        words: read("de", ["5 Tage", "1 Tag"], { style: "unit", unit: "day", unitDisplay: "long" }),
    }).toEqual({
        euro: [1234.5, 12, 12],
        dollar: [1234.5, -5, undefined],
        percent: [0.123, 0.5, 0.07],
        unit: [3, 1200],
        words: [5, 1],
    });
});

test("read no number from other text", () => {
    expect(read("en", ["", "-", "abc", "1.2.3", "1e5", "Infinity"])).toEqual(
        Array.from({ length: 6 }),
    );
});

test("read back every number it writes", () => {
    const formats: readonly NumberFormat[] = [
        { maximumFractionDigits: 2 },
        { style: "currency", currency: "EUR" },
        { style: "percent", maximumFractionDigits: 1 },
        { style: "unit", unit: "kilometer", unitDisplay: "long", maximumFractionDigits: 2 },
    ];
    const tags = ["en", "de", "de-CH", "fr", "es", "it", "nl", "pt", "ar-EG", "hi", "ja", "en-IN"];
    const values = [0, 1, -1, 1234567.5, -42.25, 0.5, 1000];
    const wrong = tags.flatMap((tag) =>
        formats.flatMap((format) => {
            const parser = new NumberParser(tag, format);
            const written = new Intl.NumberFormat(tag, format);

            return values
                .filter((value) => parser.parse(written.format(value)) !== value)
                .map((value) => [tag, written.format(value)]);
        }),
    );

    expect(wrong).toEqual([]);
});
