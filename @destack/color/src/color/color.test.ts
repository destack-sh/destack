import { expect, test } from "@destack/test";
import { Color, type ColorSpace } from "./index.ts";

/** The colors that go through every space: each corner of the cube, its middle and some between. */
const COLORS = [
    "#000000",
    "#ffffff",
    "#ff0000",
    "#00ff00",
    "#0000ff",
    "#ffff00",
    "#00ffff",
    "#ff00ff",
    "#808080",
    "#f76b15",
    "#30a46c",
    "#12345678",
] as const;

test("read each way CSS writes a color, and refuse what is none", () => {
    expect(
        [
            "#F76B15",
            "#f80",
            "#f808",
            " rgb(247 107 21) ",
            "rgb(247, 107, 21)",
            "rgba(247, 107, 21, 0.5)",
            "rgb(100% 0% 50% / 50%)",
            "hsl(23deg 93% 53%)",
            "hsl(-120 100% 50%)",
            "oklch(0.7 0.19 46)",
            "oklch(70% 47.5% 46 / none)",
            "oklch(70% 0.4 46)",
            "#ff",
            "#ggg",
            "rgb(1 2)",
            "rgb(1 2 3 4 5)",
            "rgb(1deg 2 3)",
            "hsl(a b c)",
            "orange",
        ].map((text) => Color.read(text)?.toString()),
    ).toEqual([
        "#f76b15",
        "#ff8800",
        "#ff880088",
        "#f76b15",
        "#f76b15",
        "#f76b1580",
        "#ff008080",
        "#f76d18",
        "#0000ff",
        "#f96f16",
        "#f96f1600",
        "#fc6c00",
        ...Array.from({ length: 7 }, () => undefined),
    ]);
    expect(() => Color.parse("orange")).toThrow(new TypeError("color is invalid: 'orange'"));
});

test.for(["hsl", "hsb", "oklch"] as const)("say a color in %s and back", (space: ColorSpace) => {
    expect(COLORS.map((text) => Color.parse(text).to(space).toString())).toEqual(COLORS);
});

test("write a color in each format", () => {
    const color = Color.parse("#f76b1580");

    expect(
        (["hex", "rgb", "hsl", "oklch"] as const).map((format) => color.toString(format)),
    ).toEqual([
        "#f76b1580",
        "rgb(247 107 21 / 0.5)",
        "hsl(22.8 93.4% 52.5% / 0.5)",
        "oklch(69.08% 0.1909 45.02 / 0.5)",
    ]);
});

test("change a channel in its range, keeping the hue of a color without any", () => {
    const color = Color.parse("#30a46c").to("hsb");
    const gray = color.with("saturation", 0);

    expect([
        gray.get("hue"),
        gray.toString(),
        gray.with("saturation", 70.7).toString(),
        color.with("brightness", 500).get("brightness"),
        color.with("alpha", 0.5).toString(),
    ]).toEqual([151.0344827586207, "#a4a4a4", "#30a46c", 100, "#30a46c80"]);
    expect(() => color.get("chroma")).toThrow(new RangeError("color has no channel chroma"));
});

test("tell the same color from another, another alpha and another space", () => {
    const color = Color.parse("#f76b15");

    expect(
        [
            Color.parse("#f76b15"),
            Color.parse("#f76b16"),
            Color.parse("#f76b1580"),
            color.to("hsl"),
        ].map((other) => color.equals(other)),
    ).toEqual([true, false, false, false]);
});

test("say what people call a color", () => {
    expect(
        [
            "#000000",
            "#ffffff",
            "#808080",
            "#ff0000",
            "#f76b15",
            "#8b4513",
            "#ffff00",
            "#808000",
            "#30a46c",
            "#0090ff",
            "#000080",
            "#e6e6fa",
            "#ffc0cb",
            "#d6409f",
        ].map((text) => {
            const { lightness, chroma, hues } = Color.parse(text).name();

            return [lightness, chroma, ...hues].filter((word) => word !== undefined).join(" ");
        }),
    ).toEqual([
        "black",
        "white",
        "gray",
        "vibrant red",
        "vibrant red orange",
        "dark brown",
        "very light vibrant yellow",
        "yellow green",
        "green cyan",
        "vibrant cyan blue",
        "very dark vibrant blue",
        "very light pale purple",
        "very light pale pink",
        "vibrant magenta pink",
    ]);
});
