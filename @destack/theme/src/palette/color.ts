import { defineSchema, schema } from "@destack/schema";

/** Three channels of one color. */
export type Vector = readonly [number, number, number];

/** A 3x3 color space conversion matrix. */
type Matrix = readonly [Vector, Vector, Vector];

/** The linear sRGB to LMS matrix of OKLab, after Björn Ottosson's reference implementation (2020). */
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

/** The linear channel tolerance a color keeps inside the sRGB gamut, below one 8-bit step. */
const GAMUT_TOLERANCE = 0.0001;

/** The halvings of the chroma search, enough for a chroma step below 1e-6. */
const GAMUT_ITERATIONS = 20;

/** The pattern of a six-digit sRGB hex color. */
const HEX = /^#[0-9a-f]{6}$/iu;

/** A six-digit sRGB hex color such as `#3e63dd`, in either letter case. */
export const HexColor = defineSchema(schema.string().regex(HEX));

/** A color in OKLCH: lightness from 0 to 1, chroma from 0, hue in degrees. */
export class Color {
    /** The perceptual lightness from 0 (black) to 1 (white). */
    readonly lightness: number;
    /** The colorfulness from 0 (gray), below 0.4 in sRGB. */
    readonly chroma: number;
    /** The hue angle in degrees from 0 to 360. */
    readonly hue: number;

    /** Hold an OKLCH color. */
    constructor(lightness: number, chroma: number, hue: number) {
        this.lightness = lightness;
        this.chroma = chroma;
        this.hue = hue;
    }

    /** Convert a six-digit sRGB hex color such as `#3e63dd` to OKLCH. */
    static parse(hex: string): Color {
        // decode the gamma-encoded channels to linear light
        const rgb = apply(Color.channels(hex), linear);

        // convert linear sRGB to OKLab through cone responses
        const lms = apply(multiply(LMS_FROM_RGB, rgb), Math.cbrt);
        const [lightness, greenRed, blueYellow] = multiply(LAB_FROM_LMS, lms);
        const hue = (Math.atan2(blueYellow, greenRed) * 180) / Math.PI;

        return new Color(lightness, Math.hypot(greenRed, blueYellow), (hue + 360) % 360);
    }

    /** Decode a six-digit sRGB hex color to its gamma-encoded channels from 0 to 1. */
    static channels(hex: string): Vector {
        // refuse anything but six hex digits
        if (!HEX.test(hex)) {
            throw new RangeError(`invalid hex color: ${hex}`);
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

    /** Convert to a six-digit sRGB hex color, reducing chroma at the same lightness and hue until it fits sRGB. */
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
        return Color.format(
            apply(this.rgb(high), (channel) => gamma(Math.min(1, Math.max(0, channel)))),
        );
    }

    /** Convert to linear sRGB at a chroma. */
    rgb(chroma: number): Vector {
        // convert OKLCH to OKLab, then through cone responses to linear sRGB
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
