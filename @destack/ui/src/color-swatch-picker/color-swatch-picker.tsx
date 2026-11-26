import { t } from "@destack/locale";
import { field } from "@destack/object";
import { schema } from "@destack/schema";
import { contentOr } from "../part/index.ts";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { Color, DEFAULT_HUES, HUES, type Hue } from "@destack/color";
import { color, radius, size, space, stroke, swatch } from "@destack/theme/tokens.stylex";
import { For, type JSX, omit, Show, useContext, useLocale } from "@destack/view";
import {
    ListBox,
    type ListBoxElementProperties,
    ListBoxItem,
    type ListBoxItemProperties,
    useListBox,
} from "../list-box/index.ts";
import { ColorPickerContext } from "../color-picker/control.ts";
import { HUE_NAMES, nameOf } from "../color-picker/name.ts";
import { useFieldValue, readChoice } from "../field/control.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The properties people change in a swatch picker's controls and inspector. */
const COLOR_SWATCH_PICKER_PROPERTIES = {
    columns: field
        .integer(schema.number().int().min(1).max(12))
        .labeled(t.context("label")`Columns`)
        .described(t`The swatches in each row, which the up and down arrow keys move across.`)
        .default(5),
};

/** The properties of a swatch picker besides its declared ones: its choice, colors and form. */
export interface ColorSwatchPickerAttributes extends ListBoxElementProperties<
    Omit<JSX.HTMLAttributes<HTMLDivElement>, "onKeyDown" | "onFocusOut">
> {
    /** The chosen hue or CSS color, which makes the choice controlled, undefined for none. */
    readonly value?: string | undefined;
    /** The swatch chosen at first while uncontrolled. */
    readonly defaultValue?: string;
    /** Handle the person choosing a swatch. */
    readonly onValueChange?: (value: string) => void;
    /** The hues and CSS colors offered without items of its own, every hue by default. */
    readonly colors?: readonly string[];
    /** The form field name the chosen swatch submits under, which renders a hidden input. */
    readonly name?: string;
    /** The id of the form the chosen swatch belongs to, its ancestor form by default. */
    readonly form?: string;
}

/** The properties of a swatch, the option's included. */
export interface ColorSwatchPickerItemProperties extends Omit<ListBoxItemProperties, "value"> {
    /** The theme's hue or the CSS color the swatch shows and stands for. */
    readonly value: string;
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
        width: { default: `calc(${size[1]} * 0.75)`, [media.pointerCoarse]: size[2] },
        height: { default: `calc(${size[1]} * 0.75)`, [media.pointerCoarse]: size[2] },
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

/** Report whether a swatch stands for a hue of the theme. */
function isHue(value: string): value is Hue {
    return HUES.some((hue) => hue === value);
}

/** Read the color a swatch stands for. */
function colorOf(value: string): Color {
    return Color.parse(isHue(value) ? DEFAULT_HUES[value] : value);
}

/** Render swatches as a list box in a grid layout, which set the color of a picker around them. */
export const ColorSwatchPicker = defineComponent({
    name: "colorSwatchPicker",
    properties: COLOR_SWATCH_PICKER_PROPERTIES,
    render(
        given: RenderProperties<ColorSwatchPickerAttributes, typeof COLOR_SWATCH_PICKER_PROPERTIES>,
    ): JSX.Element {
        // set the color of the picker around it, or edit the value of the field's form
        const picker = useContext(ColorPickerContext);
        const properties = useFieldValue(given, {
            value: "value",
            change: "onValueChange",
            read: readChoice,
            settles: true,
        });

        // offer the given colors in their order unless the caller lays out its own swatches
        const rest = omit(
            properties,
            "value",
            "defaultValue",
            "onValueChange",
            "colors",
            "columns",
            "name",
            "form",
            "children",
            "xstyle",
        );
        const colors = (): readonly string[] => properties.colors ?? HUES;
        const onValueChange = (value: string | undefined): void => {
            // set the picker's color and report a chosen one
            if (value !== undefined) {
                picker?.set(Color.parse(value));
                picker?.commit();
                properties.onValueChange?.(value);
            }
        };

        // choose one swatch at a time, the focus moving selecting it
        return (
            <ListBox
                data-slot="color-swatch-picker"
                layout="grid"
                columns={properties.columns}
                selectionBehavior="replace"
                selection={
                    picker !== null || "value" in properties
                        ? {
                              get value() {
                                  return picker === null ? properties.value : picker.value();
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
                {contentOr(properties, () => (
                    <For each={colors()}>{(value) => <ColorSwatchPickerItem value={value} />}</For>
                ))}
                <Show when={properties.name !== undefined || properties.form !== undefined}>
                    <ColorSwatchPickerInput name={properties.name} form={properties.form} />
                </Show>
            </ListBox>
        );
    },
});

/** The properties of a swatch picker, the native element's attributes included. */
export type ColorSwatchPickerProperties = ComponentProperties<typeof ColorSwatchPicker>;

/** Render one swatch of the nearest swatch picker. */
export function ColorSwatchPickerItem(properties: ColorSwatchPickerItemProperties): JSX.Element {
    // stand for the color as the picker around it writes it
    const picker = useContext(ColorPickerContext);
    const locale = useLocale();
    const rest = omit(properties, "value", "xstyle", "children");
    const value = (): string =>
        picker === null ? properties.value : colorOf(properties.value).toString(picker.format());
    const fill = (): string =>
        picker === null && isHue(properties.value) ? swatch[`${properties.value}Solid`] : value();
    const label = (): string =>
        isHue(properties.value)
            ? locale.render(HUE_NAMES[properties.value])
            : nameOf(colorOf(properties.value), locale);

    return (
        <ListBoxItem
            aria-label={label()}
            data-slot="color-swatch-picker-item"
            data-swatch={properties.value}
            value={value()}
            disabled={picker?.isDisabled() === true}
            {...rest}
            xstyle={[styles.swatch, fills.fill(fill()), properties.xstyle]}
        >
            {properties.children}
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
