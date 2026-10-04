import * as style from "@destack/style";
import { color, motion, radius, shadow, space, stroke } from "@destack/theme/tokens.stylex";
import type { JSX } from "@solidjs/web";
import { omit } from "solid-js";
import { useField } from "../field/control.ts";

/** The check a checked box shows, Phosphor's bold check as a mask. */
const CHECK_MASK = `url("data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 256 256'><path d='M232.49,80.49l-128,128a12,12,0,0,1-17,0l-56-56a12,12,0,1,1,17-17L96,183,215.51,63.51a12,12,0,0,1,17,17Z'/></svg>")`;

/** The dash an indeterminate box shows, Phosphor's bold minus as a mask. */
const MINUS_MASK = `url("data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 256 256'><path d='M228,128a12,12,0,0,1-12,12H40a12,12,0,0,1,0-24H216A12,12,0,0,1,228,128Z'/></svg>")`;

/** The styles of a checkbox. */
const styles = style.create({
    checkbox: {
        appearance: "none",
        position: "relative",
        flexShrink: 0,
        width: space[4],
        height: space[4],
        margin: 0,
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderColor: {
            default: color.input,
            ":checked": color.primary,
            ":indeterminate": color.primary,
        },
        borderRadius: radius[2],
        backgroundColor: {
            default: "transparent",
            ":checked": color.primary,
            ":indeterminate": color.primary,
        },
        color: color.primaryForeground,
        boxShadow: shadow.inset,
        cursor: { default: "pointer", ":disabled": "not-allowed" },
        opacity: { default: 1, ":disabled": 0.5 },
        transitionProperty: "background-color, border-color, outline-color",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
        "::after": {
            content: '""',
            position: "absolute",
            inset: "12%",
            backgroundColor: "currentColor",
            maskImage: { default: CHECK_MASK, ":indeterminate": MINUS_MASK },
            maskSize: "contain",
            maskRepeat: "no-repeat",
            opacity: { default: 0, ":checked": 1, ":indeterminate": 1 },
        },
    },
    invalid: {
        borderColor: color.destructive,
        outlineColor: `color-mix(in oklab, ${color.destructive} 20%, transparent)`,
    },
});

/** The properties of a checkbox, the native checkbox's attributes included. */
export interface CheckboxProperties extends Omit<
    JSX.InputHTMLAttributes<HTMLInputElement>,
    "class" | "style" | "type"
> {
    /** Whether the box shows neither checked nor unchecked, such as for a partly selected list. */
    readonly indeterminate?: boolean;
    /** The StyleX styles applied after the checkbox's styles. */
    readonly style?: style.Styles;
}

/** Render a native checkbox that Space toggles and its label names. */
export function Checkbox(properties: CheckboxProperties): JSX.Element {
    // take the id, state and bound value of the nearest field
    const field = useField();
    const rest = omit(properties, "indeterminate", "style");
    const isInvalid = (): boolean =>
        field?.isInvalid() === true || properties["aria-invalid"] === "true";

    return (
        <input
            type="checkbox"
            data-slot="checkbox"
            prop:indeterminate={properties.indeterminate === true}
            {...field?.attributes()}
            {...rest}
            {...field?.checkAttributes()}
            {...style.attrs(styles.checkbox, isInvalid() && styles.invalid, properties.style)}
        />
    );
}
