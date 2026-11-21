import { PackageVersion } from "@destack/package";
import { expect, test } from "@destack/test";
import { describeTheme } from "../inspect/index.ts";
import { defineTheme, THEMES } from "./index.ts";

/** The package release declaring the fixture's theme. */
const notes = PackageVersion.parse({
    id: "package-019f5530-8000-7000-8000-000000000002",
    name: "@alice/notes",
    version: "2026.9.0",
});

test("describe a declared theme with its package", () => {
    const theme = defineTheme(
        { name: "notes", base: "sand", accent: "orange", radius: "large" },
        { package: notes },
    );

    expect(describeTheme(theme)).toEqual({
        package: notes,
        name: "notes",
        base: "sand",
        accent: "orange",
        radius: "large",
    });
});

test("declare Destack's themes, the brand and one per planet from the sun outward", () => {
    // every theme passed its contrast check when its module declared it
    expect(THEMES.map((theme) => theme.definition.name)).toEqual([
        "destack",
        "mercury",
        "venus",
        "earth",
        "mars",
        "jupiter",
        "saturn",
        "uranus",
        "neptune",
    ]);
});
