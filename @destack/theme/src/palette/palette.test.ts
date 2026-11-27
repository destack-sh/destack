import { expect, test } from "@destack/test";
import { Color } from "@destack/style";
import { Palette } from "./index.ts";

/** Seeds across hues, lightness and chroma. */
const SEEDS = ["#3e63dd", "#f76b15", "#ffc53d", "#46a758", "#ff0000", "#7c3aed", "#888888"];

/** The white the tones read on. */
const WHITE = Color.parse("#ffffff");

/** Lightness from dark to near white in tenths, above the black 8-bit channels round coarsely. */
const LIGHTNESS = [0.15, 0.25, 0.35, 0.45, 0.55, 0.65, 0.75, 0.85, 0.95];

test("grow the exact seed at its lightness, and its hue at every other lightness", () => {
    for (const seed of SEEDS) {
        const palette = new Palette(seed);
        const solid = oklch(seed);

        // each tone sits at its lightness, within the rounding to 8-bit channels
        expect(palette.tone(palette.lightness).toString("hex")).toBe(seed);
        const tones = LIGHTNESS.map((lightness) => oklch(palette.tone(lightness).toString("hex")));
        expect(
            tones.filter(
                (tone, index) => Math.abs(tone.lightness - (LIGHTNESS[index] ?? 0)) > 0.004,
            ),
        ).toEqual([]);

        // a colorful seed's tones have its hue, measured in OKLab units
        const drift = tones.map((tone) => {
            const angle = ((tone.hue - solid.hue) * Math.PI) / 180;

            return Math.abs(2 * Math.sqrt(tone.chroma * solid.chroma) * Math.sin(angle / 2));
        });
        expect(solid.chroma < 0.05 || Math.max(...drift) < 0.006).toBe(true);
    }
});

/** Check that a color reads on white at a contrast. */
function reads(contrast: number): (color: Color) => boolean {
    return (color) => Math.abs(color.contrast(WHITE, "apca")) >= contrast;
}

test("fall the seed's chroma toward white and black", () => {
    const palette = new Palette("#3e63dd");
    const chroma = [0.1, 0.3, palette.lightness, 0.8, 0.98].map(
        (lightness) => oklch(palette.tone(lightness).toString("hex")).chroma,
    );

    const [black = 0, dark = 0, , light = 0, white = 0] = chroma;

    expect([chroma.indexOf(Math.max(...chroma)), black < dark, light > white]).toEqual([
        2,
        true,
        true,
    ]);
});

test("find the nearest lightness whose tone passes, darker first and only the way asked", () => {
    const palette = new Palette("#3e63dd");
    const start = palette.lightness;

    // the seed already reads at Lc 60 on white, and Lc 90 needs a darker tone
    expect(palette.nearest(start, reads(60), "either")).toBe(start);
    const darker = palette.nearest(start, reads(90), "either") ?? 1;
    expect([darker < start, Math.abs(palette.tone(darker).contrast(WHITE, "apca")) >= 90]).toEqual([
        true,
        true,
    ]);
    expect(palette.nearest(start, reads(90), "lighter")).toBeUndefined();
});

test("lowercase a seed and reuse its palette", () => {
    const palette = Palette.of("#7C3AED");

    expect([palette.seed, Palette.of("#7C3AED") === palette]).toEqual(["#7c3aed", true]);
});

/** Read a hex color's OKLCH lightness from 0 to 1, chroma and hue. */
function oklch(hex: string): {
    readonly lightness: number;
    readonly chroma: number;
    readonly hue: number;
} {
    const [lightness, chroma, hue] = Color.parse(hex).to("oklch").values;

    return { lightness: lightness / 100, chroma, hue };
}
