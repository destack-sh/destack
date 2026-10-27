import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { formatSource } from "@destack/check";
import { BunEnvironment } from "@destack/environment/bun";
import { create, type Font, type Path } from "fontkitten";

/** The tracking between the headword's letters, in em. */
const TRACKING = -0.045;

/** The dot's diameter, in em. */
const DOT = 0.13;

/** The space on each side of the dot, from the e's ink and to the s's ink, in em. */
const DOT_SPACE = 0.075;

/** The host the generator reads and writes through. */
const host = BunEnvironment.system;

/** The repository root. */
const repositoryDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");

/** The generated headword module. */
const generatedFile = join(
    repositoryDirectory,
    "@platform/site/src/view/content/generated/headword.ts",
);

/** The site's sans in bold, a master of the variable face the page draws the headword in. */
const fontFile = fileURLToPath(
    import.meta.resolve("@fontsource/ibm-plex-sans/files/ibm-plex-sans-latin-700-normal.woff2"),
);

/** Whether this run checks the generated file instead of writing it. */
const isCheck = host.process.arguments.includes("--check");

/** The headword's face. */
const font = fontOf(create(Buffer.from(await host.files.readFile(fontFile))));

/** The font's units in an em. */
const em = font.unitsPerEm;

/** The letters before the dot, set from the origin. */
const first = setLetters(font, "de");

/** The letters after the dot, set from the origin. */
const second = setLetters(font, "stack");

/** How far the second word moves so its ink starts one dot and two spaces after the e's ink. */
const shift = first.inkRight + (2 * DOT_SPACE + DOT) * em - second.inkLeft;

/** Every letter of the headword in place. */
const letters = [...first.paths, ...second.paths.map((path) => path.translate(shift, 0))];

/** The dot, centred between the e and the s at half the x-height, in font units. */
const dot = {
    x: first.inkRight + DOT_SPACE * em + (DOT * em) / 2,
    y: font.xHeight / 2,
    radius: (DOT * em) / 2,
};

/** The ink box of every letter together, in the font's upward y. */
const ink = {
    left: Math.min(...letters.map((path) => path.bbox.minX)),
    right: Math.max(...letters.map((path) => path.bbox.maxX)),
    bottom: Math.min(...letters.map((path) => path.bbox.minY)),
    top: Math.max(...letters.map((path) => path.bbox.maxY)),
};

/** The letters as one path, cropped to the ink and flipped into the page's downward y. */
const outline = letters
    .map((path) => path.translate(-ink.left, -ink.top).scale(1, -1).toSVG())
    .join("");

/** The generated module's source. */
const generatedSource = await formatSource(
    generatedFile,
    `/** The headword's ink box in font units, its letters touching every side. */\n` +
        `export const headwordBox = ${JSON.stringify({
            width: round(ink.right - ink.left),
            height: round(ink.top - ink.bottom),
        })} as const;\n\n` +
        `/** The headword's letters as one path inside its ink box. */\n` +
        `export const headwordLetters = ${JSON.stringify(roundPath(outline))};\n\n` +
        `/** The headword's dot, centred between the e and the s at half the x-height. */\n` +
        `export const headwordDot = ${JSON.stringify({
            x: round(dot.x - ink.left),
            y: round(ink.top - dot.y),
            radius: round(dot.radius),
        })} as const;\n`,
);

// check or replace the module
if (isCheck) {
    const isCurrent =
        (await host.files.exists(generatedFile)) &&
        (await host.files.readTextFile(generatedFile)) === generatedSource;
    if (!isCurrent) {
        throw new Error("generated headword is out of date, run `just @platform/site/generate`");
    }
} else {
    await host.files.mkdir(dirname(generatedFile), { recursive: true });
    const temporaryFile = `${generatedFile}.${host.process.id}.tmp`;
    await host.files.writeTextFile(temporaryFile, generatedSource);
    await host.files.rename(temporaryFile, generatedFile);
}

/** Return a single font, refusing a collection. */
function fontOf(source: ReturnType<typeof create>): Font {
    if (!("glyphsForString" in source)) {
        throw new TypeError("the headword font is a collection");
    }

    return source;
}

/** Set a word's letters from the origin, tracked: their paths and the word's ink edges. */
function setLetters(face: Font, word: string) {
    // place each glyph at the pen, then advance by its width and the tracking
    const paths: Path[] = [];
    let pen = 0;
    for (const glyph of face.glyphsForString(word)) {
        paths.push(glyph.path.translate(pen, 0));
        pen += glyph.advanceWidth + TRACKING * face.unitsPerEm;
    }

    // read the word's ink edges
    const inkLeft = Math.min(...paths.map((path) => path.bbox.minX));
    const inkRight = Math.max(...paths.map((path) => path.bbox.maxX));

    return { paths, inkLeft, inkRight };
}

/** Round a coordinate to a tenth of a font unit. */
function round(value: number): number {
    return Math.round(value * 10) / 10;
}

/** Round every number in a path to a tenth of a font unit. */
function roundPath(path: string): string {
    return path.replace(/-?\d+(\.\d+)?(e-?\d+)?/gu, (number) => String(round(Number(number))));
}
