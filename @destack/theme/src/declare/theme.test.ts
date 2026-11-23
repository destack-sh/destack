import { PackageVersion } from "@destack/package";
import { expect, test } from "@destack/test";
import { describeTheme } from "../inspect/index.ts";
import { DEFAULT_DISPLAY, TYPEFACES } from "../theme/index.ts";
import { TOKENS } from "../token/index.ts";
import { defineTheme, destackTheme, THEMES } from "./index.ts";

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

test("declare Destack's themes, the brand and its moons by their planets from the sun outward", () => {
    // every theme passed its contrast check when its module declared it
    expect(THEMES.map((theme) => theme.definition.name)).toEqual([
        "destack",
        "io",
        "europa",
        "pandora",
        "enceladus",
        "titan",
        "ariel",
        "triton",
        "nyx",
    ]);
});

test("read the person's typeface over the theme's font, without the tracking fit to the theme's", () => {
    // read the brand's Plex with its tight headings, then Inter as a person chooses it
    const brand = destackTheme.variables(DEFAULT_DISPLAY);
    const chosen = destackTheme.variables({ ...DEFAULT_DISPLAY, typeface: "inter" });
    const font = TOKENS.entry(["font", "text"]).variable;
    const tracking = TOKENS.entry(["text", "2xl"]).member("letterSpacing");
    const adjust = "--destack-font-text-adjust";

    // the chosen face is sized to Plex's lowercase, the theme's face left as it is
    expect([
        brand[font],
        brand[tracking],
        brand[adjust],
        chosen[font],
        chosen[tracking],
        chosen[adjust],
    ]).toEqual([TYPEFACES.plex, "-0.02em", "none", TYPEFACES.inter, "0px", "ex-height 0.516"]);
});

test("read the brand's face chosen by name as the theme's face, its tracking and size intact", () => {
    // choose Plex on the Destack theme, whose face it is
    const chosen = destackTheme.variables({ ...DEFAULT_DISPLAY, typeface: "plex" });
    const tracking = TOKENS.entry(["text", "2xl"]).member("letterSpacing");

    expect([chosen[tracking], chosen["--destack-font-text-adjust"]]).toEqual(["-0.02em", "none"]);
});
