import { schema } from "@destack/schema";

/** Three channels of one color. */
export type Vector = readonly [number, number, number];

/** A 3x3 color space conversion matrix. */
type Matrix = readonly [Vector, Vector, Vector];

/** The linear sRGB to LMS matrix of OKLab, after Björn Ottosson's implementation (2020). */
const LMS_FROM_RGB: Matrix = [
    [0.4122214708, 0.5363325363, 0.0514459929],
    [0.2119034982, 0.6806995451, 0.1073969566],
    [0.0883024619, 0.2817188376, 0.6299787005],
];

/** The cube-root LMS to OKLab matrix, after Björn Ottosson's reference implementation (2020). */
const LAB_FROM_LMS: Matrix = [
    [0.2104542553, 0.793617785, -0.0040720468],
    [1.9779984951, -2.428592205, 0.4505937099],
    [0.0259040371, 0.7827717662, -0.808675766],
];

/** The OKLab to cube-root LMS matrix, after Björn Ottosson's reference implementation (2020). */
const LMS_FROM_LAB: Matrix = [
    [1, 0.3963377774, 0.2158037573],
    [1, -0.1055613458, -0.0638541728],
    [1, -0.0894841775, -1.291485548],
];

/** The LMS to linear sRGB matrix, after Björn Ottosson's reference implementation (2020). */
const RGB_FROM_LMS: Matrix = [
    [4.0767416621, -3.3077115913, 0.2309699292],
    [-1.2684380046, 2.6097574011, -0.3413193965],
    [-0.0041960863, -0.7034186147, 1.707614701],
];

/** The linear channel tolerance of a color inside the sRGB gamut, below one 8-bit step. */
const GAMUT_TOLERANCE = 0.0001;

/** The halvings of the chroma search, enough for a chroma step below 1e-6. */
const GAMUT_ITERATIONS = 20;

/** The sRGB channel exponent of APCA-W3 0.0.98G. */
const MAIN_TRC = 2.4;

/** The red luminance coefficient of APCA-W3 0.0.98G. */
const RED = 0.2126729;

/** The green luminance coefficient of APCA-W3 0.0.98G. */
const GREEN = 0.7151522;

/** The blue luminance coefficient of APCA-W3 0.0.98G. */
const BLUE = 0.072175;

/** The background exponent for dark text on a light background, APCA-W3 0.0.98G `normBG`. */
const NORMAL_BACKGROUND = 0.56;
/** The text exponent for dark text on a light background, APCA-W3 0.0.98G `normTXT`. */
const NORMAL_TEXT = 0.57;
/** The text exponent for light text on a dark background, APCA-W3 0.0.98G `revTXT`. */
const REVERSE_TEXT = 0.62;
/** The background exponent for light text on a dark background, APCA-W3 0.0.98G `revBG`. */
const REVERSE_BACKGROUND = 0.65;

/** The luminance below which APCA soft-clamps near-black, APCA-W3 0.0.98G `blkThrs`. */
const BLACK_THRESHOLD = 0.022;
/** The exponent of the near-black soft clamp, APCA-W3 0.0.98G `blkClmp` of 1.414. */
const BLACK_CLAMP = 1414 / 1000;
/** The output scale of both polarities, APCA-W3 0.0.98G `scaleBoW` and `scaleWoB`. */
const SCALE = 1.14;
/** The output offset of both polarities, APCA-W3 0.0.98G `loBoWoffset` and `loWoBoffset`. */
const OFFSET = 0.027;
/** The output magnitude below which contrast reads as zero, APCA-W3 0.0.98G `loClip`. */
const LOW_CLIP = 0.1;
/** The luminance difference below which contrast reads as zero, APCA-W3 0.0.98G `deltaYmin`. */
const DELTA_MIN = 0.0005;

/** The WCAG 2.2 relative luminance coefficients of red, green and blue. */
const RELATIVE = [0.2126, 0.7152, 0.0722] as const;

/** The flare WCAG 2.2 adds to both luminances of a contrast ratio. */
const FLARE = 0.05;

/** The pattern of a six-digit sRGB hex color. */
const HEX = /^#[0-9a-f]{6}$/iu;

/** A six-digit sRGB hex color such as `#3e63dd`, in either letter case. */
export const HexColor = schema.string().regex(HEX);

/** A color in OKLCH: lightness from 0 to 1, chroma from 0, hue in degrees. */
export class Oklch {
    /** The perceptual lightness from 0 (black) to 1 (white). */
    readonly lightness: number;
    /** The colorfulness from 0 (gray), below 0.4 in sRGB. */
    readonly chroma: number;
    /** The hue angle in degrees from 0 to 360. */
    readonly hue: number;

    /** Create an OKLCH color. */
    constructor(lightness: number, chroma: number, hue: number) {
        this.lightness = lightness;
        this.chroma = chroma;
        this.hue = hue;
    }

    /** Convert a six-digit sRGB hex color such as `#3e63dd` to OKLCH. */
    static parse(hex: string): Oklch {
        // decode the gamma-encoded channels to linear light
        const rgb = apply(Srgb.channels(hex), linear);

        // convert linear sRGB to OKLab through cone responses
        const lms = apply(multiply(LMS_FROM_RGB, rgb), Math.cbrt);
        const [lightness, greenRed, blueYellow] = multiply(LAB_FROM_LMS, lms);
        const hue = (Math.atan2(blueYellow, greenRed) * 180) / Math.PI;

        return new Oklch(lightness, Math.hypot(greenRed, blueYellow), (hue + 360) % 360);
    }

    /** Convert to a six-digit sRGB hex color. */
    hex(): string {
        // search the largest displayable chroma
        let low = 0;
        let high = this.chroma;
        if (!isDisplayable(this.rgb(high))) {
            for (let iteration = 0; iteration < GAMUT_ITERATIONS; iteration++) {
                const middle = (low + high) / 2;
                if (isDisplayable(this.rgb(middle))) {
                    low = middle;
                } else {
                    high = middle;
                }
            }
            high = low;
        }

        // clamp and gamma-encode each channel
        return Srgb.format(
            apply(this.rgb(high), (channel) => gamma(Math.min(1, Math.max(0, channel)))),
        );
    }

    /** Convert to linear sRGB at a chroma. */
    rgb(chroma: number): Vector {
        // convert OKLCH through OKLab and cone responses to linear sRGB
        const radians = (this.hue * Math.PI) / 180;
        const lab: Vector = [
            this.lightness,
            chroma * Math.cos(radians),
            chroma * Math.sin(radians),
        ];
        const lms = apply(multiply(LMS_FROM_LAB, lab), (value) => value ** 3);

        return multiply(RGB_FROM_LMS, lms);
    }
}

/** A measure of contrast: APCA-W3's lightness contrast Lc, or WCAG 2.2's ratio. */
export type ContrastMeasure = "apca" | "wcag";

/** An opaque sRGB color: its six-digit hex form and the luminances contrast measures read. */
export class Srgb {
    /** The lowercase six-digit hex form, such as `#3e63dd`. */
    readonly hex: string;
    /** The screen luminance APCA-W3 0.0.98G estimates, from 0 to 1. */
    readonly luminance: number;
    /** The relative luminance WCAG 2.2 defines, from 0 to 1. */
    readonly relativeLuminance: number;

    /** Create a color with its luminances. */
    private constructor(hex: string, luminance: number, relativeLuminance: number) {
        this.hex = hex;
        this.luminance = luminance;
        this.relativeLuminance = relativeLuminance;
    }

    /** Read a six-digit sRGB hex color, computing its luminances once. */
    static parse(hex: string): Srgb {
        // estimate the screen luminance as APCA does
        const [red, green, blue] = Srgb.channels(hex);
        const luminance =
            RED * red ** MAIN_TRC + GREEN * green ** MAIN_TRC + BLUE * blue ** MAIN_TRC;

        // compute the relative luminance from the linear channels as WCAG does
        const relativeLuminance =
            RELATIVE[0] * linear(red) + RELATIVE[1] * linear(green) + RELATIVE[2] * linear(blue);

        return new Srgb(hex.toLowerCase(), luminance, relativeLuminance);
    }

    /** Decode a six-digit sRGB hex color to its gamma-encoded channels from 0 to 1. */
    static channels(hex: string): Vector {
        // refuse anything but six hex digits
        if (!HEX.test(hex)) {
            throw new RangeError(`invalid hex color: '${hex}'`);
        }

        // read each two-digit channel
        const channel = (offset: number) =>
            Number.parseInt(hex.slice(offset, offset + 2), 16) / 255;

        return [channel(1), channel(3), channel(5)];
    }

    /** Encode gamma-encoded channels from 0 to 1 as a six-digit sRGB hex color. */
    static format(channels: Vector): string {
        const digits = channels.map((channel) => byte(channel));

        return `#${digits.join("")}`;
    }

    /** Append an opacity from 0 to 1 to a six-digit hex color as two more digits. */
    static translucent(hex: string, alpha: number): string {
        return `${hex}${byte(alpha)}`;
    }

    /** Blend a color over an opaque one in gamma-encoded sRGB, as CSS composites. */
    static composite(hex: string, under: string): string {
        // return an opaque color as it is
        if (HEX.test(hex)) {
            return hex;
        }

        // weigh each channel by the color's opacity
        const alpha = Number.parseInt(hex.slice(7, 9), 16) / 255;
        const over = Srgb.channels(hex.slice(0, 7));
        const ground = Srgb.channels(under);
        const blend = (index: 0 | 1 | 2) => over[index] * alpha + ground[index] * (1 - alpha);

        return Srgb.format([blend(0), blend(1), blend(2)]);
    }

    /** Measure this color's contrast as text on a background, after Color.js's `contrast`. */
    contrast(background: Srgb, measure: ContrastMeasure): number {
        return measure === "apca"
            ? lightnessContrast(this.luminance, background.luminance)
            : contrastRatio(this.relativeLuminance, background.relativeLuminance);
    }
}

/** Multiply a 3x3 matrix by a vector. */
function multiply(matrix: Matrix, vector: Vector): Vector {
    const [x, y, z] = vector;
    const dot = ([first, second, third]: Vector) => first * x + second * y + third * z;

    return [dot(matrix[0]), dot(matrix[1]), dot(matrix[2])];
}

/** Apply a function to each channel. */
function apply(vector: Vector, transform: (channel: number) => number): Vector {
    return [transform(vector[0]), transform(vector[1]), transform(vector[2])];
}

/** Encode a value from 0 to 1 as two hex digits. */
function byte(value: number): string {
    return Math.round(value * 255)
        .toString(16)
        .padStart(2, "0");
}

/** Decode an sRGB channel to linear light, per IEC 61966-2-1. */
function linear(channel: number): number {
    return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
}

/** Encode a linear channel to sRGB, per IEC 61966-2-1. */
function gamma(channel: number): number {
    return channel <= 0.0031308 ? channel * 12.92 : 1.055 * channel ** (1 / 2.4) - 0.055;
}

/** Check that linear channels lie inside the sRGB gamut. */
function isDisplayable(rgb: Vector): boolean {
    return rgb.every((channel) => channel >= -GAMUT_TOLERANCE && channel <= 1 + GAMUT_TOLERANCE);
}

/** Measure the APCA-W3 0.0.98G lightness contrast Lc of text on a background by luminance. */
function lightnessContrast(foreground: number, background: number): number {
    // soft-clamp both luminances near black
    const text = clamp(foreground);
    const ground = clamp(background);
    if (Math.abs(ground - text) < DELTA_MIN) {
        return 0;
    }

    // measure dark text on a light background
    if (ground > text) {
        const contrast = (ground ** NORMAL_BACKGROUND - text ** NORMAL_TEXT) * SCALE;

        return contrast < LOW_CLIP ? 0 : (contrast - OFFSET) * 100;
    }
    // measure light text on a dark background
    else {
        const contrast = (ground ** REVERSE_BACKGROUND - text ** REVERSE_TEXT) * SCALE;

        return contrast > -LOW_CLIP ? 0 : (contrast + OFFSET) * 100;
    }
}

/** Soft-clamp a luminance near black. */
function clamp(value: number): number {
    return value > BLACK_THRESHOLD ? value : value + (BLACK_THRESHOLD - value) ** BLACK_CLAMP;
}

/** Measure the WCAG 2.2 contrast ratio of two relative luminances, from 1 to 21. */
function contrastRatio(first: number, second: number): number {
    return (Math.max(first, second) + FLARE) / (Math.min(first, second) + FLARE);
}
