import { Package } from "@destack/package";
import { expect, refusal, test } from "@destack/test";
import { defineTheme } from "../src/declare/index.ts";
import { apca, Color, GRAY_PRESETS, PRESET_NAMES, Scale } from "../src/palette/index.ts";
import {
    DEFAULT_PREFERENCES,
    ROLE_NAMES,
    RoleName,
    SURFACE_ROLES,
    SurfaceLevel,
    Theme,
    type ThemeStyle,
} from "../src/theme/index.ts";
import { TOKENS } from "../src/token/index.ts";

/** The package release declaring the fixture's themes. */
const notes = Package.parse({
    id: "package-019f5530-8000-7000-8000-000000000002",
    name: "@alice/notes",
    version: "2026.9.0",
});

/** The theme naming nothing beyond its name. */
const standard = defineTheme({ name: "standard" }, { package: notes });

/** List the declarations that differ from a baseline style. */
function changes(style: ThemeStyle, baseline: ThemeStyle): Record<string, string> {
    return Object.fromEntries(
        Object.entries(style).filter(([name, value]) =>
            Object.entries(baseline).every(([other, before]) => other !== name || before !== value),
        ),
    );
}

/** The spacing steps of tokens.json. */
const SPACE = [4, 8, 12, 16, 24, 32, 40, 48, 64];

/** The control heights of tokens.json. */
const SIZE = [24, 32, 40, 48];

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

/** List the scaled custom properties of a numbered token family. */
function scaled(family: string, lengths: readonly number[], factor: number): [string, string][] {
    return lengths.map((length, index) => [
        `--destack-${family}-${index + 1}`,
        pixels(length * factor),
    ]);
}

test("pass every text role's contrast for every preset accent on every gray", () => {
    const refused = GRAY_PRESETS.flatMap((gray) =>
        PRESET_NAMES.flatMap((accent) => {
            const theme = new Theme(notes, { name: "preset", gray, accent });
            try {
                theme.requireContrast();

                return [];
            } catch (error) {
                return [`${gray}/${accent}: ${String(error)}`];
            }
        }),
    );

    expect(refused).toEqual([]);
});

test("shift a seed's step 9 by a small lightness until its best label reads at Lc 60", () => {
    for (const seed of ["#ff8800", "#22c55e", "#0ea5e9"]) {
        const theme = defineTheme({ name: "seeded", accent: seed }, { package: notes });
        const solid = theme.scales.accent.color(9, "light");
        const label = theme.scales.accent.label(solid, theme.scales.gray.color(12, "light"));
        const before = Color.parse(seed);
        const after = Color.parse(solid);

        // the shift moves lightness alone, by less than 0.04, and the label reads
        const shift = Math.abs(after.lightness - before.lightness);
        expect([shift > 0, shift < 0.04]).toEqual([true, true]);
        expect(Math.abs(after.hue - before.hue)).toBeLessThan(1);
        expect(Math.abs(apca(label, solid))).toBeGreaterThanOrEqual(60);
    }
});

test("keep a legible preset's solid step and label it with the darkest candidate", () => {
    const amber = defineTheme({ name: "amber", accent: "amber" }, { package: notes });
    const preset = Scale.preset("amber");
    const label = amber.variables("light", DEFAULT_PREFERENCES)[
        "--destack-color-primary-foreground"
    ];

    expect([amber.scales.accent.light, amber.scales.accent.dark, label]).toEqual([
        preset.light,
        preset.dark,
        Scale.preset("slate").color(12, "light"),
    ]);
});

test("check the roles a theme replaces like every other role", async () => {
    const replaced = defineTheme(
        {
            name: "paper",
            roles: {
                background: { light: "#f8f5ee", dark: "#0b2029" },
                mutedForeground: { scale: "gray", step: 12 },
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
        "light-dark(#1c2024, #edeef0)",
    ]);

    // a background too close to its text is refused
    const illegible = Promise.resolve().then(() =>
        defineTheme(
            { name: "fog", roles: { background: { light: "#8b8d98", dark: "#111113" } } },
            { package: notes },
        ),
    );
    expect(await refusal(illegible)).toEqual([
        "INVALID_DEFINITION",
        "theme fog: foreground on background reads at Lc 41.0 in light with standard contrast, below 75",
    ]);
});

test("label an overridden primary with the candidate that reads best on it", () => {
    const theme = defineTheme(
        { name: "signal", roles: { primary: { scale: "amber", step: 9 } } },
        { package: notes },
    );
    const style = theme.variables("system", DEFAULT_PREFERENCES);

    // the gray's darkest step reads on amber, where the indigo accent's white label would not
    expect([style["--destack-color-primary"], style["--destack-color-primary-foreground"]]).toEqual(
        ["#ffc53d", "#1c2024"],
    );
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
            const role = standard.roles[name];

            return [
                key,
                role.color(standard.roles, standard.scales, "light", "standard"),
                role.color(standard.roles, standard.scales, "dark", "standard"),
            ];
        });

        expect(resolved).toEqual(shown);
    }
});

test("emit the default theme's custom properties for the system appearance", () => {
    // follow the device's contrast through color-mix() and its motion through the duration scale
    expect(standard.variables("system", DEFAULT_PREFERENCES)).toEqual({
        "color-scheme": "light dark",
        "--destack-color-background": "light-dark(#fcfcfd, #111113)",
        "--destack-color-foreground": "light-dark(#1c2024, #edeef0)",
        "--destack-color-card": "light-dark(#f9f9fb, #18191b)",
        "--destack-color-card-foreground": "light-dark(#1c2024, #edeef0)",
        "--destack-color-popover": "light-dark(#fcfcfd, #212225)",
        "--destack-color-popover-foreground": "light-dark(#1c2024, #edeef0)",
        "--destack-color-primary": "#3e63dd",
        "--destack-color-primary-foreground": "#ffffff",
        "--destack-color-secondary": "light-dark(#f0f0f3, #212225)",
        "--destack-color-secondary-foreground": "light-dark(#1c2024, #edeef0)",
        "--destack-color-muted": "light-dark(#f0f0f3, #212225)",
        "--destack-color-muted-foreground":
            "light-dark(color-mix(in oklab, #60646c, #1c2024 calc(var(--destack-contrast, 0) * 100%)), color-mix(in oklab, #b0b4ba, #edeef0 calc(var(--destack-contrast, 0) * 100%)))",
        "--destack-color-accent": "light-dark(#edf2fe, #182449)",
        "--destack-color-accent-foreground": "light-dark(#1f2d5c, #d6e1ff)",
        "--destack-color-destructive": "#e5484d",
        "--destack-color-destructive-foreground": "#ffffff",
        "--destack-color-success": "#30a46c",
        "--destack-color-success-foreground": "#ffffff",
        "--destack-color-warning": "#ffc53d",
        "--destack-color-warning-foreground": "#1c2024",
        "--destack-color-info": "#0090ff",
        "--destack-color-info-foreground": "#ffffff",
        "--destack-color-border":
            "light-dark(color-mix(in oklab, #d9d9e0, #b9bbc6 calc(var(--destack-contrast, 0) * 100%)), color-mix(in oklab, #363a3f, #5a6169 calc(var(--destack-contrast, 0) * 100%)))",
        "--destack-color-input":
            "light-dark(color-mix(in oklab, #cdced6, #b9bbc6 calc(var(--destack-contrast, 0) * 100%)), color-mix(in oklab, #43484e, #5a6169 calc(var(--destack-contrast, 0) * 100%)))",
        "--destack-color-ring": "light-dark(#8da4ef, #435db1)",
        "--destack-color-scrim": "light-dark(#1c202480, #11111380)",
        "--destack-surface-base": "light-dark(#fcfcfd, #111113)",
        "--destack-surface-raised": "light-dark(#f9f9fb, #18191b)",
        "--destack-surface-overlay": "light-dark(#fcfcfd, #212225)",
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

test("resolve a seed accent to the scale generated from it", () => {
    const seeded = defineTheme({ name: "violet", accent: "#7c3aed" }, { package: notes });
    const scale = Scale.generate("#7c3aed");
    const step = (number: 3 | 8 | 12) =>
        `light-dark(${scale.color(number, "light")}, ${scale.color(number, "dark")})`;

    // only the accent roles change against the default theme
    expect(
        changes(
            seeded.variables("system", DEFAULT_PREFERENCES),
            standard.variables("system", DEFAULT_PREFERENCES),
        ),
    ).toEqual({
        "--destack-color-primary": "#7c3aed",
        "--destack-color-accent": step(3),
        "--destack-color-accent-foreground": step(12),
        "--destack-color-ring": step(8),
    });
});

test("override the theme with the person's accent, density, text size, contrast and motion", () => {
    const theme = defineTheme(
        { name: "dense", accent: "orange", density: "compact" },
        { package: notes },
    );
    const preferences = {
        textSize: "xxxLarge",
        density: "spacious",
        contrast: "more",
        motion: "reduced",
        accent: "grass",
    } as const;

    // the person's grass accent replaces orange, spacious replaces compact, and more contrast picks fixed steps
    const grass = Scale.preset("grass");
    const step = (number: 3 | 8 | 9 | 12) =>
        `light-dark(${grass.color(number, "light")}, ${grass.color(number, "dark")})`;
    const changed = changes(
        theme.variables("dark", preferences),
        standard.variables("system", DEFAULT_PREFERENCES),
    );
    expect(changed).toEqual({
        "color-scheme": "dark",
        "--destack-color-primary": grass.color(9, "light"),
        "--destack-color-accent": step(3),
        "--destack-color-accent-foreground": step(12),
        "--destack-color-ring": step(8),
        "--destack-color-muted-foreground": "light-dark(#1c2024, #edeef0)",
        "--destack-color-border": "light-dark(#b9bbc6, #5a6169)",
        "--destack-color-input": "light-dark(#b9bbc6, #5a6169)",
        ...LARGEST_TEXT,
        ...Object.fromEntries(scaled("space", SPACE, 1.125)),
        ...Object.fromEntries(scaled("size", SIZE, 1.125)),
        "--destack-motion-scale": "0",
    });
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
