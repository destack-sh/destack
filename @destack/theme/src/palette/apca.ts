import type { Srgb } from "./color.ts";

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

/** Measure the APCA-W3 0.0.98G lightness contrast Lc of text on a background. */
export function apca(foreground: Srgb, background: Srgb): number {
    // soft-clamp both luminances near black
    const text = clamp(foreground.luminance);
    const ground = clamp(background.luminance);
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
