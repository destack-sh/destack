import { defineSchema, schema } from "@destack/schema";

/** The parents CLDR gives locales whose parent is not their language alone, such as en-AT reading as en-001. */
const PARENTS: Readonly<Record<string, string>> = {
    "en-150": "en-001",
    "en-AT": "en-150",
    "en-AU": "en-001",
    "en-CA": "en-001",
    "en-CH": "en-150",
    "en-DE": "en-150",
    "en-GB": "en-001",
    "en-IE": "en-001",
    "en-IN": "en-001",
    "en-NZ": "en-001",
    "es-AR": "es-419",
    "es-CL": "es-419",
    "es-CO": "es-419",
    "es-MX": "es-419",
    "es-US": "es-419",
    "pt-AO": "pt-PT",
    "pt-MZ": "pt-PT",
    "zh-HK": "zh-Hant",
    "zh-MO": "zh-Hant-HK",
    "zh-TW": "zh-Hant",
};

/** The scripts written right to left, after CLDR's character order data. */
const RIGHT_TO_LEFT_SCRIPTS = new Set([
    "Adlm",
    "Arab",
    "Hebr",
    "Mand",
    "Mend",
    "Nkoo",
    "Rohg",
    "Samr",
    "Syrc",
    "Thaa",
    "Yezi",
]);

/** A BCP 47 language tag, such as de-AT, in its canonical casing: language, script, region or UN M.49 area, variants. */
export const LocaleTag = defineSchema(
    schema
        .string()
        .regex(
            /^[a-z]{2,3}(?:-[A-Z][a-z]{3})?(?:-(?:[A-Z]{2}|\d{3}))?(?:-(?:[a-z\d]{5,8}|\d[a-z\d]{3}))*$(?![\s\S])/u,
        ),
);

/** A BCP 47 language tag. */
export type LocaleTag = schema.Infer<typeof LocaleTag>;

/** The direction a script runs in: left to right or right to left. */
export const Direction = defineSchema(schema.enum(["ltr", "rtl"]));

/** The direction a script runs in. */
export type Direction = schema.Infer<typeof Direction>;

/** The language tags people read and packages write in. */
export const Locale = {
    /** Read the direction a locale writes in from the script it most likely uses. */
    direction(tag: LocaleTag): Direction {
        const script = new Intl.Locale(tag).maximize().script;

        return script !== undefined && RIGHT_TO_LEFT_SCRIPTS.has(script) ? "rtl" : "ltr";
    },

    /** Read a language tag in its canonical form, refusing one that is no tag. */
    parse(tag: string): LocaleTag {
        const canonical = canonicalize(tag);
        if (canonical === undefined) {
            throw new TypeError(`not a language tag: ${tag}`);
        }

        return canonical;
    },

    /** List the tags to read a message in, nearest first: the tag, its parents and the source language. */
    fallback(tag: LocaleTag, source: LocaleTag): LocaleTag[] {
        // walk up the parents, CLDR's own first, else dropping the last subtag
        const chain: LocaleTag[] = [];
        for (
            let current: string | undefined = tag;
            current !== undefined;
            current = parentOf(current)
        ) {
            chain.push(current);
        }

        // end in the source language
        return chain.includes(source) ? chain : [...chain, source];
    },

    /** Read the tags an Accept-Language header asks for, the most preferred first, leaving out the wildcard and malformed tags (RFC 9110 12.5.4). */
    accepted(header: string): LocaleTag[] {
        // read each range's tag and weight, one by default
        const ranges = header.split(",").flatMap((range, index) => {
            // split the tag from its parameters, refusing the wildcard and malformed tags
            const [tag = "", ...parameters] = range.split(";").map((part) => part.trim());
            const weight = parameters.find((parameter) => parameter.startsWith("q="));
            const canonical = tag === "*" ? undefined : canonicalize(tag);
            const quality = weight === undefined ? 1 : Number(weight.slice(2));

            return canonical === undefined || !(quality > 0) ? [] : [{ canonical, quality, index }];
        });

        // order the tags by weight, keeping the header's order among equal weights
        return ranges
            .toSorted((left, right) => right.quality - left.quality || left.index - right.index)
            .map((range) => range.canonical);
    },

    /** Pick the available tag that serves a person's preferred tags best, absent when none shares a language (RFC 4647 lookup). */
    negotiate(
        preferred: readonly LocaleTag[],
        available: readonly LocaleTag[],
    ): LocaleTag | undefined {
        for (const tag of preferred) {
            const match = Locale.fallback(tag, tag).find((candidate) =>
                available.includes(candidate),
            );
            if (match !== undefined) {
                return match;
            }
        }

        return undefined;
    },
};

/** Canonicalize a language tag, absent for a malformed one. */
function canonicalize(tag: string): string | undefined {
    try {
        return Intl.getCanonicalLocales(tag)[0];
    } catch (error) {
        if (!(error instanceof RangeError)) {
            throw error;
        }

        return undefined;
    }
}

/** Read a tag's parent: the one CLDR names, else the tag without its last subtag, absent for a language alone. */
function parentOf(tag: string): string | undefined {
    // take the parent CLDR names
    const named = PARENTS[tag];
    if (named !== undefined) {
        return named;
    }

    // drop the last subtag
    const end = tag.lastIndexOf("-");

    return end === -1 ? undefined : tag.slice(0, end);
}
