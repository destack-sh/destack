import { schema } from "@destack/schema";
import { Color, HexColor, Srgb } from "./color.ts";
import { PRESETS, Preset } from "../preset/index.ts";

/** The exponent chroma falls by from the seed's lightness toward white. */
const LIGHTER_FALLOFF = 0.5;

/** The exponent chroma falls by from the seed's lightness toward black. */
const DARKER_FALLOFF = 2;

/** The OKLCH lightness of each step of a tone search, below one 8-bit step. */
const LIGHTNESS_STEP = 0.001;

/** The fine steps in each coarse stride of a tone search. */
const COARSE_STEPS = 16;

/** The palettes grown so far by seed, shared by every theme as pure functions of their seeds. */
const PALETTES = new Map<Seed, Palette>();

/** A palette's seed: a preset name or a six-digit sRGB hex color. */
export const Seed = schema.union([Preset, HexColor]);
/** A palette's seed. */
export type Seed = schema.Infer<typeof Seed>;

/** The light or dark appearance a color resolves in. */
export type Scheme = "light" | "dark";

/** The way a tone search moves from its start. */
export type Direction = "lighter" | "darker" | "either";

/** The colors of one hue at every lightness, grown from a seed in OKLCH. */
export class Palette {
    /** The seed as a lowercase six-digit sRGB hex color. */
    readonly seed: string;
    /** The seed in OKLCH. */
    readonly #color: Color;
    /** The tones read so far, by lightness. */
    readonly #tones = new Map<number, Srgb>();

    /** Grow a palette from a six-digit sRGB hex color. */
    constructor(seed: string) {
        this.seed = HexColor.parse(seed).toLowerCase();
        this.#color = Color.parse(this.seed);
    }

    /** Grow a palette from a preset or a hex seed. */
    static of(seed: Seed): Palette {
        // reuse the palette of a known seed
        const known = PALETTES.get(seed);
        if (known !== undefined) {
            return known;
        }

        // grow the palette of a preset's color or of a hex seed
        const preset = Preset.safeParse(seed);
        const palette = new Palette(preset.success ? PRESETS[preset.data] : seed);
        PALETTES.set(seed, palette);

        return palette;
    }

    /** The seed's OKLCH lightness. */
    get lightness(): number {
        return this.#color.lightness;
    }

    /** The seed's OKLCH hue in degrees. */
    get hue(): number {
        return this.#color.hue;
    }

    /** Read the color at an OKLCH lightness. */
    tone(lightness: number): Srgb {
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
    #grow(lightness: number): Srgb {
        // keep the seed exact at its own lightness
        const seed = this.#color;
        const clamped = Math.min(1, Math.max(0, lightness));
        if (Math.abs(clamped - seed.lightness) < LIGHTNESS_STEP / 2) {
            return Srgb.parse(this.seed);
        }

        // scale the seed's chroma by the distance toward white or black
        const relative =
            clamped > seed.lightness
                ? ((1 - clamped) / Math.max(1 - seed.lightness, LIGHTNESS_STEP)) ** LIGHTER_FALLOFF
                : (clamped / Math.max(seed.lightness, LIGHTNESS_STEP)) ** DARKER_FALLOFF;

        return Srgb.parse(new Color(clamped, seed.chroma * relative, seed.hue).hex());
    }

    /** Find the lightness nearest a start whose tone passes, absent when none does. */
    nearest(
        start: number,
        passes: (color: Srgb) => boolean,
        direction: Direction,
    ): number | undefined {
        // keep a passing start, else take the closer of the first passing lightness each way
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

    /** Find the first passing lightness one way from a start, in coarse strides then fine steps. */
    #first(start: number, sign: number, passes: (color: Srgb) => boolean): number | undefined {
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

            // walk back toward the start while the tones still pass
            let first = next;
            for (
                let lightness = next - sign * LIGHTNESS_STEP;
                sign * (lightness - previous) > 0 && passes(this.tone(lightness));
                lightness -= sign * LIGHTNESS_STEP
            ) {
                first = lightness;
            }

            return first;
        }

        return undefined;
    }
}
