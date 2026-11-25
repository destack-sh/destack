import { HUES } from "@destack/color";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { createSignal } from "@destack/view";
import { Button } from "../button/index.ts";
import { Cluster } from "../cluster/index.ts";
import { ColorSwatchPicker } from "../color-swatch-picker/index.ts";
import { Popover, PopoverContent, PopoverTrigger } from "../popover/index.ts";
import {
    ColorArea,
    ColorEyeDropper,
    ColorField,
    ColorPicker,
    ColorSlider,
    ColorSwatch,
} from "./color-picker.tsx";

/** The styles of the examples. */
const styles = style.create({
    row: {
        display: "flex",
        gap: space[2],
    },
    content: {
        width: "auto",
    },
});

/** A color's area, hue and text. */
export const colorPickerAccent = defineExample({
    of: ColorPicker,
    name: "accent",
    description: "a color's area, hue and text",
    properties: { defaultValue: "#f76b15" },
    render: (properties) => <ColorPicker {...properties} />,
});

/** A label's color in a popover, with presets and the screen's colors. */
export const colorPickerLabel = defineExample({
    of: ColorPicker,
    name: "label",
    description: "a label's color in a popover, with its opacity, presets and the screen's colors",
    render: () => {
        const [color, setColor] = createSignal("#0090ff");

        return (
            <Popover>
                <PopoverTrigger
                    render={(attributes) => (
                        <Button {...attributes} variant="outline">
                            <ColorSwatch color={color()} />
                            <bdi>{color()}</bdi>
                        </Button>
                    )}
                />
                <PopoverContent xstyle={styles.content} aria-label="Color">
                    <ColorPicker value={color()} onValueChange={(next) => void setColor(next)}>
                        <ColorArea />
                        <ColorSlider channel="hue" />
                        <ColorSlider channel="alpha" />
                        <div {...style.attrs(styles.row)}>
                            <ColorField />
                            <ColorEyeDropper />
                        </div>
                        <ColorSwatchPicker
                            aria-label="Presets"
                            colors={HUES.slice(1)}
                            columns={9}
                        />
                    </ColorPicker>
                </PopoverContent>
            </Popover>
        );
    },
});

/** A color as people see it, by its lightness, chroma and hue. */
export const colorPickerPerceived = defineExample({
    of: ColorPicker,
    name: "perceived",
    description: "a color as people see it: lightness and chroma, the hue, and each as a number",
    render: () => (
        <ColorPicker defaultValue="oklch(69% 0.19 45)" format="oklch">
            <ColorArea colorSpace="oklch" />
            <ColorSlider colorSpace="oklch" channel="hue" />
            <div {...style.attrs(styles.row)}>
                <ColorField colorSpace="oklch" channel="lightness" />
                <ColorField colorSpace="oklch" channel="chroma" />
                <ColorField colorSpace="oklch" channel="hue" />
            </div>
            <ColorField />
        </ColorPicker>
    ),
});

/** The channels of a color as sliders. */
export const colorPickerChannels = defineExample({
    of: ColorPicker,
    name: "channels",
    description: "red, green and blue as sliders, and a picker a person cannot use",
    render: () => (
        <Cluster space="6">
            <ColorPicker defaultValue="#30a46c">
                <ColorSlider colorSpace="rgb" channel="red" />
                <ColorSlider colorSpace="rgb" channel="green" />
                <ColorSlider colorSpace="rgb" channel="blue" />
            </ColorPicker>
            <ColorPicker disabled defaultValue="#8e4ec6">
                <ColorArea />
                <ColorSlider channel="hue" />
                <ColorField />
            </ColorPicker>
        </Cluster>
    ),
});
