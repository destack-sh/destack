import { type Message, t } from "@destack/locale";
import { schema } from "@destack/schema";

/** A family name CSS reads unquoted, one identifier, such as `serif` or `-apple-system`. */
const IDENTIFIER = /^-?[A-Za-z_][\w-]*$/u;

/** The quotes around a family's name in CSS. */
const QUOTES = /^["']|["']$/gu;

/** The fonts text is set in, the first one first, as the design token format lists them. */
const fontFamilyJson = schema
    .array(schema.string().min(1))
    .min(1)
    .meta({ id: "style.fontFamily", title: "Font family" });
/** The fonts text is set in, the first one first. */
export type FontFamily = schema.Infer<typeof fontFamilyJson>;

/** The fonts text is set in, the first one first, as the design token format lists them. */
export const FontFamily = Object.assign(fontFamilyJson, {
    /** Read the fonts of a CSS `font-family` list, none for empty text. */
    read(text: string): string[] {
        return text
            .split(",")
            .map((family) => family.trim().replaceAll(QUOTES, ""))
            .filter((family) => family.length > 0);
    },

    /** Write the fonts as CSS's `font-family`, quoting names that are no single identifier. */
    css(families: FontFamily): string {
        return families
            .map((family) => (IDENTIFIER.test(family) ? family : JSON.stringify(family)))
            .join(", ");
    },
});

/** How heavy text is set, from 1 to 1000, as CSS and the design token format weigh fonts. */
export const FontWeight = schema
    .number()
    .int()
    .min(1)
    .max(1000)
    .meta({ id: "style.fontWeight", title: "Font weight" });
/** How heavy text is set. */
export type FontWeight = schema.Infer<typeof FontWeight>;

/** The weights the design token format names, as their numbers, titled. */
export const FONT_WEIGHTS = {
    thin: { weight: 100, title: t.context("option")`Thin` },
    "extra-light": { weight: 200, title: t.context("option")`Extra light` },
    light: { weight: 300, title: t.context("option")`Light` },
    normal: { weight: 400, title: t.context("option")`Regular` },
    medium: { weight: 500, title: t.context("option")`Medium` },
    "semi-bold": { weight: 600, title: t.context("option")`Semibold` },
    bold: { weight: 700, title: t.context("option")`Bold` },
    "extra-bold": { weight: 800, title: t.context("option")`Extra bold` },
    black: { weight: 900, title: t.context("option")`Black` },
    "extra-black": { weight: 950, title: t.context("option")`Extra black` },
} satisfies Record<string, { readonly weight: FontWeight; readonly title: Message }>;

/** Whether text is set upright or slanted, as CSS's `font-style` names it. */
export const FontStyle = schema
    .enum(["normal", "italic", "oblique"])
    .meta({ id: "style.fontStyle", title: "Font style" });
/** Whether text is set upright or slanted. */
export type FontStyle = schema.Infer<typeof FontStyle>;

/** Whether text is set upright or slanted, titled. */
export const FONT_STYLES = {
    normal: { title: t.context("option")`Upright` },
    italic: { title: t.context("option")`Italic` },
    oblique: { title: t.context("option")`Oblique` },
} satisfies Record<FontStyle, { readonly title: Message }>;

/** A tag OpenType gives a feature or an axis: four letters, digits or spaces. */
const TAG = schema.string().regex(/^[\x20-\x7e]{4}$/u, { message: "a tag is four characters" });

/** The OpenType features text is set with, by tag, 0 for off and 1 for on, as CSS sets them. */
export const FontFeatures = Object.assign(
    schema
        .record(TAG, schema.number().int().nonnegative())
        .meta({ id: "style.fontFeatures", title: "Font features" }),
    {
        /** Write the features as CSS's `font-feature-settings`, normal for none. */
        css(features: FontFeatures): string {
            return css(features, "normal");
        },
    },
);
/** The OpenType features text is set with, by tag. */
export type FontFeatures = schema.Infer<typeof FontFeatures>;

/** The variable font axes text is set at, by tag, as CSS's `font-variation-settings`. */
export const FontVariations = Object.assign(
    schema.record(TAG, schema.number()).meta({ id: "style.fontVariations", title: "Font axes" }),
    {
        /** Write the axes as CSS's `font-variation-settings`, normal for none. */
        css(variations: FontVariations): string {
            return css(variations, "normal");
        },
    },
);
/** The variable font axes text is set at, by tag. */
export type FontVariations = schema.Infer<typeof FontVariations>;

/** Write tagged numbers as CSS's quoted list, or a keyword for none. */
function css(settings: Readonly<Record<string, number>>, none: string): string {
    const entries = Object.entries(settings);

    return entries.length === 0
        ? none
        : entries.map(([tag, value]) => `${JSON.stringify(tag)} ${String(value)}`).join(", ");
}
