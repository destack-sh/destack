import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, stroke, swatch, text, weight } from "../token/tokens.stylex.ts";

/** The least a finger is sure to hit, as WCAG's target size asks for. */
const TOUCH = "1.5rem";

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
        touchAction: "manipulation",
        transform: { default: null, ":active": PRESS_SCALE },
    },
    /** A control smaller than a finger, which takes presses a little around it on touch. */
    touch: {
        position: "relative",
        "::after": {
            content: { default: null, [media.pointerCoarse]: "''" },
            position: "absolute",
            top: "50%",
            left: "50%",
            width: `max(100%, ${TOUCH})`,
            height: `max(100%, ${TOUCH})`,
            transform: "translate(-50%, -50%)",
        },
    },
    /** Text a person types, large enough that an iPhone does not zoom, filled in as the theme. */
    typed: {
        fontSize: {
            default: text.smFontSize,
            [media.pointerCoarse]: `max(16px, ${text.smFontSize})`,
        },
        caretColor: color.foreground,
        boxShadow: {
            default: null,
            ":autofill": `inset 0 0 0 100vmax color-mix(in oklab, ${swatch.accentMark} 12%, ${color.background})`,
        },
        WebkitTextFillColor: { default: null, ":autofill": color.foreground },
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
    /** An element clear at rest that takes the hover layer under a pointer or an open menu. */
    hover: {
        backgroundColor: {
            default: "transparent",
            ":is([data-menu-open])": color.hover,
            ":hover": { default: null, [media.hover]: color.hover },
        },
    },
    /** A control among many whose frame high contrast leaves out, such as a day or a row. */
    unframed: {
        borderColor: { default: "transparent", [media.forcedColors]: "Canvas" },
    },
    /** A surface told apart by its fill alone, which high contrast draws a frame around. */
    framed: {
        outlineStyle: { default: null, [media.forcedColors]: "solid" },
        outlineWidth: { default: null, [media.forcedColors]: stroke.border },
        outlineOffset: { default: null, [media.forcedColors]: `calc(-1 * ${stroke.border})` },
        outlineColor: { default: null, [media.forcedColors]: "CanvasText" },
    },
    /** An item highlighted, open or selected among others, or today beside a chosen date. */
    selected: {
        backgroundColor: { default: color.selected, [media.forcedColors]: "Highlight" },
        color: { default: color.foreground, [media.forcedColors]: "HighlightText" },
        forcedColorAdjust: { default: null, [media.forcedColors]: "none" },
        [color.foreground]: { default: null, [media.forcedColors]: "HighlightText" },
        [color.mutedForeground]: { default: null, [media.forcedColors]: "HighlightText" },
    },
    /** A toggle that is on, such as a chosen view or a bold button. */
    on: {
        backgroundColor: { default: color.selected, [media.forcedColors]: "Highlight" },
        color: { default: color.foreground, [media.forcedColors]: "HighlightText" },
        forcedColorAdjust: { default: null, [media.forcedColors]: "none" },
    },
    /** A place a dragged item would land on. */
    target: {
        backgroundColor: { default: swatch.accentTint, [media.forcedColors]: "Highlight" },
    },
    /** A control that is checked, a chosen date, or today where no date is chosen. */
    checked: {
        backgroundColor: { default: swatch.accentSolid, [media.forcedColors]: "Highlight" },
        color: { default: swatch.accentLabel, [media.forcedColors]: "HighlightText" },
        forcedColorAdjust: { default: null, [media.forcedColors]: "none" },
    },
    /** The page or item the person is on, among its siblings. */
    current: {
        backgroundColor: { default: color.selected, [media.forcedColors]: "Highlight" },
        color: { default: color.foreground, [media.forcedColors]: "HighlightText" },
        forcedColorAdjust: { default: null, [media.forcedColors]: "none" },
        [color.foreground]: { default: null, [media.forcedColors]: "HighlightText" },
        [color.mutedForeground]: { default: null, [media.forcedColors]: "HighlightText" },
        fontWeight: weight.medium,
    },
});
