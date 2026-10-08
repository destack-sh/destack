import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { PATTERN_SIZING } from "../shader/sizing.ts";

import { VORONOI_DEFAULTS } from "./effect.ts";
import { Voronoi } from "./voronoi.tsx";

/** The size every example draws at. */
const styles = style.create({
    canvas: { width: "100%", height: "20rem" },
});

/** The voronoi in the default look. */
export const voronoiDefault = defineExample({
    of: Voronoi,
    name: "default",
    description: "voronoi in the default look",
    properties: VORONOI_DEFAULTS,
    render: (properties) => <Voronoi {...properties} xstyle={styles.canvas} />,
});

/** The voronoi in the cells look. */
export const voronoiCells = defineExample({
    of: Voronoi,
    name: "cells",
    description: "voronoi in the cells look",
    properties: {
        ...PATTERN_SIZING,
        scale: 0.5,
        speed: 0.5,
        frame: 0,
        colors: ["#ffffff"],
        stepsPerColor: 1,
        colorGlow: "#ffffff",
        colorGap: "#000000",
        distortion: 0.5,
        gap: 0.03,
        glow: 0.8,
    },
    render: (properties) => <Voronoi {...properties} xstyle={styles.canvas} />,
});

/** The voronoi in the bubbles look. */
export const voronoiBubbles = defineExample({
    of: Voronoi,
    name: "bubbles",
    description: "voronoi in the bubbles look",
    properties: {
        ...PATTERN_SIZING,
        scale: 0.75,
        speed: 0.5,
        frame: 0,
        colors: ["#83c9fb"],
        stepsPerColor: 1,
        colorGlow: "#ffffff",
        colorGap: "#ffffff",
        distortion: 0.4,
        gap: 0,
        glow: 1,
    },
    render: (properties) => <Voronoi {...properties} xstyle={styles.canvas} />,
});

/** The voronoi in the lights look. */
export const voronoiLights = defineExample({
    of: Voronoi,
    name: "lights",
    description: "voronoi in the lights look",
    properties: {
        ...PATTERN_SIZING,
        scale: 3.3,
        speed: 0.5,
        frame: 0,
        colors: ["#fffffffc", "#bbff00", "#00ffff"],
        colorGlow: "#ff00d0",
        colorGap: "#ff00d0",
        stepsPerColor: 2,
        distortion: 0.38,
        gap: 0.0,
        glow: 1.0,
    },
    render: (properties) => <Voronoi {...properties} xstyle={styles.canvas} />,
});
