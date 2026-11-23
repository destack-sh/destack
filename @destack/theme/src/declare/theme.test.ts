import { PackageVersion } from "@destack/package";
import { expect, test } from "@destack/test";
import { describeTheme } from "../inspect/index.ts";
import { Oklch } from "../palette/index.ts";
import { DEFAULT_DISPLAY, type Theme, TYPEFACES } from "../theme/index.ts";
import { TOKENS } from "../token/index.ts";
import { defineTheme, destackTheme, THEMES } from "./index.ts";

/** The least OKLab distance between two themes' pages, the mean of day and night. */
const LEAST_DISTANCE = 0.025;

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

test("declare Destack's themes, the brand and the neutrals before the hues of their nights", () => {
    // every theme passed its contrast check when its module declared it
    expect(THEMES.map((theme) => theme.definition.name)).toEqual([
        "destack",
        "enceladus",
        "umbriel",
        "triton",
        "iapetus",
        "pandora",
        "titan",
        "europa",
        "charon",
        "ariel",
    ]);
});

test("set every two themes' pages apart, whatever accent a person chooses", () => {
    // measure each pair's pages by day and by night, and list the pairs closer than the least
    const close = THEMES.flatMap((left, index) =>
        THEMES.slice(index + 1).flatMap((right) => {
            const day = distance(pageOf(left, "light"), pageOf(right, "light"));
            const night = distance(pageOf(left, "dark"), pageOf(right, "dark"));
            const mean = (day + night) / 2;

            return mean < LEAST_DISTANCE
                ? [`${left.definition.name} ${right.definition.name} ${mean.toFixed(3)}`]
                : [];
        }),
    );

    expect(close).toEqual([]);
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

/** Read a theme's page color at one time of day. */
function pageOf(theme: Theme, scheme: "light" | "dark"): Oklch {
    const page = theme.definition.ground?.[scheme].background;
    if (page === undefined) {
        throw new TypeError(`theme '${theme.definition.name}' is invalid: it sets no ground`);
    }

    return Oklch.parse(page);
}

/** Measure the OKLab distance of two colors, by their lightness, chroma and the angle between. */
function distance(left: Oklch, right: Oklch): number {
    const angle = ((left.hue - right.hue) * Math.PI) / 180;
    const chroma =
        left.chroma ** 2 + right.chroma ** 2 - 2 * left.chroma * right.chroma * Math.cos(angle);

    return Math.sqrt((left.lightness - right.lightness) ** 2 + chroma);
}
