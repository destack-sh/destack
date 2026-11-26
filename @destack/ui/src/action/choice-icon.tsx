import { swatches } from "@destack/theme/swatch";
import * as style from "@destack/style";
import { radius, space } from "@destack/theme/tokens.stylex";
import type { Choice } from "@destack/object";
import { createMemo, type JSX, Show } from "@destack/view";
import { Glyph } from "../glyph/index.ts";

/** The styles of a choice's icon and of its color. */
const styles = style.create({
    glyph: {
        width: space[4],
        height: space[4],
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
    // follow the choice, whose icon may be a file this draws nothing of
    const hue = () => properties.choice.color;
    const glyph = createMemo(() => {
        const { icon } = properties.choice;

        return icon === undefined || "file" in icon ? undefined : icon;
    });
    const tint = () => {
        const own = hue();

        return own === undefined ? undefined : swatches[own].mark;
    };

    return (
        <Show
            when={glyph()}
            fallback={
                <Show when={tint()}>
                    {(fill) => <span aria-hidden="true" {...style.attrs(styles.dot(fill()))} />}
                </Show>
            }
        >
            {(shown) => <Glyph glyph={shown()} xstyle={[styles.glyph, colorOf(tint())]} />}
        </Show>
    );
}

/** Color an icon, as the text around it without a color. */
function colorOf(fill: string | undefined): style.Styles {
    return fill !== undefined && styles.hue(fill);
}
