import { readFile } from "node:fs/promises";
import { basename, extname } from "node:path";
import sharp from "sharp";
import type { Plugin } from "@destack/package/build";

/** The widths a build renders images at, common device widths up to each image's own. */
const WIDTHS = [640, 828, 1080, 1920, 2560, 3840];

/** The formats a build renders beside each image's own, best first. */
const FORMATS = ["avif", "webp"] as const;

/** The width of an image's blurred rendition. */
const PLACEHOLDER_WIDTH = 16;

/** The WebP quality of an image's blurred rendition, which its blur hides. */
const PLACEHOLDER_QUALITY = 40;

/** The raster images an import turns into assets, by extension. */
const RASTER = /\.(?:png|jpe?g|webp|avif|gif)$/u;

/** The media type of each raster format sharp reads. */
const TYPES: Readonly<Record<string, string>> = {
    png: "image/png",
    jpeg: "image/jpeg",
    webp: "image/webp",
    avif: "image/avif",
    gif: "image/gif",
};

/** One rendered variant of an image: its width, its media type and its bytes. */
export interface ImageVariant {
    /** The width in pixels. */
    readonly width: number;
    /** The media type. */
    readonly type: string;
    /** The encoded bytes. */
    readonly bytes: Uint8Array;
}

/** An image's intrinsic size and type, its variants and its blurred rendition. */
export interface ProcessedImage {
    /** The intrinsic width in pixels. */
    readonly width: number;
    /** The intrinsic height in pixels. */
    readonly height: number;
    /** The original's media type. */
    readonly type: string;
    /** The variants, each width in each format. */
    readonly variants: readonly ImageVariant[];
    /** The blurred rendition as a data URL. */
    readonly placeholder: string;
}

/** Read an image's size and blurred rendition, and render its variants at the widths up to its own unless asked not to. */
export async function processImage(
    bytes: Uint8Array,
    options: { readonly variants?: boolean } = {},
): Promise<ProcessedImage> {
    // read the intrinsic size and format
    const metadata = await sharp(bytes).metadata();
    const format = metadata.format;
    const type = TYPES[format];
    if (metadata.width === undefined || metadata.height === undefined || type === undefined) {
        throw new TypeError(`image of format ${format} has no raster size`);
    }

    // render each width up to the image's own in the better formats and its own
    const widths =
        options.variants === false
            ? []
            : [...WIDTHS.filter((width) => width < metadata.width), metadata.width];
    const variants = await Promise.all(
        widths.flatMap((width) =>
            [...FORMATS, format].map(async (target) => ({
                width,
                type: TYPES[target] ?? type,
                bytes: new Uint8Array(
                    await sharp(bytes).resize({ width }).toFormat(target).toBuffer(),
                ),
            })),
        ),
    );

    // render the blurred rendition as an inline WebP
    const placeholder = await sharp(bytes)
        .resize({ width: PLACEHOLDER_WIDTH })
        .blur()
        .webp({ quality: PLACEHOLDER_QUALITY })
        .toBuffer();

    return {
        width: metadata.width,
        height: metadata.height,
        type,
        variants,
        placeholder: `data:image/webp;base64,${placeholder.toString("base64")}`,
    };
}

/** Turn raster image imports into image assets: their variants emitted by a build, their original while serving. */
export function imagePlugin(): Plugin {
    // remember whether the plugin builds or serves
    let isBuild = true;

    return {
        name: "destack-image",
        enforce: "pre",
        configResolved(configuration) {
            isBuild = configuration.command === "build";
        },
        async load(id) {
            // leave other modules, and images imported with a query such as ?url
            if (!RASTER.test(id)) {
                return null;
            }

            // describe the image, its variants emitted as hashed assets when building
            // NOTE #Performance: keep rendered variants in the build's content-addressed cache by the image's digest
            const bytes = new Uint8Array(await readFile(id));
            const image = await processImage(bytes, { variants: isBuild });
            const stem = basename(id, extname(id));
            const reference = (name: string, source: Uint8Array) =>
                `import.meta.ROLLUP_FILE_URL_${this.emitFile({ type: "asset", name, source })}`;
            const original = isBuild ? reference(basename(id), bytes) : JSON.stringify(`/@fs${id}`);
            const variants = isBuild
                ? image.variants.map((variant) => {
                      // reuse the original for its own format at its own width
                      const address =
                          variant.width === image.width && variant.type === image.type
                              ? original
                              : reference(
                                    `${stem}-${variant.width}.${variant.type.slice("image/".length)}`,
                                    variant.bytes,
                                );

                      return `{ src: ${address}, width: ${variant.width}, type: ${JSON.stringify(variant.type)} }`;
                  })
                : [];
            const asset: Omit<ProcessedImage, "variants"> = {
                width: image.width,
                height: image.height,
                type: image.type,
                placeholder: image.placeholder,
            };

            return `export default { ...${JSON.stringify(asset)}, src: ${original}, variants: [${variants.join(", ")}] };\n`;
        },
    };
}
