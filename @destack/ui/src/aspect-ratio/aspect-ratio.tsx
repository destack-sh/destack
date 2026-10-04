import * as style from "@destack/style";
import type { JSX } from "@solidjs/web";
import { omit } from "solid-js";

/** The styles of an aspect ratio box. */
const styles = style.create({
    box: {
        position: "relative",
        width: "100%",
    },
    ratio: (ratio: number) => ({
        aspectRatio: ratio,
    }),
});

/** The properties of an aspect ratio box, the native element's attributes included. */
export interface AspectRatioProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "style"
> {
    /** The width divided by the height, 1 by default. */
    readonly ratio?: number;
    /** The StyleX styles applied after the box's styles. */
    readonly style?: style.Styles;
}

/** Render a box that keeps a width-to-height ratio as its width changes, such as for a picture or video. */
export function AspectRatio(properties: AspectRatioProperties): JSX.Element {
    const rest = omit(properties, "ratio", "style");

    return (
        <div
            data-slot="aspect-ratio"
            {...rest}
            {...style.attrs(styles.box, styles.ratio(properties.ratio ?? 1), properties.style)}
        />
    );
}
