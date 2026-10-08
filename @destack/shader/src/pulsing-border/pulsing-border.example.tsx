import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { OBJECT_SIZING } from "../shader/sizing.ts";

import { PULSING_BORDER_DEFAULTS } from "./effect.ts";
import { PulsingBorder } from "./pulsing-border.tsx";

/** The size every example draws at. */
const styles = style.create({
    canvas: { width: "100%", height: "20rem" },
});

/** The pulsing border in the default look. */
export const pulsingBorderDefault = defineExample({
    of: PulsingBorder,
    name: "default",
    description: "pulsing border in the default look",
    properties: PULSING_BORDER_DEFAULTS,
    render: (properties) => <PulsingBorder {...properties} xstyle={styles.canvas} />,
});

/** The pulsing border in the circle look. */
export const pulsingBorderCircle = defineExample({
    of: PulsingBorder,
    name: "circle",
    description: "pulsing border in the circle look",
    properties: {
        ...OBJECT_SIZING,
        aspectRatio: "square",
        scale: 0.6,
        speed: 1,
        frame: 0,
        colorBack: "#000000",
        colors: ["#0dc1fd", "#d915ef", "#ff3f2ecc"],
        roundness: 1,
        margin: 0,
        marginLeft: 0,
        marginRight: 0,
        marginTop: 0,
        marginBottom: 0,
        thickness: 0,
        softness: 0.75,
        intensity: 0.2,
        bloom: 0.45,
        spots: 3,
        spotSize: 0.4,
        pulse: 0.5,
        smoke: 1,
        smokeSize: 0,
    },
    render: (properties) => <PulsingBorder {...properties} xstyle={styles.canvas} />,
});

/** The pulsing border in the northern lights look. */
export const pulsingBorderNorthernLights = defineExample({
    of: PulsingBorder,
    name: "northern-lights",
    description: "pulsing border in the northern lights look",
    properties: {
        ...OBJECT_SIZING,
        speed: 0.18,
        scale: 1.1,
        frame: 0,
        colors: ["#4c4794", "#774a7d", "#12694a", "#0aff78", "#4733cc"],
        colorBack: "#0c182c",
        roundness: 0,
        thickness: 1,
        softness: 1,
        margin: 0,
        marginLeft: 0,
        marginRight: 0,
        marginTop: 0,
        marginBottom: 0,
        aspectRatio: "auto",
        intensity: 0.1,
        bloom: 0.2,
        spots: 4,
        spotSize: 0.25,
        pulse: 0,
        smoke: 0.32,
        smokeSize: 0.5,
    },
    render: (properties) => <PulsingBorder {...properties} xstyle={styles.canvas} />,
});

/** The pulsing border in the solid line look. */
export const pulsingBorderSolidLine = defineExample({
    of: PulsingBorder,
    name: "solid-line",
    description: "pulsing border in the solid line look",
    properties: {
        ...OBJECT_SIZING,
        speed: 1,
        frame: 0,
        colors: ["#81ADEC"],
        colorBack: "#00000000",
        roundness: 0,
        thickness: 0.05,
        margin: 0,
        marginLeft: 0,
        marginRight: 0,
        marginTop: 0,
        marginBottom: 0,
        aspectRatio: "auto",
        softness: 0.0,
        intensity: 0.0,
        bloom: 0.15,
        spots: 4,
        spotSize: 1,
        pulse: 0,
        smoke: 0,
        smokeSize: 0,
    },
    render: (properties) => <PulsingBorder {...properties} xstyle={styles.canvas} />,
});
