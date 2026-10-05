import { Package } from "@destack/package";
import { describeSetting } from "@destack/setting/inspect";
import { expect, test } from "@destack/test";
import { defineTheme } from "../declare/index.ts";
import { describeTheme } from "../inspect/index.ts";
import { PRESET_NAMES } from "../radix/index.ts";
import { accent, appearance, contrast, density, motion, textSize } from "../setting/index.ts";

/** The package release declaring the fixture's theme. */
const notes = Package.parse({
    id: "package-019f5530-8000-7000-8000-000000000002",
    name: "@alice/notes",
    version: "2026.9.0",
});

/** The fields every display preference shares. */
const preference = {
    scope: "user",
    overrides: ["package", "space", "installation", "device"],
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
        owner: "@destack/theme",
        name: "appearance",
        title: "Appearance",
        description: "Use the system appearance or select a light or dark interface.",
        schema: enumeration(["system", "light", "dark"]),
        default: "system",
        ...preference,
    },
    {
        owner: "@destack/theme",
        name: "textSize",
        title: "Text size",
        description: "Read text smaller or larger, after Apple's Dynamic Type sizes.",
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
        owner: "@destack/theme",
        name: "density",
        title: "Density",
        description:
            "Pack controls and content compactly or spaciously, or keep each app's own density.",
        schema: nullable(["compact", "regular", "spacious"]),
        default: null,
        ...preference,
    },
    {
        owner: "@destack/theme",
        name: "contrast",
        title: "Contrast",
        description: "Use the system contrast or darken borders and secondary text.",
        schema: enumeration(["system", "standard", "more"]),
        default: "system",
        ...preference,
    },
    {
        owner: "@destack/theme",
        name: "motion",
        title: "Motion",
        description: "Use the system motion preference, or show or remove transitions.",
        schema: enumeration(["system", "full", "reduced"]),
        default: "system",
        ...preference,
    },
    {
        owner: "@destack/theme",
        name: "accent",
        title: "Accent color",
        description: "Use each app's own accent, or one accent color in every app.",
        schema: nullable([...PRESET_NAMES]),
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

test("describe a declared theme with its package", () => {
    const theme = defineTheme(
        { name: "notes", gray: "sand", accent: "orange", radius: "large" },
        { package: notes },
    );

    expect(describeTheme(theme)).toEqual({
        package: notes,
        name: "notes",
        gray: "sand",
        accent: "orange",
        radius: "large",
    });
});
