import { expect, test } from "@destack/test";
import { Typeface } from "./index.ts";

test("group the document's faces by family, a variable one by its weight range", () => {
    const typefaces = Typeface.document([
        { family: '"IBM Plex Mono"', weight: "700", style: "normal", stretch: "normal" },
        { family: "Inter", weight: "100 900", style: "normal", stretch: "75% 125%" },
        { family: '"IBM Plex Mono"', weight: "400", style: "normal", stretch: "normal" },
        { family: "Inter", weight: "bold", style: "oblique 10deg", stretch: "condensed" },
    ]);

    expect(typefaces).toEqual([
        new Typeface("IBM Plex Mono", "document", [
            { weight: { min: 700, max: 700 }, style: "normal", stretch: { min: 100, max: 100 } },
            { weight: { min: 400, max: 400 }, style: "normal", stretch: { min: 100, max: 100 } },
        ]),
        new Typeface("Inter", "document", [
            { weight: { min: 100, max: 900 }, style: "normal", stretch: { min: 75, max: 125 } },
            { weight: { min: 700, max: 700 }, style: "oblique", stretch: { min: 75, max: 75 } },
        ]),
    ]);
});

test("list the weights a family draws: a static one's own, a variable one's named weights", () => {
    const [mono, inter] = Typeface.document([
        { family: "Mono", weight: "600", style: "normal", stretch: "normal" },
        { family: "Mono", weight: "400", style: "normal", stretch: "normal" },
        { family: "Sans", weight: "300 700", style: "normal", stretch: "normal" },
    ]);

    expect([
        mono?.weights("normal"),
        mono?.weights("italic"),
        inter?.weights("normal"),
        inter?.isVariable,
        mono?.has(500, "normal"),
        inter?.has(500, "normal"),
    ]).toEqual([[400, 600], [400, 600], [300, 400, 500, 600, 700], true, false, true]);
});

test("read the weight and slant of the device's fonts from the names of their styles", () => {
    const [family] = Typeface.local([
        { family: "Avenir Next", style: "Demi Bold Italic" },
        { family: "Avenir Next", style: "Ultra Light" },
        { family: "Avenir Next", style: "Italic" },
        { family: "Avenir Next", style: "Heavy" },
    ]);

    expect(family?.faces.map((face) => [face.weight.min, face.style])).toEqual([
        [600, "italic"],
        [200, "normal"],
        [400, "italic"],
        [900, "normal"],
    ]);
});
