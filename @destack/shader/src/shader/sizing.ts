import type { ShaderUniforms } from "../mount/mount.ts";

/** How an effect's graphic fits its canvas: at its world size, inside the canvas, or covering it. */
export type Fit = "none" | "contain" | "cover";

/** The number the vertex shader reads for each fit. */
const FIT_CODES: Readonly<Record<Fit, number>> = { none: 0, contain: 1, cover: 2 };

/** Where an effect's graphic sits in its canvas and how large it draws. */
export interface Sizing {
    /** How the graphic fits the canvas. */
    readonly fit?: Fit;
    /** The zoom of the graphic, from 0.01 to 4. */
    readonly scale?: number;
    /** The rotation of the graphic, in degrees from 0 to 360. */
    readonly rotation?: number;
    /** The horizontal point of the canvas the world size anchors to, from 0 to 1. */
    readonly originX?: number;
    /** The vertical point of the canvas the world size anchors to, from 0 to 1. */
    readonly originY?: number;
    /** The horizontal shift of the graphic's center, from -1 to 1. */
    readonly offsetX?: number;
    /** The vertical shift of the graphic's center, from -1 to 1. */
    readonly offsetY?: number;
    /** The width of the graphic before it fits the canvas, in CSS pixels, 0 for the canvas's own. */
    readonly worldWidth?: number;
    /** The height of the graphic before it fits the canvas, in CSS pixels, 0 for the canvas's own. */
    readonly worldHeight?: number;
}

/** The names of the sizing options, which effects keep from the shader's element. */
export const SIZING_KEYS: readonly (keyof Sizing)[] = [
    "fit",
    "scale",
    "rotation",
    "originX",
    "originY",
    "offsetX",
    "offsetY",
    "worldWidth",
    "worldHeight",
];

/** The sizing of an object, such as a shape, contained in its canvas. */
export const OBJECT_SIZING: Required<Sizing> = {
    fit: "contain",
    scale: 1,
    rotation: 0,
    originX: 0.5,
    originY: 0.5,
    offsetX: 0,
    offsetY: 0,
    worldWidth: 0,
    worldHeight: 0,
};

/** The sizing of a pattern, such as a texture, drawn at its own size across its canvas. */
export const PATTERN_SIZING: Required<Sizing> = { ...OBJECT_SIZING, fit: "none" };

/** Write a sizing as the vertex shader's uniforms. */
export function sizingUniforms(sizing: Required<Sizing>): ShaderUniforms {
    return {
        u_fit: FIT_CODES[sizing.fit],
        u_scale: sizing.scale,
        u_rotation: sizing.rotation,
        u_originX: sizing.originX,
        u_originY: sizing.originY,
        u_offsetX: sizing.offsetX,
        u_offsetY: sizing.offsetY,
        u_worldWidth: sizing.worldWidth,
        u_worldHeight: sizing.worldHeight,
    };
}
