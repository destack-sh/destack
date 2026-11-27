import { type Message, t } from "@destack/locale";
import { schema } from "@destack/schema";
import { Color } from "../color/index.ts";
import { BoxPoint, Gradient, type GradientJson } from "../gradient/index.ts";

/** The color a gradient from an image ends at, as design tools start. */
const WHITE = "#ffffff";

/** A layer drawn over the whole box, once. */
const WHOLE = { size: "100% 100%", position: "0% 0%", repeat: "no-repeat" } as const;

/** The CSS background size of each way an image fills a box. */
const IMAGE_SIZES = {
    cover: "cover",
    contain: "contain",
    stretch: "100% 100%",
    tile: "auto",
} satisfies Record<ImageFit, string>;

/** A kind of paint: one color, a gradient, or an image. */
export const PaintKind = schema.enum(["solid", "gradient", "image"]);
/** A kind of paint. */
export type PaintKind = schema.Infer<typeof PaintKind>;

/** A way an image fills a box, as CSS sizes and repeats a background image. */
export const ImageFit = schema.enum(["cover", "contain", "stretch", "tile"]);
/** A way an image fills a box. */
export type ImageFit = schema.Infer<typeof ImageFit>;

/** The kinds of paint, titled and described: one color, a gradient, or an image. */
export const PAINT_KINDS = {
    solid: { title: t.context("option")`Solid`, description: t`One color across the box.` },
    gradient: {
        title: t.context("option")`Gradient`,
        description: t`Colors blended across the box.`,
    },
    image: { title: t.context("option")`Image`, description: t`A picture from an address.` },
} satisfies Record<PaintKind, { readonly title: Message; readonly description: Message }>;

/** The ways an image fills a box, titled and described. */
export const IMAGE_FITS = {
    cover: {
        title: t.context("option")`Cover`,
        description: t`Fill the box, cropping the image where it overflows.`,
    },
    contain: {
        title: t.context("option")`Contain`,
        description: t`Show the whole image inside the box.`,
    },
    stretch: {
        title: t.context("option")`Stretch`,
        description: t`Stretch the image to the box's width and height.`,
    },
    tile: {
        title: t.context("option")`Tile`,
        description: t`Repeat the image at its own size.`,
    },
} satisfies Record<ImageFit, { readonly title: Message; readonly description: Message }>;

/** What paints a box: one color, a gradient, or an image. */
const paintJson = schema
    .discriminatedUnion("kind", [
        schema.object({
            /** One color. */
            kind: schema.literal("solid"),
            /** The color, as CSS writes it. */
            color: Color.schema,
        }),
        schema.object({
            /** A gradient. */
            kind: schema.literal("gradient"),
            /** The gradient. */
            gradient: Gradient.schema,
        }),
        schema.object({
            /** An image. */
            kind: schema.literal("image"),
            /** The image's address. */
            source: schema.string().min(1),
            /** How it fills the box. */
            fit: ImageFit,
            /** The place in the box it lines up with. */
            position: BoxPoint,
        }),
    ])
    .meta({ id: "paint", title: "Paint" });

/** What paints a box. */
export type Paint = schema.Infer<typeof paintJson>;

/** A paint as one CSS background layer: its image, size, position and repeat. */
export interface PaintLayer {
    /** The layer's image. */
    readonly image: string;
    /** The layer's size. */
    readonly size: string;
    /** The layer's position. */
    readonly position: string;
    /** How the layer repeats. */
    readonly repeat: string;
}

/** What paints a box, and how CSS draws it. */
export const Paint = Object.assign(paintJson, {
    /** Black, the paint design tools start a box and a new layer at. */
    black: { kind: "solid", color: "#000000" } as const,

    /** Read the color a paint turns into: its own, its first stop's, or black for an image. */
    color(paint: Paint): string {
        // take the color itself, the gradient's first along it, or black
        if (paint.kind === "solid") {
            return paint.color;
        } else if (paint.kind === "gradient") {
            return new Gradient(paint.gradient).first().color;
        } else {
            return Paint.black.color;
        }
    },

    /** Make an image paint of an address, covering the box from its center. */
    image(source: string): Paint & { readonly kind: "image" } {
        return { kind: "image", source, fit: "cover", position: BoxPoint.center };
    },

    /** Read the gradient a paint turns into: its own, its color fading out, or black to white. */
    gradient(paint: Paint): GradientJson {
        // keep a gradient, fade a color out to the right, and start an image at black to white
        if (paint.kind === "gradient") {
            return paint.gradient;
        } else if (paint.kind === "solid") {
            const clear = Color.parse(paint.color).with("alpha", 0).toString("hex");

            return Gradient.between(paint.color, clear).toJson();
        } else {
            return Gradient.between(Paint.black.color, WHITE).toJson();
        }
    },

    /** Draw a paint as one CSS background layer, a color as a one-color gradient so it stacks. */
    layer(paint: Paint): PaintLayer {
        // draw a color and a gradient across the whole box
        if (paint.kind === "solid") {
            return { image: `linear-gradient(${paint.color}, ${paint.color})`, ...WHOLE };
        } else if (paint.kind === "gradient") {
            return { image: new Gradient(paint.gradient).css(), ...WHOLE };
        }

        // draw an image at its place, sized and repeated by its fit
        return {
            image: `url(${JSON.stringify(paint.source)})`,
            size: IMAGE_SIZES[paint.fit],
            position: BoxPoint.css(paint.position),
            repeat: paint.fit === "tile" ? "repeat" : "no-repeat",
        };
    },
});
