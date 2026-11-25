import { PackageVersion } from "@destack/package";
import { expect, refusal, test } from "@destack/test";
import { defineTheme, THEMES } from "../declare/index.ts";
import { Oklch, Srgb } from "../palette/index.ts";
import { DEFAULT_HUES, HUES } from "@destack/color";
import {
    DEFAULT_DISPLAY,
    GRAPHIC_CONTRAST,
    NEUTRAL_NAMES,
    ROLE_NAMES,
    RoleName,
    SWATCH_COLORS,
    TEXT_SCALES,
    Theme,
    type ThemeStyle,
} from "../theme/index.ts";
import { TOKENS } from "../token/index.ts";

/** The package release declaring the fixture's themes. */
const notes = PackageVersion.parse({
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

/** Read a theme root's role and scale properties, without the swatches the swatch test covers. */
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
    column: 272,
    "hover-card": 256,
    popover: 288,
    prose: 384,
    row: 160,
    sidebar: 256,
    "sidebar-icon": 48,
    tile: 120,
    toast: 356,
    dialog: 512,
    sheet: 384,
    tooltip: 320,
    palette: 720,
};

/** The text sizes and letter spacings at the largest text size. */
const LARGEST_TEXT = {
    "--destack-text-xs-font-size": "17.25px",
    "--destack-text-sm-font-size": "20.125px",
    "--destack-text-base-font-size": "23px",
    "--destack-text-lg-font-size": "25.875px",
    "--destack-text-xl-font-size": "28.75px",
    "--destack-text-2xl-font-size": "34.5px",
    "--destack-text-3xl-font-size": "43.125px",
    "--destack-text-4xl-font-size": "51.75px",
    "--destack-text-5xl-font-size": "69px",
    "--destack-text-6xl-font-size": "86.25px",
    "--destack-text-7xl-font-size": "103.5px",
    "--destack-text-8xl-font-size": "138px",
    "--destack-text-9xl-font-size": "184px",
    "--destack-text-2xl-letter-spacing": "-0.69px",
    "--destack-text-3xl-letter-spacing": "-0.862px",
    "--destack-text-4xl-letter-spacing": "-1.035px",
    "--destack-text-5xl-letter-spacing": "-1.38px",
    "--destack-text-6xl-letter-spacing": "-1.725px",
    "--destack-text-7xl-letter-spacing": "-2.07px",
    "--destack-text-8xl-letter-spacing": "-2.76px",
    "--destack-text-9xl-letter-spacing": "-3.68px",
    "--destack-text-xl-letter-spacing": "-0.288px",
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

/** Gray bases of every tint, from Radix Colors. */
const BASES = ["#8d8d8d", "#8e8c99", "#8b8d98", "#868e8b", "#898e87", "#8d8d86"];

/** Accents all around the wheel, pale, dark and without a hue. */
const ACCENTS = [
    ...HUES,
    "#e54666",
    "#6e56cf",
    "#7ce2fe",
    "#bdee63",
    "#808080",
    "#000000",
    "#ffffff",
];

test("give every accent on every base five chart series of hues apart, standing out in both appearances", () => {
    const failures = BASES.flatMap((base) =>
        ACCENTS.flatMap((accent) => {
            // resolve the series' marks and the background in both appearances
            const theme = new Theme({ name: "grid", base, accent });
            const series = theme
                .hues()
                .filter(([name]) => name.startsWith("series"))
                .map(([, seed]) => ({
                    light: theme.resolve("light", 0, seed).color("mark"),
                    dark: theme.resolve("dark", 0, seed).color("mark"),
                }));
            const background = {
                light: theme.resolve("light", 0).color("background"),
                dark: theme.resolve("dark", 0).color("background"),
            };

            // require every series to stand out and every two to lie apart
            const faint = series.filter((entry) =>
                (["light", "dark"] as const).some(
                    (scheme) =>
                        Math.abs(
                            Srgb.parse(entry[scheme]).contrast(
                                Srgb.parse(background[scheme]),
                                "apca",
                            ),
                        ) < GRAPHIC_CONTRAST.standard,
                ),
            );
            const marks = series.map((entry) => Oklch.parse(entry.light));
            const close = marks.flatMap((mark, index) =>
                marks.slice(index + 1).filter((other) => mark.distance(other) < 0.06),
            );

            return faint.length === 0 && close.length === 0 ? [] : [`${base}/${accent}`];
        }),
    );

    expect(failures).toEqual([]);
});

test("pass every role's contrast for every accent on every base", () => {
    const refused = BASES.flatMap((base) =>
        ACCENTS.flatMap((accent) => {
            const theme = new Theme({ name: "grid", base, accent });
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

test("shift a seed in lightness alone until its best label reads at Lc 60 and at 4.5 to 1", () => {
    for (const seed of ["#ff8800", "#22c55e", "#0ea5e9"]) {
        const theme = defineTheme({ name: "seeded", accent: seed }, { package: notes });
        const resolution = theme.resolve("light", 0);
        const solid = resolution.color("solid");
        const before = Oklch.parse(seed);
        const after = Oklch.parse(solid);

        // the shift moves lightness alone, and the label reads by both measures
        const shift = Math.abs(after.lightness - before.lightness);
        expect(shift).toBeGreaterThan(0);
        expect(Math.abs(after.hue - before.hue)).toBeLessThan(1);
        expect(resolution.contrast("label", "apca")).toBeGreaterThanOrEqual(60);
        expect(resolution.contrast("label", "wcag")).toBeGreaterThanOrEqual(4.5);
    }
});

test("keep a legible hue's seed and label it with the darkest text that reads best", () => {
    const yellow = defineTheme({ name: "yellow", accent: "yellow" }, { package: notes });
    const resolution = yellow.resolve("light", 0);

    expect([resolution.color("solid"), resolution.color("label")]).toEqual([
        DEFAULT_HUES.yellow,
        standard.resolve("light", 0).color("foreground"),
    ]);
});

test("draw control lines on every surface of a theme, with a ground of its own and without", () => {
    // blend each theme's control line into each surface, whose lightness it has to leave
    const bare = defineTheme({ name: "bare" }, { package: notes });
    const faint = [bare, ...THEMES].flatMap((theme) =>
        (["light", "dark"] as const).flatMap((scheme) =>
            (["background", "card", "popover"] as const).flatMap((surface) => {
                const resolution = theme.resolve(scheme, 0);
                const ground = resolution.color(surface);
                const blended = Srgb.composite(resolution.color("input"), ground);
                const step = Math.abs(
                    Oklch.parse(blended).lightness - Oklch.parse(ground).lightness,
                );

                return step < 0.08 ? [`${theme.name} ${scheme} ${surface} ${step.toFixed(3)}`] : [];
            }),
        ),
    );

    expect(faint).toEqual([]);
});

test("take a role's opacity in each appearance, or one for both", () => {
    // replace two roles, one at an opacity per appearance
    const from = { from: "foreground", lightness: { light: 0, dark: 0 } } as const;
    const theme = defineTheme(
        {
            name: "lined",
            ground: {
                light: { background: "#ffffff", foreground: "#000000", muted: "#eeeeee" },
                dark: { background: "#000000", foreground: "#ffffff", muted: "#222222" },
            },
            roles: {
                border: { ...from, alpha: 0.4 },
                input: { ...from, alpha: { light: 0.4, dark: 0.6 } },
            },
        },
        { package: notes },
    );
    const colors = (["light", "dark"] as const).map((scheme) => [
        theme.resolve(scheme, 0).color("border"),
        theme.resolve(scheme, 0).color("input"),
    ]);

    expect(colors).toEqual([
        ["#00000066", "#00000066"],
        ["#ffffff66", "#ffffff99"],
    ]);
});

test("read every text, line and graphic role against a nested surface's background", () => {
    // a selected row's accent fill becomes the background every role reads on
    const row = standard.rebase({ light: "#c9d8ff", dark: "#1d2f66" });
    for (const scheme of ["light", "dark"] as const) {
        const resolution = row.resolve(scheme, 0);
        const unread = ROLE_NAMES.filter((name) => {
            const reading = row.roles[name].reading;
            if (reading === undefined) {
                return false;
            }
            const floor = resolution.floor(reading.contrast);

            return reading.on.some((on) => resolution.contrast(name, "apca", on) < floor);
        });
        expect([resolution.color("background"), unread]).toEqual([
            scheme === "light" ? "#c9d8ff" : "#1d2f66",
            [],
        ]);
    }

    // the surfaces above the background follow it
    const card = Oklch.parse(row.resolve("light", 0).color("card")).lightness;
    expect(card - Oklch.parse("#c9d8ff").lightness).toBeCloseTo(0.02, 2);
});

test("raise every reading role's contrast continuously with the person's level", () => {
    // each level reads at least at its floor, and the floors rise from the standard to the most
    const levels = [0, 0.25, 0.5, 0.75, 1];
    const reads = levels.map((level) => {
        const resolution = standard.resolve("light", level);

        return ROLE_NAMES.filter((name) => {
            const reading = standard.roles[name].reading;
            if (reading === undefined) {
                return false;
            }
            const floor = resolution.floor(reading.contrast);

            return reading.on.some((on) => resolution.contrast(name, "apca", on) < floor);
        });
    });
    const muted = levels.map((level) =>
        standard.resolve("light", level).contrast("mutedForeground", "apca"),
    );

    expect(reads).toEqual([[], [], [], [], []]);
    expect(muted).toEqual(muted.toSorted((left, right) => left - right));
    expect(
        standard.variables({ ...DEFAULT_DISPLAY, appearance: "light", contrast: 0.5 })[
            "--destack-color-muted-foreground"
        ],
    ).toBe(
        `light-dark(${standard.resolve("light", 0.5).color("mutedForeground")}, ${standard.resolve("dark", 0.5).color("mutedForeground")})`,
    );
});

test("grow the tags, the statuses and the chart series from the theme's hues", () => {
    const hued = defineTheme(
        { name: "hued", hues: { red: "#e54d2e", yellow: "#b58900", gray: "#839496" } },
        { package: notes },
    );
    const hues = new Map(hued.hues());
    const named = ["red", "destructive", "yellow", "warning", "gray", "green", "success"];

    // the declared red deepens until its white label reads at 4.5 to 1
    expect([
        named.map((name) => hues.get(name)),
        hued.resolve("light", 0, "#e54d2e").color("solid"),
    ]).toEqual([
        [
            "#e54d2e",
            "#e54d2e",
            "#b58900",
            "#b58900",
            "#839496",
            DEFAULT_HUES.green,
            DEFAULT_HUES.green,
        ],
        "#d64121",
    ]);
});

test("draw an accent named by a hue in the theme's own color, a person's too", () => {
    // accent a theme by its own green, and let a person choose its blue and a color of their own
    const own = defineTheme(
        { name: "own", accent: "green", hues: { green: "#4d7c0f", blue: "#1d6fa5" } },
        { package: notes },
    );
    const accented = (accent: string | null) =>
        own.variables({ ...DEFAULT_DISPLAY, contrast: 0, accent })["--destack-swatch-accent-solid"];

    expect([accented(null), accented("blue"), accented("#be185d")]).toEqual([
        own.resolve("light", 0, "green").color("solid"),
        own.resolve("light", 0, "blue").color("solid"),
        own.resolve("light", 0, "#be185d").color("solid"),
    ]);
    expect([own.hues()[0], own.hues("blue")[0]]).toEqual([
        ["accent", "#4d7c0f"],
        ["accent", "#1d6fa5"],
    ]);
});

test("draw five chart series apart from hues too pale to read as given", () => {
    // give every hue a pastel, which a mark darkens until it reads
    const hues = Object.fromEntries(
        ["red", "orange", "yellow", "green", "cyan", "blue", "purple", "pink"].map(
            (name, index) => [name, new Oklch(0.92, 0.05, index * 45).hex()] as const,
        ),
    );
    const pale = new Theme({ name: "pale", hues });
    const series = pale.hues().filter(([name]) => name.startsWith("series"));
    const marks = series.map(([, seed]) => pale.resolve("light", 0, seed).contrast("mark", "apca"));

    expect([
        new Set(series.map(([, seed]) => seed)).size,
        marks.map((mark) => mark >= GRAPHIC_CONTRAST.standard),
    ]).toEqual([5, [true, true, true, true, true]]);
});

test("apply a replaced solid under the theme's own accent alone", () => {
    const branded = defineTheme(
        { name: "branded", roles: { solid: { light: "#c54e0d", dark: "#c54e0d" } } },
        { package: notes },
    );

    // another accent and every other hue's swatch take the solid their palette grows
    expect([
        branded.resolve("light", 0).color("solid"),
        branded.resolve("light", 0, "cyan").color("solid"),
        branded.variables(DEFAULT_DISPLAY)["--destack-swatch-cyan-solid"],
    ]).toEqual([
        "#c54e0d",
        standard.resolve("light", 0, "cyan").color("solid"),
        standard.variables(DEFAULT_DISPLAY)["--destack-swatch-cyan-solid"],
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
    const style = replaced.variables(DEFAULT_DISPLAY);
    expect([
        style["--destack-color-background"],
        style["--destack-color-muted-foreground"],
    ]).toEqual([
        "light-dark(#f8f5ee, #0b2029)",
        "light-dark(#1f1f22, color-mix(in oklab, #ededf2, #f0f0f5 calc(var(--destack-contrast, 0) * 100%)))",
    ]);

    // a background too close to its text is refused
    const illegible = Promise.resolve().then(() =>
        defineTheme(
            { name: "fog", roles: { background: { light: "#8b8d98", dark: "#101113" } } },
            { package: notes },
        ).requireContrast(),
    );
    expect(await refusal(illegible)).toEqual([
        "BAD_REQUEST",
        "DEFINITION_INVALID",
        "theme 'fog' is invalid: 'foreground' on 'background' reads at Lc 43.7 in 'light' with 'standard' contrast, below 75",
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

            return [resolution.color("border"), resolution.contrast("border", "apca").toFixed(1)];
        }),
    );

    // standard contrast leaves a fifth, more contrast raises it exactly to the line floor of Lc 30
    expect(borders).toEqual([
        ["#1f1f2233", "23.3"],
        ["#1f1f2241", "30.4"],
        ["#ededf233", "7.5"],
        ["#ededf276", "30.5"],
    ]);
});

test("label an overridden solid with the candidate that reads best on it", () => {
    const theme = defineTheme(
        {
            name: "signal",
            roles: { solid: { light: DEFAULT_HUES.yellow, dark: DEFAULT_HUES.yellow } },
        },
        { package: notes },
    );
    const resolution = theme.resolve("light", 0);

    // the base's darkest text reads on amber, where the indigo accent's white label would not
    expect([resolution.color("solid"), resolution.color("label")]).toEqual([
        DEFAULT_HUES.yellow,
        standard.resolve("light", 0).color("foreground"),
    ]);
});

test("resolve the default theme's neutral roles to the color values tokens.json shows", () => {
    // compare each role's standard light and dark color with its token
    const shown = TOKENS.family("color").entries.map((entry) => [
        entry.key,
        entry.token.$type === "color" ? hex(entry.token.$value) : entry.token.$type,
        entry.token.$type === "color" ? hex(entry.token.$extensions["app.destack"].dark) : "",
    ]);
    const resolved = shown.map(([key = ""]) => [
        key,
        standard.resolve("light", 0).color(RoleName.parse(key)),
        standard.resolve("dark", 0).color(RoleName.parse(key)),
    ]);

    expect(resolved).toEqual(shown);
});

test("emit one swatch per hue as the roles it takes, at the values tokens.json shows", () => {
    // read each swatch token's light and dark value and the theme's property at the standard contrast
    const style = standard.variables({ ...DEFAULT_DISPLAY, contrast: 0 });
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
        hues: swatches.length / SWATCH_COLORS.length,
        cyan: swatches.filter(([name]) => String(name).includes("-cyan-")).map(([name]) => name),
        matching: swatches.every(([, isMatching]) => isMatching === true),
        solid: standard.resolve("light", 0, "cyan").color("solid"),
    }).toEqual({
        hues: 20,
        cyan: [
            "--destack-swatch-cyan-solid",
            "--destack-swatch-cyan-label",
            "--destack-swatch-cyan-tint",
            "--destack-swatch-cyan-tint-text",
            "--destack-swatch-cyan-text",
            "--destack-swatch-cyan-mark",
        ],
        matching: true,
        solid: "#00819f",
    });
});

test("emit the default theme's custom properties for the system appearance", () => {
    // follow the device's contrast through color-mix() and its motion through the duration scale
    expect(withoutSwatches(standard.variables(DEFAULT_DISPLAY))).toEqual({
        "color-scheme": "light dark",
        "--destack-color-background": "light-dark(#fcfdfe, #131315)",
        "--destack-color-card": "light-dark(#ffffff, #28282b)",
        "--destack-color-popover": "light-dark(#ffffff, #2e2e31)",
        "--destack-color-sidebar": "light-dark(#f6f7f8, #1a1a1c)",
        "--destack-color-foreground": "light-dark(#1f1f22, #ededf2)",
        "--destack-color-muted-foreground":
            "light-dark(color-mix(in oklab, #62636c, #3c3d43 calc(var(--destack-contrast, 0) * 100%)), color-mix(in oklab, #b7b9c2, #eaebf0 calc(var(--destack-contrast, 0) * 100%)))",
        "--destack-color-muted": "light-dark(#f1f2f3, #252527)",
        "--destack-color-hover": "light-dark(#1f1f220a, #ededf20a)",
        "--destack-color-selected": "light-dark(#1f1f220f, #ededf20f)",
        "--destack-color-border":
            "light-dark(color-mix(in oklab, #1f1f222e, #1f1f2241 calc(var(--destack-contrast, 0) * 100%)), color-mix(in oklab, #ededf22e, #ededf276 calc(var(--destack-contrast, 0) * 100%)))",
        "--destack-color-input":
            "light-dark(color-mix(in oklab, #1f1f2233, #1f1f2241 calc(var(--destack-contrast, 0) * 100%)), color-mix(in oklab, #ededf240, #ededf276 calc(var(--destack-contrast, 0) * 100%)))",
        "--destack-color-ring":
            "light-dark(color-mix(in oklab, #83a6ff, #5f85f3 calc(var(--destack-contrast, 0) * 100%)), color-mix(in oklab, #6b92fa, #91b1ff calc(var(--destack-contrast, 0) * 100%)))",
        "--destack-color-scrim": "light-dark(#1f1f2259, #00000059)",
        "--destack-font-text-adjust": "none",
        "--destack-font-text":
            '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", system-ui, sans-serif, "Apple Color Emoji", "Segoe UI Emoji"',
        "--destack-font-code":
            'Menlo, Consolas, "Bitstream Vera Sans Mono", monospace, "Apple Color Emoji", "Segoe UI Emoji"',
        "--destack-text-2xl-font-family": "var(--destack-font-text)",
        "--destack-text-2xl-font-size": "24px",
        "--destack-text-2xl-font-size-adjust": "var(--destack-font-text-adjust, none)",
        "--destack-text-2xl-font-weight": "400",
        "--destack-text-2xl-letter-spacing": "-0.48px",
        "--destack-text-2xl-line-height": "1.3333",
        "--destack-text-3xl-font-family": "var(--destack-font-text)",
        "--destack-text-3xl-font-size": "30px",
        "--destack-text-3xl-font-size-adjust": "var(--destack-font-text-adjust, none)",
        "--destack-text-3xl-font-weight": "400",
        "--destack-text-3xl-letter-spacing": "-0.6px",
        "--destack-text-3xl-line-height": "1.2",
        "--destack-text-4xl-font-family": "var(--destack-font-text)",
        "--destack-text-4xl-font-size": "36px",
        "--destack-text-4xl-font-size-adjust": "var(--destack-font-text-adjust, none)",
        "--destack-text-4xl-font-weight": "400",
        "--destack-text-4xl-letter-spacing": "-0.72px",
        "--destack-text-4xl-line-height": "1.1111",
        "--destack-text-5xl-font-family": "var(--destack-font-text)",
        "--destack-text-5xl-font-size": "48px",
        "--destack-text-5xl-font-size-adjust": "var(--destack-font-text-adjust, none)",
        "--destack-text-5xl-font-weight": "400",
        "--destack-text-5xl-letter-spacing": "-0.96px",
        "--destack-text-5xl-line-height": "1",
        "--destack-text-6xl-font-family": "var(--destack-font-text)",
        "--destack-text-6xl-font-size": "60px",
        "--destack-text-6xl-font-size-adjust": "var(--destack-font-text-adjust, none)",
        "--destack-text-6xl-font-weight": "400",
        "--destack-text-6xl-letter-spacing": "-1.2px",
        "--destack-text-6xl-line-height": "1",
        "--destack-text-7xl-font-family": "var(--destack-font-text)",
        "--destack-text-7xl-font-size": "72px",
        "--destack-text-7xl-font-size-adjust": "var(--destack-font-text-adjust, none)",
        "--destack-text-7xl-font-weight": "400",
        "--destack-text-7xl-letter-spacing": "-1.44px",
        "--destack-text-7xl-line-height": "1",
        "--destack-text-8xl-font-family": "var(--destack-font-text)",
        "--destack-text-8xl-font-size": "96px",
        "--destack-text-8xl-font-size-adjust": "var(--destack-font-text-adjust, none)",
        "--destack-text-8xl-font-weight": "400",
        "--destack-text-8xl-letter-spacing": "-1.92px",
        "--destack-text-8xl-line-height": "1",
        "--destack-text-9xl-font-family": "var(--destack-font-text)",
        "--destack-text-9xl-font-size": "128px",
        "--destack-text-9xl-font-size-adjust": "var(--destack-font-text-adjust, none)",
        "--destack-text-9xl-font-weight": "400",
        "--destack-text-9xl-letter-spacing": "-2.56px",
        "--destack-text-9xl-line-height": "1",
        "--destack-text-base-font-family": "var(--destack-font-text)",
        "--destack-text-base-font-size": "16px",
        "--destack-text-base-font-size-adjust": "var(--destack-font-text-adjust, none)",
        "--destack-text-base-font-weight": "400",
        "--destack-text-base-letter-spacing": "0px",
        "--destack-text-base-line-height": "1.5",
        "--destack-text-lg-font-family": "var(--destack-font-text)",
        "--destack-text-lg-font-size": "18px",
        "--destack-text-lg-font-size-adjust": "var(--destack-font-text-adjust, none)",
        "--destack-text-lg-font-weight": "400",
        "--destack-text-lg-letter-spacing": "0px",
        "--destack-text-lg-line-height": "1.5556",
        "--destack-text-sm-font-family": "var(--destack-font-text)",
        "--destack-text-sm-font-size": "14px",
        "--destack-text-sm-font-size-adjust": "var(--destack-font-text-adjust, none)",
        "--destack-text-sm-font-weight": "400",
        "--destack-text-sm-letter-spacing": "0px",
        "--destack-text-sm-line-height": "1.4286",
        "--destack-text-xl-font-family": "var(--destack-font-text)",
        "--destack-text-xl-font-size": "20px",
        "--destack-text-xl-font-size-adjust": "var(--destack-font-text-adjust, none)",
        "--destack-text-xl-font-weight": "400",
        "--destack-text-xl-letter-spacing": "-0.2px",
        "--destack-text-xl-line-height": "1.4",
        "--destack-text-xs-font-family": "var(--destack-font-text)",
        "--destack-text-xs-font-size": "12px",
        "--destack-text-xs-font-size-adjust": "var(--destack-font-text-adjust, none)",
        "--destack-text-xs-font-weight": "400",
        "--destack-text-xs-letter-spacing": "0px",
        "--destack-text-xs-line-height": "1.3333",
        "--destack-weight-regular": "400",
        "--destack-weight-medium": "500",
        "--destack-weight-semibold": "600",
        "--destack-width-column": "272px",
        "--destack-width-dialog": "512px",
        "--destack-width-sheet": "384px",
        "--destack-width-tooltip": "320px",
        "--destack-width-palette": "720px",
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
        "--destack-radius-full": "9999px",
        "--destack-stroke-border": "1px",
        "--destack-stroke-ring": "2px",
        "--destack-shadow-edge": "light-dark(#0000000f, #ffffff18)",
        "--destack-shadow-cast": "light-dark(#0000001a, #00000066)",
        "--destack-shadow-raised":
            "0px 0px 0px 1px var(--destack-shadow-edge), 0px 1px 3px 0px var(--destack-shadow-cast)",
        "--destack-shadow-overlay":
            "0px 12px 32px -16px var(--destack-shadow-cast), 0px 6px 24px -4px var(--destack-shadow-cast)",
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

    // only the ring, the accent's swatch and the first chart series change, its white label staying
    const changed = changes(seeded.variables(DEFAULT_DISPLAY), standard.variables(DEFAULT_DISPLAY));
    const hues = ["accent", "series1"];
    expect(Object.keys(changed)).toEqual([
        "--destack-color-ring",
        ...hues.flatMap((hue) =>
            ["solid", "tint", "tint-text", "text", "mark"].map(
                (color) => `--destack-swatch-${hue}-${color}`,
            ),
        ),
    ]);
    expect(changed["--destack-swatch-accent-solid"]).toBe("#7c3aed");
});

test("override the theme with the person's accent, density, text size, contrast and motion", () => {
    const theme = defineTheme(
        { name: "dense", accent: "orange", density: "compact" },
        { package: notes },
    );
    const display = {
        textSize: "xxxLarge",
        typeface: null,
        density: "spacious",
        contrast: 1,
        motion: "reduced",
        accent: "green",
    } as const;

    // the person's green accent replaces orange at the most contrast, and spacious replaces compact
    const style = theme.variables({ ...display, appearance: "dark" });
    const color = (name: RoleName) => {
        const light = theme.resolve("light", 1, "green").color(name);
        const dark = theme.resolve("dark", 1, "green").color(name);

        return light === dark ? light : `light-dark(${light}, ${dark})`;
    };
    expect(
        NEUTRAL_NAMES.filter(
            (name) => style[TOKENS.entry(["color", name]).variable] !== color(name),
        ),
    ).toEqual([]);
    const changed = changes(withoutSwatches(style), standard.variables(DEFAULT_DISPLAY));
    expect(
        Object.fromEntries(
            Object.entries(changed).filter(([name]) => !name.startsWith("--destack-color-")),
        ),
    ).toEqual({
        "color-scheme": "dark",
        ...LARGEST_TEXT,
        // spacing and control heights grow with density and the text, widths with density alone
        ...Object.fromEntries(scaled("space", SPACE, 1.125 * TEXT_SCALES.xxxLarge)),
        ...Object.fromEntries(scaled("size", SIZE, 1.125 * TEXT_SCALES.xxxLarge)),
        ...Object.fromEntries(widths(1.125)),
        "--destack-motion-scale": "0",
    });
    expect(style["--destack-swatch-accent-solid"]).toBe(color("solid"));
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
        theme.variables({ ...DEFAULT_DISPLAY, appearance: "light" }),
        standard.variables({ ...DEFAULT_DISPLAY, appearance: "light" }),
    );

    // scale every length by 90% and the radii by the full treatment's 1.5 on top, and drop the token font's tracking
    const text = TOKENS.family("text").entries.flatMap((entry) => {
        if (entry.token.$type !== "typography") {
            return [];
        }
        const { fontSize, letterSpacing } = entry.token.$value;
        const size = [`${entry.variable}-font-size`, pixels(fontSize.value * 0.9)];
        const spacing =
            letterSpacing.value === 0 ? [] : [[`${entry.variable}-letter-spacing`, "0px"]];

        return [size, ...spacing];
    });
    expect(changed).toEqual({
        "--destack-font-text": '"IBM Plex Sans", sans-serif',
        "--destack-font-code": '"IBM Plex Mono", monospace',
        ...Object.fromEntries(text),
        ...Object.fromEntries(scaled("space", SPACE, 0.9)),
        ...Object.fromEntries(scaled("size", SIZE, 0.9)),
        ...Object.fromEntries(widths(0.9)),
        ...Object.fromEntries(scaled("radius", [3, 4, 6, 8, 12, 16], 1.35)),
        "--destack-radius-full": "13498.65px",
    });
});

test("cover every color token with one neutral role and every swatch token with one hue's role", () => {
    const hues = standard.hues().map(([name]) => name);
    expect([
        TOKENS.family("color").entries.map((entry) => entry.key),
        TOKENS.family("swatch").entries.map((entry) => entry.key),
    ]).toEqual([
        [...NEUTRAL_NAMES],
        hues.flatMap((hue) =>
            SWATCH_COLORS.map((color) => `${hue}${color.charAt(0).toUpperCase()}${color.slice(1)}`),
        ),
    ]);
});

/** Write a token color as hex, with two more digits for its opacity when translucent. */
function hex(color: {
    readonly components: readonly [number, number, number];
    readonly alpha?: number;
}): string {
    const opaque = Srgb.format(color.components);

    return color.alpha === undefined ? opaque : Srgb.translucent(opaque, color.alpha);
}

test("change the size, weight, tracking and font of the text styles a theme overrides", () => {
    const theme = defineTheme(
        {
            name: "publication",
            text: {
                "5xl": { size: { min: 32, max: 48 }, weight: "medium", tracking: -0.02 },
                "2xl": { size: 22, weight: "medium" },
                xs: { font: "code", tracking: 0.1 },
            },
        },
        { package: notes },
    );

    // point each changed value at its token or write it, growing the fluid size between a 360px and a 1280px window
    expect(
        changes(
            theme.variables({ ...DEFAULT_DISPLAY, appearance: "light" }),
            standard.variables({ ...DEFAULT_DISPLAY, appearance: "light" }),
        ),
    ).toEqual({
        "--destack-text-5xl-font-size": "clamp(32px, calc(25.739px + 1.7391vw), 48px)",
        "--destack-text-5xl-font-weight": "var(--destack-weight-medium)",
        "--destack-text-5xl-letter-spacing": "-0.02em",
        "--destack-text-2xl-font-size": "22px",
        "--destack-text-2xl-font-weight": "var(--destack-weight-medium)",
        "--destack-text-xs-font-family": "var(--destack-font-code)",
        "--destack-text-xs-letter-spacing": "0.1em",
        "--destack-text-xs-font-size-adjust": "none",
    });
});
