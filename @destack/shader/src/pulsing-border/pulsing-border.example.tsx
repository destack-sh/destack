import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";

import { PulsingBorder } from "./pulsing-border.tsx";

/** The size every example draws at. */
const styles = style.create({
    canvas: { width: "100%", height: "20rem" },
});

/** The pulsing border in the default look. */
export const pulsingBorderDefault = defineExample({
    of: PulsingBorder,
    name: "default",
    description: t`Pulsing border in the default look.`,
    render: (properties) => <PulsingBorder {...properties} xstyle={styles.canvas} />,
});

/** The pulsing border in the circle look. */
export const pulsingBorderCircle = defineExample({
    of: PulsingBorder,
    name: "circle",
    description: t`Pulsing border in the circle look.`,
    properties: {
        aspectRatio: "square",
        scale: 0.6,
        speed: 1,
        frame: 0,
        colorBack: "#000000",
        colors: ["#0dc1fd", "#d915ef", "#ff3f2ecc"],
        roundness: 1,
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
    name: "northernLights",
    description: t`Pulsing border in the northern lights look.`,
    properties: {
        speed: 0.18,
        scale: 1.1,
        frame: 0,
        colors: ["#4c4794", "#774a7d", "#12694a", "#0aff78", "#4733cc"],
        colorBack: "#0c182c",
        roundness: 0,
        thickness: 1,
        softness: 1,
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
    name: "solidLine",
    description: t`Pulsing border in the solid line look.`,
    properties: {
        speed: 1,
        frame: 0,
        colors: ["#81ADEC"],
        colorBack: "#00000000",
        roundness: 0,
        thickness: 0.05,
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
