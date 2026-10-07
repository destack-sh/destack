import { PackageError, type Package } from "@destack/package";
import { defineSchema, present, schema } from "@destack/schema";
import { apca, Palette, Seed, type Scheme } from "../palette/index.ts";
import { GRAY_PRESETS, PRESET_NAMES, type AccentPreset } from "../preset/index.ts";
import { MOTION_VARIABLE, TOKENS, VARIABLE_PREFIX, type Variable } from "../token/index.ts";
import {
    DENSITY_SCALES,
    Density,
    TEXT_SCALES,
    type Appearance,
    type Contrast,
    type Preferences,
} from "./preference.ts";
import {
    BACKGROUND_LIGHTNESS,
    COLOR_ROLES,
    GRAPHIC_CONTRAST,
    Resolution,
    ROLE_NAMES,
    RoleName,
    RoleOverride,
    SURFACE_ROLES,
    SurfaceLevel,
    type Palettes,
    type Role,
    type Roles,
} from "./role.ts";

/** The radius multiplier of each corner treatment. */
const RADIUS_SCALES = { none: 0, small: 0.75, medium: 1, large: 1.5, full: 1.5 } as const;

/** The base of a theme that names none. */
const DEFAULT_BASE = "slate";

/** The accent of a theme that names none. */
const DEFAULT_ACCENT = "indigo";

/** The status palettes of a theme that names none. */
const DEFAULT_STATUS = {
    destructive: "red",
    success: "green",
    warning: "amber",
    info: "blue",
} as const;

/** The corner treatment of a theme that names none. */
const DEFAULT_RADIUS = "medium";

/** The interface scale of a theme that names none. */
const DEFAULT_SCALING = "100%";

/** The density of a theme that names none. */
const DEFAULT_DENSITY = "regular";

/** The chart series a theme holds after its accent. */
const SERIES = ["chart2", "chart3", "chart4", "chart5"] as const;

/** The colorful presets chart series pick from, in preset order. */
const SERIES_CANDIDATES = PRESET_NAMES.filter(
    (name): name is AccentPreset => !GRAY_PRESETS.some((gray) => gray === name),
);

/** The appearances and contrast levels each theme is checked under. */
const CHECKS = [
    ["light", 0],
    ["light", 1],
    ["dark", 0],
    ["dark", 1],
] as const satisfies readonly (readonly [Scheme, number])[];

/** The custom property the stylesheet sets to 1 when the device asks for more contrast. */
const CONTRAST_VARIABLE = `${VARIABLE_PREFIX}-contrast`;

/** A package-local theme name. */
export const ThemeName = defineSchema(schema.string().regex(/^[a-z][a-zA-Z0-9]*$(?![\s\S])/u));

/** A corner treatment. */
export const Radius = defineSchema(schema.enum(["none", "small", "medium", "large", "full"]));
/** A corner treatment. */
export type Radius = schema.Infer<typeof Radius>;

/** A proportional interface scale. */
export const Scaling = defineSchema(schema.enum(["90%", "95%", "100%", "105%", "110%"]));
/** A proportional interface scale. */
export type Scaling = schema.Infer<typeof Scaling>;

/** A theme as authored, every field beside the name optional. */
export const ThemeDefinition = defineSchema(
    schema.object({
        /** The package-local name. */
        name: ThemeName,
        /** The seed of surfaces, text and lines, slate when absent. */
        base: Seed.exactOptional(),
        /** The seed of primary actions, selection, focus and the first chart series, indigo when absent. */
        accent: Seed.exactOptional(),
        /** The seeds of the status roles, red, green, amber and blue when absent. */
        status: schema
            .object({
                /** The seed of destructive actions. */
                destructive: Seed.exactOptional(),
                /** The seed of success states. */
                success: Seed.exactOptional(),
                /** The seed of warning states. */
                warning: Seed.exactOptional(),
                /** The seed of informational states. */
                info: Seed.exactOptional(),
            })
            .exactOptional(),
        /** The seeds of the chart series after the accent, the rest picked from the presets by hue. */
        chart: schema.array(Seed).max(SERIES.length).exactOptional(),
        /** The corner treatment, medium when absent. */
        radius: Radius.exactOptional(),
        /** The interface scale, 100% when absent. */
        scaling: Scaling.exactOptional(),
        /** The font stacks of interface and code text as CSS `font-family` values, the tokens' when absent. */
        fonts: schema
            .object({
                /** The interface font stack. */
                text: schema.string().min(1).exactOptional(),
                /** The code font stack. */
                code: schema.string().min(1).exactOptional(),
            })
            .exactOptional(),
        /** The density a person's density setting overrides, regular when absent. */
        density: Density.exactOptional(),
        /** The roles the theme replaces with a palette's seed or tone or explicit colors, checked like every role. */
        roles: schema.partialRecord(RoleName, RoleOverride).exactOptional(),
    }),
);
/** A theme as authored. */
export type ThemeDefinition = schema.Infer<typeof ThemeDefinition>;

/** The declarations a theme root's style attribute carries. */
export type ThemeStyle = {
    /** Browser appearance used by light-dark() and native controls. */
    "color-scheme": "light dark" | "light" | "dark";
    /** The token values. */
    [variable: Variable]: string;
};

/** A theme: palettes, roles, corners, scaling, fonts and density, resolved into token values per person. */
export class Theme {
    /** The declaring package. */
    readonly package: Package;
    /** The name, seeds, corners, scaling, fonts, density and role replacements. */
    readonly definition: ThemeDefinition;
    /** The palettes of the theme's own accent. */
    readonly palettes: Palettes;
    /** The color roles with the theme's replacements. */
    readonly roles: Roles;
    /** The palettes of each accent a person chose, by seed. */
    readonly #accents = new Map<Seed, Palettes>();
    /** The resolved roles of each accent, appearance and contrast level. */
    readonly #resolutions = new Map<string, Resolution>();

    /** Hold a theme, grow its palettes and apply its role replacements. */
    constructor(owner: Package, definition: ThemeDefinition) {
        // hold the definition and replace the roles it overrides
        this.package = owner;
        this.definition = definition;
        const roles: Record<RoleName, Role> = { ...COLOR_ROLES };
        for (const name of ROLE_NAMES) {
            const override = definition.roles?.[name];
            if (override !== undefined) {
                roles[name] = COLOR_ROLES[name].override(override);
            }
        }
        this.roles = roles;

        // grow the palettes of the theme's own accent
        this.palettes = this.#palettesOf(definition.accent ?? DEFAULT_ACCENT);
    }

    /** The package-local name. */
    get name(): string {
        return this.definition.name;
    }

    /** Derive the theme over another background, such as a selected row's, every role reading against it. */
    rebase(background: RoleOverride): Theme {
        const roles = { ...this.definition.roles, background };

        return new Theme(this.package, { ...this.definition, roles });
    }

    /** Resolve the roles of an appearance at a contrast level from 0, the standard, to 1, the most, under the theme's own accent or a person's. */
    resolve(scheme: Scheme, level: number, accent?: Seed): Resolution {
        // reuse the resolution of this accent, appearance and level
        const seed = accent ?? this.definition.accent ?? DEFAULT_ACCENT;
        const key = `${seed}:${scheme}:${level}`;
        const known = this.#resolutions.get(key);
        if (known !== undefined) {
            return known;
        }
        const resolution = new Resolution(this.roles, this.#palettesOf(seed), scheme, level);
        this.#resolutions.set(key, resolution);

        return resolution;
    }

    /** Compute the custom properties of a theme root for an appearance and a person's preferences. */
    variables(appearance: Appearance, preferences: Preferences): ThemeStyle {
        // let the appearance drive light-dark() and native controls
        const style: ThemeStyle = {
            "color-scheme": appearance === "system" ? "light dark" : appearance,
        };

        // resolve the roles with the person's accent over the theme's own at their contrast
        const css = (name: RoleName) =>
            this.#css(name, preferences.contrast, preferences.accent ?? undefined);
        for (const entry of TOKENS.family("color").entries) {
            style[entry.variable] = css(RoleName.parse(entry.key));
        }
        for (const entry of TOKENS.family("surface").entries) {
            style[entry.variable] = css(SURFACE_ROLES[SurfaceLevel.parse(entry.key)]);
        }

        // scale every other token by the theme and the person's preferences
        for (const family of TOKENS.families) {
            const factor = this.#factor(family.name, preferences);
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
        this.#customize(style, preferences);

        return style;
    }

    /** Refuse a theme whose roles read below their APCA contrast in either appearance at the standard or the most contrast. */
    requireContrast(): void {
        for (const [scheme, level] of CHECKS) {
            const resolution = this.resolve(scheme, level);
            for (const name of ROLE_NAMES) {
                // measure the role on the role it reads on
                const reading = this.roles[name].reading;
                if (reading === undefined) {
                    continue;
                }
                const lightness = resolution.reads(name);
                const floor = resolution.floor(reading.contrast);

                // refuse an illegible pair
                if (lightness < floor) {
                    const contrast = level === 0 ? "standard" : "more";
                    throw new PackageError(
                        "INVALID_DEFINITION",
                        `theme ${this.name}: ${name} on ${reading.on} reads at Lc ${lightness.toFixed(1)} in ${scheme} with ${contrast} contrast, below ${floor}`,
                    );
                }
            }
        }
    }

    /** Format a role's color in both appearances at a contrast, mixing toward the most contrast as far as the device asks for `system`. */
    #css(name: RoleName, contrast: Contrast, accent: Seed | undefined): string {
        // resolve both appearances at the explicit or the standard level
        const pair = (level: number) =>
            [
                this.resolve("light", level, accent).color(name),
                this.resolve("dark", level, accent).color(name),
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

    /** Grow the palettes of an accent once: the base, the accent, the statuses and the chart series. */
    #palettesOf(accent: Seed): Palettes {
        // reuse the palettes of a known accent
        const known = this.#accents.get(accent);
        if (known !== undefined) {
            return known;
        }

        // grow the declared seeds and pick the chart series around the accent
        const definition = this.definition;
        const status = { ...DEFAULT_STATUS, ...definition.status };
        const base = Palette.of(definition.base ?? DEFAULT_BASE);
        const accentPalette = Palette.of(accent);
        const [chart2, chart3, chart4, chart5] = seriesOf(
            accentPalette,
            (definition.chart ?? []).map((seed) => Palette.of(seed)),
            base.tone(BACKGROUND_LIGHTNESS.light),
        );
        const palettes: Palettes = {
            base,
            accent: accentPalette,
            destructive: Palette.of(status.destructive),
            success: Palette.of(status.success),
            warning: Palette.of(status.warning),
            info: Palette.of(status.info),
            chart2: present(chart2, "a second chart series"),
            chart3: present(chart3, "a third chart series"),
            chart4: present(chart4, "a fourth chart series"),
            chart5: present(chart5, "a fifth chart series"),
        };
        this.#accents.set(accent, palettes);

        return palettes;
    }

    /** Select the length factor of a token family, or none for families the theme resolves itself. */
    #factor(family: string, preferences: Preferences): number | undefined {
        const scaling = Number.parseInt(this.definition.scaling ?? DEFAULT_SCALING, 10) / 100;
        const density =
            DENSITY_SCALES[preferences.density ?? this.definition.density ?? DEFAULT_DENSITY];
        switch (family) {
            case "color":
            case "surface":
                return undefined;
            case "space":
            case "size":
            case "width":
                return scaling * density;
            case "radius":
                return scaling * RADIUS_SCALES[this.definition.radius ?? DEFAULT_RADIUS];
            case "text":
                return scaling * TEXT_SCALES[preferences.textSize];
            case "weight":
            case "stroke":
            case "shadow":
            case "motion":
                return 1;
            default:
                throw new RangeError(`theme has no factor for token family ${family}`);
        }
    }

    /** Set the theme's font stacks and full radius, and the person's motion. */
    #customize(style: ThemeStyle, preferences: Preferences): void {
        // replace the token font stacks with the theme's
        const fonts = this.definition.fonts;
        if (fonts?.text !== undefined) {
            style[TOKENS.entry(["text", "family"]).variable] = fonts.text;
        }
        if (fonts?.code !== undefined) {
            style[TOKENS.entry(["text", "codeFamily"]).variable] = fonts.code;
        }

        // square the full radius unless the theme rounds fully
        if (this.definition.radius !== "full") {
            style[TOKENS.entry(["radius", "full"]).variable] = "0px";
        }

        // pin the motion scale the stylesheet otherwise reads from the device
        if (preferences.motion !== "system") {
            style[MOTION_VARIABLE] = preferences.motion === "full" ? "1" : "0";
        }
    }
}

/** Format a color in both appearances, one color when they agree. */
function lightDark(light: string, dark: string): string {
    return light === dark ? light : `light-dark(${light}, ${dark})`;
}

/** Pick the chart series after the accent: the declared ones, then candidates reading on the light background, each the hue farthest from those taken. */
function seriesOf(accent: Palette, declared: readonly Palette[], background: string): Palette[] {
    // keep the candidates whose seeds read as graphics on the light background, where pale hues would darken into muddy tones
    const isVisible = (palette: Palette) =>
        Math.abs(apca(palette.seed, background)) >= GRAPHIC_CONTRAST.standard;
    const candidates = SERIES_CANDIDATES.map((name) => Palette.of(name)).filter((palette) =>
        isVisible(palette),
    );

    // take the candidate farthest from every hue taken until the series are full, the earlier on a tie
    const hues = [accent, ...declared].map((palette) => palette.hue);
    const picked = [...declared];
    while (picked.length < SERIES.length) {
        const distanceOf = (palette: Palette) =>
            Math.min(...hues.map((taken) => hueDistance(taken, palette.hue)));
        const farthest = present(
            candidates.reduce<Palette | undefined>(
                (best, candidate) =>
                    best === undefined || distanceOf(candidate) > distanceOf(best)
                        ? candidate
                        : best,
                undefined,
            ),
            "a visible chart series candidate",
        );
        picked.push(farthest);
        hues.push(farthest.hue);
        candidates.splice(candidates.indexOf(farthest), 1);
    }

    return picked;
}

/** Measure the angle between two hues around the color wheel, in degrees. */
function hueDistance(left: number, right: number): number {
    const difference = Math.abs(left - right) % 360;

    return Math.min(difference, 360 - difference);
}
