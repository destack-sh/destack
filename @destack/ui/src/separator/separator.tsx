import * as style from "@destack/style";
import { color, stroke } from "@destack/theme/tokens.stylex";
import { type JSX, merge, omit } from "@destack/view";

/** The orientation and role of a separator that sets neither. */
const DEFAULTS: Required<Pick<SeparatorProperties, "orientation" | "decorative">> = {
    orientation: "horizontal",
    decorative: true,
};

/** The line of a separator in each orientation. */
const orientations = style.create({
    horizontal: {
        flexShrink: 0,
        width: "100%",
        height: 0,
        borderTopWidth: stroke.border,
        borderColor: color.border,
    },
    vertical: {
        flexShrink: 0,
        alignSelf: "stretch",
        width: 0,
        borderLeftWidth: stroke.border,
        borderColor: color.border,
    },
});

/** The direction a separator divides content in. */
export type SeparatorOrientation = "horizontal" | "vertical";

/** The properties of a separator, the native element's attributes included. */
export interface SeparatorProperties extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> {
    /** The direction, horizontal by default. */
    readonly orientation?: SeparatorOrientation;
    /** Whether the separator only decorates, hidden from assistive technology, true by default. */
    readonly decorative?: boolean;
    /** The StyleX styles applied after the separator's styles. */
    readonly xstyle?: style.Styles;
}

/** Render a line between content, exposed as a separator unless it only decorates. */
export function Separator(properties: SeparatorProperties): JSX.Element {
    const separator = merge(DEFAULTS, properties);
    const rest = omit(separator, "orientation", "decorative", "xstyle", "style");

    return (
        <div
            data-slot="separator"
            data-orientation={separator.orientation}
            role={separator.decorative ? "none" : "separator"}
            aria-orientation={
                !separator.decorative && separator.orientation === "vertical"
                    ? "vertical"
                    : undefined
            }
            {...rest}
            {...style.attributes(
                [orientations[separator.orientation], separator.xstyle],
                separator.style,
            )}
        />
    );
}
