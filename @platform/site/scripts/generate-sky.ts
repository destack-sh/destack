import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { BunEnvironment } from "@destack/environment/bun";
import { schema } from "@destack/schema";

/** The Yale Bright Star Catalogue, 5th revised edition, as JSON, with its J2000 positions. */
const CATALOG =
    "https://raw.githubusercontent.com/brettonw/YaleBrightStarCatalog/master/bsc5-short.json";

/** The faintest visual magnitude the plate draws, about 1,600 stars. */
const FAINTEST = 5;

/** The magnitude classes, brightest first: the faintest magnitude, dot diameter in px, opacity. */
const CLASSES = [
    [1.5, 3.2, 1],
    [2.5, 2.5, 0.95],
    [3.5, 1.9, 0.85],
    [4.5, 1.4, 0.7],
    [FAINTEST, 1, 0.55],
] as const;

/** The obliquity of the ecliptic at J2000, in degrees. */
const OBLIQUITY = 23.4393;

/** The ecliptic longitude between the ecliptic's points, in degrees, about 7px at desktop width. */
const ECLIPTIC_STEP = 2;

/** The galactic longitude between the galactic plane's points, in degrees. */
const GALACTIC_STEP = 4;

/** The graticule's spacing in right ascension and declination, two hours and 30 degrees. */
const GRATICULE = 30;

/** The rotation from equatorial to galactic coordinates at J2000, by row, from Hipparcos. */
const GALACTIC = [
    [-0.0548755604, -0.8734370902, -0.4838350155],
    [0.4941094279, -0.44482963, 0.7469822445],
    [-0.867666149, -0.1980763734, 0.4559837762],
] as const;

/** The Milky Way's glow as strokes along the galactic plane: width in px, then opacity. */
const GLOW = [
    [130, 0.012],
    [96, 0.014],
    [64, 0.016],
    [34, 0.018],
] as const;

/** The pale blue the Milky Way glows in. */
const GLOW_INK = "#c9d6ff";

/** The cream the plate draws its lines in, the sky's starlight. */
const INK = "#f1eadb";

/** The tints of the stars by surface temperature, hottest first: the least kelvin, and the tint. */
const TINTS = [
    [10_000, "#a9c8ff"],
    [7500, "#dfe7ff"],
    [6000, INK],
    [4500, "#ffd2a0"],
    [0, "#ffaa78"],
] as const;

/** A star as the catalogue lists it, each entry under its catalogue column. */
const Star = schema.record(schema.string(), schema.string());

/** The host the generator reads and writes through. */
const host = BunEnvironment.system;

/** The plate the site serves. */
const plateFile = join(resolve(dirname(fileURLToPath(import.meta.url)), ".."), "public/sky.svg");

await host.files.writeTextFile(plateFile, await plateOf(CATALOG));

/** Draw the plate from a catalogue: the Milky Way, the graticule, the ecliptic and the stars. */
async function plateOf(catalog: string): Promise<string> {
    // read the catalogue
    const response = await host.network.fetch(new Request(catalog));
    if (!response.ok) {
        throw new Error(`cannot read the star catalogue: '${response.status}'`);
    }
    const stars = schema.array(Star).parse(await response.json());

    // place each star bright enough by magnitude and tint, east to the left as charts draw it
    const dots = CLASSES.map(() => TINTS.map((): string[] => []));
    for (const star of stars) {
        const ascension = star["RA"];
        const declination = star["Dec"];
        const size = CLASSES.findIndex(([faintest]) => Number(star["V"]) <= faintest);
        const tint = TINTS.findIndex(([least]) => Number(star["K"]) >= least);
        if (ascension !== undefined && declination !== undefined && size !== -1 && tint !== -1) {
            const at = pointOf(degreesOf(ascension, 15), degreesOf(declination, 1));
            const radius = (CLASSES[size]?.[1] ?? 0) / 2;
            dots[size]?.[tint]?.push(`<circle cx="${at.x}" cy="${at.y}" r="${radius}"/>`);
        }
    }

    // trace the ecliptic as a chain line: two segments drawn, then a dot in the gap
    const chain: string[] = [];
    for (let longitude = 0; longitude < 360; longitude += ECLIPTIC_STEP) {
        const from = eclipticOf(longitude);
        const to = eclipticOf(longitude + ECLIPTIC_STEP);
        const middle = eclipticOf(longitude + ECLIPTIC_STEP / 2);
        const index = longitude / ECLIPTIC_STEP;
        chain.push(
            index % 3 === 2
                ? `<circle ${circleOf(middle)} r="0.6"/>`
                : `<line ${lineOf(from, to)}/>`,
        );
    }

    // trace the galactic plane, skipping the segment that wraps around the 24 hours
    const plane: string[] = [];
    for (let longitude = 0; longitude < 360; longitude += GALACTIC_STEP) {
        const from = galacticOf(longitude);
        const to = galacticOf(longitude + GALACTIC_STEP);
        if (Math.abs(from.ascension - to.ascension) < 180) {
            plane.push(`<line ${lineOf(from, to)}/>`);
        }
    }

    // rule the graticule: hour lines down, declination lines across, the equator stronger
    const grid: string[] = [];
    for (let line = GRATICULE; line < 360; line += GRATICULE) {
        grid.push(
            `<line x1="${percentOf(line, 360)}" y1="0" x2="${percentOf(line, 360)}" y2="100%"/>`,
        );
    }
    for (let line = GRATICULE; line < 180; line += GRATICULE) {
        const opacity = line === 90 ? 0.14 : 0.07;
        grid.push(
            `<line x1="0" y1="${percentOf(line, 180)}" x2="100%" y2="${percentOf(line, 180)}" stroke-opacity="${opacity}"/>`,
        );
    }

    // place everything in percentages and size it in pixels, so the plate stretches with round stars
    const layers = [
        `<defs><g id="plane">${plane.join("")}</g></defs>`,
        ...GLOW.map(
            ([width, opacity]) =>
                `<use href="#plane" stroke="${GLOW_INK}" stroke-width="${width}" stroke-linecap="round" opacity="${opacity}"/>`,
        ),
        `<g stroke="${INK}" stroke-opacity="0.07" stroke-width="1">${grid.join("")}</g>`,
        `<g stroke="${INK}" fill="${INK}" stroke-opacity="0.32" fill-opacity="0.32" stroke-width="1">${chain.join("")}</g>`,
        ...CLASSES.flatMap(([, , opacity], size) =>
            TINTS.map(
                ([, tint], index) =>
                    `<g fill="${tint}" fill-opacity="${opacity}">${dots[size]?.[index]?.join("")}</g>`,
            ),
        ),
    ];

    return `<svg xmlns="http://www.w3.org/2000/svg" width="100%" height="100%">${layers.join("")}</svg>\n`;
}

/** Write a place on the plate as percentages across and down, east to the left. */
function pointOf(ascension: number, declination: number): { x: string; y: string } {
    return { x: percentOf(360 - ascension, 360), y: percentOf(90 - declination, 180) };
}

/** Write a share of a whole as a percentage. */
function percentOf(part: number, whole: number): string {
    return `${((part / whole) * 100).toFixed(2)}%`;
}

/** Write a circle's centre at a place. */
function circleOf(place: { ascension: number; declination: number }): string {
    const at = pointOf(place.ascension, place.declination);

    return `cx="${at.x}" cy="${at.y}"`;
}

/** Write a line between two places. */
function lineOf(
    from: { ascension: number; declination: number },
    to: { ascension: number; declination: number },
): string {
    const start = pointOf(from.ascension, from.declination);
    const end = pointOf(to.ascension, to.declination);

    return `x1="${start.x}" y1="${start.y}" x2="${end.x}" y2="${end.y}"`;
}

/** Read a sexagesimal angle such as "05h 55m 10.3s" or "-08° 12′ 05″" into degrees. */
function degreesOf(text: string, unit: number): number {
    // read the three parts and the sign
    const [whole = 0, minutes = 0, seconds = 0] = (text.match(/[\d.]+/gu) ?? []).map(Number);
    const sign = text.trim().startsWith("-") ? -1 : 1;

    return sign * unit * (whole + minutes / 60 + seconds / 3600);
}

/** Find the right ascension and declination of an ecliptic longitude, in degrees. */
function eclipticOf(longitude: number): { ascension: number; declination: number } {
    // rotate the ecliptic point by the obliquity onto the equator
    const lambda = (longitude * Math.PI) / 180;
    const epsilon = (OBLIQUITY * Math.PI) / 180;
    const ascension =
        (Math.atan2(Math.sin(lambda) * Math.cos(epsilon), Math.cos(lambda)) * 180) / Math.PI;
    const declination = (Math.asin(Math.sin(epsilon) * Math.sin(lambda)) * 180) / Math.PI;

    // keep the ascension rising across the whole turn, ending at 360
    const unwrapped = longitude === 360 ? 360 : ascension < 0 ? ascension + 360 : ascension;

    return { ascension: unwrapped, declination };
}

/** Find the right ascension and declination of a galactic longitude on the plane, in degrees. */
function galacticOf(longitude: number): { ascension: number; declination: number } {
    // turn the point on the plane back from galactic to equatorial axes
    const lambda = (longitude * Math.PI) / 180;
    const [first, second] = GALACTIC;
    const x = first[0] * Math.cos(lambda) + second[0] * Math.sin(lambda);
    const y = first[1] * Math.cos(lambda) + second[1] * Math.sin(lambda);
    const z = first[2] * Math.cos(lambda) + second[2] * Math.sin(lambda);

    // read its angles, the ascension from 0 to 360
    const ascension = ((Math.atan2(y, x) * 180) / Math.PI + 360) % 360;

    return { ascension, declination: (Math.asin(z) * 180) / Math.PI };
}
