import * as style from "@destack/style";
import { color, stroke } from "@destack/theme/tokens.stylex";
import { type JSX, merge, omit } from "@destack/view";

/** The orientation of a scroll area that sets none. */
const DEFAULTS: Required<Pick<ScrollAreaProperties, "orientation">> = { orientation: "vertical" };

/** The styles of a scroll area. */
const styles = style.create({
    area: {
        position: "relative",
        scrollbarWidth: "thin",
        scrollbarColor: `${color.border} transparent`,
        overscrollBehavior: "contain",
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
    },
});

/** The axes a scroll area scrolls along in each orientation. */
const orientations = style.create({
    vertical: { overflowX: "hidden", overflowY: "auto" },
    horizontal: { overflowX: "auto", overflowY: "hidden" },
    both: { overflow: "auto" },
});

/** The axes a scroll area scrolls along. */
export type ScrollAreaOrientation = "vertical" | "horizontal" | "both";

/** The properties of a scroll area, the native element's attributes included. */
export interface ScrollAreaProperties extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> {
    /** The axes it scrolls along, vertical by default. */
    readonly orientation?: ScrollAreaOrientation;
    /** The StyleX styles applied after the scroll area's styles, usually its size. */
    readonly xstyle?: style.Styles;
}

/** Render a region that scrolls with thin themed scrollbars and takes the focus, so arrow keys scroll it. */
export function ScrollArea(properties: ScrollAreaProperties): JSX.Element {
    const area = merge(DEFAULTS, properties);
    const rest = omit(area, "orientation", "xstyle", "style");

    return (
        <div
            tabindex={0}
            data-slot="scroll-area"
            data-orientation={area.orientation}
            {...rest}
            {...style.attributes(
                [styles.area, orientations[area.orientation], area.xstyle],
                area.style,
            )}
        />
    );
}
