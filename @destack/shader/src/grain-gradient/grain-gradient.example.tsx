import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { OBJECT_SIZING, PATTERN_SIZING } from "../shader/sizing.ts";

import { GRAIN_GRADIENT_DEFAULTS } from "./effect.ts";
import { GrainGradient } from "./grain-gradient.tsx";

/** The size every example draws at. */
const styles = style.create({
    canvas: { width: "100%", height: "20rem" },
});

/** The grain gradient in the default look. */
export const grainGradientDefault = defineExample({
    of: GrainGradient,
    name: "default",
    description: "grain gradient in the default look",
    properties: GRAIN_GRADIENT_DEFAULTS,
    render: (properties) => <GrainGradient {...properties} xstyle={styles.canvas} />,
});

/** The grain gradient in the wave look. */
export const grainGradientWave = defineExample({
    of: GrainGradient,
    name: "wave",
    description: "grain gradient in the wave look",
    properties: {
        ...PATTERN_SIZING,
        speed: 1,
        frame: 0,
        colorBack: "#000a0f",
        colors: ["#c4730b", "#bdad5f", "#d8ccc7"],
        softness: 0.7,
        intensity: 0.15,
        noise: 0.5,
        shape: "wave",
    },
    render: (properties) => <GrainGradient {...properties} xstyle={styles.canvas} />,
});

/** The grain gradient in the dots look. */
export const grainGradientDots = defineExample({
    of: GrainGradient,
    name: "dots",
    description: "grain gradient in the dots look",
    properties: {
        ...PATTERN_SIZING,
        scale: 0.6,
        speed: 1,
        frame: 0,
        colorBack: "#0a0000",
        colors: ["#6f0000", "#0080ff", "#f2ebc9", "#33cc33"],
        softness: 1,
        intensity: 1,
        noise: 0.7,
        shape: "dots",
    },
    render: (properties) => <GrainGradient {...properties} xstyle={styles.canvas} />,
});

/** The grain gradient in the truchet look. */
export const grainGradientTruchet = defineExample({
    of: GrainGradient,
    name: "truchet",
    description: "grain gradient in the truchet look",
    properties: {
        ...PATTERN_SIZING,
        speed: 1,
        frame: 0,
        colorBack: "#0a0000",
        colors: ["#6f2200", "#eabb7c", "#39b523"],
        softness: 0,
        intensity: 0.2,
        noise: 1,
        shape: "truchet",
    },
    render: (properties) => <GrainGradient {...properties} xstyle={styles.canvas} />,
});

/** The grain gradient in the ripple look. */
export const grainGradientRipple = defineExample({
    of: GrainGradient,
    name: "ripple",
    description: "grain gradient in the ripple look",
    properties: {
        ...OBJECT_SIZING,
        scale: 0.5,
        speed: 1,
        frame: 0,
        colorBack: "#140a00",
        colors: ["#6f2d00", "#88ddae", "#2c0b1d"],
        softness: 0.5,
        intensity: 0.5,
        noise: 0.5,
        shape: "ripple",
    },
    render: (properties) => <GrainGradient {...properties} xstyle={styles.canvas} />,
});

/** The grain gradient in the blob look. */
export const grainGradientBlob = defineExample({
    of: GrainGradient,
    name: "blob",
    description: "grain gradient in the blob look",
    properties: {
        ...OBJECT_SIZING,
        scale: 1.3,
        speed: 1,
        frame: 0,
        colorBack: "#0f0e18",
        colors: ["#3e6172", "#a49b74", "#568c50"],
        softness: 0,
        intensity: 0.15,
        noise: 0.5,
        shape: "blob",
    },
    render: (properties) => <GrainGradient {...properties} xstyle={styles.canvas} />,
});
