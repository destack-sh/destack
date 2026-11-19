import { testSimulation } from "@destack/test/world";
import { expect, test } from "@destack/test";
import { join } from "node:path";
import sharp from "sharp";
import { build } from "vite";
import { schema } from "@destack/schema";
import { ImageMetadata } from "../media/asset.ts";
import type { TransformCache } from "@destack/package/build";
import { imagePlugin, imageFiles, transformImage } from "./image.ts";

/** A 1200 by 600 PNG of one color. */
async function hero(): Promise<Uint8Array<ArrayBuffer>> {
    const image = sharp({
        create: { width: 1200, height: 600, channels: 3, background: "#3366ff" },
    });

    return new Uint8Array(await image.png().toBuffer());
}

test("render each rendition of an image once, reading it from the build's cache by digest, width and format", async () => {
    // render a wide PNG twice through one cache, counting what each pass writes
    await using simulation = testSimulation();
    const { crypto } = simulation.environment("test");
    const kept = new Map<string, Uint8Array<ArrayBuffer>>();
    const written: string[] = [];
    const cache: TransformCache = {
        read: async (key) => kept.get(key),
        write: async (key, bytes) => {
            kept.set(key, bytes);
            written.push(key);
        },
    };
    const bytes = await hero();
    const first = await transformImage(bytes, {}, crypto, cache);
    const firstWrites = written.length;
    const second = await transformImage(bytes, {}, crypto, cache);

    // keep every rendition but the original once, and read the same bytes back
    expect({
        firstWrites,
        secondWrites: written.length - firstWrites,
        isSame: second.files.every((file, index) => file.data === first.files[index]?.data),
    }).toEqual({ firstWrites: first.files.length - 1, secondWrites: 0, isSame: true });
});

test("render an image's source set up to its own width in AVIF, WebP and its format, with a blurred rendition", async () => {
    // transform a wide PNG
    await using simulation = testSimulation();
    const { crypto } = simulation.environment("test");
    const bytes = await hero();
    const image = await transformImage(bytes, {}, crypto);

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
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await files.makeTemporaryDirectory("image-");
    await files.writeFile(join(directory, "hero.png"), await hero());
    await files.writeTextFile(
        join(directory, "index.js"),
        'export { default } from "./hero.png";\n',
    );
    await build({
        root: directory,
        logLevel: "silent",
        plugins: [imagePlugin(environment)],
        build: {
            outDir: "dist",
            lib: { entry: join(directory, "index.js"), formats: ["es"], fileName: "index" },
        },
    });

    // emit the original and fourteen more files, the original serving its own width and format
    const emitted = await files.readDirectory(join(directory, "dist"));
    const built = schema
        .object({ default: ImageMetadata })
        .parse(await import(join(directory, "dist", "index.mjs")));
    expect({
        files: emitted.filter((file) => file.name !== "index.mjs").length,
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
