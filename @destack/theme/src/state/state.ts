import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, stroke, weight } from "../token/tokens.stylex.ts";

/** The scale of a pressed control: subtle, between 0.95 and 0.98. */
const PRESS_SCALE = "scale(0.97)";

/** The states every component shows the same way, after shadcn/ui. */
export const state = style.create({
    /** The ring around a control the keyboard focuses. */
    focus: {
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineOffset: stroke.border,
        outlineColor: color.ring,
    },
    /** The ring inside an element whose edges are clipped, such as a row or a cell. */
    focusInset: {
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineOffset: `calc(-1 * ${stroke.ring})`,
        outlineColor: color.ring,
    },
    /** A control while it is pressed, which shrinks a little to answer the press. */
    press: {
        transform: { default: null, ":active": PRESS_SCALE },
    },
    /** A native control while it is disabled. */
    disabled: {
        opacity: { default: null, ":disabled": 0.5 },
        cursor: { default: null, ":disabled": "not-allowed" },
        pointerEvents: { default: null, ":disabled": "none" },
    },
    /** An element that takes no input, where no native state says so. */
    inert: {
        opacity: 0.5,
        pointerEvents: "none",
    },
    /** A field whose value is refused. */
    invalid: {
        borderColor: color.destructive,
        outlineColor: `color-mix(in oklab, ${color.destructive} 20%, transparent)`,
    },
    /** An element that is clear at rest and fills under a mouse or pen. */
    hover: {
        backgroundColor: {
            default: "transparent",
            ":hover": { default: null, [media.hover]: color.muted },
        },
    },
    /** An item that is highlighted, open or selected among others. */
    selected: {
        backgroundColor: color.accent,
        color: color.accentForeground,
    },
    /** A place a dragged item would land on. */
    target: {
        backgroundColor: `color-mix(in oklab, ${color.primary} 10%, transparent)`,
    },
    /** A control that is checked, a chosen date, or today. */
    checked: {
        backgroundColor: color.primary,
        color: color.primaryForeground,
    },
    /** The page or item the person is on, among its siblings. */
    current: {
        backgroundColor: `color-mix(in oklab, ${color.foreground} 10%, transparent)`,
        fontWeight: weight.medium,
    },
});
