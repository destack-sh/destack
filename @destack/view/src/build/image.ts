import { readFile } from "node:fs/promises";
import { basename, extname } from "node:path";
import sharp from "sharp";
import type { Plugin } from "@destack/package/build";
import { ImageFormat } from "../media/asset.ts";

/** The widths a build renders images at by default, Next.js's device sizes. */
export const IMAGE_WIDTHS: readonly number[] = [640, 750, 828, 1080, 1200, 1920, 2048, 3840];

/** The formats a build renders beside each image's own by default, best first. */
export const IMAGE_FORMATS: readonly ImageFormat[] = ["avif", "webp"];

/** The width of an image's blurred rendition. */
const BLUR_WIDTH = 16;

/** The WebP quality of an image's blurred rendition, which its blur hides. */
const BLUR_QUALITY = 40;

/** The raster images an import turns into image metadata, by extension. */
const RASTER = /\.(?:png|jpe?g|webp|avif|gif)$/u;

/** The widths and formats an image renders at, Next.js's device sizes and AVIF and WebP by default. */
export interface ImageOptions {
    /** The widths to render at, each below the image's own, beside its own. */
    readonly widths?: readonly number[];
    /** The formats to render in beside the image's own. */
    readonly formats?: readonly ImageFormat[];
}

/** One file of an image's source set: its width and format. */
export interface ImageFile {
    /** The width in pixels. */
    readonly width: number;
    /** The format. */
    readonly format: ImageFormat;
}

/** An image rendered for a build: its size and format, its blurred rendition, and each file of its source set with its bytes. */
export interface TransformedImage {
    /** The intrinsic width in pixels. */
    readonly width: number;
    /** The intrinsic height in pixels. */
    readonly height: number;
    /** The original's format. */
    readonly format: ImageFormat;
    /** The blurred rendition as a data URL. */
    readonly blurDataURL: string;
    /** The rendered files, the original's own bytes at its own width and format. */
    readonly files: readonly (ImageFile & { readonly data: Uint8Array })[];
}

/** Plan an image's files: each width below its own and its own, in the formats and its own. */
export function imageFiles(
    width: number,
    format: ImageFormat,
    options: ImageOptions = {},
): readonly ImageFile[] {
    const widths = [...(options.widths ?? IMAGE_WIDTHS).filter((entry) => entry < width), width];
    const formats = [...new Set([...(options.formats ?? IMAGE_FORMATS), format])];

    return widths.flatMap((entry) => formats.map((each) => ({ width: entry, format: each })));
}

/** Read an image's size and format, and render its blurred rendition and the source set the options plan. */
export async function transformImage(
    bytes: Uint8Array,
    options: ImageOptions = {},
): Promise<TransformedImage> {
    // read the intrinsic size and format
    const metadata = await sharp(bytes).metadata();
    const format = ImageFormat.safeParse(metadata.format);
    if (metadata.width === undefined || metadata.height === undefined || !format.success) {
        throw new TypeError(`image of format ${metadata.format} has no raster size`);
    }

    // render each planned file, keeping the original's own bytes at its own width and format
    const files = await Promise.all(
        imageFiles(metadata.width, format.data, options).map(async (file) => ({
            ...file,
            data:
                file.width === metadata.width && file.format === format.data
                    ? bytes
                    : new Uint8Array(
                          await sharp(bytes)
                              .resize({ width: file.width })
                              .toFormat(file.format)
                              .toBuffer(),
                      ),
        })),
    );

    // render the blurred rendition as an inline WebP
    const blur = await sharp(bytes)
        .resize({ width: BLUR_WIDTH })
        .blur()
        .webp({ quality: BLUR_QUALITY })
        .toBuffer();

    return {
        width: metadata.width,
        height: metadata.height,
        format: format.data,
        blurDataURL: `data:image/webp;base64,${blur.toString("base64")}`,
        files,
    };
}

/** Turn raster image imports into image metadata: their source sets emitted by a build, their original while serving. */
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

            // describe the image, its source set emitted as hashed assets when building
            // NOTE #Performance: keep rendered files in the build's content-addressed cache by the image's digest
            const bytes = new Uint8Array(await readFile(id));
            const image = await transformImage(bytes, isBuild ? {} : { widths: [], formats: [] });
            const stem = basename(id, extname(id));
            const reference = (name: string, source: Uint8Array) =>
                `import.meta.ROLLUP_FILE_URL_${this.emitFile({ type: "asset", name, source })}`;
            const original = isBuild ? reference(basename(id), bytes) : JSON.stringify(`/@fs${id}`);
            const entries = image.files.map((file) => {
                // serve the original for its own format at its own width
                const address =
                    file.width === image.width && file.format === image.format
                        ? original
                        : reference(`${stem}-${file.width}.${file.format}`, file.data);

                return `{ src: ${address}, width: ${file.width}, format: ${JSON.stringify(file.format)} }`;
            });
            const metadata = {
                width: image.width,
                height: image.height,
                format: image.format,
                blurDataURL: image.blurDataURL,
            };

            return `export default { ...${JSON.stringify(metadata)}, src: ${original}, srcSet: [${entries.join(", ")}] };\n`;
        },
    };
}
