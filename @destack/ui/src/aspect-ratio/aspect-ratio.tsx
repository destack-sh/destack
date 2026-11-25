import { renderPart, type Render } from "../part/index.ts";
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
    /** Render another element with its attributes, its own by default. */
    readonly render?: Render;
}

/** Render a box at a width-to-height ratio as its width changes. */
export function AspectRatio(properties: AspectRatioProperties): JSX.Element {
    const rest = omit(properties, "ratio");

    return renderPart("div", "aspect-ratio", rest, () => [
        styles.box,
        styles.ratio(properties.ratio ?? 1),
    ]);
}
