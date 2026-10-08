import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { OBJECT_SIZING } from "../shader/sizing.ts";

import { GOD_RAYS_DEFAULTS } from "./effect.ts";
import { GodRays } from "./god-rays.tsx";

/** The size every example draws at. */
const styles = style.create({
    canvas: { width: "100%", height: "20rem" },
});

/** The god rays in the default look. */
export const godRaysDefault = defineExample({
    of: GodRays,
    name: "default",
    description: "god rays in the default look",
    properties: GOD_RAYS_DEFAULTS,
    render: (properties) => <GodRays {...properties} xstyle={styles.canvas} />,
});

/** The god rays in the warp look. */
export const godRaysWarp = defineExample({
    of: GodRays,
    name: "warp",
    description: "god rays in the warp look",
    properties: {
        ...OBJECT_SIZING,
        colorBack: "#000000",
        colorBloom: "#222288",
        colors: ["#ff47d4", "#ff8c00", "#ffffff"],
        density: 0.45,
        spotty: 0.15,
        midIntensity: 0.4,
        midSize: 0.33,
        intensity: 0.79,
        bloom: 0.4,
        speed: 2,
        frame: 0,
    },
    render: (properties) => <GodRays {...properties} xstyle={styles.canvas} />,
});

/** The god rays in the linear look. */
export const godRaysLinear = defineExample({
    of: GodRays,
    name: "linear",
    description: "god rays in the linear look",
    properties: {
        ...OBJECT_SIZING,
        offsetX: 0.2,
        offsetY: -0.8,
        colorBack: "#000000",
        colorBloom: "#eeeeee",
        colors: ["#ffffff1f", "#ffffff3d", "#ffffff29"],
        density: 0.41,
        spotty: 0.25,
        midSize: 0.1,
        midIntensity: 0.75,
        intensity: 0.79,
        bloom: 1,
        speed: 0.5,
        frame: 0,
    },
    render: (properties) => <GodRays {...properties} xstyle={styles.canvas} />,
});

/** The god rays in the ether look. */
export const godRaysEther = defineExample({
    of: GodRays,
    name: "ether",
    description: "god rays in the ether look",
    properties: {
        ...OBJECT_SIZING,
        offsetX: -0.6,
        colorBack: "#090f1d",
        colorBloom: "#ffffff",
        colors: ["#148effa6", "#c4dffebe", "#232a47"],
        density: 0.03,
        spotty: 0.77,
        midSize: 0.1,
        midIntensity: 0.6,
        intensity: 0.6,
        bloom: 0.6,
        speed: 1,
        frame: 0,
    },
    render: (properties) => <GodRays {...properties} xstyle={styles.canvas} />,
});
