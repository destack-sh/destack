import { describeSetting } from "@destack/setting/inspect";
import { expect, test } from "@destack/test";
import { PRESET_NAMES } from "@destack/theme";
import { accent, appearance, contrast, density, motion, textSize } from "./index.ts";

/** The fields every display preference shares. */
const preference = {
    scope: "user",
    overrides: ["package", "space", "installation", "client"],
    apply: "immediate",
};

/** The JSON Schema of a string enumeration. */
function enumeration(values: readonly string[]): Record<string, unknown> {
    return {
        $schema: "https://json-schema.org/draft/2020-12/schema",
        type: "string",
        enum: values,
    };
}

/** The JSON Schema of a nullable string enumeration. */
function nullable(values: readonly string[]): Record<string, unknown> {
    return {
        $schema: "https://json-schema.org/draft/2020-12/schema",
        anyOf: [{ type: "string", enum: values }, { type: "null" }],
    };
}

/** The descriptions of the display preference settings in declaration order. */
const DESCRIPTIONS = [
    {
        owner: "@destack/view",
        name: "appearance",
        title: "Appearance",
        description: "Use the system appearance or select a light or dark interface.",
        schema: enumeration(["system", "light", "dark"]),
        default: "system",
        ...preference,
    },
    {
        owner: "@destack/view",
        name: "textSize",
        title: "Text size",
        description: "Read text smaller or larger.",
        schema: enumeration([
            "xSmall",
            "small",
            "medium",
            "large",
            "xLarge",
            "xxLarge",
            "xxxLarge",
        ]),
        default: "medium",
        ...preference,
    },
    {
        owner: "@destack/view",
        name: "density",
        title: "Density",
        description:
            "Pack controls and content compactly or spaciously, or keep each app's own density.",
        schema: nullable(["compact", "regular", "spacious"]),
        default: null,
        ...preference,
    },
    {
        owner: "@destack/view",
        name: "contrast",
        title: "Contrast",
        description:
            "Follow the device's contrast or set how far text, lines and graphics stand out.",
        schema: {
            $schema: "https://json-schema.org/draft/2020-12/schema",
            anyOf: [
                { type: "string", const: "system" },
                { type: "number", minimum: 0, maximum: 1 },
            ],
        },
        default: "system",
        ...preference,
    },
    {
        owner: "@destack/view",
        name: "motion",
        title: "Motion",
        description: "Use the system motion preference, or show or remove transitions.",
        schema: enumeration(["system", "full", "reduced"]),
        default: "system",
        ...preference,
    },
    {
        owner: "@destack/view",
        name: "accent",
        title: "Accent color",
        description: "Use each app's own accent, or one accent color in every app.",
        schema: {
            $schema: "https://json-schema.org/draft/2020-12/schema",
            anyOf: [
                {
                    anyOf: [
                        { type: "string", enum: [...PRESET_NAMES] },
                        { type: "string", pattern: "^#[0-9a-f]{6}$" },
                    ],
                },
                { type: "null" },
            ],
        },
        default: null,
        ...preference,
    },
];

test("declare the display preferences as user settings with every override", () => {
    const described = [appearance, textSize, density, contrast, motion, accent].map((setting) => {
        const { package: owner, ...description } = describeSetting(setting);

        return { owner: owner.name, ...description };
    });
    expect(described).toEqual(DESCRIPTIONS);
});
