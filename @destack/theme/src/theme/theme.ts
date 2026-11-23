import { PackageVersion, PackageError } from "@destack/package";
import { present, schema } from "@destack/schema";
import { HexColor, Oklch, Palette, type ColorScheme } from "../palette/index.ts";
import { DEFAULT_HUES, Hue, HUES } from "@destack/color";
import {
    FONT_SIZE_ADJUST,
    MOTION_SCALE,
    TOKENS,
    VARIABLE_PREFIX,
    type Variable,
} from "../token/index.ts";
import {
    Accent,
    DENSITY_SCALES,
    Density,
    TEXT_SCALES,
    TYPEFACES,
    type Contrast,
    type Display,
} from "./display.ts";
import {
    COLOR_ROLES,
    LABEL_RATIO,
    Resolution,
    ROLE_NAMES,
    RoleName,
    RoleOverride,
    isSwatchColor,
    SWATCH_COLORS,
    type Palettes,
    type Role,
    type Roles,
} from "./role.ts";

/** The radius multiplier of each corner treatment. */
const RADIUS_SCALES = { none: 0, small: 0.75, medium: 1, large: 1.5, full: 1.5 } as const;

/** The base of a theme without one. */
const DEFAULT_BASE = DEFAULT_HUES.gray;

/** The accent of a theme without one, Radix Colors' indigo. */
const DEFAULT_ACCENT = "#3e63dd";

/** The statuses a theme colors. */
const STATUS_NAMES = ["destructive", "success", "warning", "info"] as const;

/** The hue of each status. */
const STATUS_HUES = {
    destructive: "red",
    success: "green",
    warning: "yellow",
    info: "blue",
} as const satisfies Record<(typeof STATUS_NAMES)[number], Hue>;

/** The swatches drawn in the accent, under the theme's overrides of its roles. */
const ACCENT_SWATCHES: ReadonlySet<string> = new Set(["accent", "series1"]);

/** The hues chart series take: the colorful ones, without red, which reads as an error. */
const SERIES_HUES = ["orange", "yellow", "green", "cyan", "blue", "purple", "pink"] as const;

/** The corner treatment of a theme that names none. */
const DEFAULT_RADIUS = "medium";

/** The interface scale of a theme that names none. */
const DEFAULT_SCALING = "100%";

/** The density of a theme that names none. */
const DEFAULT_DENSITY = "regular";

/** The windows a fluid text size grows between: a small phone, and the 1280px frame. */
const FLUID_WINDOWS = { min: 360, max: 1280 } as const;

/** The chart series a theme has, the accent first. */
const SERIES_COUNT = 5;

/** The opacity of a line drawn in the text color, a hairline on any ground. */
const LINE_ALPHA = 0.18;

/** The appearances a theme's contrast is checked under. */
const SCHEMES = ["light", "dark"] as const satisfies readonly ColorScheme[];

/** The contrast levels a theme is checked at: standard and more. */
const LEVELS = [0, 1] as const;

/** The custom property the stylesheet sets to 1 when the device asks for more contrast. */
const CONTRAST_VARIABLE = `${VARIABLE_PREFIX}-contrast`;

/** A package-local theme name. */
export const ThemeName = schema.string().regex(/^[a-z][a-zA-Z0-9]*$/u);

/** A corner treatment. */
export const Radius = schema.enum(["none", "small", "medium", "large", "full"]);
/** A corner treatment. */
export type Radius = schema.Infer<typeof Radius>;

/** A proportional interface scale. */
export const Scaling = schema.enum(["90%", "95%", "100%", "105%", "110%"]);
/** A proportional interface scale. */
export type Scaling = schema.Infer<typeof Scaling>;

/** A text style of the type scale. */
export const TextStyleName = schema.enum([
    "xs",
    "sm",
    "base",
    "lg",
    "xl",
    "2xl",
    "3xl",
    "4xl",
    "5xl",
    "6xl",
    "7xl",
    "8xl",
    "9xl",
]);
/** A text style of the type scale. */
export type TextStyleName = schema.Infer<typeof TextStyleName>;

/** A font weight by its token name. */
export const WeightName = schema.enum(["regular", "medium", "semibold", "bold"]);
/** A font weight by its token name. */
export type WeightName = schema.Infer<typeof WeightName>;

/** A font size in pixels at the medium text size, fixed or growing with the window. */
export const FontSize = schema.union([
    schema.number().positive(),
    schema.object({
        /** The size in the narrowest window. */
        min: schema.number().positive(),
        /** The size in the widest window. */
        max: schema.number().positive(),
    }),
]);
/** A font size in pixels at the medium text size, fixed or growing with the window. */
export type FontSize = schema.Infer<typeof FontSize>;

/** A theme's change to one text style. */
export const TextOverride = schema.object({
    /** The size the style takes instead of its token size. */
    size: FontSize.exactOptional(),
    /** The weight the style takes instead of its token weight. */
    weight: WeightName.exactOptional(),
    /** The letter spacing in ems instead of the token spacing. */
    tracking: schema.number().exactOptional(),
    /** The font the style sets in instead of the text font. */
    font: schema.enum(["text", "code"]).exactOptional(),
});
/** A theme's change to one text style. */
export type TextOverride = schema.Infer<typeof TextOverride>;

/** A theme's page, text and quiet fill at one time of day, as six-digit sRGB hex colors. */
export const Ground = schema.object({
    /** The page. */
    background: HexColor,
    /** The text on it. */
    foreground: HexColor,
    /** The quiet fills on it. */
    muted: HexColor,
});
/** A theme's page, text and quiet fill at one time of day. */
export type Ground = schema.Infer<typeof Ground>;

/** A theme as authored, every field beside the name optional. */
export const ThemeDefinition = schema.object({
    /** The name, package-local for a declared theme. */
    name: ThemeName,
    /** The seed of surfaces, text and lines, slate when absent. */
    base: HexColor.exactOptional(),
    /** The seed of primary actions, checked controls, focus and the first chart series. */
    accent: Accent.exactOptional(),
    /** The page, text and quiet fills by day and by night, which lines then follow. */
    ground: schema
        .object({
            /** The ground by day. */
            light: Ground,
            /** The ground by night. */
            dark: Ground,
        })
        .exactOptional(),
    /** The seeds of the hues, the base for gray and Destack's own for each other one left out. */
    hues: schema.partialRecord(Hue, HexColor).exactOptional(),
    /** The corner treatment, medium when absent. */
    radius: Radius.exactOptional(),
    /** The interface scale, 100% when absent. */
    scaling: Scaling.exactOptional(),
    /** The font stacks of interface and code text as CSS `font-family` values. */
    fonts: schema
        .object({
            /** The interface font stack. */
            text: schema.string().min(1).exactOptional(),
            /** The code font stack. */
            code: schema.string().min(1).exactOptional(),
        })
        .exactOptional(),
    /** The text styles the theme changes, such as lighter headings for its font. */
    text: schema.partialRecord(TextStyleName, TextOverride).exactOptional(),
    /** The density a person's density setting overrides, regular when absent. */
    density: Density.exactOptional(),
    /** The roles the theme replaces after its ground, with a palette's tone or explicit colors. */
    roles: schema.partialRecord(RoleName, RoleOverride).exactOptional(),
});
/** A theme as authored. */
export type ThemeDefinition = schema.Infer<typeof ThemeDefinition>;

/** A theme declaration as manifests describe it. */
export const ThemeDescription = ThemeDefinition.extend({
    /** The declaring package. */
    package: PackageVersion,
});
/** A theme declaration as manifests describe it. */
export type ThemeDescription = schema.Infer<typeof ThemeDescription>;

/** The declarations of a theme root's style attribute. */
export type ThemeStyle = {
    /** Browser appearance used by light-dark() and native controls. */
    "color-scheme": "light dark" | "light" | "dark";
    /** The token values. */
    [variable: Variable]: string;
};

/** IBM Plex Sans's x-height over its em, 516 of its 1000 units, which other faces are sized to. */
const PLEX_EX_HEIGHT = 0.516;

/** A theme: seeds, ground, roles, corners, scaling, fonts and density. */
export class Theme {
    /** The name, seeds, ground, corners, scaling, fonts, density and role replacements. */
    readonly definition: ThemeDefinition;
    /** The color roles with the theme's replacements, under its accent. */
    readonly roles: Roles;
    /** The seed of the theme's accent. */
    readonly #accent: string;
    /** The palettes of each hue, by seed. */
    readonly #palettes = new Map<string, Palettes>();
    /** The chart series seeds around each accent, by the accent's seed. */
    readonly #series = new Map<string, readonly string[]>();
    /** The resolved roles of each hue, appearance and contrast level. */
    readonly #resolutions = new Map<string, Resolution>();

    /** Create a theme, applying its role replacements under its accent. */
    constructor(definition: ThemeDefinition) {
        // set the definition, then apply its replacements under its accent
        this.definition = definition;
        this.#accent = this.color(definition.accent ?? DEFAULT_ACCENT);
        this.roles = this.#rolesOf();
    }

    /** The name. */
    get name(): string {
        return this.definition.name;
    }

    /** Derive the theme over another background, such as a selected row's. */
    rebase(background: RoleOverride): Theme {
        const roles = { ...this.definition.roles, background };

        return new Theme({ ...this.definition, roles });
    }

    /** Resolve the roles of an appearance at a contrast level from 0 to 1, under a hue. */
    resolve(scheme: ColorScheme, level: number, hue?: Accent): Resolution {
        // reuse the resolution of this hue, appearance and level
        const seed = hue === undefined ? this.#accent : this.color(hue);
        const key = `${hue ?? ""}:${scheme}:${level}`;
        const known = this.#resolutions.get(key);
        if (known !== undefined) {
            return known;
        }
        const resolution = new Resolution(
            this.#rolesOf(hue),
            this.#palettesOf(seed),
            scheme,
            level,
        );
        this.#resolutions.set(key, resolution);

        return resolution;
    }

    /** Compute a theme root's custom properties for a person's display. */
    variables(display: Display): ThemeStyle {
        // set the appearance light-dark() and native controls follow
        const { appearance } = display;
        const style: ThemeStyle = {
            "color-scheme": appearance === "system" ? "light dark" : appearance,
        };

        // resolve the neutral roles under the person's accent over the theme's, at their contrast
        const accent = display.accent === null ? this.#accent : this.color(display.accent);
        const own = accent === this.#accent ? undefined : accent;
        for (const entry of TOKENS.family("color").entries) {
            style[entry.variable] = this.#css(RoleName.parse(entry.key), display.contrast, own);
        }

        // resolve the swatch of every hue, the accent's own under its overrides
        for (const [hue, seed] of this.hues(accent)) {
            for (const name of SWATCH_COLORS) {
                const entry = TOKENS.entry(["swatch", `${hue}${capitalize(name)}`]);
                const shown = ACCENT_SWATCHES.has(hue) ? own : seed;
                style[entry.variable] = this.#css(name, display.contrast, shown);
            }
        }

        // scale every other token by the theme and the person's display
        for (const family of TOKENS.families) {
            const factor = this.#factor(family.name, display);
            if (factor === undefined) {
                continue;
            }
            for (const entry of family.entries) {
                for (const [variable, value] of entry.values(factor)) {
                    style[variable] = value;
                }
            }
        }

        // apply the theme's fonts and corners and the person's motion
        this.#customize(style, display);

        return style;
    }

    /** Refuse text below its APCA floor and labels below 4.5 to 1, at each contrast level. */
    requireContrast(levels: readonly number[] = LEVELS): void {
        // check each appearance at each level
        const checks = SCHEMES.flatMap((scheme) =>
            levels.map((level) => ({ scheme, level }) as const),
        );
        for (const { scheme, level } of checks) {
            for (const [hue, seed] of this.hues(this.#accent)) {
                // check every role under the accent, and each other hue's swatch
                const shown = ACCENT_SWATCHES.has(hue) ? undefined : seed;
                const resolution = this.resolve(scheme, level, shown);
                const roles = this.#rolesOf(shown);
                const names = hue === "accent" ? ROLE_NAMES : SWATCH_COLORS;
                const nameOf = (name: RoleName) => (isSwatchColor(name) ? `${hue} ${name}` : name);
                for (const name of names) {
                    // skip a role that reads on nothing, such as a fill
                    const reading = roles[name].reading;
                    if (reading === undefined) {
                        continue;
                    }
                    const floor = resolution.floor(reading.contrast);
                    for (const on of reading.on) {
                        // refuse an illegible pair on any ground the role reads on
                        const lightness = resolution.contrast(name, "apca", on);
                        if (lightness < floor) {
                            const contrast = level === 0 ? "standard" : "more";
                            throw PackageError.of(
                                "DEFINITION_INVALID",
                                `theme '${this.name}' is invalid: '${nameOf(name)}' on '${nameOf(on)}' reads at Lc ${lightness.toFixed(1)} in '${scheme}' with '${contrast}' contrast, below ${floor}`,
                            );
                        }

                        // refuse a label below the WCAG ratio audits measure
                        const ratio = resolution.contrast(name, "wcag", on);
                        if (roles[name].source.kind === "label" && ratio < LABEL_RATIO) {
                            throw PackageError.of(
                                "DEFINITION_INVALID",
                                `theme '${this.name}' is invalid: '${nameOf(name)}' on '${nameOf(on)}' reads at ${ratio.toFixed(2)} to 1 in '${scheme}', below ${LABEL_RATIO} to 1`,
                            );
                        }
                    }
                }
            }
        }
    }

    /** Read the color of an accent: the theme's own for a hue, else as given. */
    color(accent: Accent): string {
        const hue = Hue.safeParse(accent);

        return hue.success ? this.seed(hue.data) : accent;
    }

    /** Read the seed of a hue: the theme's, the base for gray, else Destack's. */
    seed(hue: Hue): string {
        const own = this.definition.hues?.[hue];

        return own ?? (hue === "gray" ? (this.definition.base ?? DEFAULT_BASE) : DEFAULT_HUES[hue]);
    }

    /** List the swatches' seeds by token name: the accent, the statuses, the series, the hues. */
    hues(accent: Accent = this.#accent): readonly (readonly [string, string])[] {
        const color = this.color(accent);
        const series = this.#seriesOf(color).map(
            (seed, index) => [`series${String(index + 1)}`, seed] as const,
        );

        return [
            ["accent", color],
            ...STATUS_NAMES.map((name) => [name, this.seed(STATUS_HUES[name])] as const),
            ...series,
            ...HUES.map((hue) => [hue, this.seed(hue)] as const),
        ];
    }

    /** Format a role's color under a hue in both appearances at a contrast. */
    #css(name: RoleName, contrast: Contrast, hue: string | undefined): string {
        // resolve both appearances at the explicit or the standard level
        const pair = (level: number) =>
            [
                this.resolve("light", level, hue).color(name),
                this.resolve("dark", level, hue).color(name),
            ] as const;
        const [light, dark] = pair(contrast === "system" ? 0 : contrast);
        if (contrast !== "system") {
            return lightDark(light, dark);
        }

        // mix toward the most contrasted colors as far as the device contrast the stylesheet sets
        const [lightMore, darkMore] = pair(1);
        if (lightMore === light && darkMore === dark) {
            return lightDark(light, dark);
        }
        const amount = `calc(var(${CONTRAST_VARIABLE}, 0) * 100%)`;
        const mix = (standard: string, more: string) =>
            standard === more ? standard : `color-mix(in oklab, ${standard}, ${more} ${amount})`;

        return lightDark(mix(light, lightMore), mix(dark, darkMore));
    }

    /** List the roles under a hue, replacing the hue's roles only under the theme's own accent. */
    #rolesOf(hue?: string): Roles {
        // replace each role the ground and the definition replace, skipping hue roles under another hue
        const isOwn = hue === undefined;
        const ground = this.definition.ground;
        const overrides = {
            ...(ground === undefined ? {} : groundOf(ground)),
            ...this.definition.roles,
        };
        const roles: Record<RoleName, Role> = { ...COLOR_ROLES };
        for (const name of ROLE_NAMES) {
            const override = overrides[name];
            const { source } = COLOR_ROLES[name];
            const isSwatch = isSwatchColor(name);
            const isHue = isSwatch || ("palette" in source && source.palette === "hue");
            if (override !== undefined && (isOwn || !isHue)) {
                roles[name] = COLOR_ROLES[name].override(override);
            }
        }

        return roles;
    }

    /** Grow the palettes of a hue once: the base's, and the hue's. */
    #palettesOf(hue: string): Palettes {
        // reuse the palettes of a known hue
        const known = this.#palettes.get(hue);
        if (known !== undefined) {
            return known;
        }

        // grow the base and the hue
        const palettes: Palettes = {
            base: Palette.of(this.definition.base ?? DEFAULT_BASE),
            hue: Palette.of(hue),
        };
        this.#palettes.set(hue, palettes);

        return palettes;
    }

    /** Pick the chart series seeds around an accent once, the accent first. */
    #seriesOf(accent: string): readonly string[] {
        // reuse the series of a known accent
        const known = this.#series.get(accent);
        if (known !== undefined) {
            return known;
        }

        // take the accent, then the theme's own hues farthest apart
        const series = [
            accent,
            ...seriesOf(
                accent,
                SERIES_HUES.map((hue) => this.seed(hue)),
            ),
        ];
        this.#series.set(accent, series);

        return series;
    }

    /** Select the length factor of a token family. */
    #factor(family: string, display: Display): number | undefined {
        const scaling = Number.parseInt(this.definition.scaling ?? DEFAULT_SCALING, 10) / 100;
        const density =
            DENSITY_SCALES[display.density ?? this.definition.density ?? DEFAULT_DENSITY];
        switch (family) {
            case "color":
            case "swatch":
                return undefined;
            // grow spacing and control heights with the text, so rows fit larger text
            case "space":
            case "size":
                return scaling * density * TEXT_SCALES[display.textSize];
            case "width":
                return scaling * density;
            case "radius":
                return scaling * RADIUS_SCALES[this.definition.radius ?? DEFAULT_RADIUS];
            case "text":
                return scaling * TEXT_SCALES[display.textSize];
            case "font":
            case "weight":
            case "stroke":
            case "shadow":
            case "motion":
                return 1;
            default:
                throw new RangeError(`theme has no factor for token family '${family}'`);
        }
    }

    /** Set the theme's font stacks and full radius, and the person's motion. */
    #customize(style: ThemeStyle, display: Display): void {
        // replace the token text font with the person's face, else the theme's
        const fonts = this.definition.fonts;
        const chosen = display.typeface === null ? undefined : TYPEFACES[display.typeface];
        const isThemeFace = chosen === undefined || chosen === fonts?.text;
        const face = chosen ?? fonts?.text;
        if (face !== undefined) {
            style[TOKENS.entry(["font", "text"]).variable] = face;
        }

        // size a face other than the theme's to Plex's lowercase, so faces read at one size
        style[FONT_SIZE_ADJUST] = isThemeFace ? "none" : `ex-height ${String(PLEX_EX_HEIGHT)}`;

        // drop the tracking that fits only the token font
        if (face !== undefined) {
            for (const entry of TOKENS.family("text").entries) {
                style[entry.member("letterSpacing")] = "0px";
            }
        }

        // replace the token code font with the theme's
        if (fonts?.code !== undefined) {
            style[TOKENS.entry(["font", "code"]).variable] = fonts.code;
        }

        // change the text styles the theme overrides, sizes scaled by the person's text size
        const scale = present(this.#factor("text", display), "the text factor");
        for (const [name, override] of Object.entries(this.definition.text ?? {})) {
            const entry = TOKENS.entry(["text", name]);
            if (override.size !== undefined) {
                style[entry.member("fontSize")] = sizeOf(override.size, scale);
            }
            if (override.weight !== undefined) {
                const weight = TOKENS.entry(["weight", override.weight]).variable;
                style[entry.member("fontWeight")] = `var(${weight})`;
            }
            if (override.tracking !== undefined && isThemeFace) {
                style[entry.member("letterSpacing")] = `${override.tracking}em`;
            }
            if (override.font !== undefined) {
                style[entry.member("fontFamily")] =
                    `var(${TOKENS.entry(["font", override.font]).variable})`;
                style[entry.member("fontSizeAdjust")] =
                    override.font === "text" ? `var(${FONT_SIZE_ADJUST}, none)` : "none";
            }
        }

        // square the full radius of a theme with square corners
        if (this.definition.radius === "none") {
            style[TOKENS.entry(["radius", "full"]).variable] = "0px";
        }

        // set the motion scale the stylesheet otherwise reads from the device
        if (display.motion !== "system") {
            style[MOTION_SCALE] = display.motion === "full" ? "1" : "0";
        }
    }
}

/** Read a theme's ground as the roles it replaces: the page, text, quiet fills and lines. */
function groundOf(
    ground: NonNullable<ThemeDefinition["ground"]>,
): Partial<Record<RoleName, RoleOverride>> {
    const { light, dark } = ground;

    return {
        background: { light: light.background, dark: dark.background },
        foreground: { light: light.foreground, dark: dark.foreground },
        muted: { light: light.muted, dark: dark.muted },
        border: { from: "foreground", lightness: { light: 0, dark: 0 }, alpha: LINE_ALPHA },
    };
}

/** Capitalize a word's first letter, as token keys join a hue and a color: `cyanSolid`. */
function capitalize(word: string): string {
    return `${word.charAt(0).toUpperCase()}${word.slice(1)}`;
}

/** Write a text size scaled by the person's text size, a fluid size as a clamp. */
function sizeOf(size: FontSize, scale: number): string {
    // write a fixed size in pixels
    if (typeof size === "number") {
        return `${size * scale}px`;
    }

    // grow linearly from the narrowest window to the widest, as Utopia's fluid type scales do
    const slope = (size.max - size.min) / (FLUID_WINDOWS.max - FLUID_WINDOWS.min);
    const intercept = size.min - slope * FLUID_WINDOWS.min;
    const preferred = `calc(${(intercept * scale).toFixed(3)}px + ${(slope * scale * 100).toFixed(4)}vw)`;

    return `clamp(${size.min * scale}px, ${preferred}, ${size.max * scale}px)`;
}

/** Format a color in both appearances, one color when they agree. */
function lightDark(light: string, dark: string): string {
    return light === dark ? light : `light-dark(${light}, ${dark})`;
}

/** Read the color a seed grows from. */
function colorOf(seed: string): Oklch {
    return Oklch.parse(Palette.of(seed).seed);
}

/** Pick the chart series after the accent from the theme's hues. */
function seriesOf(accent: string, hues: readonly string[]): string[] {
    // start from the accent
    const candidates = hues.map((seed) => ({ seed, color: colorOf(seed) }));
    const taken = [colorOf(accent)];

    return Array.from({ length: SERIES_COUNT - 1 }, () => {
        // take the hue farthest from every color taken, the earlier on a tie
        const distances = candidates.map(({ color }) =>
            Math.min(...taken.map((other) => color.distance(other))),
        );
        const farthest = distances.indexOf(Math.max(...distances));
        const series = present(candidates.splice(farthest, 1)[0], "a hue for a chart series");
        taken.push(series.color);

        return series.seed;
    });
}
