import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { OBJECT_SIZING } from "../shader/sizing.ts";

import { MESH_GRADIENT_DEFAULTS } from "./effect.ts";
import { MeshGradient } from "./mesh-gradient.tsx";

/** The size every example draws at. */
const styles = style.create({
    canvas: { width: "100%", height: "20rem" },
});

/** The mesh gradient in the default look. */
export const meshGradientDefault = defineExample({
    of: MeshGradient,
    name: "default",
    description: "mesh gradient in the default look",
    properties: MESH_GRADIENT_DEFAULTS,
    render: (properties) => <MeshGradient {...properties} xstyle={styles.canvas} />,
});

/** The mesh gradient in the purple look. */
export const meshGradientPurple = defineExample({
    of: MeshGradient,
    name: "purple",
    description: "mesh gradient in the purple look",
    properties: {
        ...OBJECT_SIZING,
        speed: 0.6,
        frame: 0,
        colors: ["#aaa7d7", "#3c2b8e"],
        distortion: 1,
        swirl: 1,
        grainMixer: 0,
        grainOverlay: 0,
    },
    render: (properties) => <MeshGradient {...properties} xstyle={styles.canvas} />,
});

/** The mesh gradient in the beach look. */
export const meshGradientBeach = defineExample({
    of: MeshGradient,
    name: "beach",
    description: "mesh gradient in the beach look",
    properties: {
        ...OBJECT_SIZING,
        speed: 0.1,
        frame: 0,
        colors: ["#bcecf6", "#00aaff", "#00f7ff", "#ffd447"],
        distortion: 0.8,
        swirl: 0.35,
        grainMixer: 0,
        grainOverlay: 0,
    },
    render: (properties) => <MeshGradient {...properties} xstyle={styles.canvas} />,
});

/** The mesh gradient in the ink look. */
export const meshGradientInk = defineExample({
    of: MeshGradient,
    name: "ink",
    description: "mesh gradient in the ink look",
    properties: {
        ...OBJECT_SIZING,
        speed: 1,
        frame: 0,
        colors: ["#ffffff", "#000000"],
        distortion: 1,
        swirl: 0.2,
        rotation: 90,
        grainMixer: 0,
        grainOverlay: 0,
    },
    render: (properties) => <MeshGradient {...properties} xstyle={styles.canvas} />,
});
