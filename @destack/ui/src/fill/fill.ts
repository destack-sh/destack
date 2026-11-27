import { swatches } from "@destack/theme/swatch";
import type { Hue } from "@destack/style";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { swatch } from "@destack/theme/tokens.stylex";

/** The fills of a surface, from most to least prominent, each in its hue. */
const FILLS = {
    solid: {
        title: t.context("option")`Solid`,
        description: t`Filled with its color, for what stands out.`,
    },
    soft: {
        title: t.context("option")`Soft`,
        description: t`Filled with its color's tint, for a status or a tag.`,
    },
    outline: {
        title: t.context("option")`Outline`,
        description: t`Drawn around in its color, for the least weight.`,
    },
};

/** The fills in the accent's swatch. */
const accents = style.create({
    solid: { backgroundColor: swatch.accentSolid, color: swatch.accentLabel },
    soft: { backgroundColor: swatch.accentTint, color: swatch.accentTintText },
    outline: {
        borderColor: `color-mix(in oklab, ${swatch.accentText} 40%, transparent)`,
        color: swatch.accentText,
    },
});

/** The fills in any hue's swatch. */
const hues = style.create({
    paint: (background: string, foreground: string, border: string) => ({
        backgroundColor: background,
        color: foreground,
        borderColor: border,
    }),
});

/** The fill of a surface in its hue: solid, soft or outline. */
export type Fill = keyof typeof FILLS;

/** The fills of a surface: their choices and their styles. */
export const Fill = {
    /** The fills as a person chooses one. */
    choices: FILLS,

    /** Return the StyleX styles of a fill, static in the accent and dynamic in another hue. */
    styles(fill: Fill, hue: Hue | undefined): style.Styles {
        // fill in the accent's swatch
        if (hue === undefined) {
            return accents[fill];
        }
        // fill with the hue
        else if (fill === "solid") {
            return hues.paint(swatches[hue].solid, swatches[hue].label, "transparent");
        }
        // fill with the hue's tint
        else if (fill === "soft") {
            return hues.paint(swatches[hue].tint, swatches[hue].tintText, "transparent");
        }
        // draw the edge in the hue
        else {
            const foreground = swatches[hue].text;

            return hues.paint(
                "transparent",
                foreground,
                `color-mix(in oklab, ${foreground} 40%, transparent)`,
            );
        }
    },
};
