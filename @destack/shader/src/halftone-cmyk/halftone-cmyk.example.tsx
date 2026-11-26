import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { SAMPLE_IMAGE } from "../shader/sample.ts";
import { HalftoneCmyk } from "./halftone-cmyk.tsx";

/** The size every example draws at. */
const styles = style.create({
    canvas: { width: "100%", height: "20rem" },
});

/** The CMYK halftone in the default look. */
export const halftoneCmykDefault = defineExample({
    of: HalftoneCmyk,
    name: "default",
    description: t`CMYK halftone in the default look.`,
    render: (properties) => (
        <HalftoneCmyk {...properties} image={SAMPLE_IMAGE} xstyle={styles.canvas} />
    ),
});

/** The CMYK halftone in the drops look. */
export const halftoneCmykDrops = defineExample({
    of: HalftoneCmyk,
    name: "drops",
    description: t`CMYK halftone in the drops look.`,
    properties: {
        scale: 1,
        fit: "cover",
        speed: 0,
        frame: 0,
        colorBack: "#eeefd7",
        colorC: "#00b2ff",
        colorM: "#fc4f4f",
        colorY: "#ffd900",
        colorK: "#231f20",
        size: 0.88,
        contrast: 1.15,
        softness: 0,
        grainSize: 0.01,
        grainMixer: 0.05,
        grainOverlay: 0.25,
        gridNoise: 0.5,
        floodC: 0.15,
        floodM: 0,
        floodY: 0,
        floodK: 0,
        gainC: 1.0,
        gainM: 0.44,
        gainY: -1.0,
        gainK: 0,
        type: "ink",
    },
    render: (properties) => (
        <HalftoneCmyk {...properties} image={SAMPLE_IMAGE} xstyle={styles.canvas} />
    ),
});

/** The CMYK halftone in the newspaper look. */
export const halftoneCmykNewspaper = defineExample({
    of: HalftoneCmyk,
    name: "newspaper",
    description: t`CMYK halftone in the newspaper look.`,
    properties: {
        scale: 1,
        fit: "cover",
        speed: 0,
        frame: 0,
        colorBack: "#f2f1e8",
        colorC: "#7a7a75",
        colorM: "#7a7a75",
        colorY: "#7a7a75",
        colorK: "#231f20",
        size: 0.01,
        contrast: 2,
        softness: 0.2,
        grainSize: 0,
        grainMixer: 0,
        grainOverlay: 0.2,
        gridNoise: 0.6,
        floodC: 0,
        floodM: 0,
        floodY: 0,
        floodK: 0.1,
        gainC: -0.17,
        gainM: -0.45,
        gainY: -0.45,
        gainK: 0,
        type: "dots",
    },
    render: (properties) => (
        <HalftoneCmyk {...properties} image={SAMPLE_IMAGE} xstyle={styles.canvas} />
    ),
});

/** The CMYK halftone in the vintage look. */
export const halftoneCmykVintage = defineExample({
    of: HalftoneCmyk,
    name: "vintage",
    description: t`CMYK halftone in the vintage look.`,
    properties: {
        scale: 1,
        fit: "cover",
        speed: 0,
        frame: 0,
        colorBack: "#fffaf0",
        colorC: "#59afc5",
        colorM: "#d8697c",
        colorY: "#fad85c",
        colorK: "#2d2824",
        size: 0.2,
        contrast: 1.25,
        softness: 0.4,
        grainSize: 0.5,
        grainMixer: 0.15,
        grainOverlay: 0.1,
        gridNoise: 0.45,
        floodC: 0.15,
        floodM: 0,
        floodY: 0,
        floodK: 0,
        gainC: 0.3,
        gainM: 0,
        gainY: 0.2,
        gainK: 0,
        type: "sharp",
    },
    render: (properties) => (
        <HalftoneCmyk {...properties} image={SAMPLE_IMAGE} xstyle={styles.canvas} />
    ),
});
