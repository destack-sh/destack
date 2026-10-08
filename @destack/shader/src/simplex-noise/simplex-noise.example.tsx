import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { PATTERN_SIZING } from "../shader/sizing.ts";

import { SIMPLEX_NOISE_DEFAULTS } from "./effect.ts";
import { SimplexNoise } from "./simplex-noise.tsx";

/** The size every example draws at. */
const styles = style.create({
    canvas: { width: "100%", height: "20rem" },
});

/** The simplex noise in the default look. */
export const simplexNoiseDefault = defineExample({
    of: SimplexNoise,
    name: "default",
    description: "simplex noise in the default look",
    properties: SIMPLEX_NOISE_DEFAULTS,
    render: (properties) => <SimplexNoise {...properties} xstyle={styles.canvas} />,
});

/** The simplex noise in the bubblegum look. */
export const simplexNoiseBubblegum = defineExample({
    of: SimplexNoise,
    name: "bubblegum",
    description: "simplex noise in the bubblegum look",
    properties: {
        ...PATTERN_SIZING,
        speed: 2,
        frame: 0,
        colors: ["#ffffff", "#ff9e9e", "#5f57ff", "#00f7ff"],
        stepsPerColor: 1,
        softness: 1.0,
        scale: 1.6,
    },
    render: (properties) => <SimplexNoise {...properties} xstyle={styles.canvas} />,
});

/** The simplex noise in the spots look. */
export const simplexNoiseSpots = defineExample({
    of: SimplexNoise,
    name: "spots",
    description: "simplex noise in the spots look",
    properties: {
        ...PATTERN_SIZING,
        speed: 0.6,
        frame: 0,
        colors: ["#ff7b00", "#f9ffeb", "#320d82"],
        stepsPerColor: 1,
        softness: 0.0,
        scale: 1.0,
    },
    render: (properties) => <SimplexNoise {...properties} xstyle={styles.canvas} />,
});

/** The simplex noise in the first contact look. */
export const simplexNoiseFirstContact = defineExample({
    of: SimplexNoise,
    name: "first-contact",
    description: "simplex noise in the first contact look",
    properties: {
        ...PATTERN_SIZING,
        speed: 2,
        frame: 0,
        colors: ["#e8cce6", "#120d22", "#442c44", "#e6baba", "#fff5f5"],
        stepsPerColor: 2,
        softness: 0.0,
        scale: 0.2,
    },
    render: (properties) => <SimplexNoise {...properties} xstyle={styles.canvas} />,
});
