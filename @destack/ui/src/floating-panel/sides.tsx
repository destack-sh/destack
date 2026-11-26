import { t } from "@destack/locale";
import { media } from "@destack/style/media.stylex";
import * as style from "@destack/style";
import { state } from "@destack/theme/state";
import { radius, space } from "@destack/theme/tokens.stylex";
import { For, type JSX, untrack, useLocale } from "@destack/view";
import { type Frame, FRAME_SIDES, type FrameSide } from "../interaction/index.ts";
import { mergeProperties } from "../part/index.ts";

/** How far past its side a side takes the pointer, more for a finger. */
const REACH = {
    default: `calc(-1 * ${space[1]})`,
    [media.pointerCoarse]: `calc(-1 * ${space[3]})`,
};

/** How far along its sides a corner takes the pointer. */
const CORNER = space[3];

/** The styles of a frame's sides. */
const styles = style.create({
    side: {
        position: "absolute",
        pointerEvents: "auto",
        touchAction: "none",
        borderRadius: radius[2],
    },
});

/** The place and cursor of each side and corner. */
const sides = style.create({
    start: { insetBlock: 0, insetInlineStart: REACH, width: space[2], cursor: "ew-resize" },
    end: { insetBlock: 0, insetInlineEnd: REACH, width: space[2], cursor: "ew-resize" },
    top: { insetInline: 0, top: REACH, height: space[2], cursor: "ns-resize" },
    bottom: { insetInline: 0, bottom: REACH, height: space[2], cursor: "ns-resize" },
    "top-start": {
        top: REACH,
        insetInlineStart: REACH,
        width: CORNER,
        height: CORNER,
        cursor: { default: "nwse-resize", ":dir(rtl)": "nesw-resize" },
    },
    "top-end": {
        top: REACH,
        insetInlineEnd: REACH,
        width: CORNER,
        height: CORNER,
        cursor: { default: "nesw-resize", ":dir(rtl)": "nwse-resize" },
    },
    "bottom-start": {
        bottom: REACH,
        insetInlineStart: REACH,
        width: CORNER,
        height: CORNER,
        cursor: { default: "nesw-resize", ":dir(rtl)": "nwse-resize" },
    },
    "bottom-end": {
        bottom: REACH,
        insetInlineEnd: REACH,
        width: CORNER,
        height: CORNER,
        cursor: { default: "nwse-resize", ":dir(rtl)": "nesw-resize" },
    },
});

/** Render the sides and corners that resize a frame, around what is positioned. */
export function FrameSides(properties: {
    /** The frame. */
    readonly frame: Frame;
}): JSX.Element {
    return <For each={FRAME_SIDES}>{(side) => <Side frame={properties.frame} side={side} />}</For>;
}

/** Render a side or a corner of a frame, the last corner in reach of the keys. */
function Side(properties: {
    /** The frame. */
    readonly frame: Frame;
    /** The side or the corner. */
    readonly side: FrameSide;
}): JSX.Element {
    // resize by the pointer, and by the arrow keys at the last corner
    const locale = useLocale();
    const side = untrack(() => properties.side);
    const move = untrack(() => properties.frame).resize(side);
    const isLast = side === "bottom-end";

    return (
        <div
            {...mergeProperties(move.attributes(), {
                "data-slot": "frame-side",
                "data-side": side,
                role: isLast ? ("button" as const) : ("presentation" as const),
                tabindex: isLast ? 0 : undefined,
                get "aria-label"() {
                    return isLast ? locale.render(t.context("action")`Resize`) : undefined;
                },
            })}
            {...style.attributes([state.focus, styles.side, sides[side]])}
        />
    );
}
