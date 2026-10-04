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

/** The styles of a select, its picker and its options. */
const styles = style.create({
    select: {
        appearance: "base-select",
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "space-between",
        gap: space[2],
        width: "fit-content",
        height: size[3],
        paddingInline: space[3],
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderColor: { default: color.input, ":focus-visible": color.ring },
        borderRadius: radius[3],
        backgroundColor: "transparent",
        boxShadow: shadow.inset,
        color: color.foreground,
        whiteSpace: "nowrap",
        cursor: { default: "pointer", ":disabled": "not-allowed" },
        opacity: { default: 1, ":disabled": 0.5 },
        transitionProperty: "border-color, outline-color",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
        "::picker-icon": {
            color: color.mutedForeground,
        },
        "::picker(select)": {
            appearance: "base-select",
            marginBlock: space[1],
            padding: space[1],
            borderStyle: "solid",
            borderWidth: stroke.border,
            borderColor: color.border,
            borderRadius: radius[3],
            backgroundColor: color.popover,
            color: color.popoverForeground,
            boxShadow: shadow.overlay,
        },
    },
    invalid: {
        borderColor: color.destructive,
        outlineColor: `color-mix(in oklab, ${color.destructive} 20%, transparent)`,
    },
    trigger: {
        display: "contents",
    },
    item: {
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        gap: space[2],
        paddingBlock: space[2],
        paddingInline: space[2],
        borderRadius: radius[2],
        backgroundColor: { default: "transparent", ":hover": color.accent, ":focus": color.accent },
        color: {
            default: "inherit",
            ":hover": color.accentForeground,
            ":focus": color.accentForeground,
        },
        outlineStyle: "none",
        cursor: { default: "default", ":disabled": "not-allowed" },
        opacity: { default: 1, ":disabled": 0.5 },
        "::checkmark": {
            order: 1,
        },
    },
    label: {
        paddingBlock: space[1],
        paddingInline: space[2],
        color: color.mutedForeground,
        fontWeight: weight.medium,
    },
    separator: {
        marginBlock: space[1],
        marginInline: `calc(-1 * ${space[1]})`,
        height: 0,
        borderWidth: 0,
        borderTopWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
    },
});

/** The properties of a select, the native select's attributes included. */
export interface SelectProperties extends Omit<
    JSX.SelectHTMLAttributes<HTMLSelectElement>,
    "class" | "style"
> {
    /** The StyleX styles applied after the select's styles. */
    readonly style?: style.Styles;
}

/** The properties of an element of a select, the native element's attributes included. */
export type SelectElementProperties<Attributes> = Omit<Attributes, "class" | "style"> & {
    /** The StyleX styles applied after the element's styles. */
    readonly style?: style.Styles;
};

/** Render a native select whose button and picker take the theme where the browser allows. */
export function Select(properties: SelectProperties): JSX.Element {
    // take the id, state and bound value of the nearest field
    const field = useField();
    const join = useJoin();
    const rest = omit(properties, "style");
    const isInvalid = (): boolean =>
        field?.isInvalid() === true || properties["aria-invalid"] === "true";

    return (
        <select
            data-slot="select"
            {...field?.attributes()}
            {...rest}
            {...field?.valueAttributes<HTMLSelectElement>()}
            {...style.attrs(
                text.callout,
                styles.select,
                isInvalid() && styles.invalid,
                join(),
                properties.style,
            )}
        />
    );
}

/** Render the button a select shows its chosen option in. */
export function SelectTrigger(
    properties: SelectElementProperties<JSX.ButtonHTMLAttributes<HTMLButtonElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <button
            data-slot="select-trigger"
            {...rest}
            {...style.attrs(styles.trigger, properties.style)}
        />
    );
}

/** Render a copy of the chosen option inside a select's button. */
export function SelectValue(
    properties: SelectElementProperties<JSX.HTMLAttributes<HTMLElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <selectedcontent data-slot="select-value" {...rest} {...style.attrs(properties.style)} />
    );
}

/** Group the options a select's picker lists. */
export function SelectContent(properties: { readonly children?: JSX.Element }): JSX.Element {
    return <>{properties.children}</>;
}

/** Render an option of a select, checked when chosen. */
export function SelectItem(
    properties: SelectElementProperties<JSX.OptionHTMLAttributes<HTMLOptionElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <option data-slot="select-item" {...rest} {...style.attrs(styles.item, properties.style)} />
    );
}

/** Render a group of a select's options under a label. */
export function SelectGroup(
    properties: SelectElementProperties<JSX.OptgroupHTMLAttributes<HTMLOptGroupElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return <optgroup data-slot="select-group" {...rest} {...style.attrs(properties.style)} />;
}

/** Render the label of a group of a select's options. */
export function SelectLabel(
    properties: SelectElementProperties<JSX.HTMLAttributes<HTMLLegendElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <legend
            data-slot="select-label"
            {...rest}
            {...style.attrs(text.caption, styles.label, properties.style)}
        />
    );
}

/** Render a line between a select's options. */
export function SelectSeparator(
    properties: SelectElementProperties<JSX.HTMLAttributes<HTMLHRElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <hr
            data-slot="select-separator"
            {...rest}
            {...style.attrs(styles.separator, properties.style)}
        />
    );
}
