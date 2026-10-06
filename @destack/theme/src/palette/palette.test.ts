import { expect, test } from "@destack/test";
import { apca, Color, Scale, STEPS, type Scheme, type Step } from "./index.ts";
import { ACCENT_SOLIDS, GRAY_SOLIDS, PRESET_NAMES } from "../preset/index.ts";

/** Seeds across hues, lightness and chroma. */
const SEEDS = ["#3e63dd", "#f76b15", "#ffc53d", "#46a758", "#ff0000", "#7c3aed", "#888888"];

/** The steps whose lightness orders from background to border. */
const BACKGROUNDS = STEPS.slice(0, 8);

/** The text steps, ordered from lower to higher contrast. */
const TEXT = STEPS.slice(10);

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

test("roundtrip every preset solid through OKLCH", () => {
    const solids = [
        ...Object.values(ACCENT_SOLIDS),
        ...Object.values(GRAY_SOLIDS).flatMap((gray) => [gray.light, gray.dark]),
    ];

    expect(solids.filter((hex) => Color.parse(hex).hex() !== hex)).toEqual([]);
});

test("order generated steps from background to text in both appearances", () => {
    for (const seed of SEEDS) {
        const scale = Scale.generate(seed);
        const lightness = (steps: readonly Step[], scheme: Scheme) =>
            steps.map((step) => Color.parse(scale.color(step, scheme)).lightness);

        // light steps darken from background to border and from step 11 to 12, dark steps lighten
        for (const steps of [BACKGROUNDS, TEXT]) {
            const light = lightness(steps, "light");
            const dark = lightness(steps, "dark");
            expect(light).toEqual(light.toSorted((left, right) => right - left));
            expect(dark).toEqual(dark.toSorted((left, right) => left - right));
            expect(new Set([...light, ...dark]).size).toBe(steps.length * 2);
        }
    }
});

test("keep the seed as both appearances' step 9 and its hue on every colored step", () => {
    for (const seed of SEEDS.filter((candidate) => Color.parse(candidate).chroma > 0.05)) {
        const scale = Scale.generate(seed);
        const solid = Color.parse(seed);
        const hue = solid.hue;

        // measure each step's hue difference in OKLab units, within the rounding to 8-bit channels
        expect([scale.color(9, "light"), scale.color(9, "dark")]).toEqual([seed, seed]);
        const drift = [...scale.light, ...scale.dark].map((hex) => {
            const color = Color.parse(hex);
            const angle = ((color.hue - hue) * Math.PI) / 180;

            return Math.abs(2 * Math.sqrt(color.chroma * solid.chroma) * Math.sin(angle / 2));
        });
        expect(Math.max(...drift)).toBeLessThan(0.006);
    }
});

test("grow every preset around its solids, ordering its steps from background to text", () => {
    const misordered = PRESET_NAMES.filter((name) => {
        // read the preset's lightness along its background and text steps
        const scale = Scale.preset(name);
        const lightness = (steps: readonly Step[], scheme: Scheme) =>
            steps.map((step) => Color.parse(scale.color(step, scheme)).lightness);

        // light steps darken and dark steps lighten
        return [BACKGROUNDS, TEXT].some((steps) => {
            const light = lightness(steps, "light");
            const dark = lightness(steps, "dark");

            return (
                light.join() !== light.toSorted((left, right) => right - left).join() ||
                dark.join() !== dark.toSorted((left, right) => left - right).join()
            );
        });
    });
    const solids = [
        Scale.preset("slate").color(9, "dark"),
        Scale.preset("indigo").color(9, "light"),
    ];

    expect([misordered, solids]).toEqual([[], [GRAY_SOLIDS.slate.dark, ACCENT_SOLIDS.indigo]]);
});
