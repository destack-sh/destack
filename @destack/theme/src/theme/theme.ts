import { PackageError, type Package } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";
import { apca, HexColor, Scale, type Scheme } from "../palette/index.ts";
import { GRAY_PRESETS, Preset, type GrayPreset } from "../radix/index.ts";
import { MOTION_VARIABLE, TOKENS, type Variable } from "../token/index.ts";
import {
    DENSITY_SCALES,
    Density,
    TEXT_SCALES,
    type Appearance,
    type Preferences,
} from "./preference.ts";
import {
    COLOR_ROLES,
    CONTENT_CONTRAST,
    ROLE_NAMES,
    RoleName,
    RoleOverride,
    SURFACE_ROLES,
    SurfaceLevel,
    type Role,
    type RoleContrast,
    type Roles,
    type RoleScales,
} from "./role.ts";

/** The radius multiplier of each corner treatment, after Radix Themes. */
const RADIUS_SCALES = { none: 0, small: 0.75, medium: 1, large: 1.5, full: 1.5 } as const;

/** The accent of a theme that names none, after Radix Themes. */
const DEFAULT_ACCENT = "indigo";

/** The gray of a theme that names none, after Radix Themes. */
const DEFAULT_GRAY = "slate";

/** The corner treatment of a theme that names none, after Radix Themes. */
const DEFAULT_RADIUS = "medium";

/** The interface scale of a theme that names none. */
const DEFAULT_SCALING = "100%";

/** The density of a theme that names none. */
const DEFAULT_DENSITY = "regular";

/** The schemes and contrasts each theme is checked under. */
const CHECKS = [
    ["light", "standard"],
    ["light", "more"],
    ["dark", "standard"],
    ["dark", "more"],
] as const satisfies readonly (readonly [Scheme, RoleContrast])[];

/** A package-local theme name. */
export const ThemeName = defineSchema(schema.string().regex(/^[a-z][a-zA-Z0-9]*$(?![\s\S])/u));

/** A theme's accent: a preset name or a six-digit sRGB hex seed. */
export const Accent = defineSchema(schema.union([Preset, HexColor]));
/** A theme's accent. */
export type Accent = schema.Infer<typeof Accent>;

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
        /** The accent of primary actions and selection, indigo when absent. */
        accent: Accent.exactOptional(),
        /** The neutral scale of backgrounds, borders and text, slate when absent. */
        gray: schema.enum(GRAY_PRESETS).exactOptional(),
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
        /** The roles the theme replaces with a scale step or explicit colors, checked like every role. */
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

/** A theme: scales, corners, scaling, fonts and density, resolved into token values per person. */
export class Theme {
    /** The declaring package. */
    readonly package: Package;
    /** The name, accent, gray, radius, scaling, fonts, density and role replacements. */
    readonly definition: ThemeDefinition;
    /** The scales of the theme's own accent. */
    readonly scales: RoleScales;
    /** The color roles with the theme's replacements. */
    readonly roles: Roles;

    /** Hold a theme, generate its scales and apply its role replacements. */
    constructor(owner: Package, definition: ThemeDefinition) {
        // hold the definition and the scales of its own accent
        this.package = owner;
        this.definition = definition;
        this.scales = roleScales(
            definition.gray ?? DEFAULT_GRAY,
            definition.accent ?? DEFAULT_ACCENT,
        );

        // replace the roles the theme overrides
        const roles: Record<RoleName, Role> = { ...COLOR_ROLES };
        for (const name of ROLE_NAMES) {
            const override = definition.roles?.[name];
            if (override !== undefined) {
                roles[name] = COLOR_ROLES[name].override(override);
            }
        }
        this.roles = roles;
    }

    /** The package-local name. */
    get name(): string {
        return this.definition.name;
    }

    /** Compute the custom properties of a theme root for an appearance and a person's preferences. */
    variables(appearance: Appearance, preferences: Preferences): ThemeStyle {
        // let the appearance drive light-dark() and native controls
        const style: ThemeStyle = {
            "color-scheme": appearance === "system" ? "light dark" : appearance,
        };

        // resolve the roles with the person's accent over the theme's own
        const scales =
            preferences.accent === null
                ? this.scales
                : roleScales(this.definition.gray ?? DEFAULT_GRAY, preferences.accent);
        for (const entry of TOKENS.family("color").entries) {
            const role = this.roles[RoleName.parse(entry.key)];
            style[entry.variable] = role.css(this.roles, scales, preferences.contrast);
        }
        for (const entry of TOKENS.family("surface").entries) {
            const role = this.roles[SURFACE_ROLES[SurfaceLevel.parse(entry.key)]];
            style[entry.variable] = role.css(this.roles, scales, preferences.contrast);
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

    /** Refuse a theme whose text roles read below their APCA contrast in either appearance and contrast. */
    requireContrast(): void {
        for (const [scheme, contrast] of CHECKS) {
            for (const name of ROLE_NAMES) {
                // measure the text role on its background
                const role = this.roles[name];
                const text = role.text;
                if (text === undefined) {
                    continue;
                }
                const foreground = role.color(this.roles, this.scales, scheme, contrast);
                const background = this.roles[text.on].color(
                    this.roles,
                    this.scales,
                    scheme,
                    contrast,
                );
                const lightness = Math.abs(apca(foreground, background));

                // refuse an illegible pair
                if (lightness < text.contrast) {
                    throw new PackageError(
                        "INVALID_DEFINITION",
                        `theme ${this.name}: ${name} on ${text.on} reads at Lc ${lightness.toFixed(1)} in ${scheme} with ${contrast} contrast, below ${text.contrast}`,
                    );
                }
            }
        }
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

/** Build the scales roles read from: the gray, the accent preset or seed, and the status presets. */
function roleScales(gray: GrayPreset, accent: Accent): RoleScales {
    // shift each solid step until its label reads, against the gray's darkest step as a candidate
    const neutral = Scale.preset(gray);
    const text = neutral.color(12, "light");
    const preset = Preset.safeParse(accent);
    const solid = (scale: Scale) => scale.legible(text, CONTENT_CONTRAST);

    return {
        gray: neutral,
        accent: solid(preset.success ? Scale.preset(preset.data) : Scale.generate(accent)),
        red: solid(Scale.preset("red")),
        green: solid(Scale.preset("green")),
        amber: solid(Scale.preset("amber")),
        blue: solid(Scale.preset("blue")),
    };
}
