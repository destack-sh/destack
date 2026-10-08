import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { OBJECT_SIZING } from "../shader/sizing.ts";
import { SAMPLE_IMAGE } from "../shader/sample.ts";
import { WATER_DEFAULTS } from "./effect.ts";
import { Water } from "./water.tsx";

/** The size every example draws at. */
const styles = style.create({
    canvas: { width: "100%", height: "20rem" },
});

/** The water in the default look. */
export const waterDefault = defineExample({
    of: Water,
    name: "default",
    description: "water in the default look",
    properties: { ...WATER_DEFAULTS, image: SAMPLE_IMAGE },
    render: (properties) => <Water {...properties} xstyle={styles.canvas} />,
});

/** The water in the abstract look. */
export const waterAbstract = defineExample({
    of: Water,
    name: "abstract",
    description: "water in the abstract look",
    properties: {
        ...OBJECT_SIZING,
        fit: "cover",
        scale: 3,
        speed: 1,
        frame: 0,
        colorBack: "#909090",
        colorHighlight: "#ffffff",
        highlights: 0,
        layering: 0,
        edges: 1,
        waves: 1,
        caustic: 0.4,
        size: 0.15,
        image: SAMPLE_IMAGE,
    },
    render: (properties) => <Water {...properties} xstyle={styles.canvas} />,
});

/** The water in the streaming look. */
export const waterStreaming = defineExample({
    of: Water,
    name: "streaming",
    description: "water in the streaming look",
    properties: {
        ...OBJECT_SIZING,
        fit: "contain",
        scale: 0.4,
        speed: 2,
        frame: 0,
        colorBack: "#909090",
        colorHighlight: "#ffffff",
        highlights: 0,
        layering: 0,
        edges: 0,
        waves: 0.5,
        caustic: 0,
        size: 0.5,
        image: SAMPLE_IMAGE,
    },
    render: (properties) => <Water {...properties} xstyle={styles.canvas} />,
});

/** The water in the slow mo look. */
export const waterSlowMo = defineExample({
    of: Water,
    name: "slow-mo",
    description: "water in the slow mo look",
    properties: {
        ...OBJECT_SIZING,
        fit: "cover",
        scale: 1,
        speed: 0.1,
        frame: 0,
        colorBack: "#909090",
        colorHighlight: "#ffffff",
        highlights: 0.4,
        layering: 0,
        edges: 0,
        waves: 0,
        caustic: 0.2,
        size: 0.7,
        image: SAMPLE_IMAGE,
    },
    render: (properties) => <Water {...properties} xstyle={styles.canvas} />,
});
