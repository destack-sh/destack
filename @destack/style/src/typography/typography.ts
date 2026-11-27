import { schema } from "@destack/schema";
import { Dimension } from "../dimension/index.ts";
import { FontFamily, FontWeight } from "../font/index.ts";
import { type Bindable, Reference } from "../reference/index.ts";

/** How text is set, as the design token format's typography. */
const typographyJson = schema
    .object({
        /** The fonts, the first one first. */
        fontFamily: FontFamily.bindable,
        /** The size. */
        fontSize: Dimension.bindable,
        /** The weight, 400 for regular and 700 for bold. */
        fontWeight: FontWeight.bindable,
        /** The room between letters. */
        letterSpacing: Dimension.bindable,
        /** The height of a line, as a multiple of the size. */
        lineHeight: schema.number().positive(),
    })
    .meta({ id: "style.typography", title: "Typography" });
/** How text is set. */
export type Typography = schema.Infer<typeof typographyJson>;

/** How text is set, as the design token format's typography. */
export const Typography = Object.assign(typographyJson, {
    /** Typography, or a reference to a typography token in its place. */
    bindable: Reference.or(typographyJson, "typography"),

    /** Set text as CSS's font properties, a reference as its token's member variables. */
    css(typography: Bindable<Typography>): TypographyStyle {
        // read each member of a typography token from its own variable
        if (Reference.is(typography)) {
            const path = Reference.path(typography);
            const member = (name: string): string => `var(${Reference.property([...path, name])})`;

            return {
                fontFamily: member("fontFamily"),
                fontSize: member("fontSize"),
                fontWeight: member("fontWeight"),
                letterSpacing: member("letterSpacing"),
                lineHeight: member("lineHeight"),
            };
        }
        // write each member of the value itself
        else {
            return {
                fontFamily: FontFamily.css(typography.fontFamily),
                fontSize: Dimension.css(typography.fontSize),
                fontWeight: FontWeight.css(typography.fontWeight),
                letterSpacing: Dimension.css(typography.letterSpacing),
                lineHeight: String(typography.lineHeight),
            };
        }
    },
});

/** Typography as CSS properties. */
export interface TypographyStyle {
    /** The font family. */
    readonly fontFamily: string;
    /** The font size. */
    readonly fontSize: string;
    /** The font weight. */
    readonly fontWeight: string;
    /** The letter spacing. */
    readonly letterSpacing: string;
    /** The line height. */
    readonly lineHeight: string;
}
