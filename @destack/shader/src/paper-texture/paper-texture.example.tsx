import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { OBJECT_SIZING } from "../shader/sizing.ts";
import { SAMPLE_IMAGE } from "../shader/sample.ts";
import { PAPER_TEXTURE_DEFAULTS } from "./effect.ts";
import { PaperTexture } from "./paper-texture.tsx";

/** The size every example draws at. */
const styles = style.create({
    canvas: { width: "100%", height: "20rem" },
});

/** The paper texture in the default look. */
export const paperTextureDefault = defineExample({
    of: PaperTexture,
    name: "default",
    description: "paper texture in the default look",
    properties: { ...PAPER_TEXTURE_DEFAULTS, image: SAMPLE_IMAGE },
    render: (properties) => <PaperTexture {...properties} xstyle={styles.canvas} />,
});

/** The paper texture in the creased look. */
export const paperTextureCreased = defineExample({
    of: PaperTexture,
    name: "creased",
    description: "paper texture in the creased look",
    properties: {
        ...OBJECT_SIZING,
        fit: "contain",
        scale: 0.9,
        speed: 0,
        frame: 0,
        colorBack: "#d3d2ab",
        colorPaper: "#ffffff",
        colorShadow: "#b3b3b3",
        blending: 1,
        distortion: -0.5,
        clip: false,
        angle: 60,
        seed: 49,
        roughness: 0.4,
        roughnessSize: 0.25,
        roughnessRows: 0,
        fiber: 0.4,
        fiberSize: 0.5,
        folds: 0,
        foldSizeX: 0.6,
        foldSizeY: 0.89,
        foldOffsetX: 0.59,
        foldOffsetY: 1,
        wrinkles: 0,
        wrinkleSize: 0.65,
        crumples: 1,
        crumpleCount: 4,
        drops: 0,
        image: SAMPLE_IMAGE,
    },
    render: (properties) => <PaperTexture {...properties} xstyle={styles.canvas} />,
});

/** The paper texture in the spread look. */
export const paperTextureSpread = defineExample({
    of: PaperTexture,
    name: "spread",
    description: "paper texture in the spread look",
    properties: {
        ...OBJECT_SIZING,
        fit: "contain",
        scale: 0.9,
        speed: 0,
        frame: 0,
        colorBack: "#ffffff00",
        colorPaper: "#ffffffb5",
        colorShadow: "#b3b3b3",
        blending: 1,
        distortion: -0.3,
        clip: true,
        angle: 236,
        seed: 613,
        roughness: 0.15,
        roughnessSize: 0.25,
        roughnessRows: 0,
        fiber: 0.5,
        fiberSize: 0.5,
        folds: 1,
        foldSizeX: 0.4,
        foldSizeY: 0.89,
        foldOffsetX: 0,
        foldOffsetY: 0,
        wrinkles: 0,
        wrinkleSize: 0.65,
        crumples: 0,
        crumpleCount: 9,
        drops: 0,
        image: SAMPLE_IMAGE,
    },
    render: (properties) => <PaperTexture {...properties} xstyle={styles.canvas} />,
});

/** The paper texture in the flat look. */
export const paperTextureFlat = defineExample({
    of: PaperTexture,
    name: "flat",
    description: "paper texture in the flat look",
    properties: {
        ...OBJECT_SIZING,
        fit: "contain",
        scale: 0.9,
        speed: 0,
        frame: 0,
        colorBack: "#d4cdab",
        colorPaper: "#ffffffa8",
        colorShadow: "#b3b3b3",
        blending: 1,
        distortion: 0,
        clip: false,
        angle: 0,
        seed: 455,
        roughness: 1,
        roughnessSize: 0.5,
        roughnessRows: 0.6,
        fiber: 0.7,
        fiberSize: 1,
        folds: 0,
        foldSizeX: 0,
        foldSizeY: 0.44,
        foldOffsetX: 0,
        foldOffsetY: 0,
        wrinkles: 0,
        wrinkleSize: 0,
        crumples: 0,
        crumpleCount: 6,
        drops: 0.2,
        image: SAMPLE_IMAGE,
    },
    render: (properties) => <PaperTexture {...properties} xstyle={styles.canvas} />,
});
