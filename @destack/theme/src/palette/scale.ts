import { aligned } from "@destack/schema";
import { apca } from "./apca.ts";
import { Color } from "./color.ts";
import { PRESETS, type Preset } from "../radix/index.ts";

/** The steps of a scale. */
export const STEPS = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12] as const;

/** The OKLCH targets of generated light steps, averaged over the Radix Colors 3.0.0 accent scales. */
const LIGHT_TARGET: Target = {
    background: [0.993, 0.982, 0.959, 0.931, 0.9, 0.86, 0.807, 0.736],
    hover: -0.029,
    text: [0.538, 0.335],
    chroma: [0.02, 0.08, 0.19, 0.3, 0.39, 0.48, 0.58, 0.75, 1, 0.99, 0.89, 0.46],
};

/** The OKLCH targets of generated dark steps, averaged over the Radix Colors 3.0.0 accent scales. */
const DARK_TARGET: Target = {
    background: [0.187, 0.211, 0.264, 0.305, 0.348, 0.396, 0.456, 0.532],
    hover: 0.04,
    text: [0.795, 0.915],
    chroma: [0.1, 0.14, 0.3, 0.43, 0.49, 0.53, 0.59, 0.7, 1, 0.96, 0.87, 0.38],
};

/** The light label candidate on solid steps. */
const WHITE = "#ffffff";

/** The OKLCH lightness change of each search step when shifting step 9 for its label, below one 8-bit step. */
const SHIFT_STEP = 0.001;

/** A step of a scale, with Radix semantics: 1–2 backgrounds, 3–5 component fills, 6–8 borders, 9–10 solid, 11–12 text. */
export type Step = (typeof STEPS)[number];

/** The light or dark appearance a scale step resolves in. */
export type Scheme = "light" | "dark";

/** Twelve colors of one hue from background to text, for the light and the dark appearance. */
export class Scale {
    /** The light steps 1 through 12 as six-digit sRGB hex colors. */
    readonly light: readonly string[];
    /** The dark steps 1 through 12 as six-digit sRGB hex colors. */
    readonly dark: readonly string[];

    /** Hold the twelve light and dark steps. */
    constructor(light: readonly string[], dark: readonly string[]) {
        if (light.length !== STEPS.length || dark.length !== STEPS.length) {
            throw new RangeError(`a scale has ${STEPS.length} light and dark steps`);
        }
        this.light = light;
        this.dark = dark;
    }

    /** Read a preset scale. */
    static preset(name: Preset): Scale {
        const { light, dark } = PRESETS[name];

        return new Scale(light, dark);
    }

    /** Generate a scale around a seed color in OKLCH: the seed is step 9 of both appearances, at its hue. */
    static generate(seed: string): Scale {
        // read the seed's lightness, chroma and hue
        const solid = Color.parse(seed);

        // place each step at its target lightness and relative chroma, with the seed as step 9
        const generate = (target: Target) => {
            const lightness = [
                ...target.background,
                solid.lightness,
                solid.lightness + target.hover,
                ...target.text,
            ];

            return STEPS.map((step, index) => {
                const chroma = solid.chroma * aligned(target.chroma, index);

                return step === 9
                    ? seed.toLowerCase()
                    : new Color(aligned(lightness, index), chroma, solid.hue).hex();
            });
        };

        return new Scale(generate(LIGHT_TARGET), generate(DARK_TARGET));
    }

    /** Pick the label that reads best on a background: white, this scale's darkest step or another dark text color. */
    label(background: string, text: string): string {
        // measure each candidate on the background and keep the strongest
        const candidates = [WHITE, this.color(12, "light"), text];
        const reading = (candidate: string) => Math.abs(apca(candidate, background));

        return candidates.reduce((best, candidate) =>
            reading(candidate) > reading(best) ? candidate : best,
        );
    }

    /** Shift step 9 by the least OKLCH lightness that lets its best label read at a contrast, step 10 following. */
    legible(text: string, contrast: number): Scale {
        // keep a scale whose best label already reads in both appearances
        const reads = (scale: Scale) =>
            (["light", "dark"] as const).every((scheme) => {
                const solid = scale.color(9, scheme);

                return Math.abs(apca(scale.label(solid, text), solid)) >= contrast;
            });
        if (reads(this)) {
            return this;
        }

        // try darker and lighter step 9 colors at growing distance, darker first
        for (let count = 1; count * SHIFT_STEP < 1; count++) {
            for (const shift of [-count * SHIFT_STEP, count * SHIFT_STEP]) {
                const shifted = this.#shift(shift);
                if (reads(shifted)) {
                    return shifted;
                }
            }
        }
        throw new RangeError(`no step 9 lightness lets a label read at Lc ${contrast}`);
    }

    /** Shift step 9's OKLCH lightness in each appearance and derive step 10 from it. */
    #shift(shift: number): Scale {
        const steps = (scheme: Scheme, target: Target) => {
            // move step 9 by the shift and place step 10 at its hover lightness
            const solid = Color.parse(this.color(9, scheme));
            const lightness = solid.lightness + shift;
            const chroma = solid.chroma * aligned(target.chroma, 9);
            const shifted = new Color(lightness, solid.chroma, solid.hue).hex();
            const hover = new Color(lightness + target.hover, chroma, solid.hue).hex();

            // keep every other step
            const current = scheme === "light" ? this.light : this.dark;

            return [...current.slice(0, 8), shifted, hover, ...current.slice(10)];
        };

        return new Scale(steps("light", LIGHT_TARGET), steps("dark", DARK_TARGET));
    }

    /** Read a step in one appearance. */
    color(step: Step, scheme: Scheme): string {
        return aligned(scheme === "light" ? this.light : this.dark, step - 1);
    }
}

/** The OKLCH targets of one appearance's generated steps. */
interface Target {
    /** The lightness of steps 1 through 8. */
    readonly background: readonly number[];
    /** The lightness of step 10 relative to step 9. */
    readonly hover: number;
    /** The lightness of steps 11 and 12. */
    readonly text: readonly number[];
    /** The chroma of steps 1 through 12 relative to step 9. */
    readonly chroma: readonly number[];
}
