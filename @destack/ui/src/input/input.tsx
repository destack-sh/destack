import * as style from "@destack/style";
import {
    color,
    motion,
    radius,
    shadow,
    size,
    space,
    stroke,
    weight,
} from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import type { JSX } from "@solidjs/web";
import { omit } from "solid-js";
import { useField } from "../field/control.ts";
import { useJoin } from "../join/index.ts";

/** The styles of an input. */
const styles = style.create({
    input: {
        width: "100%",
        minWidth: 0,
        height: size[3],
        paddingInline: space[3],
        paddingBlock: space[1],
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderColor: { default: color.input, ":focus-visible": color.ring },
        borderRadius: radius[3],
        backgroundColor: "transparent",
        boxShadow: shadow.inset,
        color: color.foreground,
        transitionProperty: "border-color, outline-color",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
        cursor: { default: "text", ":disabled": "not-allowed" },
        opacity: { default: 1, ":disabled": 0.5 },
        "::placeholder": { color: color.mutedForeground },
        "::selection": { backgroundColor: color.primary, color: color.primaryForeground },
        "::file-selector-button": {
            height: "100%",
            marginInlineEnd: space[2],
            padding: 0,
            borderWidth: 0,
            backgroundColor: "transparent",
            color: color.foreground,
            fontFamily: "inherit",
            fontSize: "inherit",
            fontWeight: weight.medium,
        },
    },
    invalid: {
        borderColor: color.destructive,
        outlineColor: `color-mix(in oklab, ${color.destructive} 20%, transparent)`,
    },
});

/** The properties of an input, the native input's attributes included. */
export interface InputProperties extends Omit<
    JSX.InputHTMLAttributes<HTMLInputElement>,
    "class" | "style"
> {
    /** The StyleX styles applied after the input's styles. */
    readonly style?: style.Styles;
}

/** Render a native input, tied to the label, descriptions and state of its field. */
export function Input(properties: InputProperties): JSX.Element {
    // take the id, descriptions and state of the nearest field
    const field = useField();
    const join = useJoin();
    const rest = omit(properties, "style");
    const isInvalid = (): boolean =>
        field?.isInvalid() === true || properties["aria-invalid"] === "true";

    return (
        <input
            data-slot="input"
            {...field?.attributes()}
            {...rest}
            {...field?.valueAttributes<HTMLInputElement>()}
            {...style.attrs(
                text.callout,
                styles.input,
                isInvalid() && styles.invalid,
                join(),
                properties.style,
            )}
        />
    );
}
