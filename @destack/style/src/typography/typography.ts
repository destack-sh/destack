import { schema } from "@destack/schema";
import { Dimension } from "../dimension/index.ts";
import { FontFamily, FontWeight } from "../font/index.ts";

/** How text is set, as the design token format's typography. */
const typographyJson = schema
    .object({
        /** The fonts, the first one first. */
        fontFamily: FontFamily,
        /** The size. */
        fontSize: Dimension,
        /** The weight, 400 for regular and 700 for bold. */
        fontWeight: FontWeight,
        /** The room between letters. */
        letterSpacing: Dimension,
        /** The height of a line, as a multiple of the size. */
        lineHeight: schema.number().positive(),
    })
    .meta({ id: "style.typography", title: "Typography" });
/** How text is set. */
export type Typography = schema.Infer<typeof typographyJson>;

/** How text is set, as the design token format's typography. */
export const Typography = Object.assign(typographyJson, {
    /** Set text as CSS's font properties. */
    css(typography: Typography): TypographyStyle {
        return {
            fontFamily: FontFamily.css(typography.fontFamily),
            fontSize: Dimension.css(typography.fontSize),
            fontWeight: String(typography.fontWeight),
            letterSpacing: Dimension.css(typography.letterSpacing),
            lineHeight: String(typography.lineHeight),
        };
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
