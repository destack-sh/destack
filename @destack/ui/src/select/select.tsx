import { media } from "@destack/style/media.stylex";
import { state } from "@destack/theme/state";
import * as style from "@destack/style";
import { color, radius, size, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { createContext, createEffect, type JSX, omit, Show, useContext } from "@destack/view";
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
import {
    type ComponentProperties,
    type RenderProperties,
    defineComponent,
} from "@destack/view/declare";
import { t } from "@destack/locale";
import { field } from "@destack/object";
import { schema } from "@destack/schema";

/** The heights of a select. */
const SELECT_SIZES = {
    sm: { title: t.context("option")`Small` },
    default: { title: t.context("option")`Default` },
};

/** The looks of a select: framed as an input, or quiet as a toolbar's button. */
const SELECT_VARIANTS = {
    outline: {
        title: t.context("option")`Outline`,
        description: t`A framed select, as in a form.`,
    },
    ghost: {
        title: t.context("option")`Ghost`,
        description: t`A select without a frame until pointed at, as in a toolbar.`,
    },
};

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
    // drop the frame until a pointer rests on it, as a toolbar's buttons do
    ghost: {
        borderColor: "transparent",
        backgroundColor: {
            default: "transparent",
            ":hover": { default: null, [media.hover]: color.hover },
        },
        boxShadow: "none",
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
export type SelectSize = keyof typeof SELECT_SIZES;

/** The look of a select. */
export type SelectVariant = keyof typeof SELECT_VARIANTS;

/** The properties of a select, the native select's attributes included. */
export type SelectAttributes = Omit<
    JSX.SelectHTMLAttributes<HTMLSelectElement>,
    "class" | "value" | "multiple" | "onChange" | "size" | "disabled" | "placeholder"
> & {
    /** The StyleX styles applied after the select's styles. */
    readonly xstyle?: style.Styles;
} & SelectionProperties;

/** The properties of an element of a select, the native element's attributes included. */
export type SelectElementProperties<Attributes> = Omit<Attributes, "class"> & ElementPartProperties;

/** The selection of the nearest select, which its options follow, null outside one. */
const SelectContext = createContext<Selection | null>(null);

/** The properties people change in a select's controls and inspector. */
const SELECT_PROPERTIES = {
    size: field
        .enum(SELECT_SIZES)
        .labeled(t.context("label")`Size`)
        .described(t`How tall the select is.`)
        .default("default"),
    variant: field
        .enum(SELECT_VARIANTS)
        .labeled(t.context("label")`Variant`)
        .described(t`Whether the select shows its frame at rest.`)
        .default("outline"),
    placeholder: field
        .string(schema.string())
        .labeled(t.context("label")`Placeholder`)
        .described(t`The hint the select shows until a choice is made.`)
        .optional(),
    disabled: field
        .boolean()
        .labeled(t.context("label")`Disabled`)
        .described(
            t`Refuses choices where the value cannot change now, as its field does when absent.`,
        )
        .optional(),
};

/** Render a native select whose button and picker take the theme where the browser allows. */
export const Select = defineComponent({
    name: "select",
    properties: SELECT_PROPERTIES,
    render(given: RenderProperties<SelectAttributes, typeof SELECT_PROPERTIES>): JSX.Element {
        // edit the value of the field's form
        const properties = useFieldValue(given, {
            value: "value",
            change: "onValueChange",
            read: readChoices,
            settles: true,
        });

        // take the id and state of the nearest field
        const fieldControl = useFieldControl();
        const join = useJoin();
        const rest = omit(
            properties,
            ...SELECTION_OPTIONS,
            "placeholder",
            "size",
            "variant",
            "children",
            "xstyle",
            "style",
        );
        const isInvalid = (): boolean =>
            fieldControl?.isInvalid() === true || properties["aria-invalid"] === "true";

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
            <SelectContext value={selection}>
                <select
                    data-slot="select"
                    multiple={properties.multiple}
                    {...fieldControl?.attributes()}
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
                            properties.variant === "ghost" && styles.ghost,
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
            </SelectContext>
        );
    },
});

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
    // follow the select's selection from the moment the option exists, however late it renders
    const selection = useContext(SelectContext);
    let option: HTMLOptionElement | undefined;
    createEffect(
        () => selection?.isSelected(String(properties.value ?? "")),
        (isSelected) => {
            if (option !== undefined && isSelected !== undefined) {
                option.selected = isSelected;
            }
        },
    );

    return renderPart("option", "select-item", properties, [state.disabled, styles.item], {
        ref: (created: HTMLOptionElement) => {
            option = created;
        },
    });
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

/** The properties of a select, its native attributes included. */
export type SelectProperties = ComponentProperties<typeof Select>;
