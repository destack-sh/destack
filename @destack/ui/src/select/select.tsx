import { media } from "@destack/style/media.stylex";
import { state } from "@destack/theme/state";
import * as style from "@destack/style";
import { color, radius, size, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { createEffect, type JSX, omit, Show } from "@destack/view";
import {
    SELECTION_OPTIONS,
    Selection,
    type SelectionProperties,
    valuesOf,
} from "../selection/index.ts";
import { readChoices, useFieldControl, useFieldValue } from "../field/control.ts";
import { inputStyle } from "../input/index.ts";
import { useJoin } from "../join/index.ts";
import { type ElementPartProperties, mergeProperties, renderPart } from "../part/index.ts";

/** The styles of a select, its picker and its options. */
const styles = style.create({
    select: {
        width: "fit-content",
        height: size[2],
        whiteSpace: "nowrap",
        cursor: { default: "pointer", ":disabled": "not-allowed" },
    },
    sm: {
        height: size[1],
        paddingBlock: 0,
        paddingInline: space[2],
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
        minHeight: size[2],
        paddingBlock: space[1],
        paddingInline: space[2],
        borderRadius: radius[2],
        backgroundColor: {
            default: "transparent",
            ":hover": { default: null, [media.hover]: color.hover },
            ":focus": color.selected,
        },
        color: {
            default: "inherit",
            ":focus": color.foreground,
        },
        outlineStyle: "none",
        cursor: { default: "default", ":disabled": "not-allowed" },
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

/** The height of a select. */
export type SelectSize = "default" | "sm";

/** The properties of a select, the native select's attributes included. */
export type SelectProperties = Omit<
    JSX.SelectHTMLAttributes<HTMLSelectElement>,
    "class" | "value" | "multiple" | "onChange" | "size"
> & {
    /** The prompt a single select shows until an option is chosen. */
    readonly placeholder?: string;
    /** The height, default by default. */
    readonly size?: SelectSize;
    /** The StyleX styles applied after the select's styles. */
    readonly xstyle?: style.Styles;
} & SelectionProperties;

/** The properties of an element of a select, the native element's attributes included. */
export type SelectElementProperties<Attributes> = Omit<Attributes, "class"> & ElementPartProperties;

/** Render a native select whose button and picker take the theme where the browser allows. */
export function Select(given: SelectProperties): JSX.Element {
    // edit the value of the field's form
    const properties = useFieldValue(given, {
        value: "value",
        change: "onValueChange",
        read: readChoices,
        settles: true,
    });

    // take the id and state of the nearest field
    const field = useFieldControl();
    const join = useJoin();
    const rest = omit(
        properties,
        ...SELECTION_OPTIONS,
        "placeholder",
        "size",
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
            {...mergeProperties(
                {
                    ref: (created: HTMLSelectElement) => {
                        element = created;
                    },
                },
                rest,
            )}
            onChange={(event) => {
                // record the person's choice and show the owner's until the owner takes it
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
                    properties.size === "sm" && styles.sm,
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

/** The properties of a select's button, whose label the select itself takes. */
export type SelectTriggerProperties = Omit<
    SelectElementProperties<JSX.ButtonHTMLAttributes<HTMLButtonElement>>,
    "aria-label" | "aria-labelledby"
>;

/** Render the button a select shows its chosen option in. */
export function SelectTrigger(properties: SelectTriggerProperties): JSX.Element {
    return renderPart("button", "select-trigger", properties, styles.trigger);
}

/** Render a copy of the chosen option inside a select's button. */
export function SelectValue(
    properties: SelectElementProperties<JSX.HTMLAttributes<HTMLElement>>,
): JSX.Element {
    return renderPart("selectedcontent", "select-value", properties, null);
}

/** Group the options a select's picker lists. */
export function SelectContent(properties: { readonly children?: JSX.Element }): JSX.Element {
    return <>{properties.children}</>;
}

/** Render an option of a select, checked when chosen. */
export function SelectItem(
    properties: SelectElementProperties<JSX.OptionHTMLAttributes<HTMLOptionElement>>,
): JSX.Element {
    return renderPart("option", "select-item", properties, [state.disabled, styles.item]);
}

/** Render a group of a select's options under a label. */
export function SelectGroup(
    properties: SelectElementProperties<JSX.OptgroupHTMLAttributes<HTMLOptGroupElement>>,
): JSX.Element {
    return renderPart("optgroup", "select-group", properties, null);
}

/** Render the label of a group of a select's options. */
export function SelectLabel(
    properties: SelectElementProperties<JSX.HTMLAttributes<HTMLLegendElement>>,
): JSX.Element {
    return renderPart("legend", "select-label", properties, [text.xs, styles.label]);
}

/** Render a line between a select's options. */
export function SelectSeparator(
    properties: SelectElementProperties<JSX.HTMLAttributes<HTMLHRElement>>,
): JSX.Element {
    return renderPart("hr", "select-separator", properties, styles.separator);
}
