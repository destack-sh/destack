import { Package } from "@destack/package";
import { expect, test } from "@destack/test";
import { describeTheme } from "../inspect/index.ts";
import { defineTheme } from "./index.ts";

/** The package release declaring the fixture's theme. */
const notes = Package.parse({
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
