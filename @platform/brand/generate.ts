import { BunEnvironment } from "@destack/environment/bun";

/** The square clip of each icon master, which the rounded variant replaces. */
const SQUARE = '<rect width="32" height="32"/>';

/** The rounded clip, with a corner radius of 7 on the 32 grid. */
const ROUNDED = '<rect width="32" height="32" rx="7"/>';

/** The icon sizes to render, the 1024 one without a size suffix. */
const ICON_SIZES = [180, 256, 512, 1024];

/** The favicon sizes browsers ask for. */
const FAVICON_SIZES = [16, 32, 48];

/** The social layouts, each rendered at its intended upload size. */
const BANNERS = ["og", "github-social", "x-header", "linkedin-company", "linkedin-profile"];

/** The system's files and processes this script works with. */
const { files, process } = BunEnvironment.system;

/** The brand package's directory, where every source and output lies. */
const directory = import.meta.dirname;

/** Render an SVG to a PNG with rsvg-convert, at a square size or its own. */
async function render(source: string, output: string, size?: number): Promise<void> {
    const dimensions = size === undefined ? [] : ["-w", String(size), "-h", String(size)];
    const { code, stderr } = await process.run([
        "rsvg-convert",
        ...dimensions,
        "-o",
        output,
        source,
    ]);
    if (code !== 0) {
        const message = new TextDecoder().decode(stderr).trim();
        throw new Error(`could not render '${source}': rsvg-convert answered ${message}`);
    }
}

// derive the rounded outline from each square master
for (const channel of ["", "-local"]) {
    const master = `${directory}/icon/icon${channel}`;
    const source = await files.readTextFile(`${master}.svg`);
    if (!source.includes(SQUARE)) {
        throw new Error(`icon '${master}.svg' is invalid: it must clip to a square`);
    }
    await files.writeTextFile(`${master}-rounded.svg`, source.replace(SQUARE, ROUNDED));

    // render the square and rounded icons at each size
    for (const variant of [master, `${master}-rounded`]) {
        for (const size of ICON_SIZES) {
            const output = `${variant}${size === 1024 ? "" : `-${size}`}.png`;
            await render(`${variant}.svg`, output, size);
        }
    }
}

// render the favicon at each size
for (const size of FAVICON_SIZES) {
    const favicon = `${directory}/favicon/favicon`;
    await render(`${favicon}.svg`, `${favicon}-${size}.png`, size);
}

// render each social layout
for (const name of BANNERS) {
    const banner = `${directory}/banner/${name}`;
    await render(`${banner}.svg`, `${banner}.png`);
}
