import { expect, onTestFinished, test } from "@destack/test";
import { mkdtemp, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import sharp from "sharp";
import { build } from "vite";
import { schema } from "@destack/schema";
import { ImageMetadata } from "../media/asset.ts";
import { imagePlugin, imageFiles, transformImage } from "./image.ts";

/** A 1200 by 600 PNG of one color. */
async function hero(): Promise<Uint8Array> {
    const image = sharp({
        create: { width: 1200, height: 600, channels: 3, background: "#3366ff" },
    });

    return new Uint8Array(await image.png().toBuffer());
}

test("render an image's source set up to its own width in AVIF, WebP and its format, with a blurred rendition", async () => {
    // transform a wide PNG
    const bytes = await hero();
    const image = await transformImage(bytes);

    // keep its size, render five widths in three formats, reuse the original's bytes, and inline a tiny WebP
    expect({
        size: [image.width, image.height, image.format],
        files: image.files.map((file) => `${file.width} ${file.format}`),
        widths: await Promise.all(
            image.files.slice(0, 3).map(async (file) => (await sharp(file.data).metadata()).width),
        ),
        original: image.files.at(-1)?.data === bytes,
        blur: image.blurDataURL.startsWith("data:image/webp;base64,"),
    }).toEqual({
        size: [1200, 600, "png"],
        files: [640, 750, 828, 1080, 1200].flatMap((width) =>
            ["avif", "webp", "png"].map((format) => `${width} ${format}`),
        ),
        widths: [640, 640, 640],
        original: true,
        blur: true,
    });
});

test("build an imported image into metadata whose source set the build emits", async () => {
    // build a module importing a PNG
    const directory = await mkdtemp(join(tmpdir(), "destack-image-"));
    onTestFinished(() => rm(directory, { recursive: true }));
    await writeFile(join(directory, "hero.png"), await hero());
    await writeFile(join(directory, "index.js"), 'export { default } from "./hero.png";\n');
    await build({
        root: directory,
        logLevel: "silent",
        plugins: [imagePlugin()],
        build: {
            outDir: "dist",
            lib: { entry: join(directory, "index.js"), formats: ["es"], fileName: "index" },
        },
    });

    // emit the original and fourteen more files, the original serving its own width and format
    const emitted = await readdir(join(directory, "dist"));
    const built = schema
        .object({ default: ImageMetadata })
        .parse(await import(join(directory, "dist", "index.mjs")));
    expect({
        files: emitted.filter((file) => file !== "index.mjs").length,
        size: [built.default.width, built.default.height, built.default.format],
        srcSet: built.default.srcSet.length,
        original: built.default.srcSet.at(-1)?.src === built.default.src,
    }).toEqual({ files: 15, size: [1200, 600, "png"], srcSet: 15, original: true });
});

test("plan an image's files at each width below its own and its own, in the formats and its own", () => {
    expect(imageFiles(1000, "png", { widths: [640, 828, 1920], formats: ["avif"] })).toEqual([
        { width: 640, format: "avif" },
        { width: 640, format: "png" },
        { width: 828, format: "avif" },
        { width: 828, format: "png" },
        { width: 1000, format: "avif" },
        { width: 1000, format: "png" },
    ]);
});
