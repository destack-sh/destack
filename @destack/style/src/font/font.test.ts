import { expect, test } from "@destack/test";
import { FontFamily, FontFeatures, FontVariations, FontWeight } from "./index.ts";

test("read and write a CSS font family list, quoting names that are no identifier", () => {
    const families = FontFamily.read(`"Inter Variable", Inter, , sans-serif`);

    expect({
        families,
        css: FontFamily.css(families),
        weights: [400, 0, 1001, 450.5].map((weight) => FontWeight.safeParse(weight).success),
    }).toEqual({
        families: ["Inter Variable", "Inter", "sans-serif"],
        css: `"Inter Variable", Inter, sans-serif`,
        weights: [true, false, false, false],
    });
});

test("write OpenType features and font axes as CSS's quoted lists, normal for none", () => {
    expect([
        FontFeatures.css({ tnum: 1, liga: 0 }),
        FontVariations.css({ wght: 650 }),
        FontFeatures.css({}),
        FontFeatures.safeParse({ toolong: 1 }).success,
    ]).toEqual([`"tnum" 1, "liga" 0`, `"wght" 650`, "normal", false]);
});
