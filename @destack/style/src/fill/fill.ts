import { type Message, t } from "@destack/locale";
import { schema } from "@destack/schema";
import { Paint } from "../paint/index.ts";

/** A way a layer blends with those below it, as CSS's blend modes name it. */
export const BlendMode = schema.enum([
    "normal",
    "multiply",
    "screen",
    "overlay",
    "darken",
    "lighten",
    "color-dodge",
    "color-burn",
    "hard-light",
    "soft-light",
    "difference",
    "exclusion",
    "hue",
    "saturation",
    "color",
    "luminosity",
]);
/** A way a layer blends with those below it. */
export type BlendMode = schema.Infer<typeof BlendMode>;

/** The ways a layer blends with those below it, titled. */
export const BLEND_MODES = {
    normal: { title: t.context("option")`Normal` },
    multiply: { title: t.context("option")`Multiply` },
    screen: { title: t.context("option")`Screen` },
    overlay: { title: t.context("option")`Overlay` },
    darken: { title: t.context("option")`Darken` },
    lighten: { title: t.context("option")`Lighten` },
    "color-dodge": { title: t.context("option")`Color dodge` },
    "color-burn": { title: t.context("option")`Color burn` },
    "hard-light": { title: t.context("option")`Hard light` },
    "soft-light": { title: t.context("option")`Soft light` },
    difference: { title: t.context("option")`Difference` },
    exclusion: { title: t.context("option")`Exclusion` },
    hue: { title: t.context("option")`Hue` },
    saturation: { title: t.context("option")`Saturation` },
    color: { title: t.context("option")`Color` },
    luminosity: { title: t.context("option")`Luminosity` },
} satisfies Record<BlendMode, { readonly title: Message }>;

/** One layer of a box's fill: its paint, how it blends, and whether it shows. */
export const Fill = schema.object({
    /** What paints the layer. */
    paint: Paint,
    /** How it blends with the layers below it. */
    blend: BlendMode,
    /** Whether it shows, a hidden layer kept while a person works on the others. */
    isVisible: schema.boolean(),
});
/** One layer of a box's fill. */
export type Fill = schema.Infer<typeof Fill>;

/** A box's fill as CSS background properties. */
export interface FillStyle {
    /** The layers' images, top first. */
    readonly backgroundImage: string;
    /** The layers' sizes. */
    readonly backgroundSize: string;
    /** The layers' positions. */
    readonly backgroundPosition: string;
    /** How the layers repeat. */
    readonly backgroundRepeat: string;
    /** How the layers blend. */
    readonly backgroundBlendMode: string;
}

/** The layers filling a box, top first, as CSS lists background layers. */
export const FillStack = Object.assign(
    schema.array(Fill).meta({ id: "fillStack", title: "Fill" }),
    {
        /** Draw the shown layers as CSS background properties, none for no layer. */
        css(stack: readonly Fill[]): FillStyle {
            // draw each shown layer, top first
            const shown = stack.filter((fill) => fill.isVisible);
            const layers = shown.map((fill) => Paint.layer(fill.paint));

            return {
                backgroundImage:
                    layers.length === 0 ? "none" : layers.map((layer) => layer.image).join(", "),
                backgroundSize: layers.map((layer) => layer.size).join(", "),
                backgroundPosition: layers.map((layer) => layer.position).join(", "),
                backgroundRepeat: layers.map((layer) => layer.repeat).join(", "),
                backgroundBlendMode: shown.map((fill) => fill.blend).join(", "),
            };
        },
    },
);
