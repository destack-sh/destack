import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { SAMPLE_IMAGE } from "../shader/sample.ts";
import { FlutedGlass } from "./fluted-glass.tsx";

/** The size every example draws at. */
const styles = style.create({
    canvas: { width: "100%", height: "20rem" },
});

/** The fluted glass in the default look. */
export const flutedGlassDefault = defineExample({
    of: FlutedGlass,
    name: "default",
    description: t`Fluted glass in the default look.`,
    render: (properties) => (
        <FlutedGlass {...properties} image={SAMPLE_IMAGE} xstyle={styles.canvas} />
    ),
});

/** The fluted glass in the waves look. */
export const flutedGlassWaves = defineExample({
    of: FlutedGlass,
    name: "waves",
    description: t`Fluted glass in the waves look.`,
    properties: {
        fit: "cover",
        scale: 1.2,
        speed: 0,
        frame: 0,
        colorBack: "#00000000",
        colorShadow: "#000000",
        colorHighlight: "#ffffff",
        shadows: 0,
        size: 0.9,
        angle: 0,
        distortionShape: "contour",
        highlights: 0,
        shape: "wave",
        distortion: 0.5,
        shift: 0,
        blur: 0.1,
        edges: 0.5,
        stretch: 1,
        marginLeft: 0,
        marginRight: 0,
        marginTop: 0,
        marginBottom: 0,
        grainMixer: 0,
        grainOverlay: 0.05,
    },
    render: (properties) => (
        <FlutedGlass {...properties} image={SAMPLE_IMAGE} xstyle={styles.canvas} />
    ),
});

/** The fluted glass in the abstract look. */
export const flutedGlassAbstract = defineExample({
    of: FlutedGlass,
    name: "abstract",
    description: t`Fluted glass in the abstract look.`,
    properties: {
        fit: "cover",
        scale: 4,
        speed: 0,
        frame: 0,
        colorBack: "#00000000",
        colorShadow: "#000000",
        colorHighlight: "#ffffff",
        shadows: 0,
        size: 0.7,
        angle: 30,
        distortionShape: "flat",
        highlights: 0,
        shape: "linesIrregular",
        distortion: 1,
        shift: 0,
        blur: 1,
        edges: 0.5,
        stretch: 1,
        marginLeft: 0,
        marginRight: 0,
        marginTop: 0,
        marginBottom: 0,
        grainMixer: 0.1,
        grainOverlay: 0.1,
    },
    render: (properties) => (
        <FlutedGlass {...properties} image={SAMPLE_IMAGE} xstyle={styles.canvas} />
    ),
});

/** The fluted glass in the folds look. */
export const flutedGlassFolds = defineExample({
    of: FlutedGlass,
    name: "folds",
    description: t`Fluted glass in the folds look.`,
    properties: {
        fit: "cover",
        speed: 0,
        frame: 0,
        colorBack: "#00000000",
        colorShadow: "#000000",
        colorHighlight: "#ffffff",
        shadows: 0.4,
        size: 0.4,
        angle: 0,
        distortionShape: "cascade",
        highlights: 0,
        shape: "lines",
        distortion: 0.75,
        shift: 0,
        blur: 0.25,
        edges: 0.5,
        stretch: 0,
        marginLeft: 0.1,
        marginRight: 0.1,
        marginTop: 0.1,
        marginBottom: 0.1,
        grainMixer: 0,
        grainOverlay: 0,
    },
    render: (properties) => (
        <FlutedGlass {...properties} image={SAMPLE_IMAGE} xstyle={styles.canvas} />
    ),
});
