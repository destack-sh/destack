import { expect, onTestFinished, test } from "@destack/test";
import { mkdtemp, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import sharp from "sharp";
import { build } from "vite";
import { schema } from "@destack/schema";
import { imagePlugin, processImage } from "./image.ts";

/** A 1200 by 600 PNG of one colour. */
async function hero(): Promise<Uint8Array> {
    const image = sharp({
        create: { width: 1200, height: 600, channels: 3, background: "#3366ff" },
    });

    return new Uint8Array(await image.png().toBuffer());
}

test("render an image's widths up to its own in AVIF, WebP and its format, with a blurred rendition", async () => {
    // process a wide PNG
    const image = await processImage(await hero());

    // keep its size, render four widths in three formats, and inline a tiny WebP
    expect({
        size: [image.width, image.height, image.type],
        variants: image.variants.map((variant) => `${variant.width} ${variant.type}`),
        widths: await Promise.all(
            image.variants
                .slice(0, 3)
                .map(async (variant) => (await sharp(variant.bytes).metadata()).width),
        ),
        placeholder: image.placeholder.startsWith("data:image/webp;base64,"),
    }).toEqual({
        size: [1200, 600, "image/png"],
        variants: [640, 828, 1080, 1200].flatMap((width) =>
            ["image/avif", "image/webp", "image/png"].map((type) => `${width} ${type}`),
        ),
        widths: [640, 640, 640],
        placeholder: true,
    });
});

test("build an imported image into an asset whose variants the build emits", async () => {
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

    // emit the original and eleven more variants, the original serving its own width and format
    const emitted = await readdir(join(directory, "dist"));
    const built = schema
        .object({
            default: schema.looseObject({
                src: schema.string(),
                width: schema.number(),
                height: schema.number(),
                type: schema.string(),
                variants: schema.array(schema.looseObject({ src: schema.string() })),
            }),
        })
        .parse(await import(join(directory, "dist", "index.mjs")));
    expect({
        files: emitted.filter((file) => file !== "index.mjs").length,
        size: [built.default.width, built.default.height, built.default.type],
        variants: built.default.variants.length,
        original: built.default.variants.at(-1)?.src === built.default.src,
    }).toEqual({ files: 12, size: [1200, 600, "image/png"], variants: 12, original: true });
});
