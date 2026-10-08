import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { OBJECT_SIZING, PATTERN_SIZING } from "../shader/sizing.ts";

import { DITHERING_DEFAULTS } from "./effect.ts";
import { Dithering } from "./dithering.tsx";

/** The size every example draws at. */
const styles = style.create({
    canvas: { width: "100%", height: "20rem" },
});

/** The dithering in the default look. */
export const ditheringDefault = defineExample({
    of: Dithering,
    name: "default",
    description: "dithering in the default look",
    properties: DITHERING_DEFAULTS,
    render: (properties) => <Dithering {...properties} xstyle={styles.canvas} />,
});

/** The dithering in the sine wave look. */
export const ditheringSineWave = defineExample({
    of: Dithering,
    name: "sine-wave",
    description: "dithering in the sine wave look",
    properties: {
        ...PATTERN_SIZING,
        speed: 1,
        frame: 0,
        colorBack: "#730d54",
        colorFront: "#00becc",
        shape: "wave",
        type: "4x4",
        size: 11,
        scale: 1.2,
    },
    render: (properties) => <Dithering {...properties} xstyle={styles.canvas} />,
});

/** The dithering in the bugs look. */
export const ditheringBugs = defineExample({
    of: Dithering,
    name: "bugs",
    description: "dithering in the bugs look",
    properties: {
        ...PATTERN_SIZING,
        speed: 1,
        frame: 0,
        colorBack: "#000000",
        colorFront: "#008000",
        shape: "dots",
        type: "random",
        size: 9,
    },
    render: (properties) => <Dithering {...properties} xstyle={styles.canvas} />,
});

/** The dithering in the ripple look. */
export const ditheringRipple = defineExample({
    of: Dithering,
    name: "ripple",
    description: "dithering in the ripple look",
    properties: {
        ...OBJECT_SIZING,
        speed: 1,
        frame: 0,
        colorBack: "#603520",
        colorFront: "#c67953",
        shape: "ripple",
        type: "2x2",
        size: 3,
    },
    render: (properties) => <Dithering {...properties} xstyle={styles.canvas} />,
});

/** The dithering in the swirl look. */
export const ditheringSwirl = defineExample({
    of: Dithering,
    name: "swirl",
    description: "dithering in the swirl look",
    properties: {
        ...OBJECT_SIZING,
        speed: 1,
        frame: 0,
        colorBack: "#00000000",
        colorFront: "#47a8e1",
        shape: "swirl",
        type: "8x8",
        size: 2,
    },
    render: (properties) => <Dithering {...properties} xstyle={styles.canvas} />,
});

/** The dithering in the warp look. */
export const ditheringWarp = defineExample({
    of: Dithering,
    name: "warp",
    description: "dithering in the warp look",
    properties: {
        ...OBJECT_SIZING,
        speed: 1,
        frame: 0,
        colorBack: "#301c2a",
        colorFront: "#56ae6c",
        shape: "warp",
        type: "4x4",
        size: 2.5,
    },
    render: (properties) => <Dithering {...properties} xstyle={styles.canvas} />,
});
