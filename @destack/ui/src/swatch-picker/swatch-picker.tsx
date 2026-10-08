import * as style from "@destack/style";
import { ACCENT_PRESETS, type AccentPreset } from "@destack/theme";
import { color, radius, size, space, stroke, swatch } from "@destack/theme/tokens.stylex";
import { For, type JSX, omit, Show } from "@destack/view";
import {
    ListBox,
    type ListBoxElementProperties,
    ListBoxItem,
    type ListBoxItemProperties,
    useListBox,
} from "../list-box/index.ts";

/** The swatches in each row of a swatch picker by default. */
const COLUMNS = 9;

/** The properties of a swatch picker, the native element's attributes included. */
export interface SwatchPickerProperties extends ListBoxElementProperties<
    Omit<JSX.HTMLAttributes<HTMLDivElement>, "onKeyDown" | "onFocusOut">
> {
    /** The chosen swatch, which makes the choice controlled, undefined for none. */
    readonly value?: AccentPreset | undefined;
    /** The swatch chosen at first while uncontrolled. */
    readonly defaultValue?: AccentPreset;
    /** Handle the person choosing a swatch. */
    readonly onValueChange?: (value: AccentPreset) => void;
    /** The swatches offered when the picker has no items of its own, every colorful preset by default. */
    readonly presets?: readonly AccentPreset[];
    /** The swatches in each row, which the up and down arrow keys move across, nine by default. */
    readonly columns?: number;
    /** The form field name the chosen swatch submits under, which renders a hidden input. */
    readonly name?: string;
    /** The id of the form the chosen swatch belongs to, its ancestor form by default. */
    readonly form?: string;
}

/** The properties of a swatch, the option's included. */
export interface SwatchPickerItemProperties extends Omit<ListBoxItemProperties, "value"> {
    /** The preset the swatch shows and stands for. */
    readonly value: AccentPreset;
}

/** The styles of a swatch picker and its swatches. */
const styles = style.create({
    picker: {
        gap: space[1],
    },
    swatch: {
        boxSizing: "border-box",
        width: `calc(${size[1]} * 0.75)`,
        height: `calc(${size[1]} * 0.75)`,
        padding: 0,
        borderRadius: radius.full,
        cursor: "pointer",
        boxShadow: {
            default: null,
            ":is([aria-selected=true])": `0 0 0 ${stroke.ring} ${color.background}, 0 0 0 calc(${stroke.ring} * 2) ${color.ring}`,
        },
    },
});

/** The solid fill of a swatch. */
const fills = style.create({
    fill: (fill: string) => ({ backgroundColor: fill }),
});

/** Render the theme's swatches as a list box in a grid layout. */
export function SwatchPicker(properties: SwatchPickerProperties): JSX.Element {
    // offer the given presets in the theme's order unless the caller lays out its own swatches
    const rest = omit(
        properties,
        "value",
        "defaultValue",
        "onValueChange",
        "presets",
        "columns",
        "name",
        "form",
        "children",
        "xstyle",
    );
    const presets = () => properties.presets ?? ACCENT_PRESETS;
    const onValueChange = (value: string | undefined): void => {
        // report a chosen preset
        const chosen = ACCENT_PRESETS.find((preset) => preset === value);
        if (chosen !== undefined) {
            properties.onValueChange?.(chosen);
        }
    };

    // choose one swatch at a time, the focus moving selecting it
    return (
        <ListBox
            data-slot="swatch-picker"
            layout="grid"
            columns={properties.columns ?? COLUMNS}
            selectionBehavior="replace"
            selection={
                "value" in properties
                    ? {
                          get value() {
                              return properties.value;
                          },
                          onValueChange,
                      }
                    : {
                          ...(properties.defaultValue === undefined
                              ? {}
                              : { defaultValue: properties.defaultValue }),
                          onValueChange,
                      }
            }
            {...rest}
            xstyle={[styles.picker, properties.xstyle]}
        >
            {properties.children ?? (
                <For each={presets()}>{(preset) => <SwatchPickerItem value={preset} />}</For>
            )}
            <Show when={properties.name !== undefined || properties.form !== undefined}>
                <SwatchPickerInput name={properties.name} form={properties.form} />
            </Show>
        </ListBox>
    );
}

/** Render one swatch of the nearest swatch picker, an option filled with its preset's color and ringed while chosen. */
export function SwatchPickerItem(properties: SwatchPickerItemProperties): JSX.Element {
    const rest = omit(properties, "xstyle", "children");

    return (
        <ListBoxItem
            aria-label={properties.value}
            data-slot="swatch-picker-item"
            data-swatch={properties.value}
            {...rest}
            xstyle={[
                styles.swatch,
                fills.fill(swatch[`${properties.value}Solid`]),
                properties.xstyle,
            ]}
        >
            {properties.children ?? false}
        </ListBoxItem>
    );
}

/** Render the hidden input that carries the nearest swatch picker's chosen swatch into its form, none while none is chosen. */
function SwatchPickerInput(properties: {
    /** The form field name. */
    readonly name: string | undefined;
    /** The id of the form. */
    readonly form: string | undefined;
}): JSX.Element {
    const list = useListBox();
    const chosen = (): string | undefined => list.selection?.values()[0];

    return (
        <Show when={chosen()}>
            {(value) => (
                <input
                    type="hidden"
                    name={properties.name}
                    form={properties.form}
                    value={value()}
                />
            )}
        </Show>
    );
}
