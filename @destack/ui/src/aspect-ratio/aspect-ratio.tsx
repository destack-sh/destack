import * as style from "@destack/style";
import { type JSX, omit } from "@destack/view";

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
export interface AspectRatioProperties extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> {
    /** The width divided by the height, 1 by default. */
    readonly ratio?: number;
    /** The StyleX styles applied after the box's styles. */
    readonly xstyle?: style.Styles;
}

/** Render a box that keeps a width-to-height ratio as its width changes, such as for a picture or video. */
export function AspectRatio(properties: AspectRatioProperties): JSX.Element {
    const rest = omit(properties, "ratio", "xstyle", "style");

    return (
        <div
            data-slot="aspect-ratio"
            {...rest}
            {...style.attributes(
                [styles.box, styles.ratio(properties.ratio ?? 1), properties.xstyle],
                properties.style,
            )}
        />
    );
}
