import { Color, type ColorFormat, type ColorSpace } from "@destack/style";
import { type Message, t } from "@destack/locale";
import type { FieldControl } from "../field/control.ts";
import type { ColorChannel } from "@destack/style";
import {
    type Accessor,
    createContext,
    createControllableSignal,
    untrack,
    useContext,
} from "@destack/view";

/** The color a picker without one starts at. */
const START = "#000000";

/** What each channel is called. */
export const CHANNEL_NAMES: Readonly<Record<ColorChannel, Message>> = {
    red: t.context("label")`Red`,
    green: t.context("label")`Green`,
    blue: t.context("label")`Blue`,
    hue: t.context("label")`Hue`,
    saturation: t.context("label")`Saturation`,
    lightness: t.context("label")`Lightness`,
    brightness: t.context("label")`Brightness`,
    chroma: t.context("label")`Chroma`,
    alpha: t.context("label")`Opacity`,
};

/** The color of a picker, and what it tells its owner. */
export interface ColorPickerOptions {
    /** The color in CSS, such as #f76b15 or oklch(69% 0.19 45). */
    readonly value?: string | undefined;
    /** The color at first while uncontrolled, black by default. */
    readonly defaultValue?: string;
    /** Handle each change of the color, as it is dragged. */
    readonly onValueChange?: (value: string) => void;
    /** Handle the color a person settles on, as a drag ends. */
    readonly onValueCommitted?: (value: string) => void;
    /** How the color is written, hexadecimal by default. */
    readonly format?: ColorFormat;
    /** Whether the parts ignore the person, as its field does when absent. */
    readonly disabled?: boolean | undefined;
}

/** The color of a picker, which its parts share whatever space each shows. */
export class ColorPickerControl {
    /** The color as it is written. */
    readonly value: Accessor<string>;
    /** The picker's properties. */
    readonly #options: ColorPickerOptions;
    /** The field around the picker, null outside one. */
    readonly #field: FieldControl | null;
    /** Replace the written color and tell the change handler. */
    readonly #setValue: (value: string) => void;
    /** The color last picked, whose hue a gray one keeps, and the way it was written. */
    #picked: { readonly color: Color; readonly value: string } | undefined;

    /** Follow a picker's color. */
    constructor(options: ColorPickerOptions, fieldControl: FieldControl | null) {
        // follow the controlled color or the picker's own
        const [value, setValue] = createControllableSignal<string>({
            isControlled: () => "value" in options,
            value: () => options.value ?? START,
            defaultValue: untrack(() => options.defaultValue) ?? START,
            onChange: (next) => options.onValueChange?.(next),
        });
        this.value = value;
        this.#options = options;
        this.#field = fieldControl;
        this.#setValue = setValue;
    }

    /** Read the color in a space. */
    color(space: ColorSpace): Color {
        // take the one picked while it is the one written
        const value = this.value();
        const picked = this.#picked?.value === value ? this.#picked.color : undefined;

        return (picked ?? Color.parse(value)).to(space);
    }

    /** Set the color. */
    set(color: Color): void {
        const value = color.toString(this.format());
        this.#picked = { color, value };
        this.#setValue(value);
    }

    /** Tell the owner the color the person settled on. */
    commit(): void {
        this.#options.onValueCommitted?.(this.#picked?.value ?? untrack(this.value));
    }

    /** Read how the color is written. */
    format(): ColorFormat {
        return this.#options.format ?? "hex";
    }

    /** Report whether the parts ignore the person. */
    isDisabled(): boolean {
        return this.#options.disabled === true || this.#field?.isDisabled() === true;
    }
}

/** The nearest picker, null outside one. */
export const ColorPickerContext = createContext<ColorPickerControl | null>(null);

/** Read the nearest color picker, refusing a part outside one. */
export function useColorPicker(): ColorPickerControl {
    const control = useContext(ColorPickerContext);
    if (control === null) {
        throw new TypeError("a color picker part needs a color picker around it");
    }

    return control;
}
