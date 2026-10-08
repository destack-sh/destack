import { Package } from "@destack/package";
import { expect, refusal, test } from "@destack/test";
import { defineTheme } from "../declare/index.ts";
import { apca, Color, Palette } from "../palette/index.ts";
import { GRAY_PRESETS, PRESET_NAMES, PRESETS } from "../preset/index.ts";
import {
    DEFAULT_PREFERENCES,
    GRAPHIC_CONTRAST,
    ROLE_NAMES,
    RoleName,
    SURFACE_ROLES,
    SurfaceLevel,
    Theme,
    type ThemeStyle,
} from "../theme/index.ts";
import { TOKENS } from "../token/index.ts";

/** The package release declaring the fixture's themes. */
const notes = Package.parse({
    id: "package-019f5530-8000-7000-8000-000000000002",
    name: "@alice/notes",
    version: "2026.9.0",
});

/** The theme naming nothing beyond its name. */
const standard = defineTheme({ name: "standard" }, { package: notes });

/** List the declarations that differ from a baseline style. */
function changes(
    style: Readonly<Record<string, string>>,
    baseline: ThemeStyle,
): Record<string, string> {
    return Object.fromEntries(
        Object.entries(style).filter(([name, value]) =>
            Object.entries(baseline).every(([other, before]) => other !== name || before !== value),
        ),
    );
}

/** Keep a theme root's role and scale properties, leaving out the swatches the swatch test covers. */
function withoutSwatches(style: ThemeStyle): Readonly<Record<string, string>> {
    return Object.fromEntries(
        Object.entries(style).filter(([name]) => !name.startsWith("--destack-swatch-")),
    );
}

/** The spacing steps of tokens.json. */
const SPACE = [4, 8, 12, 16, 24, 32, 40, 48, 64];

/** The control heights of tokens.json. */
const SIZE = [24, 32, 40, 48];

/** The panel and overlay widths of tokens.json, by their custom property's name. */
const WIDTH = {
    "hover-card": 256,
    popover: 288,
    prose: 384,
    row: 160,
    sidebar: 256,
    "sidebar-icon": 48,
    tile: 120,
    toast: 356,
};

/** The text sizes and letter spacings at the largest text size, 23 over 16 points of the default. */
const LARGEST_TEXT = {
    "--destack-text-caption-font-size": "15.813px",
    "--destack-text-footnote-font-size": "17.25px",
    "--destack-text-footnote-letter-spacing": "-0.106px",
    "--destack-text-body-font-size": "23px",
    "--destack-text-body-letter-spacing": "-0.582px",
    "--destack-text-callout-font-size": "21.563px",
    "--destack-text-callout-letter-spacing": "-0.418px",
    "--destack-text-headline-font-size": "23px",
    "--destack-text-headline-letter-spacing": "-0.582px",
    "--destack-text-title1-font-size": "38.813px",
    "--destack-text-title1-letter-spacing": "0.528px",
    "--destack-text-title2-font-size": "30.188px",
    "--destack-text-title2-letter-spacing": "-0.356px",
    "--destack-text-title3-font-size": "27.313px",
    "--destack-text-title3-letter-spacing": "-0.614px",
    "--destack-text-large-title-font-size": "47.438px",
    "--destack-text-large-title-letter-spacing": "0.559px",
};

/** Format a length in pixels as themes do, to three decimals. */
function pixels(length: number): string {
    return `${Number(length.toFixed(3))}px`;
}

/** List the scaled custom properties of the width family. */
function widths(factor: number): [string, string][] {
    return Object.entries(WIDTH).map(([name, length]) => [
        `--destack-width-${name}`,
        pixels(length * factor),
    ]);
}

/** List the scaled custom properties of a numbered token family. */
function scaled(family: string, lengths: readonly number[], factor: number): [string, string][] {
    return lengths.map((length, index) => [
        `--destack-${family}-${index + 1}`,
        pixels(length * factor),
    ]);
}

test("give every preset accent on every base five chart series of hues apart, standing out in both appearances", () => {
    const failures = GRAY_PRESETS.flatMap((base) =>
        PRESET_NAMES.flatMap((accent) => {
            // resolve the series and the background in both appearances
            const theme = new Theme(notes, { name: "preset", base, accent });
            const series = (["chart1", "chart2", "chart3", "chart4", "chart5"] as const).map(
                (name) => ({
                    light: theme.resolve("light", 0).color(name),
                    dark: theme.resolve("dark", 0).color(name),
                }),
            );
            const background = {
                light: theme.resolve("light", 0).color("background"),
                dark: theme.resolve("dark", 0).color("background"),
            };

            // require every series to stand out and every pair of hues to lie apart
            const faint = series.filter((entry) =>
                (["light", "dark"] as const).some(
                    (scheme) =>
                        Math.abs(apca(entry[scheme], background[scheme])) <
                        GRAPHIC_CONTRAST.standard,
                ),
            );
            const hues = series.map((entry) => Color.parse(entry.light).hue);
            const close = hues.flatMap((hue, index) =>
                hues.slice(index + 1).filter((other) => {
                    const difference = Math.abs(hue - other) % 360;

                    return Math.min(difference, 360 - difference) < 20;
                }),
            );

            return faint.length === 0 && close.length === 0 ? [] : [`${base}/${accent}`];
        }),
    );

    expect(failures).toEqual([]);
});

test("pass every role's contrast for every preset accent on every base", () => {
    const refused = GRAY_PRESETS.flatMap((base) =>
        PRESET_NAMES.flatMap((accent) => {
            const theme = new Theme(notes, { name: "preset", base, accent });
            try {
                theme.requireContrast();

                return [];
            } catch (error) {
                return [`${base}/${accent}: ${String(error)}`];
            }
        }),
    );

    expect(refused).toEqual([]);
});

test("shift a seed by a small lightness until its best label reads at Lc 60", () => {
    for (const seed of ["#ff8800", "#22c55e", "#0ea5e9"]) {
        const theme = defineTheme({ name: "seeded", accent: seed }, { package: notes });
        const resolution = theme.resolve("light", 0);
        const solid = resolution.color("primary");
        const before = Color.parse(seed);
        const after = Color.parse(solid);

        // the shift moves lightness alone, by less than 0.04, and the label reads
        const shift = Math.abs(after.lightness - before.lightness);
        expect([shift > 0, shift < 0.04]).toEqual([true, true]);
        expect(Math.abs(after.hue - before.hue)).toBeLessThan(1);
        expect(resolution.reads("primaryForeground")).toBeGreaterThanOrEqual(60);
    }
});

test("keep a legible preset's seed and label it with its own palette's darkest text", () => {
    const amber = defineTheme({ name: "amber", accent: "amber" }, { package: notes });
    const resolution = amber.resolve("light", 0);

    expect([resolution.color("primary"), resolution.color("primaryForeground")]).toEqual([
        PRESETS.amber,
        Palette.of("amber").tone(0.242),
    ]);
});

test("read every text, line and graphic role against a nested surface's background", () => {
    // a selected row's accent fill becomes the background every role reads on
    const row = standard.rebase({ light: "#c9d8ff", dark: "#1d2f66" });
    for (const scheme of ["light", "dark"] as const) {
        const resolution = row.resolve(scheme, 0);
        const unread = ROLE_NAMES.filter((name) => {
            const reading = row.roles[name].reading;

            return (
                reading !== undefined && resolution.reads(name) < resolution.floor(reading.contrast)
            );
        });
        expect([resolution.color("background"), unread]).toEqual([
            scheme === "light" ? "#c9d8ff" : "#1d2f66",
            [],
        ]);
    }

    // the surfaces above the background follow it
    const card = Color.parse(row.resolve("light", 0).color("card")).lightness;
    expect(card - Color.parse("#c9d8ff").lightness).toBeCloseTo(-0.011, 2);
});

test("raise every reading role's contrast continuously with the person's level", () => {
    // each level reads at least at its floor, and the floors rise from the standard to the most
    const levels = [0, 0.25, 0.5, 0.75, 1];
    const reads = levels.map((level) => {
        const resolution = standard.resolve("light", level);

        return ROLE_NAMES.filter((name) => {
            const reading = standard.roles[name].reading;

            return (
                reading !== undefined && resolution.reads(name) < resolution.floor(reading.contrast)
            );
        });
    });
    const muted = levels.map((level) => standard.resolve("light", level).reads("mutedForeground"));

    expect(reads).toEqual([[], [], [], [], []]);
    expect(muted).toEqual(muted.toSorted((left, right) => left - right));
    expect(
        standard.variables("light", { ...DEFAULT_PREFERENCES, contrast: 0.5 })[
            "--destack-color-muted-foreground"
        ],
    ).toBe(
        `light-dark(${standard.resolve("light", 0.5).color("mutedForeground")}, ${standard.resolve("dark", 0.5).color("mutedForeground")})`,
    );
});

test("grow the status roles and the first chart series from the theme's declared seeds", () => {
    const declared = defineTheme(
        { name: "declared", status: { destructive: "tomato" }, chart: ["#d946ef"] },
        { package: notes },
    );
    const resolution = declared.resolve("light", 0);

    expect([resolution.color("destructive"), resolution.color("chart2")]).toEqual([
        PRESETS.tomato,
        "#d946ef",
    ]);
});

test("check the roles a theme replaces like every other role", async () => {
    const replaced = defineTheme(
        {
            name: "paper",
            roles: {
                background: { light: "#f8f5ee", dark: "#0b2029" },
                mutedForeground: { palette: "base", lightness: { light: 0.242, dark: 0.948 } },
            },
        },
        { package: notes },
    );
    const style = replaced.variables("system", DEFAULT_PREFERENCES);
    expect([
        style["--destack-color-background"],
        style["--destack-surface-base"],
        style["--destack-color-muted-foreground"],
    ]).toEqual([
        "light-dark(#f8f5ee, #0b2029)",
        "light-dark(#f8f5ee, #0b2029)",
        "light-dark(#1f2021, #ededf2)",
    ]);

    // a background too close to its text is refused
    const illegible = Promise.resolve().then(() =>
        defineTheme(
            { name: "fog", roles: { background: { light: "#8b8d98", dark: "#101113" } } },
            { package: notes },
        ),
    );
    expect(await refusal(illegible)).toEqual([
        "INVALID_DEFINITION",
        "theme fog: foreground on background reads at Lc 43.7 in light with standard contrast, below 75",
    ]);
});

test("raise a translucent role's opacity until its blend reads, keeping the declared opacity where it does", () => {
    // a rule of the text at a fifth, as a paper page draws its lines
    const theme = defineTheme(
        {
            name: "rule",
            roles: { border: { from: "foreground", lightness: { light: 0, dark: 0 }, alpha: 0.2 } },
        },
        { package: notes },
    );
    const borders = (["light", "dark"] as const).flatMap((scheme) =>
        [0, 1].map((level) => {
            const resolution = theme.resolve(scheme, level);

            return [resolution.color("border"), resolution.reads("border").toFixed(1)];
        }),
    );

    // standard contrast keeps a fifth, more contrast raises it exactly to the line floor of Lc 30
    expect(borders).toEqual([
        ["#1f202133", "23.3"],
        ["#1f202141", "30.0"],
        ["#ededf233", "7.5"],
        ["#ededf275", "30.0"],
    ]);
});

test("label an overridden primary with the candidate that reads best on it", () => {
    const theme = defineTheme(
        { name: "signal", roles: { primary: { palette: "warning" } } },
        { package: notes },
    );
    const resolution = theme.resolve("light", 0);

    // the warning palette's darkest text reads on amber, where the indigo accent's white label would not
    expect([resolution.color("primary"), resolution.color("primaryForeground")]).toEqual([
        PRESETS.amber,
        Palette.of("amber").tone(0.242),
    ]);
});

test("resolve the default theme to the color values tokens.json shows", () => {
    for (const family of ["color", "surface"] as const) {
        // compare each role's standard light and dark color with its token
        const shown = TOKENS.family(family).entries.map((entry) => [
            entry.key,
            entry.token.$type === "color" ? hex(entry.token.$value) : entry.token.$type,
            entry.token.$type === "color" ? hex(entry.token.$extensions["app.destack"].dark) : "",
        ]);
        const resolved = shown.map(([key = ""]) => {
            const name =
                family === "color" ? RoleName.parse(key) : SURFACE_ROLES[SurfaceLevel.parse(key)];

            return [
                key,
                standard.resolve("light", 0).color(name),
                standard.resolve("dark", 0).color(name),
            ];
        });

        expect(resolved).toEqual(shown);
    }
});

test("emit one swatch per colorful preset as the roles it takes as accent, at the values tokens.json shows", () => {
    // read each swatch token's light and dark value and the theme's property at the standard contrast
    const style = standard.variables("system", { ...DEFAULT_PREFERENCES, contrast: 0 });
    const swatches = TOKENS.family("swatch").entries.map((entry) => {
        const light = entry.token.$type === "color" ? hex(entry.token.$value) : "";
        const dark =
            entry.token.$type === "color" ? hex(entry.token.$extensions["app.destack"].dark) : "";

        return [
            entry.variable,
            style[entry.variable] === (light === dark ? light : `light-dark(${light}, ${dark})`),
        ];
    });

    expect({
        presets: swatches.length / 4,
        teal: swatches.filter(([name]) => String(name).includes("-teal-")).map(([name]) => name),
        matching: swatches.every(([, isMatching]) => isMatching === true),
        solid: standard.resolve("light", 0, "teal").color("primary"),
    }).toEqual({
        presets: 25,
        teal: [
            "--destack-swatch-teal-solid",
            "--destack-swatch-teal-label",
            "--destack-swatch-teal-tint",
            "--destack-swatch-teal-text",
        ],
        matching: true,
        solid: "#12a594",
    });
});

test("emit the default theme's custom properties for the system appearance", () => {
    // follow the device's contrast through color-mix() and its motion through the duration scale
    expect(withoutSwatches(standard.variables("system", DEFAULT_PREFERENCES))).toEqual({
        "color-scheme": "light dark",
        "--destack-color-background": "light-dark(#fcfdfe, #131314)",
        "--destack-color-foreground": "light-dark(#1f2021, #ededf2)",
        "--destack-color-card": "light-dark(#f8f9fa, #19191a)",
        "--destack-color-card-foreground": "light-dark(#1f2021, #ededf2)",
        "--destack-color-popover": "light-dark(#fcfdfe, #242425)",
        "--destack-color-popover-foreground": "light-dark(#1f2021, #ededf2)",
        "--destack-color-primary": "#3e63dd",
        "--destack-color-primary-foreground": "#ffffff",
        "--destack-color-secondary": "light-dark(#f1f2f3, #252526)",
        "--destack-color-secondary-foreground": "light-dark(#1f2021, #ededf2)",
        "--destack-color-muted": "light-dark(#f1f2f3, #252526)",
        "--destack-color-muted-foreground":
            "light-dark(color-mix(in oklab, #62636a, #47474b calc(var(--destack-contrast, 0) * 100%)), color-mix(in oklab, #b1b3bc, #e4e5ea calc(var(--destack-contrast, 0) * 100%)))",
        "--destack-color-accent": "light-dark(#ecf2ff, #1b243b)",
        "--destack-color-accent-foreground":
            "light-dark(color-mix(in oklab, #26345c, #253359 calc(var(--destack-contrast, 0) * 100%)), color-mix(in oklab, #d7e3ff, #dfe9ff calc(var(--destack-contrast, 0) * 100%)))",
        "--destack-color-destructive":
            "color-mix(in oklab, #e5484d, #d2484b calc(var(--destack-contrast, 0) * 100%))",
        "--destack-color-destructive-foreground": "#ffffff",
        "--destack-color-success":
            "color-mix(in oklab, #30a46c, #37885e calc(var(--destack-contrast, 0) * 100%))",
        "--destack-color-success-foreground": "#ffffff",
        "--destack-color-warning":
            "color-mix(in oklab, #ffc53d, #ffc642 calc(var(--destack-contrast, 0) * 100%))",
        "--destack-color-warning-foreground": "#231f19",
        "--destack-color-info":
            "color-mix(in oklab, #0090ff, #237bd0 calc(var(--destack-contrast, 0) * 100%))",
        "--destack-color-info-foreground": "#ffffff",
        "--destack-color-border":
            "light-dark(color-mix(in oklab, #d8d9e0, #c3c4cd calc(var(--destack-contrast, 0) * 100%)), color-mix(in oklab, #393a3c, #76777f calc(var(--destack-contrast, 0) * 100%)))",
        "--destack-color-input":
            "light-dark(color-mix(in oklab, #ccced5, #c3c4cd calc(var(--destack-contrast, 0) * 100%)), color-mix(in oklab, #47474b, #76777f calc(var(--destack-contrast, 0) * 100%)))",
        "--destack-color-ring":
            "light-dark(color-mix(in oklab, #83a6ff, #5f85f3 calc(var(--destack-contrast, 0) * 100%)), color-mix(in oklab, #6b92fa, #91b1ff calc(var(--destack-contrast, 0) * 100%)))",
        "--destack-color-scrim": "light-dark(#1f202180, #00000080)",
        "--destack-color-chart1":
            "light-dark(#3e63dd, color-mix(in oklab, #6b92fa, #91b1ff calc(var(--destack-contrast, 0) * 100%)))",
        "--destack-color-chart2":
            "light-dark(#978365, color-mix(in oklab, #a69477, #c1b098 calc(var(--destack-contrast, 0) * 100%)))",
        "--destack-color-chart3":
            "light-dark(color-mix(in oklab, #29a383, #2e9b7d calc(var(--destack-contrast, 0) * 100%)), color-mix(in oklab, #35a888, #69c2a6 calc(var(--destack-contrast, 0) * 100%)))",
        "--destack-color-chart4":
            "light-dark(#d6409f, color-mix(in oklab, #ea65b4, #fe8ccd calc(var(--destack-contrast, 0) * 100%)))",
        "--destack-color-chart5":
            "light-dark(color-mix(in oklab, #00a2c7, #1c96b7 calc(var(--destack-contrast, 0) * 100%)), color-mix(in oklab, #00a2c7, #5abede calc(var(--destack-contrast, 0) * 100%)))",
        "--destack-surface-base": "light-dark(#fcfdfe, #131314)",
        "--destack-surface-raised": "light-dark(#f8f9fa, #19191a)",
        "--destack-surface-overlay": "light-dark(#fcfdfe, #242425)",
        "--destack-text-family":
            '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", system-ui, sans-serif, "Apple Color Emoji", "Segoe UI Emoji"',
        "--destack-text-code-family":
            'Menlo, Consolas, "Bitstream Vera Sans Mono", monospace, "Apple Color Emoji", "Segoe UI Emoji"',
        "--destack-text-caption-font-family": "var(--destack-text-family)",
        "--destack-text-caption-font-size": "11px",
        "--destack-text-caption-font-weight": "400",
        "--destack-text-caption-line-height": "1.3333",
        "--destack-text-caption-letter-spacing": "0px",
        "--destack-text-footnote-font-family": "var(--destack-text-family)",
        "--destack-text-footnote-font-size": "12px",
        "--destack-text-footnote-font-weight": "400",
        "--destack-text-footnote-line-height": "1.3846",
        "--destack-text-footnote-letter-spacing": "-0.074px",
        "--destack-text-body-font-family": "var(--destack-text-family)",
        "--destack-text-body-font-size": "16px",
        "--destack-text-body-font-weight": "400",
        "--destack-text-body-line-height": "1.2941",
        "--destack-text-body-letter-spacing": "-0.405px",
        "--destack-text-callout-font-family": "var(--destack-text-family)",
        "--destack-text-callout-font-size": "15px",
        "--destack-text-callout-font-weight": "400",
        "--destack-text-callout-line-height": "1.3125",
        "--destack-text-callout-letter-spacing": "-0.291px",
        "--destack-text-headline-font-family": "var(--destack-text-family)",
        "--destack-text-headline-font-size": "16px",
        "--destack-text-headline-font-weight": "600",
        "--destack-text-headline-line-height": "1.2941",
        "--destack-text-headline-letter-spacing": "-0.405px",
        "--destack-text-title1-font-family": "var(--destack-text-family)",
        "--destack-text-title1-font-size": "27px",
        "--destack-text-title1-font-weight": "400",
        "--destack-text-title1-line-height": "1.2143",
        "--destack-text-title1-letter-spacing": "0.367px",
        "--destack-text-title2-font-family": "var(--destack-text-family)",
        "--destack-text-title2-font-size": "21px",
        "--destack-text-title2-font-weight": "400",
        "--destack-text-title2-line-height": "1.2727",
        "--destack-text-title2-letter-spacing": "-0.248px",
        "--destack-text-title3-font-family": "var(--destack-text-family)",
        "--destack-text-title3-font-size": "19px",
        "--destack-text-title3-font-weight": "400",
        "--destack-text-title3-line-height": "1.25",
        "--destack-text-title3-letter-spacing": "-0.427px",
        "--destack-text-large-title-font-family": "var(--destack-text-family)",
        "--destack-text-large-title-font-size": "33px",
        "--destack-text-large-title-font-weight": "400",
        "--destack-text-large-title-line-height": "1.2059",
        "--destack-text-large-title-letter-spacing": "0.389px",
        "--destack-weight-regular": "400",
        "--destack-weight-medium": "500",
        "--destack-weight-semibold": "600",
        "--destack-width-hover-card": "256px",
        "--destack-width-popover": "288px",
        "--destack-width-prose": "384px",
        "--destack-width-row": "160px",
        "--destack-width-sidebar": "256px",
        "--destack-width-sidebar-icon": "48px",
        "--destack-width-tile": "120px",
        "--destack-width-toast": "356px",
        "--destack-weight-bold": "700",
        "--destack-space-1": "4px",
        "--destack-space-2": "8px",
        "--destack-space-3": "12px",
        "--destack-space-4": "16px",
        "--destack-space-5": "24px",
        "--destack-space-6": "32px",
        "--destack-space-7": "40px",
        "--destack-space-8": "48px",
        "--destack-space-9": "64px",
        "--destack-size-1": "24px",
        "--destack-size-2": "32px",
        "--destack-size-3": "40px",
        "--destack-size-4": "48px",
        "--destack-radius-1": "3px",
        "--destack-radius-2": "4px",
        "--destack-radius-3": "6px",
        "--destack-radius-4": "8px",
        "--destack-radius-5": "12px",
        "--destack-radius-6": "16px",
        "--destack-radius-full": "0px",
        "--destack-stroke-border": "1px",
        "--destack-stroke-ring": "3px",
        "--destack-shadow-edge": "light-dark(#0000000f, #ffffff2c)",
        "--destack-shadow-cast": "light-dark(#0000001a, #00000066)",
        "--destack-shadow-inset":
            "inset 0px 0px 0px 1px var(--destack-shadow-edge), inset 0px 1.5px 2px 0px var(--destack-shadow-cast)",
        "--destack-shadow-raised":
            "0px 0px 0px 1px var(--destack-shadow-edge), 0px 1px 3px 0px var(--destack-shadow-cast)",
        "--destack-shadow-overlay":
            "0px 0px 0px 1px var(--destack-shadow-edge), 0px 12px 32px -16px var(--destack-shadow-cast), 0px 12px 60px 0px var(--destack-shadow-cast)",
        "--destack-motion-duration-short": "calc(150ms * var(--destack-motion-scale, 1))",
        "--destack-motion-duration-medium": "calc(300ms * var(--destack-motion-scale, 1))",
        "--destack-motion-duration-long": "calc(500ms * var(--destack-motion-scale, 1))",
        "--destack-motion-easing-standard": "cubic-bezier(0.2, 0, 0, 1)",
        "--destack-motion-easing-emphasised": "cubic-bezier(0.05, 0.7, 0.1, 1)",
        "--destack-motion-easing-spring":
            "linear(0, 0.036, 0.123, 0.239, 0.365, 0.489, 0.604, 0.705, 0.79, 0.86, 0.914, 0.955, 0.985, 1.005, 1.018, 1.025, 1.028, 1.028, 1.026, 1.023, 1.02, 1.016, 1.013, 1.01, 1.007, 1.005, 1.003, 1.002, 1.001, 1, 1, 0.999, 1)",
    });
});

test("resolve a seed accent's roles from the palette grown from it", () => {
    const seeded = defineTheme({ name: "violet", accent: "#7c3aed" }, { package: notes });

    // only the accent roles and the chart series picked around the accent change against the default theme
    const changed = changes(
        seeded.variables("system", DEFAULT_PREFERENCES),
        standard.variables("system", DEFAULT_PREFERENCES),
    );
    expect(Object.keys(changed)).toEqual(
        [
            "primary",
            "accent",
            "accentForeground",
            "ring",
            "chart1",
            "chart2",
            "chart3",
            "chart4",
            "chart5",
        ].flatMap((name) => {
            const variable = TOKENS.entry(["color", name]).variable;

            return variable in changed ? [variable] : [];
        }),
    );
    expect(changed["--destack-color-primary"]).toBe("#7c3aed");
});

test("override the theme with the person's accent, density, text size, contrast and motion", () => {
    const theme = defineTheme(
        { name: "dense", accent: "orange", density: "compact" },
        { package: notes },
    );
    const preferences = {
        textSize: "xxxLarge",
        density: "spacious",
        contrast: 1,
        motion: "reduced",
        accent: "grass",
    } as const;

    // the person's grass accent replaces orange at the most contrast, and spacious replaces compact
    const style = theme.variables("dark", preferences);
    const color = (name: RoleName) => {
        const light = theme.resolve("light", 1, "grass").color(name);
        const dark = theme.resolve("dark", 1, "grass").color(name);

        return light === dark ? light : `light-dark(${light}, ${dark})`;
    };
    expect(
        ROLE_NAMES.filter((name) => style[TOKENS.entry(["color", name]).variable] !== color(name)),
    ).toEqual([]);
    const changed = changes(
        withoutSwatches(style),
        standard.variables("system", DEFAULT_PREFERENCES),
    );
    expect(
        Object.fromEntries(
            Object.entries(changed).filter(([name]) => !name.startsWith("--destack-color-")),
        ),
    ).toEqual({
        "color-scheme": "dark",
        ...LARGEST_TEXT,
        ...Object.fromEntries(scaled("space", SPACE, 1.125)),
        ...Object.fromEntries(scaled("size", SIZE, 1.125)),
        ...Object.fromEntries(widths(1.125)),
        "--destack-motion-scale": "0",
    });
    expect(style["--destack-color-primary"]).toBe(color("primary"));
});

test("apply the theme's radius, scaling and fonts", () => {
    const theme = defineTheme(
        {
            name: "publication",
            radius: "full",
            scaling: "90%",
            fonts: { text: '"IBM Plex Sans", sans-serif', code: '"IBM Plex Mono", monospace' },
        },
        { package: notes },
    );
    const changed = changes(
        theme.variables("light", DEFAULT_PREFERENCES),
        standard.variables("light", DEFAULT_PREFERENCES),
    );

    // scale every length by 90% and the radii by the full treatment's 1.5 on top
    const text = TOKENS.family("text").entries.flatMap((entry) => {
        if (entry.token.$type !== "typography") {
            return [];
        }
        const { fontSize, letterSpacing } = entry.token.$value;
        const spacing = letterSpacing.value === 0 ? [] : [["letter-spacing", letterSpacing.value]];

        return [["font-size", fontSize.value], ...spacing].map(([member, length]) => [
            `${entry.variable}-${member}`,
            pixels(Number(length) * 0.9),
        ]);
    });
    expect(changed).toEqual({
        "--destack-text-family": '"IBM Plex Sans", sans-serif',
        "--destack-text-code-family": '"IBM Plex Mono", monospace',
        ...Object.fromEntries(text),
        ...Object.fromEntries(scaled("space", SPACE, 0.9)),
        ...Object.fromEntries(scaled("size", SIZE, 0.9)),
        ...Object.fromEntries(widths(0.9)),
        ...Object.fromEntries(scaled("radius", [3, 4, 6, 8, 12, 16], 1.35)),
        "--destack-radius-full": "13498.65px",
    });
});

test("cover every color and surface token with exactly one role", () => {
    expect(TOKENS.family("color").entries.map((entry) => entry.key)).toEqual([...ROLE_NAMES]);
    expect(TOKENS.family("surface").entries.map((entry) => entry.key)).toEqual(
        Object.keys(SURFACE_ROLES),
    );
});

/** Write a token color as hex, with two more digits for its opacity when translucent. */
function hex(color: {
    readonly components: readonly [number, number, number];
    readonly alpha?: number;
}): string {
    const opaque = Color.format(color.components);

    return color.alpha === undefined ? opaque : Color.translucent(opaque, color.alpha);
}
