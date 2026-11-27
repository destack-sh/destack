import { FONT_WEIGHTS, type FontStyle, type FontWeight } from "./font.ts";

/** The widths CSS names in `font-stretch`, as percentages of the normal width. */
const STRETCHES: Readonly<Record<string, number>> = {
    "ultra-condensed": 50,
    "extra-condensed": 62.5,
    condensed: 75,
    "semi-condensed": 87.5,
    normal: 100,
    "semi-expanded": 112.5,
    expanded: 125,
    "extra-expanded": 150,
    "ultra-expanded": 200,
};

/** The weights the names of styles carry, as OpenType's weight classes name them, longest first. */
const STYLE_WEIGHTS: readonly (readonly [name: string, weight: FontWeight])[] = [
    ["extrablack", 950],
    ["ultrablack", 950],
    ["extralight", 200],
    ["ultralight", 200],
    ["extrabold", 800],
    ["ultrabold", 800],
    ["semibold", 600],
    ["demibold", 600],
    ["hairline", 100],
    ["regular", 400],
    ["medium", 500],
    ["normal", 400],
    ["black", 900],
    ["heavy", 900],
    ["light", 300],
    ["thin", 100],
    ["book", 400],
    ["bold", 700],
];

/** The generic families CSS names, which the browser maps to its own fonts. */
const GENERIC_FAMILIES = [
    "system-ui",
    "sans-serif",
    "serif",
    "monospace",
    "cursive",
    "ui-sans-serif",
    "ui-serif",
    "ui-monospace",
    "ui-rounded",
] as const;

/** Where a typeface comes from: the document's faces, the device's, or CSS's generic families. */
export type TypefaceSource = "document" | "local" | "generic";

/** A range of a face's axis, a single value where the face is static. */
export interface FaceRange {
    /** The least value. */
    readonly min: number;
    /** The greatest value. */
    readonly max: number;
}

/** One face of a typeface: the weights, slant and widths it draws. */
export interface Face {
    /** The weights it draws, a range where the face is variable. */
    readonly weight: FaceRange;
    /** Whether it is upright or slanted. */
    readonly style: FontStyle;
    /** The widths it draws, as percentages of the normal width. */
    readonly stretch: FaceRange;
}

/** A face as CSS describes it, such as a `FontFace` of `document.fonts`. */
export interface FaceDescription {
    /** The family it belongs to. */
    readonly family: string;
    /** Its `font-weight` descriptor, such as `400` or `100 900`. */
    readonly weight: string;
    /** Its `font-style` descriptor, such as `italic`. */
    readonly style: string;
    /** Its `font-stretch` descriptor, such as `normal` or `75% 125%`. */
    readonly stretch: string;
}

/** A font installed on the device, as the Local Font Access API lists it. */
export interface LocalFace {
    /** The family it belongs to. */
    readonly family: string;
    /** The name of its style, such as `Bold Italic`. */
    readonly style: string;
}

/** A typeface: a family and the faces it has, from the document, the device or CSS. */
export class Typeface {
    /** The family's name, as `font-family` names it. */
    readonly family: string;
    /** Where it comes from. */
    readonly source: TypefaceSource;
    /** Its faces. */
    readonly faces: readonly Face[];

    /** Describe a family by its faces. */
    constructor(family: string, source: TypefaceSource, faces: readonly Face[]) {
        this.family = family;
        this.source = source;
        this.faces = faces;
    }

    /** Group the faces of `document.fonts` by family, as the CSS Font Loading API lists them. */
    static document(faces: Iterable<FaceDescription>): Typeface[] {
        return Typeface.#group(
            [...faces].map((face) => ({
                family: face.family.replaceAll(/^["']|["']$/gu, ""),
                face: {
                    weight: weightOf(face.weight),
                    style: styleOf(face.style),
                    stretch: stretchOf(face.stretch),
                },
            })),
            "document",
        );
    }

    /** Group the device's fonts by family, as the Local Font Access API lists them. */
    static local(fonts: Iterable<LocalFace>): Typeface[] {
        return Typeface.#group(
            [...fonts].map((font) => {
                // read the weight and slant the style name carries, regular where it names no weight
                const name = font.style.toLowerCase().replaceAll(/[\s_-]/gu, "");
                const weight = STYLE_WEIGHTS.find(([word]) => name.includes(word))?.[1] ?? 400;
                const isSlanted = name.includes("italic") || name.includes("oblique");

                return {
                    family: font.family,
                    face: {
                        weight: { min: weight, max: weight },
                        style: isSlanted ? ("italic" as const) : ("normal" as const),
                        stretch: { min: 100, max: 100 },
                    },
                };
            }),
            "local",
        );
    }

    /** List CSS's generic families, each drawing every weight, upright and slanted. */
    static generic(): Typeface[] {
        const every = { min: 1, max: 1000 };
        const normal = { min: 100, max: 100 };

        return GENERIC_FAMILIES.map(
            (family) =>
                new Typeface(family, "generic", [
                    { weight: every, style: "normal", stretch: normal },
                    { weight: every, style: "italic", stretch: normal },
                ]),
        );
    }

    /** Tell whether any face draws a range of weights, as a variable font does. */
    get isVariable(): boolean {
        return this.faces.some((face) => face.weight.min < face.weight.max);
    }

    /** List the weights it draws in a style: static faces' own, and named ones a range covers. */
    weights(style: FontStyle): FontWeight[] {
        // take the faces of the style, else the upright ones the browser slants
        const faces = this.faces.filter((face) => face.style === style);
        const shown =
            faces.length > 0 ? faces : this.faces.filter((face) => face.style === "normal");

        // name the weights each face draws, without repeats, lightest first
        const named = Object.values(FONT_WEIGHTS).map((each) => each.weight);
        const weights = shown.flatMap((face) =>
            face.weight.min === face.weight.max
                ? [face.weight.min]
                : named.filter((weight) => weight >= face.weight.min && weight <= face.weight.max),
        );

        return [...new Set(weights)].toSorted((left, right) => left - right);
    }

    /** Tell whether a face draws a weight in a style, so the browser need not synthesize it. */
    has(weight: FontWeight, style: FontStyle): boolean {
        return this.faces.some(
            (face) =>
                face.style === style && weight >= face.weight.min && weight <= face.weight.max,
        );
    }

    /** Group faces by family, families in alphabetical order. */
    static #group(
        entries: readonly { readonly family: string; readonly face: Face }[],
        source: TypefaceSource,
    ): Typeface[] {
        const families = Map.groupBy(entries, (entry) => entry.family);

        return [...families]
            .map(
                ([family, faces]) =>
                    new Typeface(
                        family,
                        source,
                        faces.map((each) => each.face),
                    ),
            )
            .toSorted((left, right) => left.family.localeCompare(right.family));
    }
}

/** Read a `font-weight` descriptor: a weight, a range, or a keyword. */
function weightOf(descriptor: string): FaceRange {
    const values = descriptor
        .trim()
        .split(/\s+/u)
        .map((word) => (word === "bold" ? 700 : word === "normal" ? 400 : Number(word)));

    return rangeOf(values, 400);
}

/** Read a `font-style` descriptor's slant, an oblique angle or range read as oblique. */
function styleOf(descriptor: string): FontStyle {
    const keyword = descriptor.trim().split(/\s+/u)[0];

    return keyword === "italic" || keyword === "oblique" ? keyword : "normal";
}

/** Read a `font-stretch` descriptor: a percentage, a range, or a keyword. */
function stretchOf(descriptor: string): FaceRange {
    const values = descriptor
        .trim()
        .split(/\s+/u)
        .map((word) => STRETCHES[word] ?? Number.parseFloat(word));

    return rangeOf(values, 100);
}

/** Read one or two numbers as a range, the normal value where none reads. */
function rangeOf(values: readonly number[], normal: number): FaceRange {
    const numbers = values.filter((value) => Number.isFinite(value));
    const min = numbers[0] ?? normal;

    return { min, max: numbers[1] ?? min };
}
