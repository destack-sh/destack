import * as style from "@destack/style";
import { color, motion, radius, shadow, space, stroke } from "@destack/theme/tokens.stylex";
import type { JSX } from "@solidjs/web";
import { createContext, createUniqueId, omit, useContext } from "solid-js";

/** The name the radios of the nearest group share, null outside a group. */
const RadioGroupContext = createContext<string | null>(null);

/** The styles of a radio group and its radios. */
const styles = style.create({
    group: {
        display: "grid",
        gap: space[3],
    },
    item: {
        appearance: "none",
        position: "relative",
        flexShrink: 0,
        width: space[4],
        height: space[4],
        margin: 0,
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderColor: { default: color.input, ":checked": color.primary },
        borderRadius: radius.full,
        backgroundColor: "transparent",
        boxShadow: shadow.inset,
        cursor: { default: "pointer", ":disabled": "not-allowed" },
        opacity: { default: 1, ":disabled": 0.5 },
        transitionProperty: "border-color, outline-color",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
        "::after": {
            content: '""',
            position: "absolute",
            inset: "25%",
            borderRadius: radius.full,
            backgroundColor: color.primary,
            opacity: { default: 0, ":checked": 1 },
        },
    },
});

/** The properties of a radio group, the native element's attributes included. */
export interface RadioGroupProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "style"
> {
    /** The form field name the radios share, a generated one by default. */
    readonly name?: string;
    /** The StyleX styles applied after the group's styles. */
    readonly style?: style.Styles;
}

/** The properties of a radio, the native radio's attributes included. */
export interface RadioGroupItemProperties extends Omit<
    JSX.InputHTMLAttributes<HTMLInputElement>,
    "class" | "style" | "type" | "name"
> {
    /** The StyleX styles applied after the radio's styles. */
    readonly style?: style.Styles;
}

/** Render a group of native radios that share a name, which arrow keys move between. */
export function RadioGroup(properties: RadioGroupProperties): JSX.Element {
    const rest = omit(properties, "name", "style");
    const name = properties.name ?? createUniqueId();

    return (
        <RadioGroupContext value={name}>
            <div
                role="radiogroup"
                data-slot="radio-group"
                {...rest}
                {...style.attrs(styles.group, properties.style)}
            />
        </RadioGroupContext>
    );
}

/** Render a native radio of the nearest radio group. */
export function RadioGroupItem(properties: RadioGroupItemProperties): JSX.Element {
    // read the name of the group, refusing a radio outside one
    const name = useContext(RadioGroupContext);
    if (name === null) {
        throw new TypeError("a radio group item needs a radio group around it");
    }
    const rest = omit(properties, "style");

    return (
        <input
            type="radio"
            name={name}
            data-slot="radio-group-item"
            {...rest}
            {...style.attrs(styles.item, properties.style)}
        />
    );
}
