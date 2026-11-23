import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, stroke, swatch, weight } from "../token/tokens.stylex.ts";

/** The scale of a pressed control: subtle, between 0.95 and 0.98. */
const PRESS_SCALE = "scale(0.97)";

/** The states that disable a control: native, or by ARIA on links and other elements. */
const DISABLED = ":is(:disabled, [aria-disabled=true])";

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
    /** A control while it is disabled, natively or by ARIA as a link is. */
    disabled: {
        opacity: { default: null, [DISABLED]: 0.5 },
        cursor: { default: null, [DISABLED]: "not-allowed" },
        pointerEvents: { default: null, [DISABLED]: "none" },
    },
    /** An element that takes no input, where no native state says so. */
    inert: {
        opacity: 0.5,
        pointerEvents: "none",
    },
    /** A field whose value is refused. */
    invalid: {
        borderColor: swatch.destructiveMark,
        outlineColor: `color-mix(in oklab, ${swatch.destructiveMark} 20%, transparent)`,
    },
    /** An element that is clear at rest and takes the hover layer under a mouse or pen. */
    hover: {
        backgroundColor: {
            default: "transparent",
            ":hover": { default: null, [media.hover]: color.hover },
        },
    },
    /** An item highlighted, open or selected among others, or today beside a chosen date. */
    selected: {
        backgroundColor: color.selected,
    },
    /** A toggle that is on, such as a chosen view or a bold button. */
    on: {
        backgroundColor: swatch.accentTint,
        color: swatch.accentTintText,
    },
    /** A place a dragged item would land on. */
    target: {
        backgroundColor: swatch.accentTint,
    },
    /** A control that is checked, a chosen date, or today where no date is chosen. */
    checked: {
        backgroundColor: swatch.accentSolid,
        color: swatch.accentLabel,
    },
    /** The page or item the person is on, among its siblings. */
    current: {
        backgroundColor: color.selected,
        fontWeight: weight.medium,
    },
});
