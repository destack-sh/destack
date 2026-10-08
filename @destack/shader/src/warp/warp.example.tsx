import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { PATTERN_SIZING } from "../shader/sizing.ts";

import { WARP_DEFAULTS } from "./effect.ts";
import { Warp } from "./warp.tsx";

/** The size every example draws at. */
const styles = style.create({
    canvas: { width: "100%", height: "20rem" },
});

/** The warp in the default look. */
export const warpDefault = defineExample({
    of: Warp,
    name: "default",
    description: "warp in the default look",
    properties: WARP_DEFAULTS,
    render: (properties) => <Warp {...properties} xstyle={styles.canvas} />,
});

/** The warp in the cauldron pot look. */
export const warpCauldronPot = defineExample({
    of: Warp,
    name: "cauldron-pot",
    description: "warp in the cauldron pot look",
    properties: {
        ...PATTERN_SIZING,
        scale: 0.9,
        rotation: 160,
        speed: 10,
        frame: 0,
        colors: ["#a7e58b", "#324472", "#0a180d"],
        proportion: 0.64,
        softness: 1.5,
        distortion: 0.2,
        swirl: 0.86,
        swirlIterations: 7,
        shapeScale: 0.6,
        shape: "edge",
    },
    render: (properties) => <Warp {...properties} xstyle={styles.canvas} />,
});

/** The warp in the live ink look. */
export const warpLiveInk = defineExample({
    of: Warp,
    name: "live-ink",
    description: "warp in the live ink look",
    properties: {
        ...PATTERN_SIZING,
        scale: 1.2,
        rotation: 44,
        offsetY: -0.3,
        speed: 2.5,
        frame: 0,
        colors: ["#111314", "#9faeab", "#f3fee7", "#f3fee7"],
        proportion: 0.05,
        softness: 0,
        distortion: 0.25,
        swirl: 0.8,
        swirlIterations: 10,
        shapeScale: 0.28,
        shape: "checks",
    },
    render: (properties) => <Warp {...properties} xstyle={styles.canvas} />,
});

/** The warp in the kelp look. */
export const warpKelp = defineExample({
    of: Warp,
    name: "kelp",
    description: "warp in the kelp look",
    properties: {
        ...PATTERN_SIZING,
        scale: 0.8,
        rotation: 50,
        speed: 20,
        frame: 0,
        colors: ["#dbff8f", "#404f3e", "#091316"],
        proportion: 0.67,
        softness: 0,
        distortion: 0,
        swirl: 0.2,
        swirlIterations: 3,
        shapeScale: 1,
        shape: "stripes",
    },
    render: (properties) => <Warp {...properties} xstyle={styles.canvas} />,
});

/** The warp in the nectar look. */
export const warpNectar = defineExample({
    of: Warp,
    name: "nectar",
    description: "warp in the nectar look",
    properties: {
        ...PATTERN_SIZING,
        scale: 2,
        offsetY: 0.6,
        rotation: 0,
        speed: 4.2,
        frame: 0,
        colors: ["#151310", "#d3a86b", "#f0edea"],
        proportion: 0.24,
        softness: 1,
        distortion: 0.21,
        swirl: 0.57,
        swirlIterations: 10,
        shapeScale: 0.75,
        shape: "edge",
    },
    render: (properties) => <Warp {...properties} xstyle={styles.canvas} />,
});

/** The warp in the passion look. */
export const warpPassion = defineExample({
    of: Warp,
    name: "passion",
    description: "warp in the passion look",
    properties: {
        ...PATTERN_SIZING,
        scale: 2.5,
        rotation: 1.35,
        speed: 3,
        frame: 0,
        colors: ["#3b1515", "#954751", "#ffc085"],
        proportion: 0.5,
        softness: 1,
        distortion: 0.09,
        swirl: 0.9,
        swirlIterations: 6,
        shapeScale: 0.25,
        shape: "checks",
    },
    render: (properties) => <Warp {...properties} xstyle={styles.canvas} />,
});
