import { expect, test } from "@destack/test";
import { apca, Color, Palette } from "./index.ts";
import { PRESETS } from "../preset/index.ts";

/** Seeds across hues, lightness and chroma. */
const SEEDS = ["#3e63dd", "#f76b15", "#ffc53d", "#46a758", "#ff0000", "#7c3aed", "#888888"];

/** Lightness from dark to near white in tenths, above the black 8-bit channels round coarsely. */
const LIGHTNESS = [0.15, 0.25, 0.35, 0.45, 0.55, 0.65, 0.75, 0.85, 0.95];

test("measure the APCA-W3 reference contrasts", () => {
    // the pairs and values the apca-w3 0.0.98G package documents
    expect(apca("#888888", "#ffffff")).toBeCloseTo(63.056469930209424, 10);
    expect(apca("#ffffff", "#888888")).toBeCloseTo(-68.54146436644962, 10);
    expect(apca("#000000", "#aaaaaa")).toBeCloseTo(58.146262578561334, 10);
    expect(apca("#aaaaaa", "#000000")).toBeCloseTo(-56.24113336839742, 10);
    expect(apca("#112233", "#ddeeff")).toBeCloseTo(91.66830811481631, 10);
    expect(apca("#ddeeff", "#112233")).toBeCloseTo(-93.06770049484275, 10);
    expect(apca("#777777", "#777777")).toBe(0);
});

test("roundtrip every preset seed through OKLCH", () => {
    expect(Object.values(PRESETS).filter((hex) => Color.parse(hex).hex() !== hex)).toEqual([]);
});

test("keep the seed at its own lightness and its lightness and hue at every other", () => {
    for (const seed of SEEDS) {
        const palette = new Palette(seed);
        const solid = Color.parse(seed);

        // each tone sits at its lightness, within the rounding to 8-bit channels
        expect(palette.tone(palette.lightness)).toBe(seed);
        const tones = LIGHTNESS.map((lightness) => Color.parse(palette.tone(lightness)));
        expect(
            tones.filter(
                (tone, index) => Math.abs(tone.lightness - (LIGHTNESS[index] ?? 0)) > 0.004,
            ),
        ).toEqual([]);

        // a colorful seed's tones keep its hue, measured in OKLab units
        const drift = tones.map((tone) => {
            const angle = ((tone.hue - solid.hue) * Math.PI) / 180;

            return Math.abs(2 * Math.sqrt(tone.chroma * solid.chroma) * Math.sin(angle / 2));
        });
        expect(solid.chroma < 0.05 || Math.max(...drift) < 0.006).toBe(true);
    }
});

/** Check that a color reads on white at a contrast. */
function reads(contrast: number): (color: string) => boolean {
    return (color) => Math.abs(apca(color, "#ffffff")) >= contrast;
}

test("fall the seed's chroma toward white and black", () => {
    const palette = new Palette("#3e63dd");
    const chroma = [0.1, 0.3, palette.lightness, 0.8, 0.98].map(
        (lightness) => Color.parse(palette.tone(lightness)).chroma,
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
    expect([darker < start, Math.abs(apca(palette.tone(darker), "#ffffff")) >= 90]).toEqual([
        true,
        true,
    ]);
    expect(palette.nearest(start, reads(90), "lighter")).toBeUndefined();
});

test("grow a preset's palette from its seed", () => {
    expect([Palette.of("indigo").seed, Palette.of("#7C3AED").seed]).toEqual([
        PRESETS.indigo,
        "#7c3aed",
    ]);
});
