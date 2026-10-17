import { schema } from "@destack/schema";

/** A raster image format, by the name sharp and Astro give it. */
export const ImageFormat = schema.enum(["avif", "webp", "png", "jpeg", "gif"]);
/** A raster image format. */
export type ImageFormat = schema.Infer<typeof ImageFormat>;

/** An image a build generated, after Astro's `ImageMetadata`. */
export const ImageMetadata = schema.object({
    /** The original image's address. */
    src: schema.string().min(1),
    /** The intrinsic width in pixels. */
    width: schema.number().int().positive(),
    /** The intrinsic height in pixels. */
    height: schema.number().int().positive(),
    /** The original's format. */
    format: ImageFormat,
    /** The generated files, each one width in one format, the original among them. */
    srcSet: schema.array(
        schema.object({
            /** The file's address. */
            src: schema.string().min(1),
            /** The file's width in pixels. */
            width: schema.number().int().positive(),
            /** The file's format. */
            format: ImageFormat,
        }),
    ),
    /** A tiny blurred rendition as a data URL, absent when the build made none. */
    blurDataURL: schema.string().startsWith("data:").exactOptional(),
});
/** An image a build generated. */
export type ImageMetadata = schema.Infer<typeof ImageMetadata>;

/** An image to show: a build's image, or an address with its dimensions given beside it. */
export type ImageSource = ImageMetadata | string;
