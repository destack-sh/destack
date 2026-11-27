import { HUES } from "@destack/style";
import { PackageVersion } from "@destack/package";
import { expect, test } from "@destack/test";
import { describeTheme } from "../inspect/index.ts";
import { Color } from "@destack/style";
import { DEFAULT_DISPLAY, type Theme, TYPEFACES } from "../theme/index.ts";
import { TOKENS } from "../token/index.ts";
import { defineTheme, destackTheme, THEMES } from "./index.ts";

/** The least OKLab distance between two themes' pages, the mean of day and night. */
const LEAST_DISTANCE = 0.025;

/** The least OKLab distance between two hues of a theme, as graphics. */
const LEAST_HUE_DISTANCE = 0.03;

/** The package release declaring the fixture's theme. */
const notes = PackageVersion.parse({
    id: "package-019f5530-8000-7000-8000-000000000002",
    name: "@alice/notes",
    version: "2026.9.0",
});

test("describe a declared theme with its package", () => {
    const theme = defineTheme(
        { name: "notes", base: "#8d8d86", accent: "orange", radius: "large" },
        { package: notes },
    );

    expect(describeTheme(theme)).toEqual({
        package: notes,
        name: "notes",
        base: "#8d8d86",
        accent: "orange",
        radius: "large",
    });
});

test("declare Destack's themes, the brand and the neutrals before the hues of their nights", () => {
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

test("read every role of Destack's themes at its standard contrast", () => {
    const refused = THEMES.flatMap((theme) => {
        try {
            theme.requireContrast([0]);

            return [];
        } catch (error) {
            return [String(error)];
        }
    });

    expect(refused).toEqual([]);
});

test("draw every two hues of each of Destack's themes apart", () => {
    // measure each pair of hues as graphics by day and by night, and list the pairs too close
    const close = THEMES.flatMap((theme) =>
        (["light", "dark"] as const).flatMap((scheme) => {
            const marks = HUES.map((hue) => ({
                hue,
                color: Color.parse(theme.resolve(scheme, 0, theme.seed(hue)).color("mark")),
            }));

            return marks.flatMap((left, index) =>
                marks
                    .slice(index + 1)
                    .filter((right) => left.color.distance(right.color) < LEAST_HUE_DISTANCE)
                    .map((right) => `${theme.name} ${scheme} ${left.hue} ${right.hue}`),
            );
        }),
    );

    expect(close).toEqual([]);
});

test("set every two themes' pages apart, whatever accent a person chooses", () => {
    // measure each pair's pages by day and by night, and list the pairs closer than the least
    const close = THEMES.flatMap((left, index) =>
        THEMES.slice(index + 1).flatMap((right) => {
            const day = pageOf(left, "light").distance(pageOf(right, "light"));
            const night = pageOf(left, "dark").distance(pageOf(right, "dark"));
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
function pageOf(theme: Theme, scheme: "light" | "dark"): Color {
    const page = theme.definition.ground?.[scheme].background;
    if (page === undefined) {
        throw new TypeError(`theme '${theme.definition.name}' is invalid: it sets no ground`);
    }

    return Color.parse(page);
}
