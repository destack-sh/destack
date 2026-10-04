import { Color } from "./color.ts";

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

/** Measure the APCA-W3 0.0.98G lightness contrast Lc of text on a background: positive for dark text on light, negative for light on dark. */
export function apca(foreground: string, background: string): number {
    // soft-clamp both luminances near black
    const text = clamp(luminance(foreground));
    const ground = clamp(luminance(background));
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

/** Estimate the screen luminance APCA reads from a six-digit sRGB hex color. */
function luminance(hex: string): number {
    const [red, green, blue] = Color.channels(hex);

    return RED * red ** MAIN_TRC + GREEN * green ** MAIN_TRC + BLUE * blue ** MAIN_TRC;
}

/** Soft-clamp a luminance near black. */
function clamp(value: number): number {
    return value > BLACK_THRESHOLD ? value : value + (BLACK_THRESHOLD - value) ** BLACK_CLAMP;
}
