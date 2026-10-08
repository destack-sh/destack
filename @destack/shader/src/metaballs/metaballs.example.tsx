import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { OBJECT_SIZING } from "../shader/sizing.ts";

import { METABALLS_DEFAULTS } from "./effect.ts";
import { Metaballs } from "./metaballs.tsx";

/** The size every example draws at. */
const styles = style.create({
    canvas: { width: "100%", height: "20rem" },
});

/** The metaballs in the default look. */
export const metaballsDefault = defineExample({
    of: Metaballs,
    name: "default",
    description: "metaballs in the default look",
    properties: METABALLS_DEFAULTS,
    render: (properties) => <Metaballs {...properties} xstyle={styles.canvas} />,
});

/** The metaballs in the ink drops look. */
export const metaballsInkDrops = defineExample({
    of: Metaballs,
    name: "ink-drops",
    description: "metaballs in the ink drops look",
    properties: {
        ...OBJECT_SIZING,
        scale: 1,
        speed: 2,
        frame: 0,
        colorBack: "#ffffff00",
        colors: ["#000000"],
        count: 18,
        size: 0.1,
    },
    render: (properties) => <Metaballs {...properties} xstyle={styles.canvas} />,
});

/** The metaballs in the background look. */
export const metaballsBackground = defineExample({
    of: Metaballs,
    name: "background",
    description: "metaballs in the background look",
    properties: {
        ...OBJECT_SIZING,
        speed: 0.5,
        frame: 0,
        colors: ["#ae00ff", "#00ff95", "#ffc105"],
        colorBack: "#2a273f",
        count: 13,
        size: 0.81,
        scale: 4.0,
        rotation: 0,
        offsetX: -0.3,
    },
    render: (properties) => <Metaballs {...properties} xstyle={styles.canvas} />,
});

/** The metaballs in the solar look. */
export const metaballsSolar = defineExample({
    of: Metaballs,
    name: "solar",
    description: "metaballs in the solar look",
    properties: {
        ...OBJECT_SIZING,
        speed: 1,
        frame: 0,
        colors: ["#ffc800", "#ff5500", "#ffc105"],
        colorBack: "#102f84",
        count: 7,
        size: 0.75,
        scale: 1,
    },
    render: (properties) => <Metaballs {...properties} xstyle={styles.canvas} />,
});
