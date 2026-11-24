import * as style from "@destack/style";
import { radius, space, swatch } from "@destack/theme/tokens.stylex";
import type { Choice } from "@destack/object";
import type { JSX } from "@destack/view";
import { Glyph } from "../glyph/index.ts";

/** The styles of a choice's icon and of its color. */
const styles = style.create({
    glyph: {
        width: "1rem",
        height: "1rem",
        fontSize: "1em",
    },
    hue: (fill: string) => ({
        color: fill,
    }),
    dot: (fill: string) => ({
        width: space[2],
        height: space[2],
        borderRadius: radius.full,
        backgroundColor: fill,
        forcedColorAdjust: "none",
    }),
});

/** Render a choice's icon, else its color as a dot, nothing without either. */
export function ChoiceIcon(properties: {
    /** The choice. */
    readonly choice: Choice;
}): JSX.Element {
    const { icon, color: hue } = properties.choice;
    if (icon !== undefined && !("file" in icon)) {
        return (
            <Glyph
                glyph={icon}
                xstyle={[styles.glyph, hue !== undefined && styles.hue(swatch[`${hue}Mark`])]}
            />
        );
    }

    return hue === undefined ? undefined : (
        <span aria-hidden="true" {...style.attrs(styles.dot(swatch[`${hue}Mark`]))} />
    );
}
