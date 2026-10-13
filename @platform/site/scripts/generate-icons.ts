import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

/** The directory the site serves its icons from. */
const OUTPUT = join(dirname(fileURLToPath(import.meta.url)), "..", "public", "diagram");

/** The share of a two-tone icon's colour its bodies fill with, under their outline. */
const TONE = 0.3;

/** The width of every outline, cut and two-tone line, on the 24 pixel grid. */
const STROKE = 2;

/** The width of a solid icon's lines, heavier so they hold beside its filled bodies. */
const SOLID_LINE = 2.5;

/** The gap a front shape leaves in the shapes behind it, in a two-tone and a solid icon. */
const GAPS = { tone: 4, solid: 3.5 } as const;

/** A closed shape of an icon: a rounded rectangle, a circle or a path. */
type Shape =
    | { kind: "rect"; x: number; y: number; width: number; height: number; radius: number }
    | { kind: "circle"; x: number; y: number; radius: number }
    | { kind: "path"; outline: string };

/** One layer of an icon, drawn over the layers before it with a gap around its shapes. */
type Layer = {
    /** The closed shapes: filled solid, or tinted under an outline. */
    bodies: readonly Shape[];
    /** The open strokes drawn in both styles. */
    lines: readonly string[];
    /** The open strokes knocked out of a solid icon and drawn on a two-tone one. */
    cuts: readonly string[];
    /** The closed shapes knocked out of a solid icon and outlined on a two-tone one. */
    holes: readonly string[];
};

/** How an icon draws: two-tone inside an app, or solid on its tile. */
type Style = keyof typeof GAPS;

/** Every icon of the site, back layer first. */
const icons: Readonly<Record<string, readonly Layer[]>> = {
    pages: [
        layer({
            bodies: [rect(4.5, 2.5, 15, 19, 3.5)],
            cuts: ["M8.5 8.5h7", "M8.5 12h7", "M8.5 15.5h4"],
        }),
    ],
    file: [
        layer({
            bodies: [
                path(
                    "M8 2.5h5.6a3 3 0 0 1 2.1.9l3 3a3 3 0 0 1 .9 2.1V18a3.5 3.5 0 0 1-3.5 3.5H8A3.5 3.5 0 0 1 4.5 18V6A3.5 3.5 0 0 1 8 2.5Z",
                ),
            ],
            cuts: ["M13.5 3v3.5a1.5 1.5 0 0 0 1.5 1.5h4"],
        }),
    ],
    tasks: [layer({ bodies: [rect(3.5, 3.5, 17, 17, 4.5)], cuts: ["M8 12.3l2.8 2.8L16.2 9.4"] })],
    chat: [
        layer({
            bodies: [
                path(
                    "M7.5 4h9A4.5 4.5 0 0 1 21 8.5v4.5a4.5 4.5 0 0 1-4.5 4.5h-5.3l-4 3.2c-.5.4-1.2 0-1.2-.6v-2.7A4.5 4.5 0 0 1 3 13V8.5A4.5 4.5 0 0 1 7.5 4Z",
                ),
            ],
            cuts: [dot(8.5, 10.75), dot(12, 10.75), dot(15.5, 10.75)],
        }),
    ],
    source: [
        layer({ lines: ["M7 8.9v6.2", "M17 10.9c0 3.4-4.4 3.6-7.6 5.1"] }),
        layer({ bodies: [circle(7, 6, 2.9), circle(7, 18, 2.9), circle(17, 8, 2.9)] }),
    ],
    calendar: [
        layer({
            bodies: [rect(3, 4.5, 18, 16.5, 3.5)],
            cuts: [
                "M3.5 9.5h17",
                dot(8, 13.5),
                dot(12, 13.5),
                dot(16, 13.5),
                dot(8, 17),
                dot(12, 17),
            ],
        }),
        layer({ lines: ["M8 2.5v4", "M16 2.5v4"] }),
    ],
    table: [
        layer({
            bodies: [rect(3, 4, 18, 16, 3.5)],
            cuts: ["M3.5 9.5h17", "M3.5 14.75h17", "M9.5 10v9.5"],
        }),
    ],
    mail: [layer({ bodies: [rect(2.5, 5, 19, 14, 3.5)], cuts: ["M4 7.5l8 5.75 8-5.75"] })],
    canvas: [
        layer({ bodies: [rect(3, 3, 11, 11, 3)] }),
        layer({ bodies: [circle(15.5, 15.5, 5.5)] }),
    ],
    user: [
        layer({
            bodies: [
                circle(12, 8, 4),
                path("M12 13.5a7.5 7 0 0 1 7.5 7v.5h-15v-.5a7.5 7 0 0 1 7.5-7Z"),
            ],
        }),
    ],
    accounts: [
        layer({ bodies: [circle(16.5, 7.5, 3), path("M16.5 12.5a5.5 5 0 0 1 5.5 5v.5h-6Z")] }),
        layer({
            bodies: [
                circle(9.5, 9, 3.5),
                path("M9.5 14.5a6.5 6 0 0 1 6.5 6v.5H3v-.5a6.5 6 0 0 1 6.5-6Z"),
            ],
        }),
    ],
    vault: [
        layer({
            bodies: [rect(3, 3.5, 18, 16, 3.5)],
            cuts: ["M12 7.75a3.75 3.75 0 1 1 0 7.5 3.75 3.75 0 0 1 0-7.5Z", dot(12, 11.5)],
            lines: ["M7 19.5v2", "M17 19.5v2"],
        }),
    ],
    storage: [
        layer({
            bodies: [
                path(
                    "M4.5 6c0-1.9 3.4-3.5 7.5-3.5s7.5 1.6 7.5 3.5v12c0 1.9-3.4 3.5-7.5 3.5S4.5 19.9 4.5 18Z",
                ),
            ],
            cuts: [
                "M4.5 6c0 1.9 3.4 3.5 7.5 3.5s7.5-1.6 7.5-3.5",
                "M4.5 12c0 1.9 3.4 3.5 7.5 3.5s7.5-1.6 7.5-3.5",
            ],
        }),
    ],
    bucket: [
        layer({
            bodies: [
                path(
                    "M2.5 7A3 3 0 0 1 5.5 4h3.3a2 2 0 0 1 1.5.7L12 6.5h6.5a3 3 0 0 1 3 3V17a3 3 0 0 1-3 3h-13a3 3 0 0 1-3-3Z",
                ),
            ],
            cuts: ["M3 10h18"],
        }),
    ],
    search: [layer({ lines: ["M15.5 15.5l5 5"] }), layer({ bodies: [circle(10.5, 10.5, 6.5)] })],
    notify: [
        layer({
            lines: ["M10 20.5h4"],
            bodies: [
                path(
                    "M12 3a6 6 0 0 0-6 6v3.6l-1.6 2.6c-.4.7.1 1.5.9 1.5h13.4c.8 0 1.3-.8.9-1.5L18 12.6V9a6 6 0 0 0-6-6Z",
                ),
            ],
        }),
    ],
    sync: [
        layer({
            bodies: [
                path(
                    "M13.2 2.6 5.3 12.9c-.5.7 0 1.6.8 1.6H11l-1 6.4c-.1.8.9 1.2 1.4.6l7.9-10.3c.5-.7 0-1.6-.8-1.6H13.6l1-6.4c.1-.8-.9-1.2-1.4-.6Z",
                ),
            ],
        }),
    ],
    telemetry: [
        layer({
            bodies: [
                rect(3.5, 13, 4, 7.5, 1.75),
                rect(10, 3.5, 4, 17, 1.75),
                rect(16.5, 8.5, 4, 12, 1.75),
            ],
        }),
    ],
    hosts: [
        layer({
            bodies: [rect(3, 3.5, 18, 7, 2.5), rect(3, 13.5, 18, 7, 2.5)],
            cuts: [dot(7, 7), dot(7, 17), "M11 7h6", "M11 17h6"],
        }),
    ],
    ai: [
        layer({
            lines: [
                "M9 2.5v3",
                "M15 2.5v3",
                "M9 18.5v3",
                "M15 18.5v3",
                "M2.5 9h3",
                "M2.5 15h3",
                "M18.5 9h3",
                "M18.5 15h3",
            ],
        }),
        layer({
            bodies: [rect(5.5, 5.5, 13, 13, 3)],
            cuts: [
                "M10 9.5h4a.5.5 0 0 1 .5.5v4a.5.5 0 0 1-.5.5h-4a.5.5 0 0 1-.5-.5v-4a.5.5 0 0 1 .5-.5Z",
            ],
        }),
    ],
    audit: [
        layer({
            bodies: [rect(4.5, 4, 15, 17.5, 3)],
            cuts: ["M8.5 11h7", "M8.5 14.5h7", "M8.5 18h4"],
        }),
        layer({ bodies: [rect(8.5, 2, 7, 4, 1.5)] }),
    ],
    auth: [
        layer({ lines: ["M12 12h9.5", "M19 12v3.5", "M15.75 12v2.75"] }),
        layer({ bodies: [circle(7, 12, 5.5)], cuts: [dot(7, 12)] }),
    ],
    build: [
        layer({ lines: ["M11.25 12.75 4.5 19.5"] }),
        layer({
            bodies: [
                path(
                    "M12.6 4.1a2 2 0 0 1 2.8 0l4.5 4.5a2 2 0 0 1 0 2.8l-1 1a2 2 0 0 1-2.8 0l-4.5-4.5a2 2 0 0 1 0-2.8Z",
                ),
            ],
        }),
    ],
    card: [layer({ bodies: [rect(2.5, 5, 19, 14, 3)], cuts: ["M3 9.5h18", "M6.5 15h3"] })],
    computer: [
        layer({ lines: ["M9 20.5h6", "M12 16.5v4"] }),
        layer({ bodies: [rect(3, 4, 18, 12.5, 2.5)] }),
    ],
    cloud: [
        layer({
            bodies: [
                path(
                    "M7 19.5h10a4.25 4.25 0 0 0 .6-8.45 5.75 5.75 0 0 0-11.1-1A4.75 4.75 0 0 0 7 19.5Z",
                ),
            ],
        }),
    ],
    lock: [
        layer({ lines: ["M8 10.5V8a4 4 0 0 1 8 0v2.5"] }),
        layer({ bodies: [rect(4.5, 10.5, 15, 10.5, 3)], cuts: ["M12 14.5v2.5"] }),
    ],
    presence: [
        layer({
            bodies: [
                path(
                    "M5.5 3.6 18.6 9.3c.8.4.8 1.5-.1 1.8l-5.2 1.8-1.8 5.2c-.3.9-1.4.9-1.8.1L4 5.1c-.4-.9.6-1.8 1.5-1.5Z",
                ),
            ],
        }),
    ],
    services: [
        layer({
            lines: [
                "M8.5 3.5c-2 0-2.5 1-2.5 2.75v2.5C6 10.5 5.25 12 3.5 12c1.75 0 2.5 1.5 2.5 3.25v2.5c0 1.75.5 2.75 2.5 2.75",
                "M15.5 3.5c2 0 2.5 1 2.5 2.75v2.5c0 1.75.75 3.25 2.5 3.25-1.75 0-2.5 1.5-2.5 3.25v2.5c0 1.75-.5 2.75-2.5 2.75",
            ],
            bodies: [circle(12, 12, 1.75)],
        }),
    ],
    registry: [
        layer({
            bodies: [
                path(
                    "M11 2.8a2 2 0 0 1 2 0l6.5 3.7a2 2 0 0 1 1 1.7v7.6a2 2 0 0 1-1 1.7L13 21.2a2 2 0 0 1-2 0l-6.5-3.7a2 2 0 0 1-1-1.7V8.2a2 2 0 0 1 1-1.7Z",
                ),
            ],
            cuts: ["M4 7.5 12 12l8-4.5", "M12 12v9"],
        }),
    ],
    settings: [
        layer({ lines: ["M3.5 7h17", "M3.5 17h17"] }),
        layer({ bodies: [circle(15, 7, 2.75), circle(9, 17, 2.75)] }),
    ],
    template: [
        layer({
            bodies: [
                rect(3, 3, 7.5, 7.5, 2.25),
                rect(13.5, 3, 7.5, 7.5, 2.25),
                rect(3, 13.5, 7.5, 7.5, 2.25),
                rect(13.5, 13.5, 7.5, 7.5, 2.25),
            ],
        }),
    ],
};

/** Write every icon twice: two-tone beside the others, solid under `fill/`. */
async function generate(): Promise<void> {
    // make room for the solid icons
    await mkdir(join(OUTPUT, "fill"), { recursive: true });

    // write both styles of each icon
    for (const [name, layers] of Object.entries(icons)) {
        await writeFile(join(OUTPUT, `${name}.svg`), render(layers, "tone"));
        await writeFile(join(OUTPUT, "fill", `${name}.svg`), render(layers, "solid"));
    }
}

/** Render an icon's layers as one SVG document in a style. */
function render(layers: readonly Layer[], style: Style): string {
    // collect each layer's masks and group, drawn at the style's line width
    const definitions: string[] = [];
    const groups: string[] = [];
    const lineWidth = style === "tone" ? STROKE : SOLID_LINE;
    for (const [index, current] of layers.entries()) {
        // cut a gap in this layer around every shape and line of the layers in front
        const front = layers.slice(index + 1);
        const hidden = [
            ...front.flatMap((later) => later.bodies.map((shape) => halo(shape, GAPS[style]))),
            ...front.flatMap((later) =>
                later.lines.map((line) => stroke(line, lineWidth + GAPS[style] - 1.5, "#000")),
            ),
        ].join("");

        // knock this layer's cuts and holes out of its solid bodies
        const knocked =
            style === "solid"
                ? [
                      ...current.cuts.map((cut) => stroke(cut, STROKE, "#000")),
                      ...current.holes.map((hole) => `<path d="${hole}" fill="#000"/>`),
                  ].join("")
                : "";

        // fill the bodies solid, or tint them under an outline, then draw the lines
        const bodies = current.bodies
            .map((shape) =>
                style === "solid"
                    ? markup(shape, 'fill="currentColor"')
                    : markup(
                          shape,
                          `fill="currentColor" fill-opacity="${String(TONE)}" stroke="currentColor" stroke-width="${String(STROKE)}" stroke-linejoin="round"`,
                      ),
            )
            .join("");
        const details =
            style === "tone"
                ? [
                      ...current.cuts.map((cut) => stroke(cut, STROKE, "currentColor")),
                      ...current.holes.map(
                          (hole) =>
                              `<path d="${hole}" fill="none" stroke="currentColor" stroke-width="${String(STROKE)}" stroke-linejoin="round"/>`,
                      ),
                  ].join("")
                : "";
        const lines = current.lines.map((line) => stroke(line, lineWidth, "currentColor")).join("");

        // mask the layer where it is knocked out or sits behind another
        const body = knocked === "" ? bodies : `<g mask="url(#b${String(index)})">${bodies}</g>`;
        if (knocked !== "") {
            definitions.push(mask(`b${String(index)}`, knocked));
        }
        if (hidden !== "" || knocked !== "") {
            definitions.push(mask(`m${String(index)}`, hidden));
            groups.push(`<g mask="url(#m${String(index)})">${body}${details}${lines}</g>`);
        } else {
            groups.push(`<g>${body}${details}${lines}</g>`);
        }
    }
    const defs = definitions.length === 0 ? "" : `<defs>${definitions.join("")}</defs>`;

    return `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" color="#000">${defs}${groups.join("")}</svg>\n`;
}

/** Return an icon layer, with every kind of element it leaves out empty. */
function layer(parts: Partial<Layer>): Layer {
    return { bodies: [], lines: [], cuts: [], holes: [], ...parts };
}

/** Return a rounded rectangle. */
function rect(x: number, y: number, width: number, height: number, radius: number): Shape {
    return { kind: "rect", x, y, width, height, radius };
}

/** Return a circle. */
function circle(x: number, y: number, radius: number): Shape {
    return { kind: "circle", x, y, radius };
}

/** Return a closed path. */
function path(outline: string): Shape {
    return { kind: "path", outline };
}

/** Return a round dot at a point, drawn as a stroke of no length. */
function dot(x: number, y: number): string {
    return `M${String(x)} ${String(y)}h.01`;
}

/** Return a shape's SVG element with its paint. */
function markup(shape: Shape, paint: string): string {
    if (shape.kind === "rect") {
        return `<rect x="${String(shape.x)}" y="${String(shape.y)}" width="${String(shape.width)}" height="${String(shape.height)}" rx="${String(shape.radius)}" ${paint}/>`;
    } else if (shape.kind === "circle") {
        return `<circle cx="${String(shape.x)}" cy="${String(shape.y)}" r="${String(shape.radius)}" ${paint}/>`;
    }

    return `<path d="${shape.outline}" ${paint}/>`;
}

/** Return a shape filled and stroked black, the area a gap around it removes. */
function halo(shape: Shape, width: number): string {
    return markup(
        shape,
        `fill="#000" stroke="#000" stroke-width="${String(width)}" stroke-linejoin="round"`,
    );
}

/** Return an open stroke with round ends. */
function stroke(outline: string, width: number, paint: string): string {
    return `<path d="${outline}" fill="none" stroke="${paint}" stroke-width="${String(width)}" stroke-linecap="round" stroke-linejoin="round"/>`;
}

/** Return a mask that keeps the icon's square and removes what it is given. */
function mask(id: string, removed: string): string {
    return `<mask id="${id}" maskUnits="userSpaceOnUse" x="0" y="0" width="24" height="24"><rect width="24" height="24" fill="#fff"/>${removed}</mask>`;
}

await generate();
