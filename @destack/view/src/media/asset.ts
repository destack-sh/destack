import { defineSchema, schema } from "@destack/schema";

/** An image a build generated: its original, its variants by width and format, and a blurred placeholder. */
export const ImageAsset = defineSchema(
    schema.object({
        /** The original image's address. */
        src: schema.string().min(1),
        /** The intrinsic width in pixels. */
        width: schema.number().int().positive(),
        /** The intrinsic height in pixels. */
        height: schema.number().int().positive(),
        /** The original's media type, such as `image/png`. */
        type: schema.string().min(1),
        /** The generated variants, each one width in one format. */
        variants: schema.array(
            schema.object({
                /** The variant's address. */
                src: schema.string().min(1),
                /** The variant's width in pixels. */
                width: schema.number().int().positive(),
                /** The variant's media type, such as `image/avif`. */
                type: schema.string().min(1),
            }),
        ),
        /** A tiny blurred rendition as a data URL, absent when the build made none. */
        placeholder: schema.string().startsWith("data:").exactOptional(),
    }),
);
/** An image a build generated. */
export type ImageAsset = schema.Infer<typeof ImageAsset>;

/** An image to show: a build's asset, or an address with its dimensions given beside it. */
export type ImageSource = ImageAsset | string;
