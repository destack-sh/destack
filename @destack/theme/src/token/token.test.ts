import { readFile } from "node:fs/promises";
import { expect, test } from "@destack/test";
import { Source } from "../token/source.ts";
import { TextStyleName, WeightName } from "../theme/theme.ts";
import { TOKENS, TokenTree } from "../token/token.ts";

/** Read a package file. */
function read(path: string): Promise<string> {
    return readFile(new URL(`../../${path}`, import.meta.url), "utf8");
}

/** List the constant keys of a token family. */
function keys(family: string): string[] {
    return TOKENS.family(family).entries.flatMap((entry) =>
        entry.members().map((member) => member.key),
    );
}

test("generate the token constants and text styles tokens.json describes", async () => {
    expect(await read("src/token/tokens.stylex.ts")).toBe(Source.constants(TOKENS));
    expect(await read("src/text/text.ts")).toBe(Source.text(TOKENS));
});

test("declare the token families with their text styles and motion tokens", () => {
    // every family in document order
    expect(TOKENS.families.map((family) => family.name)).toEqual([
        "color",
        "surface",
        "swatch",
        "font",
        "text",
        "weight",
        "space",
        "size",
        "width",
        "radius",
        "stroke",
        "shadow",
        "motion",
    ]);

    // the interface and code font stacks the text styles and code take
    expect(keys("font")).toEqual(["text", "code"]);

    // the text styles a theme names, one custom property per typography member
    const styles = TextStyleName.options;
    const members = ["FontFamily", "FontSize", "FontWeight", "LineHeight", "LetterSpacing"];
    expect(keys("text")).toEqual(
        styles.flatMap((style) => members.map((member) => style + member)),
    );

    // the font weights a theme names, and the line widths
    expect(keys("weight")).toEqual(WeightName.options);
    expect(keys("stroke")).toEqual(["border", "ring"]);

    // the durations and easings
    expect(keys("motion")).toEqual([
        "durationShort",
        "durationMedium",
        "durationLong",
        "easingStandard",
        "easingEmphasised",
        "easingSpring",
    ]);
});

test("refuse a token color whose hex differs from its channels", () => {
    // a white token written with a black hex
    const document = {
        color: {
            $description: "Colors.",
            background: {
                $type: "color",
                $value: { colorSpace: "srgb", components: [1, 1, 1], hex: "#000000" },
                $description: "The background.",
                $extensions: {
                    "app.destack": { dark: { colorSpace: "srgb", components: [0, 0, 0] } },
                },
            },
        },
    };

    expect(() => TokenTree.parse(document)).toThrow(
        new RangeError("token color.background has hex #000000 for #ffffff"),
    );
});
