import * as style from "@destack/style";
import { color, motion, radius, shadow, space, stroke } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import type { JSX } from "@solidjs/web";
import { omit } from "solid-js";
import { useField } from "../field/control.ts";

/** The styles of a textarea. */
const styles = style.create({
    textarea: {
        display: "flex",
        fieldSizing: "content",
        width: "100%",
        minHeight: space[9],
        paddingInline: space[3],
        paddingBlock: space[2],
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
    },
    invalid: {
        borderColor: color.destructive,
        outlineColor: `color-mix(in oklab, ${color.destructive} 20%, transparent)`,
    },
});

/** The properties of a textarea, the native textarea's attributes included. */
export interface TextareaProperties extends Omit<
    JSX.TextareaHTMLAttributes<HTMLTextAreaElement>,
    "class" | "style"
> {
    /** The StyleX styles applied after the textarea's styles. */
    readonly style?: style.Styles;
}

/** Render a native textarea that grows with its content, tied to its field. */
export function Textarea(properties: TextareaProperties): JSX.Element {
    // take the id, descriptions and state of the nearest field
    const field = useField();
    const rest = omit(properties, "style");
    const isInvalid = (): boolean =>
        field?.isInvalid() === true || properties["aria-invalid"] === "true";

    return (
        <textarea
            data-slot="textarea"
            {...field?.attributes()}
            {...rest}
            {...field?.valueAttributes<HTMLTextAreaElement>()}
            {...style.attrs(
                text.callout,
                styles.textarea,
                isInvalid() && styles.invalid,
                properties.style,
            )}
        />
    );
}
