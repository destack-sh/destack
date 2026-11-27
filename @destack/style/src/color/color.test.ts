import { expect, test } from "@destack/test";
import { DEFAULT_HUES } from "../hue/index.ts";
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

test("validate text as a color exactly where Color.read reads one, refusing any other", () => {
    // the schema, which JSON Schema describes, agrees with the parser on every form
    const texts = [
        "#f76b15",
        "#FFF",
        "#f76b1580",
        " #000 ",
        "rgb(0 144 255 / 50%)",
        "RGBA(0, 144, 255, 0.5)",
        "hsl(120deg 50% 50%)",
        "hsl(none 0% 50%)",
        "oklch(69% 0.19 45)",
        "oklch(0.69 0.19 45deg / 1)",
        "orange",
        "#12",
        "rgb(0 0)",
        "rgb(0deg 0 0)",
        "oklch(1 2 3 4 5)",
        "hsl(a b c)",
    ];

    expect(texts.map((text) => Color.schema.safeParse(text).success)).toEqual(
        texts.map((text) => Color.read(text) !== undefined),
    );
});

test("write a color as the design token format does, and read it back", () => {
    // write a translucent hex color, an OKLCH one and an HSB one
    const colors = [
        Color.parse("#ff000080"),
        Color.parse("oklch(70% 0.1 200)"),
        new Color("hsb", [0, 0, 100]),
    ];
    const tokens = colors.map((color) => color.token());

    expect({
        tokens,
        back: tokens.map((token) => Color.fromToken(token).toString("hex")),
    }).toEqual({
        tokens: [
            { colorSpace: "srgb", components: [1, 0, 0], alpha: 128 / 255, hex: "#ff0000" },
            { colorSpace: "oklch", components: [0.7, 0.1, 200] },
            { colorSpace: "srgb", components: [1, 1, 1], hex: "#ffffff" },
        ],
        back: colors.map((color) => color.toString("hex")),
    });
});

test("measure the APCA-W3 reference contrasts", () => {
    // the pairs and values the apca-w3 0.0.98G package documents
    expect(Color.parse("#888888").contrast(Color.parse("#ffffff"), "apca")).toBeCloseTo(
        63.056469930209424,
        10,
    );
    expect(Color.parse("#ffffff").contrast(Color.parse("#888888"), "apca")).toBeCloseTo(
        -68.54146436644962,
        10,
    );
    expect(Color.parse("#000000").contrast(Color.parse("#aaaaaa"), "apca")).toBeCloseTo(
        58.146262578561334,
        10,
    );
    expect(Color.parse("#aaaaaa").contrast(Color.parse("#000000"), "apca")).toBeCloseTo(
        -56.24113336839742,
        10,
    );
    expect(Color.parse("#112233").contrast(Color.parse("#ddeeff"), "apca")).toBeCloseTo(
        91.66830811481631,
        10,
    );
    expect(Color.parse("#ddeeff").contrast(Color.parse("#112233"), "apca")).toBeCloseTo(
        -93.06770049484275,
        10,
    );
    expect(Color.parse("#777777").contrast(Color.parse("#777777"), "apca")).toBe(0);
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
        Color.parse(text).contrast(Color.parse(background), "wcag").toFixed(2),
    );

    expect(ratios).toEqual(["21.00", "21.00", "4.54", "4.48", "1.00"]);
});

test("roundtrip every default hue through OKLCH", () => {
    expect(
        Object.values(DEFAULT_HUES).filter(
            (hex) => Color.parse(hex).to("oklch").toString("hex") !== hex,
        ),
    ).toEqual([]);
});

test("write a color in every CSS Color 4 space, and read each back", () => {
    // the values CSS Color 4 gives for sRGB red
    const red = Color.parse("#ff0000");
    const formats = [
        "hwb",
        "lab",
        "lch",
        "oklab",
        "srgb-linear",
        "display-p3",
        "a98-rgb",
        "prophoto-rgb",
        "rec2020",
        "xyz-d50",
        "xyz-d65",
    ] as const;
    const written = formats.map((format) => red.toString(format));

    expect({
        written,
        back: written.map((text) => Color.read(text)?.toString()),
        valid: written.every((text) => Color.schema.safeParse(text).success),
    }).toEqual({
        written: [
            "hwb(0 0% 0%)",
            "lab(54.29% 80.8 69.89)",
            "lch(54.29% 106.84 40.86)",
            "oklab(62.8% 0.2249 0.1258)",
            "color(srgb-linear 1 0 0)",
            "color(display-p3 0.91749 0.20029 0.13856)",
            "color(a98-rgb 0.85859 0 0)",
            "color(prophoto-rgb 0.70225 0.27572 0.10355)",
            "color(rec2020 0.79198 0.23098 0.07376)",
            "color(xyz-d50 0.43607 0.22249 0.01392)",
            "color(xyz-d65 0.41239 0.21264 0.01933)",
        ],
        back: formats.map(() => "#ff0000"),
        valid: true,
    });
});

test("map a wide gamut color into sRGB by its chroma, keeping it exact in its own space", () => {
    const red = Color.parse("color(display-p3 1 0 0)");

    expect([
        red.toString(),
        red.toString("display-p3"),
        Color.parse("color(srgb 1 0.5 0)").toString(),
        Color.parse("hwb(120 20% 30%)").toString(),
        red.token(),
    ]).toEqual([
        "#ff3428",
        "color(display-p3 1 0 0)",
        "#ff8000",
        "#33b333",
        { colorSpace: "display-p3", components: [1, 0, 0] },
    ]);
});

test("write a color plainly: hexadecimal where sRGB shows it, else OKLCH", () => {
    expect(
        ["oklab(0.295167 -0.0296881 -0.0283 / 0.18)", "rgb(255 0 0)", "oklch(0.7 0.3 145)"].map(
            (text) => Color.parse(text).plain(),
        ),
    ).toEqual(["#12313c2e", "#ff0000", "oklch(70% 0.3 145)"]);
});
