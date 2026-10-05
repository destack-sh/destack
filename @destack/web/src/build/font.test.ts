import { expect, test } from "@destack/test";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { readFont } from "./font.ts";

/** IBM Plex Mono's regular Latin file. */
const PLEX_MONO = fileURLToPath(
    new URL(
        "../../node_modules/@fontsource/ibm-plex-mono/files/ibm-plex-mono-latin-400-normal.woff2",
        import.meta.url,
    ),
);

test("read a font file's family and adjust each local fallback to its metrics", async () => {
    // read IBM Plex Mono
    const font = await readFont(new Uint8Array(await readFile(PLEX_MONO)));

    // name its family, and size Arial and Times New Roman to its width with overrides as percentages
    expect({
        family: font.family,
        fallbacks: Object.keys(font.fallbacks),
        isSized: Object.values(font.fallbacks).every(
            (metrics) => metrics.size.endsWith("%") && metrics.ascent.endsWith("%"),
        ),
        isWiderThanArial: Number.parseFloat(font.fallbacks.Arial.size) > 100,
    }).toEqual({
        family: "IBM Plex Mono",
        fallbacks: ["Arial", "Times New Roman"],
        isSized: true,
        isWiderThanArial: true,
    });
});
