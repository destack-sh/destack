import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { PATTERN_SIZING } from "../shader/sizing.ts";

import { DOT_GRID_DEFAULTS } from "./effect.ts";
import { DotGrid } from "./dot-grid.tsx";

/** The size every example draws at. */
const styles = style.create({
    canvas: { width: "100%", height: "20rem" },
});

/** The dot grid in the default look. */
export const dotGridDefault = defineExample({
    of: DotGrid,
    name: "default",
    description: "dot grid in the default look",
    properties: DOT_GRID_DEFAULTS,
    render: (properties) => <DotGrid {...properties} xstyle={styles.canvas} />,
});

/** The dot grid in the triangles look. */
export const dotGridTriangles = defineExample({
    of: DotGrid,
    name: "triangles",
    description: "dot grid in the triangles look",
    properties: {
        ...PATTERN_SIZING,
        colorBack: "#ffffff",
        colorFill: "#ffffff",
        colorStroke: "#808080",
        size: 5,
        gapX: 32,
        gapY: 32,
        strokeWidth: 1,
        sizeRange: 0,
        opacityRange: 0,
        shape: "triangle",
    },
    render: (properties) => <DotGrid {...properties} xstyle={styles.canvas} />,
});

/** The dot grid in the tree line look. */
export const dotGridTreeLine = defineExample({
    of: DotGrid,
    name: "tree-line",
    description: "dot grid in the tree line look",
    properties: {
        ...PATTERN_SIZING,
        colorBack: "#f4fce7",
        colorFill: "#052e19",
        colorStroke: "#000000",
        size: 8,
        gapX: 20,
        gapY: 90,
        strokeWidth: 0,
        sizeRange: 1,
        opacityRange: 0.6,
        shape: "circle",
    },
    render: (properties) => <DotGrid {...properties} xstyle={styles.canvas} />,
});

/** The dot grid in the wallpaper look. */
export const dotGridWallpaper = defineExample({
    of: DotGrid,
    name: "wallpaper",
    description: "dot grid in the wallpaper look",
    properties: {
        ...PATTERN_SIZING,
        colorBack: "#204030",
        colorFill: "#000000",
        colorStroke: "#bd955b",
        size: 9,
        gapX: 32,
        gapY: 32,
        strokeWidth: 1,
        sizeRange: 0,
        opacityRange: 0,
        shape: "diamond",
    },
    render: (properties) => <DotGrid {...properties} xstyle={styles.canvas} />,
});
