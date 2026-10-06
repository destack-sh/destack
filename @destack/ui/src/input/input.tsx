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
import { type JSX, omit } from "@destack/view";
import { useFieldControl } from "../field/control.ts";
import { useJoin } from "../join/index.ts";

/** The styles of an input and of every control that looks like one. */
const styles = style.create({
    box: {
        paddingInline: space[3],
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
        opacity: { default: 1, ":disabled": 0.5 },
        "::placeholder": { color: color.mutedForeground },
    },
    invalid: {
        borderColor: color.destructive,
        outlineColor: `color-mix(in oklab, ${color.destructive} 20%, transparent)`,
    },
    input: {
        width: "100%",
        minWidth: 0,
        height: size[3],
        paddingBlock: space[1],
        cursor: { default: "text", ":disabled": "not-allowed" },
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
});

/** The state of an input's styles. */
export interface InputStyleOptions {
    /** Whether the value is refused, which marks the box destructive. */
    readonly invalid?: boolean;
}

/** Return the StyleX styles of an input's box, for textareas, selects and other controls that look like inputs. */
export function inputStyle(options: InputStyleOptions = {}): style.Styles {
    return [text.callout, styles.box, options.invalid === true && styles.invalid];
}

/** The properties of an input, the native input's attributes included. */
export interface InputProperties extends Omit<JSX.InputHTMLAttributes<HTMLInputElement>, "class"> {
    /** The StyleX styles applied after the input's styles. */
    readonly xstyle?: style.Styles;
}

/** Render a native input, tied to the label, descriptions and state of its field. */
export function Input(properties: InputProperties): JSX.Element {
    // take the id, descriptions and state of the nearest field
    const field = useFieldControl();
    const join = useJoin();
    const rest = omit(properties, "xstyle", "style");
    const isInvalid = (): boolean =>
        field?.isInvalid() === true || properties["aria-invalid"] === "true";

    return (
        <input
            data-slot="input"
            {...field?.attributes()}
            {...rest}
            {...style.attributes(
                [inputStyle({ invalid: isInvalid() }), styles.input, join(), properties.xstyle],
                properties.style,
            )}
        />
    );
}
