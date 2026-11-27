import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import type { GradientJson, Paint } from "@destack/style";
import { GradientPicker, PaintPicker, PaintSwatch } from "./paint-picker.tsx";

/** A sunset sky from deep blue through pink to gold, blended round the hues. */
const SUNSET: GradientJson = {
    shape: { kind: "linear", angle: 180 },
    stops: [
        { color: "#241d9a", position: 0 },
        { color: "#f75092", position: 0.55 },
        { color: "#ffc53d", position: 1 },
    ],
    space: "oklch",
    hue: "shorter",
};

/** A glow from the top left corner, fading out. */
const GLOW: Paint = {
    kind: "gradient",
    gradient: {
        shape: { kind: "radial", center: { x: 0.2, y: 0.2 } },
        stops: [
            { color: "#ffc53d", position: 0 },
            { color: "#ffc53d00", position: 1 },
        ],
        space: "oklab",
        hue: "shorter",
    },
};

/** A card's background: a sunset sky. */
export const paintPickerDefault = defineExample({
    of: PaintPicker,
    name: "default",
    description: t`A card's background: a sunset sky, its kind, shape, stops and blend.`,
    properties: { defaultValue: { kind: "gradient", gradient: SUNSET } },
    render: (properties) => <PaintPicker {...properties} />,
});

/** A label's color, one color with its opacity. */
export const paintPickerSolid = defineExample({
    of: PaintPicker,
    name: "solid",
    description: t`A label's color, one color with its opacity.`,
    properties: { defaultValue: { kind: "solid", color: "#0090ff99" } },
    render: (properties) => <PaintPicker {...properties} />,
});

/** A page's paper, an image tiled from the top. */
export const paintPickerImage = defineExample({
    of: PaintPicker,
    name: "image",
    description: t`A page's paper, an image tiled from the top.`,
    properties: {
        defaultValue: {
            kind: "image",
            source: "https://images.example.com/paper.png",
            fit: "tile",
            position: { x: 0.5, y: 0 },
        },
    },
    render: (properties) => <PaintPicker {...properties} />,
});

/** A sunset sky's stops on their own. */
export const gradientPickerDefault = defineExample({
    of: GradientPicker,
    name: "default",
    description: t`A sunset sky's shape, stops and blend.`,
    properties: { defaultValue: SUNSET },
    render: (properties) => <GradientPicker {...properties} />,
});

/** A glow, over squares where it is see-through. */
export const paintSwatchDefault = defineExample({
    of: PaintSwatch,
    name: "default",
    description: t`A glow from the top left, over squares where it is see-through.`,
    properties: { paint: GLOW },
    render: (properties) => <PaintSwatch {...properties} />,
});
