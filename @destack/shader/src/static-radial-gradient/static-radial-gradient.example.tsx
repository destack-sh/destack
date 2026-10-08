import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { OBJECT_SIZING } from "../shader/sizing.ts";

import { STATIC_RADIAL_GRADIENT_DEFAULTS } from "./effect.ts";
import { StaticRadialGradient } from "./static-radial-gradient.tsx";

/** The size every example draws at. */
const styles = style.create({
    canvas: { width: "100%", height: "20rem" },
});

/** The static radial gradient in the default look. */
export const staticRadialGradientDefault = defineExample({
    of: StaticRadialGradient,
    name: "default",
    description: "static radial gradient in the default look",
    properties: STATIC_RADIAL_GRADIENT_DEFAULTS,
    render: (properties) => <StaticRadialGradient {...properties} xstyle={styles.canvas} />,
});

/** The static radial gradient in the cross section look. */
export const staticRadialGradientCrossSection = defineExample({
    of: StaticRadialGradient,
    name: "cross-section",
    description: "static radial gradient in the cross section look",
    properties: {
        ...OBJECT_SIZING,
        scale: 1,
        speed: 0,
        frame: 0,
        colorBack: "#3d348b",
        colors: ["#7678ed", "#f7b801", "#f18701", "#37a066"],
        radius: 1,
        focalDistance: 0,
        focalAngle: 0,
        falloff: 0,
        mixing: 0,
        distortion: 1,
        distortionShift: 0,
        distortionFreq: 12,
        grainMixer: 0,
        grainOverlay: 0,
    },
    render: (properties) => <StaticRadialGradient {...properties} xstyle={styles.canvas} />,
});

/** The static radial gradient in the radial look. */
export const staticRadialGradientRadial = defineExample({
    of: StaticRadialGradient,
    name: "radial",
    description: "static radial gradient in the radial look",
    properties: {
        ...OBJECT_SIZING,
        scale: 1,
        speed: 0,
        frame: 0,
        colorBack: "#264653",
        colors: ["#9c2b2b", "#f4a261", "#ffffff"],
        radius: 1,
        focalDistance: 0,
        focalAngle: 0,
        falloff: 0,
        mixing: 1,
        distortion: 0,
        distortionShift: 0,
        distortionFreq: 12,
        grainMixer: 0,
        grainOverlay: 0,
    },
    render: (properties) => <StaticRadialGradient {...properties} xstyle={styles.canvas} />,
});

/** The static radial gradient in the lo fi look. */
export const staticRadialGradientLoFi = defineExample({
    of: StaticRadialGradient,
    name: "lo-fi",
    description: "static radial gradient in the lo fi look",
    properties: {
        ...OBJECT_SIZING,
        speed: 0,
        frame: 0,
        colorBack: "#2e1f27",
        colors: ["#d72638", "#3f88c5", "#f49d37"],
        radius: 1,
        focalDistance: 0,
        focalAngle: 0,
        falloff: 0.9,
        mixing: 0.7,
        distortion: 0,
        distortionShift: 0,
        distortionFreq: 12,
        grainMixer: 1,
        grainOverlay: 0.5,
    },
    render: (properties) => <StaticRadialGradient {...properties} xstyle={styles.canvas} />,
});
