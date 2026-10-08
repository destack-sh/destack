import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { OBJECT_SIZING } from "../shader/sizing.ts";

import { STATIC_MESH_GRADIENT_DEFAULTS } from "./effect.ts";
import { StaticMeshGradient } from "./static-mesh-gradient.tsx";

/** The size every example draws at. */
const styles = style.create({
    canvas: { width: "100%", height: "20rem" },
});

/** The static mesh gradient in the default look. */
export const staticMeshGradientDefault = defineExample({
    of: StaticMeshGradient,
    name: "default",
    description: "static mesh gradient in the default look",
    properties: STATIC_MESH_GRADIENT_DEFAULTS,
    render: (properties) => <StaticMeshGradient {...properties} xstyle={styles.canvas} />,
});

/** The static mesh gradient in the sea look. */
export const staticMeshGradientSea = defineExample({
    of: StaticMeshGradient,
    name: "sea",
    description: "static mesh gradient in the sea look",
    properties: {
        ...OBJECT_SIZING,
        speed: 0,
        frame: 0,
        colors: ["#013b65", "#03738c", "#a3d3ff", "#f2faef"],
        positions: 0,
        waveX: 0.53,
        waveXShift: 0.0,
        waveY: 0.95,
        waveYShift: 0.64,
        mixing: 0.5,
        grainMixer: 0.0,
        grainOverlay: 0.0,
    },
    render: (properties) => <StaticMeshGradient {...properties} xstyle={styles.canvas} />,
});

/** The static mesh gradient in the sixties look. */
export const staticMeshGradientSixties = defineExample({
    of: StaticMeshGradient,
    name: "sixties",
    description: "static mesh gradient in the sixties look",
    properties: {
        ...OBJECT_SIZING,
        speed: 0,
        frame: 0,
        colors: ["#000000", "#082400", "#b1aa91", "#8e8c15"],
        positions: 42,
        waveX: 0.45,
        waveXShift: 0.0,
        waveY: 1.0,
        waveYShift: 0.0,
        mixing: 0.0,
        grainMixer: 0.37,
        grainOverlay: 0.78,
    },
    render: (properties) => <StaticMeshGradient {...properties} xstyle={styles.canvas} />,
});

/** The static mesh gradient in the sunset look. */
export const staticMeshGradientSunset = defineExample({
    of: StaticMeshGradient,
    name: "sunset",
    description: "static mesh gradient in the sunset look",
    properties: {
        ...OBJECT_SIZING,
        speed: 0,
        frame: 0,
        colors: ["#264653", "#9c2b2b", "#f4a261", "#ffffff"],
        positions: 0,
        waveX: 0.6,
        waveXShift: 0.7,
        waveY: 0.7,
        waveYShift: 0.7,
        mixing: 0.5,
        grainMixer: 0,
        grainOverlay: 0,
    },
    render: (properties) => <StaticMeshGradient {...properties} xstyle={styles.canvas} />,
});
