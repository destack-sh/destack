import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { PATTERN_SIZING } from "../shader/sizing.ts";

import { WAVES_DEFAULTS } from "./effect.ts";
import { Waves } from "./waves.tsx";

/** The size every example draws at. */
const styles = style.create({
    canvas: { width: "100%", height: "20rem" },
});

/** The waves in the default look. */
export const wavesDefault = defineExample({
    of: Waves,
    name: "default",
    description: "waves in the default look",
    properties: WAVES_DEFAULTS,
    render: (properties) => <Waves {...properties} xstyle={styles.canvas} />,
});

/** The waves in the groovy look. */
export const wavesGroovy = defineExample({
    of: Waves,
    name: "groovy",
    description: "waves in the groovy look",
    properties: {
        ...PATTERN_SIZING,
        scale: 5,
        rotation: 90,
        colorFront: "#fcfcee",
        colorBack: "#ff896b",
        shape: 3,
        frequency: 0.2,
        amplitude: 0.25,
        spacing: 1.17,
        proportion: 0.57,
        softness: 0,
    },
    render: (properties) => <Waves {...properties} xstyle={styles.canvas} />,
});

/** The waves in the tangled up look. */
export const wavesTangledUp = defineExample({
    of: Waves,
    name: "tangled-up",
    description: "waves in the tangled up look",
    properties: {
        ...PATTERN_SIZING,
        scale: 0.5,
        rotation: 0,
        colorFront: "#133a41",
        colorBack: "#c2d8b6",
        shape: 2.07,
        frequency: 0.44,
        amplitude: 0.57,
        spacing: 1.05,
        proportion: 0.75,
        softness: 0,
    },
    render: (properties) => <Waves {...properties} xstyle={styles.canvas} />,
});

/** The waves in the ride the wave look. */
export const wavesRideTheWave = defineExample({
    of: Waves,
    name: "ride-the-wave",
    description: "waves in the ride the wave look",
    properties: {
        ...PATTERN_SIZING,
        scale: 1.7,
        rotation: 0,
        colorFront: "#fdffe6",
        colorBack: "#1f1f1f",
        shape: 2.25,
        frequency: 0.2,
        amplitude: 1,
        spacing: 1.25,
        proportion: 1,
        softness: 0,
    },
    render: (properties) => <Waves {...properties} xstyle={styles.canvas} />,
});
