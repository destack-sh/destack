import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { HUES, type Hue } from "@destack/color";
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
const COLUMNS = 5;

/** The properties of a swatch picker, the native element's attributes included. */
export interface ColorSwatchPickerProperties extends ListBoxElementProperties<
    Omit<JSX.HTMLAttributes<HTMLDivElement>, "onKeyDown" | "onFocusOut">
> {
    /** The chosen swatch, which makes the choice controlled, undefined for none. */
    readonly value?: Hue | undefined;
    /** The swatch chosen at first while uncontrolled. */
    readonly defaultValue?: Hue;
    /** Handle the person choosing a swatch. */
    readonly onValueChange?: (value: Hue) => void;
    /** The swatches offered when the picker has no items of its own. */
    readonly hues?: readonly Hue[];
    /** The swatches in each row, which the up and down arrow keys move across, five by default. */
    readonly columns?: number;
    /** The form field name the chosen swatch submits under, which renders a hidden input. */
    readonly name?: string;
    /** The id of the form the chosen swatch belongs to, its ancestor form by default. */
    readonly form?: string;
}

/** The properties of a swatch, the option's included. */
export interface ColorSwatchPickerItemProperties extends Omit<ListBoxItemProperties, "value"> {
    /** The hue the swatch shows and stands for. */
    readonly value: Hue;
}

/** The styles of a swatch picker and its swatches. */
const styles = style.create({
    picker: {
        gap: space[1],
    },
    swatch: {
        forcedColorAdjust: { default: null, [media.forcedColors]: "none" },
        borderWidth: { default: null, [media.forcedColors]: "1px" },
        borderStyle: { default: null, [media.forcedColors]: "solid" },
        borderColor: { default: null, [media.forcedColors]: "CanvasText" },
        width: `calc(${size[1]} * 0.75)`,
        height: `calc(${size[1]} * 0.75)`,
        minHeight: 0,
        paddingBlock: 0,
        paddingInline: 0,
        borderRadius: radius.full,
        cursor: "pointer",
        boxShadow: {
            default: null,
            ":is([aria-selected=true])": {
                default: `0 0 0 ${stroke.ring} ${color.background}, 0 0 0 calc(${stroke.ring} * 2) ${color.ring}`,
                [media.forcedColors]: `0 0 0 ${stroke.ring} Canvas, 0 0 0 calc(${stroke.ring} * 2) Highlight`,
            },
        },
    },
});

/** The solid fill of a swatch. */
const fills = style.create({
    fill: (fill: string) => ({ backgroundColor: fill }),
});

/** Render the theme's swatches as a list box in a grid layout. */
export function ColorSwatchPicker(properties: ColorSwatchPickerProperties): JSX.Element {
    // offer the given hues in their order unless the caller lays out its own swatches
    const rest = omit(
        properties,
        "value",
        "defaultValue",
        "onValueChange",
        "hues",
        "columns",
        "name",
        "form",
        "children",
        "xstyle",
    );
    const hues = () => properties.hues ?? HUES;
    const onValueChange = (value: string | undefined): void => {
        // report a chosen hue
        const chosen = HUES.find((hue) => hue === value);
        if (chosen !== undefined) {
            properties.onValueChange?.(chosen);
        }
    };

    // choose one swatch at a time, the focus moving selecting it
    return (
        <ListBox
            data-slot="color-swatch-picker"
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
                <For each={hues()}>{(hue) => <ColorSwatchPickerItem value={hue} />}</For>
            )}
            <Show when={properties.name !== undefined || properties.form !== undefined}>
                <ColorSwatchPickerInput name={properties.name} form={properties.form} />
            </Show>
        </ListBox>
    );
}

/** Render one swatch of the nearest swatch picker. */
export function ColorSwatchPickerItem(properties: ColorSwatchPickerItemProperties): JSX.Element {
    const rest = omit(properties, "xstyle", "children");

    return (
        <ListBoxItem
            aria-label={properties.value}
            data-slot="color-swatch-picker-item"
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

/** Render the hidden input that submits the nearest swatch picker's chosen swatch with its form. */
function ColorSwatchPickerInput(properties: {
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
