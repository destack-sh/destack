import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { OBJECT_SIZING } from "../shader/sizing.ts";
import { SAMPLE_IMAGE } from "../shader/sample.ts";
import { IMAGE_DITHERING_DEFAULTS } from "./effect.ts";
import { ImageDithering } from "./image-dithering.tsx";

/** The size every example draws at. */
const styles = style.create({
    canvas: { width: "100%", height: "20rem" },
});

/** The image dithering in the default look. */
export const imageDitheringDefault = defineExample({
    of: ImageDithering,
    name: "default",
    description: "image dithering in the default look",
    properties: { ...IMAGE_DITHERING_DEFAULTS, image: SAMPLE_IMAGE },
    render: (properties) => <ImageDithering {...properties} xstyle={styles.canvas} />,
});

/** The image dithering in the retro look. */
export const imageDitheringRetro = defineExample({
    of: ImageDithering,
    name: "retro",
    description: "image dithering in the retro look",
    properties: {
        ...OBJECT_SIZING,
        fit: "cover",
        speed: 0,
        frame: 0,
        colorFront: "#eeeeee",
        colorBack: "#5452ff",
        colorHighlight: "#eeeeee",
        type: "2x2",
        size: 3,
        colorSteps: 1,
        originalColors: true,
        inverted: false,
        image: SAMPLE_IMAGE,
    },
    render: (properties) => <ImageDithering {...properties} xstyle={styles.canvas} />,
});

/** The image dithering in the noise look. */
export const imageDitheringNoise = defineExample({
    of: ImageDithering,
    name: "noise",
    description: "image dithering in the noise look",
    properties: {
        ...OBJECT_SIZING,
        fit: "cover",
        speed: 0,
        frame: 0,
        colorFront: "#a2997c",
        colorBack: "#000000",
        colorHighlight: "#ededed",
        type: "random",
        size: 1,
        colorSteps: 1,
        originalColors: false,
        inverted: false,
        image: SAMPLE_IMAGE,
    },
    render: (properties) => <ImageDithering {...properties} xstyle={styles.canvas} />,
});

/** The image dithering in the natural look. */
export const imageDitheringNatural = defineExample({
    of: ImageDithering,
    name: "natural",
    description: "image dithering in the natural look",
    properties: {
        ...OBJECT_SIZING,
        fit: "cover",
        speed: 0,
        frame: 0,
        colorFront: "#ffffff",
        colorBack: "#000000",
        colorHighlight: "#ffffff",
        type: "8x8",
        size: 2,
        colorSteps: 5,
        originalColors: true,
        inverted: false,
        image: SAMPLE_IMAGE,
    },
    render: (properties) => <ImageDithering {...properties} xstyle={styles.canvas} />,
});
