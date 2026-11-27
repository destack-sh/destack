import { schema } from "@destack/schema";
import { Dimension } from "../dimension/index.ts";

/** How text is set: its family, size, weight, line height and letter spacing, as CSS sets fonts. */
const typographyJson = schema
    .object({
        /** The font family, as CSS lists one with its fallbacks. */
        family: schema.string().min(1),
        /** The size. */
        size: Dimension,
        /** The weight, from 1 to 1000, 400 for regular and 700 for bold. */
        weight: schema.number().int().min(1).max(1000),
        /** The height of a line, as a multiple of the size. */
        lineHeight: schema.number().positive(),
        /** The room between letters. */
        letterSpacing: Dimension,
    })
    .meta({ id: "typography", title: "Typography" });
/** How text is set. */
export type Typography = schema.Infer<typeof typographyJson>;

/** How text is set: its family, size, weight, line height and letter spacing, as CSS sets fonts. */
export const Typography = Object.assign(typographyJson, {
    /** Set text as CSS's font properties. */
    css(typography: Typography): TypographyStyle {
        return {
            fontFamily: typography.family,
            fontSize: typography.size,
            fontWeight: String(typography.weight),
            lineHeight: String(typography.lineHeight),
            letterSpacing: typography.letterSpacing,
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
    /** The line height. */
    readonly lineHeight: string;
    /** The letter spacing. */
    readonly letterSpacing: string;
}
