import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { OBJECT_SIZING } from "../shader/sizing.ts";
import { SAMPLE_IMAGE } from "../shader/sample.ts";
import { HALFTONE_DOTS_DEFAULTS } from "./effect.ts";
import { HalftoneDots } from "./halftone-dots.tsx";

/** The size every example draws at. */
const styles = style.create({
    canvas: { width: "100%", height: "20rem" },
});

/** The halftone dots in the default look. */
export const halftoneDotsDefault = defineExample({
    of: HalftoneDots,
    name: "default",
    description: "halftone dots in the default look",
    properties: { ...HALFTONE_DOTS_DEFAULTS, image: SAMPLE_IMAGE },
    render: (properties) => <HalftoneDots {...properties} xstyle={styles.canvas} />,
});

/** The halftone dots in the led screen look. */
export const halftoneDotsLedScreen = defineExample({
    of: HalftoneDots,
    name: "led-screen",
    description: "halftone dots in the led screen look",
    properties: {
        ...OBJECT_SIZING,
        fit: "cover",
        speed: 0,
        frame: 0,
        colorBack: "#000000",
        colorFront: "#29ff7b",
        size: 0.5,
        radius: 1.5,
        contrast: 0.3,
        originalColors: false,
        inverted: false,
        grainMixer: 0,
        grainOverlay: 0,
        grainSize: 0.5,
        grid: "square",
        type: "soft",
        image: SAMPLE_IMAGE,
    },
    render: (properties) => <HalftoneDots {...properties} xstyle={styles.canvas} />,
});

/** The halftone dots in the mosaic look. */
export const halftoneDotsMosaic = defineExample({
    of: HalftoneDots,
    name: "mosaic",
    description: "halftone dots in the mosaic look",
    properties: {
        ...OBJECT_SIZING,
        fit: "cover",
        speed: 0,
        frame: 0,
        colorBack: "#000000",
        colorFront: "#b2aeae",
        size: 0.6,
        radius: 2,
        contrast: 0.01,
        originalColors: true,
        inverted: false,
        grainMixer: 0,
        grainOverlay: 0,
        grainSize: 0.5,
        grid: "hex",
        type: "classic",
        image: SAMPLE_IMAGE,
    },
    render: (properties) => <HalftoneDots {...properties} xstyle={styles.canvas} />,
});

/** The halftone dots in the round and square look. */
export const halftoneDotsRoundAndSquare = defineExample({
    of: HalftoneDots,
    name: "round-and-square",
    description: "halftone dots in the round and square look",
    properties: {
        ...OBJECT_SIZING,
        fit: "cover",
        speed: 0,
        frame: 0,
        colorBack: "#141414",
        colorFront: "#ff8000",
        size: 0.8,
        radius: 1,
        contrast: 1,
        originalColors: false,
        inverted: true,
        grainMixer: 0.05,
        grainOverlay: 0.3,
        grainSize: 0.5,
        grid: "square",
        type: "holes",
        image: SAMPLE_IMAGE,
    },
    render: (properties) => <HalftoneDots {...properties} xstyle={styles.canvas} />,
});
