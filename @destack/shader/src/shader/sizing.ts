import { t } from "@destack/locale";
import { field } from "@destack/object";
import { schema } from "@destack/schema";
import type { DeclaredProperties } from "@destack/view/declare";
import type { ShaderUniforms } from "../mount/mount.ts";

/** How an effect's graphic fits its canvas, as a person picks one. */
const FITS = {
    none: {
        title: t.context("option")`None`,
        description: t`At its world size.`,
    },
    contain: {
        title: t.context("option")`Contain`,
        description: t`Inside the canvas.`,
    },
    cover: {
        title: t.context("option")`Cover`,
        description: t`Covering the canvas.`,
    },
};

/** How an effect's graphic fits its canvas: at world size, inside the canvas, or covering it. */
export type Fit = keyof typeof FITS;

/** The number the vertex shader reads for each fit. */
const FIT_CODES: Readonly<Record<Fit, number>> = { none: 0, contain: 1, cover: 2 };

/** Where an object's graphic sits in its canvas and how large it draws, contained in it. */
export const SIZING_PROPERTIES = {
    fit: field
        .enum(FITS)
        .labeled(t.context("label")`Fit`)
        .described(t`How the graphic fits its canvas.`)
        .default("contain"),
    scale: field
        .number(schema.number().min(0.01).max(10))
        .labeled(t.context("label")`Scale`)
        .described(t`The zoom of the graphic.`)
        .default(1),
    rotation: field
        .number(schema.number().min(0).max(360))
        .labeled(t.context("label")`Rotation`)
        .described(t`The rotation of the graphic, in degrees.`)
        .default(0),
    originX: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Horizontal origin`)
        .described(t`The point across the canvas the world size anchors to.`)
        .default(0.5),
    originY: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Vertical origin`)
        .described(t`The point down the canvas the world size anchors to.`)
        .default(0.5),
    offsetX: field
        .number(schema.number().min(-1).max(1))
        .labeled(t.context("label")`Horizontal offset`)
        .described(t`How far the graphic's center moves across.`)
        .default(0),
    offsetY: field
        .number(schema.number().min(-1).max(1))
        .labeled(t.context("label")`Vertical offset`)
        .described(t`How far the graphic's center moves down.`)
        .default(0),
    worldWidth: field
        .number(schema.number().min(0))
        .labeled(t.context("label")`World width`)
        .described(
            t`The width of the graphic before it fits the canvas, in pixels, the canvas's at zero.`,
        )
        .default(0),
    worldHeight: field
        .number(schema.number().min(0))
        .labeled(t.context("label")`World height`)
        .described(
            t`The height of the graphic before it fits the canvas, in pixels, the canvas's at zero.`,
        )
        .default(0),
};

/** Where a pattern's graphic sits in its canvas, drawn at its own size across it. */
export const PATTERN_SIZING_PROPERTIES = {
    ...SIZING_PROPERTIES,
    fit: SIZING_PROPERTIES.fit.default("none"),
};

/** Where an effect's graphic sits in its canvas and how large it draws. */
export type Sizing = DeclaredProperties<typeof SIZING_PROPERTIES>;

/** The names of the sizing options, which effects keep from the shader's element. */
export const SIZING_KEYS = [
    "fit",
    "scale",
    "rotation",
    "originX",
    "originY",
    "offsetX",
    "offsetY",
    "worldWidth",
    "worldHeight",
] as const satisfies readonly (keyof Sizing)[];

/** Write a sizing as the vertex shader's uniforms. */
export function sizingUniforms(sizing: Sizing): ShaderUniforms {
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
