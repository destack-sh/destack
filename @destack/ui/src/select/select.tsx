import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, radius, shadow, size, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { createEffect, type JSX, omit, Show } from "@destack/view";
import { Selection, type SelectionProperties, valuesOf } from "../selection/index.ts";
import { useFieldControl } from "../field/control.ts";
import { inputStyle } from "../input/index.ts";
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
        whiteSpace: "nowrap",
        cursor: { default: "pointer", ":disabled": "not-allowed" },
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
    multiple: {
        appearance: "auto",
        height: "auto",
        paddingBlock: space[1],
        whiteSpace: "normal",
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
        backgroundColor: {
            default: "transparent",
            ":hover": { default: null, [media.hover]: color.accent },
            ":focus": color.accent,
        },
        color: {
            default: "inherit",
            ":hover": { default: null, [media.hover]: color.accentForeground },
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
        borderTopWidth: stroke.border,
        borderColor: color.border,
    },
});

/** The properties of a select, the native select's attributes included. */
export type SelectProperties = SelectLook & SelectionProperties;

/** The look and prompt of a select, the native select's attributes included. */
export interface SelectLook extends Omit<
    JSX.SelectHTMLAttributes<HTMLSelectElement>,
    "class" | "value" | "multiple" | "onChange"
> {
    /** The prompt a single select shows until an option is chosen. */
    readonly placeholder?: string;
    /** The StyleX styles applied after the select's styles. */
    readonly xstyle?: style.Styles;
}

/** The properties of an element of a select, the native element's attributes included. */
export type SelectElementProperties<Attributes> = Omit<Attributes, "class"> & {
    /** The StyleX styles applied after the element's styles. */
    readonly xstyle?: style.Styles;
};

/** Render a native select whose button and picker take the theme where the browser allows, a list box when multiple. */
export function Select(properties: SelectProperties): JSX.Element {
    // take the id and state of the nearest field
    const field = useFieldControl();
    const join = useJoin();
    const rest = omit(
        properties,
        "multiple",
        "value",
        "defaultValue",
        "onValueChange",
        "placeholder",
        "children",
        "xstyle",
        "style",
    );
    const isInvalid = (): boolean =>
        field?.isInvalid() === true || properties["aria-invalid"] === "true";

    // follow the controlled values or the select's own
    const selection = new Selection(properties);

    // show the chosen options on the element once its options exist
    let element: HTMLSelectElement | undefined;
    createEffect(selection.values, (values) => {
        if (element !== undefined) {
            choose(element, values);
        }
    });

    return (
        <select
            data-slot="select"
            multiple={properties.multiple}
            {...field?.attributes()}
            {...rest}
            ref={(created) => {
                element = created;
            }}
            onChange={(event) => {
                // keep the person's choice and show the owner's until the owner takes it
                const select = event.currentTarget;
                selection.replace(
                    [...select.options]
                        .filter((option) => option.selected)
                        .map((option) => option.value),
                );
                if ("value" in properties) {
                    choose(select, valuesOf(properties.value));
                }
            }}
            {...style.attributes(
                [
                    inputStyle({ invalid: isInvalid() }),
                    styles.select,
                    properties.multiple === true && styles.multiple,
                    join(),
                    properties.xstyle,
                ],
                properties.style,
            )}
        >
            {properties.children}
            <Show when={properties.multiple === true ? undefined : properties.placeholder}>
                {(placeholder) => (
                    <option value="" disabled hidden data-slot="select-placeholder">
                        {placeholder()}
                    </option>
                )}
            </Show>
        </select>
    );
}

/** Choose exactly the options of a select whose values are listed, the placeholder when none is. */
function choose(select: HTMLSelectElement, values: readonly string[]): void {
    for (const option of select.options) {
        option.selected =
            values.includes(option.value) ||
            (values.length === 0 && option.value === "" && option.hasAttribute("hidden"));
    }
}

/** Render the button a select shows its chosen option in. */
export function SelectTrigger(
    properties: SelectElementProperties<JSX.ButtonHTMLAttributes<HTMLButtonElement>>,
): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <button
            data-slot="select-trigger"
            {...rest}
            {...style.attributes([styles.trigger, properties.xstyle], properties.style)}
        />
    );
}

/** Render a copy of the chosen option inside a select's button. */
export function SelectValue(
    properties: SelectElementProperties<JSX.HTMLAttributes<HTMLElement>>,
): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <selectedcontent
            data-slot="select-value"
            {...rest}
            {...style.attributes([properties.xstyle], properties.style)}
        />
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
    const rest = omit(properties, "xstyle", "style");

    return (
        <option
            data-slot="select-item"
            {...rest}
            {...style.attributes([styles.item, properties.xstyle], properties.style)}
        />
    );
}

/** Render a group of a select's options under a label. */
export function SelectGroup(
    properties: SelectElementProperties<JSX.OptgroupHTMLAttributes<HTMLOptGroupElement>>,
): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <optgroup
            data-slot="select-group"
            {...rest}
            {...style.attributes([properties.xstyle], properties.style)}
        />
    );
}

/** Render the label of a group of a select's options. */
export function SelectLabel(
    properties: SelectElementProperties<JSX.HTMLAttributes<HTMLLegendElement>>,
): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <legend
            data-slot="select-label"
            {...rest}
            {...style.attributes([text.caption, styles.label, properties.xstyle], properties.style)}
        />
    );
}

/** Render a line between a select's options. */
export function SelectSeparator(
    properties: SelectElementProperties<JSX.HTMLAttributes<HTMLHRElement>>,
): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <hr
            data-slot="select-separator"
            {...rest}
            {...style.attributes([styles.separator, properties.xstyle], properties.style)}
        />
    );
}
