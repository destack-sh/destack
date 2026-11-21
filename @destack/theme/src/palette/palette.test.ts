import { expect, test } from "@destack/test";
import { Oklch, Palette, Srgb } from "./index.ts";
import { PRESETS } from "@destack/color";

/** Seeds across hues, lightness and chroma. */
const SEEDS = ["#3e63dd", "#f76b15", "#ffc53d", "#46a758", "#ff0000", "#7c3aed", "#888888"];

/** The white the tones read on. */
const WHITE = Srgb.parse("#ffffff");

/** Lightness from dark to near white in tenths, above the black 8-bit channels round coarsely. */
const LIGHTNESS = [0.15, 0.25, 0.35, 0.45, 0.55, 0.65, 0.75, 0.85, 0.95];

test("measure the APCA-W3 reference contrasts", () => {
    // the pairs and values the apca-w3 0.0.98G package documents
    expect(Srgb.parse("#888888").contrast(Srgb.parse("#ffffff"), "apca")).toBeCloseTo(
        63.056469930209424,
        10,
    );
    expect(Srgb.parse("#ffffff").contrast(Srgb.parse("#888888"), "apca")).toBeCloseTo(
        -68.54146436644962,
        10,
    );
    expect(Srgb.parse("#000000").contrast(Srgb.parse("#aaaaaa"), "apca")).toBeCloseTo(
        58.146262578561334,
        10,
    );
    expect(Srgb.parse("#aaaaaa").contrast(Srgb.parse("#000000"), "apca")).toBeCloseTo(
        -56.24113336839742,
        10,
    );
    expect(Srgb.parse("#112233").contrast(Srgb.parse("#ddeeff"), "apca")).toBeCloseTo(
        91.66830811481631,
        10,
    );
    expect(Srgb.parse("#ddeeff").contrast(Srgb.parse("#112233"), "apca")).toBeCloseTo(
        -93.06770049484275,
        10,
    );
    expect(Srgb.parse("#777777").contrast(Srgb.parse("#777777"), "apca")).toBe(0);
});

test("measure the WCAG 2.2 reference ratios", () => {
    // the extremes, and the grays either side of AA's 4.5 to 1 on white
    const pairs = [
        ["#000000", "#ffffff"],
        ["#ffffff", "#000000"],
        ["#767676", "#ffffff"],
        ["#777777", "#ffffff"],
        ["#777777", "#777777"],
    ] as const;
    const ratios = pairs.map(([text, background]) =>
        Srgb.parse(text).contrast(Srgb.parse(background), "wcag").toFixed(2),
    );

    expect(ratios).toEqual(["21.00", "21.00", "4.54", "4.48", "1.00"]);
});

test("roundtrip every preset seed through OKLCH", () => {
    expect(Object.values(PRESETS).filter((hex) => Oklch.parse(hex).hex() !== hex)).toEqual([]);
});

test("grow the exact seed at its lightness, and its hue at every other lightness", () => {
    for (const seed of SEEDS) {
        const palette = new Palette(seed);
        const solid = Oklch.parse(seed);

        // each tone sits at its lightness, within the rounding to 8-bit channels
        expect(palette.tone(palette.lightness).hex).toBe(seed);
        const tones = LIGHTNESS.map((lightness) => Oklch.parse(palette.tone(lightness).hex));
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
function reads(contrast: number): (color: Srgb) => boolean {
    return (color) => Math.abs(color.contrast(WHITE, "apca")) >= contrast;
}

test("fall the seed's chroma toward white and black", () => {
    const palette = new Palette("#3e63dd");
    const chroma = [0.1, 0.3, palette.lightness, 0.8, 0.98].map(
        (lightness) => Oklch.parse(palette.tone(lightness).hex).chroma,
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

test("grow a preset's palette from its seed", () => {
    expect([Palette.of("indigo").seed, Palette.of("#7C3AED").seed]).toEqual([
        PRESETS.indigo,
        "#7c3aed",
    ]);
});
