import { Color, type ColorValues, HexColor } from "@destack/style";

/** The exponent chroma falls by from the seed's lightness toward white. */
const LIGHTER_FALLOFF = 0.5;

/** The OKLCH lightness of each step of a tone search, below one 8-bit step. */
const LIGHTNESS_STEP = 0.001;

/** The fine steps in each coarse stride of a tone search. */
const COARSE_STEPS = 16;

/** The palettes grown so far by seed, shared by every theme as pure functions of their seeds. */
const PALETTES = new Map<string, Palette>();

/** The light or dark appearance a color resolves in. */
export type ColorScheme = "light" | "dark";

/** The way a tone search moves from its start. */
export type Direction = "lighter" | "darker" | "either";

/** The colors of one hue at every lightness, grown from a seed in OKLCH. */
export class Palette {
    /** The seed as a lowercase six-digit sRGB hex color. */
    readonly seed: string;
    /** The seed's OKLCH lightness from 0 to 1, chroma and hue. */
    readonly #oklch: ColorValues;
    /** The tones read so far, by lightness. */
    readonly #tones = new Map<number, Color>();

    /** Grow a palette from a six-digit sRGB hex color. */
    constructor(seed: string) {
        this.seed = HexColor.parse(seed).toLowerCase();
        const [lightness, chroma, hue] = Color.parse(this.seed).to("oklch").values;
        this.#oklch = [lightness / 100, chroma, hue];
    }

    /** Read the palette of a hex seed, grown once. */
    static of(seed: string): Palette {
        // reuse the palette of a known seed
        const known = PALETTES.get(seed);
        if (known !== undefined) {
            return known;
        }

        // grow the palette
        const palette = new Palette(seed);
        PALETTES.set(seed, palette);

        return palette;
    }

    /** The seed's OKLCH lightness. */
    get lightness(): number {
        return this.#oklch[0];
    }

    /** The seed's OKLCH hue in degrees. */
    get hue(): number {
        return this.#oklch[2];
    }

    /** Read the color at an OKLCH lightness. */
    tone(lightness: number): Color {
        // reuse a tone read before
        const known = this.#tones.get(lightness);
        if (known !== undefined) {
            return known;
        }
        const tone = this.#grow(lightness);
        this.#tones.set(lightness, tone);

        return tone;
    }

    /** Grow the color at an OKLCH lightness from the seed. */
    #grow(lightness: number): Color {
        // return the seed exactly at its lightness
        const [seedLightness, seedChroma, seedHue] = this.#oklch;
        const clamped = Math.min(1, Math.max(0, lightness));
        if (Math.abs(clamped - seedLightness) < LIGHTNESS_STEP / 2) {
            return Color.parse(this.seed);
        }

        // scale the seed's chroma by the distance toward white or black
        const relative =
            clamped > seedLightness
                ? ((1 - clamped) / Math.max(1 - seedLightness, LIGHTNESS_STEP)) ** LIGHTER_FALLOFF
                : smoothstep(clamped / Math.max(seedLightness, LIGHTNESS_STEP));
        const grown = new Color("oklch", [clamped * 100, seedChroma * relative, seedHue]);

        // round the tone to the hex color a page shows
        return Color.parse(grown.toString("hex"));
    }

    /** Find the lightness nearest a start whose tone passes, absent when none does. */
    nearest(
        start: number,
        passes: (color: Color) => boolean,
        direction: Direction,
    ): number | undefined {
        // return a passing start, else take the closer of the first passing lightness each way
        if (passes(this.tone(start))) {
            return start;
        }
        const darker = direction === "lighter" ? undefined : this.#first(start, -1, passes);
        const lighter = direction === "darker" ? undefined : this.#first(start, 1, passes);
        if (darker === undefined || lighter === undefined) {
            return darker ?? lighter;
        }

        return start - darker <= lighter - start ? darker : lighter;
    }

    /** Find the first passing lightness one way from a start, by coarse strides then halving. */
    #first(start: number, sign: number, passes: (color: Color) => boolean): number | undefined {
        // stride from the start toward the bound
        const bound = sign < 0 ? 0 : 1;
        const stride = LIGHTNESS_STEP * COARSE_STEPS;
        for (let previous = start; previous !== bound;) {
            // stride until a tone passes
            const next =
                sign < 0 ? Math.max(bound, previous - stride) : Math.min(bound, previous + stride);
            if (!passes(this.tone(next))) {
                previous = next;
                continue;
            }

            // halve the fine steps back toward the start to the last that passes
            const steps = Math.round(Math.abs(next - previous) / LIGHTNESS_STEP);
            const at = (step: number): number => next - sign * step * LIGHTNESS_STEP;
            let [passing, failing] = [0, steps];
            while (failing - passing > 1) {
                const middle = Math.floor((passing + failing) / 2);
                [passing, failing] = passes(this.tone(at(middle)))
                    ? [middle, failing]
                    : [passing, middle];
            }

            return at(passing);
        }

        return undefined;
    }
}

/** Ease a ratio from 0 to 1 with a flat start and end, the Hermite curve 3t² − 2t³. */
function smoothstep(ratio: number): number {
    return ratio * ratio * (3 - 2 * ratio);
}
