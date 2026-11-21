import { schema } from "@destack/schema";
import {
    Oklch,
    type ContrastMeasure,
    HexColor,
    type Palette,
    type ColorScheme,
    Srgb,
} from "../palette/index.ts";

/** An APCA lightness contrast Lc a role needs. */
export interface Floor {
    /** The magnitude at the standard contrast. */
    readonly standard: number;
    /** The magnitude at the most contrast. */
    readonly more: number;
}

/** The APCA-W3 bronze level for body text, Lc 75, and its fluent level, Lc 90. */
export const BODY_CONTRAST: Floor = { standard: 75, more: 90 };

/** The APCA-W3 bronze level for content text other than body text, Lc 60. */
export const CONTENT_CONTRAST: Floor = { standard: 60, more: 90 };

/** The contrast of labels on solid fills, Lc 60, and Lc 75 at the most. */
export const LABEL_CONTRAST: Floor = { standard: 60, more: 75 };

/** The WCAG 2.2 AA contrast ratio of normal text, the least a label on a solid fill takes. */
export const LABEL_RATIO = 4.5;

/** The contrast of solid graphics such as chart series and focus rings. */
export const GRAPHIC_CONTRAST: Floor = { standard: 45, more: 60 };

/** The contrast of borders and dividers, free at the standard contrast and Lc 30 at the most. */
export const LINE_CONTRAST: Floor = { standard: 0, more: 30 };

/** The background's OKLCH lightness in each appearance. */
export const BACKGROUND_LIGHTNESS: Lightness = { light: 0.993, dark: 0.187 };

/** The OKLCH lightness body text starts its search from in each appearance. */
const TEXT_LIGHTNESS: Lightness = { light: 0.242, dark: 0.948 };

/** The OKLCH lightness secondary text starts its search from in each appearance. */
const MUTED_LIGHTNESS: Lightness = { light: 0.502, dark: 0.768 };

/** The OKLCH lightness of subtle fills relative to the background in each appearance. */
const SUBTLE_OFFSET: Lightness = { light: -0.034, dark: 0.077 };

/** The light label candidate on solid colors. */
const WHITE = Srgb.parse("#ffffff");

/** The halvings of the opacity search, enough for a step below one 8-bit alpha level. */
const OPACITY_STEPS = 9;

/** The opacity of the scrim behind modal surfaces: half strength. */
const SCRIM_ALPHA = 0.5;

/** The color role names, with status roles. */
export const ROLE_NAMES = [
    "background",
    "foreground",
    "card",
    "cardForeground",
    "popover",
    "popoverForeground",
    "primary",
    "primaryForeground",
    "primaryText",
    "secondary",
    "secondaryForeground",
    "muted",
    "mutedForeground",
    "accent",
    "accentForeground",
    "destructive",
    "destructiveForeground",
    "destructiveText",
    "success",
    "successForeground",
    "successText",
    "warning",
    "warningForeground",
    "warningText",
    "info",
    "infoForeground",
    "infoText",
    "border",
    "input",
    "ring",
    "scrim",
    "chart1",
    "chart2",
    "chart3",
    "chart4",
    "chart5",
] as const;

/** The palette names of a theme. */
export const PALETTE_NAMES = [
    "base",
    "accent",
    "destructive",
    "success",
    "warning",
    "info",
    "chart2",
    "chart3",
    "chart4",
    "chart5",
] as const;

/** A palette name of a theme. */
export const PaletteName = schema.enum(PALETTE_NAMES);
/** A palette name of a theme. */
export type PaletteName = schema.Infer<typeof PaletteName>;

/** The palettes of a theme by name. */
export type Palettes = Readonly<Record<PaletteName, Palette>>;

/** An OKLCH lightness in each appearance. */
export interface Lightness {
    /** The light appearance's lightness. */
    readonly light: number;
    /** The dark appearance's lightness. */
    readonly dark: number;
}

/** The role a role reads on and the contrast it needs there. */
export interface Reading {
    /** The role it reads on. */
    readonly on: RoleName;
    /** The contrast it needs. */
    readonly contrast: Floor;
}

/** Where a role's color comes from. */
export type RoleSource =
    | {
          /** Another role's color at a lightness offset, keeping its chroma and hue. */
          readonly kind: "shift";
          /** The role. */
          readonly from: RoleName;
          /** The lightness offset. */
          readonly lightness: Lightness;
          /** The opacity from 0 to 1, opaque when absent. */
          readonly alpha?: number;
      }
    | {
          /** A palette's tone, moved away from the role it reads on until it reads. */
          readonly kind: "tone";
          /** The palette. */
          readonly palette: PaletteName;
          /** The lightness, or its offset from another role's. */
          readonly lightness: Lightness;
          /** The role whose lightness the lightness offsets, absolute when absent. */
          readonly from?: RoleName;
          /** The opacity from 0 to 1, opaque when absent. */
          readonly alpha?: number;
      }
    | {
          /** A palette's seed, shifted by the least lightness that lets it and its label read. */
          readonly kind: "solid";
          /** The palette. */
          readonly palette: PaletteName;
      }
    | {
          /** White, or the darkest text of the base or of the palette under it. */
          readonly kind: "label";
      }
    | {
          /** Explicit colors. */
          readonly kind: "color";
          /** The light appearance's six-digit sRGB hex color. */
          readonly light: string;
          /** The dark appearance's six-digit sRGB hex color. */
          readonly dark: string;
          /** The opacity from 0 to 1, opaque when absent. */
          readonly alpha?: number;
      };

/** A color role name. */
export const RoleName = schema.enum(ROLE_NAMES);
/** A color role name. */
export type RoleName = schema.Infer<typeof RoleName>;

/** An OKLCH lightness or lightness offset in each appearance. */
const LightnessPair = schema.object({
    /** The light appearance's value. */
    light: schema.number().min(-1).max(1),
    /** The dark appearance's value. */
    dark: schema.number().min(-1).max(1),
});

/** An opacity from 0 to 1. */
const Alpha = schema.number().min(0).max(1);

/** A theme's replacement for one role, in the sources the built-in roles use. */
export const RoleOverride = schema.union([
    schema.object({
        /** The palette. */
        palette: PaletteName,
        /** The tone's OKLCH lightness, or its offset from the role it follows. */
        lightness: LightnessPair,
        /** The role whose lightness the lightness offsets, absolute when absent. */
        from: RoleName.exactOptional(),
        /** The opacity, opaque when absent. */
        alpha: Alpha.exactOptional(),
    }),
    schema.object({
        /** The palette whose seed the role takes. */
        palette: PaletteName,
    }),
    schema.object({
        /** The role whose color this one takes, keeping its chroma and hue. */
        from: RoleName,
        /** The lightness offset from that role. */
        lightness: LightnessPair,
        /** The opacity, opaque when absent. */
        alpha: Alpha.exactOptional(),
    }),
    schema.object({
        /** The light appearance's color. */
        light: HexColor,
        /** The dark appearance's color. */
        dark: HexColor,
        /** The opacity, opaque when absent. */
        alpha: Alpha.exactOptional(),
    }),
]);
/** A theme's replacement for one role. */
export type RoleOverride = schema.Infer<typeof RoleOverride>;

/** A color role: where its color comes from, what it reads on and what its label needs. */
export class Role {
    /** Where the color comes from. */
    readonly source: RoleSource;
    /** The role this one reads on and the contrast it needs there. */
    readonly reading: Reading | undefined;
    /** The contrast the best label on this role needs, for solid roles carrying text. */
    readonly labelled: Floor | undefined;

    /** Create a role. */
    constructor(source: RoleSource, reading?: Reading, labelled?: Floor) {
        this.source = source;
        this.reading = reading;
        this.labelled = labelled;
    }

    /** Replace the color source, keeping what the role reads on and what its label needs. */
    override(override: RoleOverride): Role {
        return new Role(sourceOf(override), this.reading, this.labelled);
    }
}

/** Read an override as the role source it names. */
function sourceOf(override: RoleOverride): RoleSource {
    // a palette's tone, at its opacity
    if ("palette" in override && "lightness" in override) {
        return { kind: "tone", ...override };
    }
    // a palette's seed
    else if ("palette" in override) {
        return { kind: "solid", palette: override.palette };
    }
    // another role moved in lightness, at its opacity
    else if ("from" in override) {
        return { kind: "shift", ...override };
    }

    // explicit colors, at their opacity
    return { kind: "color", ...override };
}

/** The color roles of a theme by name. */
export type Roles = Readonly<Record<RoleName, Role>>;

/** The colors of a theme's roles in one appearance at one contrast level, each resolved once. */
export class Resolution {
    /** The roles. */
    readonly #roles: Roles;
    /** The palettes the roles read. */
    readonly #palettes: Palettes;
    /** The appearance. */
    readonly scheme: ColorScheme;
    /** The contrast level from 0, the standard floors, to 1, the most. */
    readonly level: number;
    /** The colors resolved so far. */
    readonly #colors = new Map<RoleName, string>();
    /** The roles being resolved, which a role must not read through again. */
    readonly #resolving = new Set<RoleName>();

    /** Create the resolution of one appearance and level. */
    constructor(roles: Roles, palettes: Palettes, scheme: ColorScheme, level: number) {
        // set what every color resolves from
        this.#roles = roles;
        this.#palettes = palettes;
        this.scheme = scheme;
        this.level = level;
    }

    /** Resolve a role's color. */
    color(name: RoleName): string {
        // reuse a resolved color and refuse a role reading through itself
        const known = this.#colors.get(name);
        if (known !== undefined) {
            return known;
        }
        if (this.#resolving.has(name)) {
            throw new RangeError(`role '${name}' refers to itself`);
        }

        // resolve the role once
        this.#resolving.add(name);
        const color = this.#resolve(this.#roles[name]);
        this.#resolving.delete(name);
        this.#colors.set(name, color);

        return color;
    }

    /** Measure a role's contrast on the role it reads on, APCA's as a magnitude. */
    contrast(name: RoleName, measure: ContrastMeasure): number {
        // require the role it reads on
        const reading = this.#roles[name].reading;
        if (reading === undefined) {
            throw new RangeError(`role '${name}' has no background role`);
        }

        // measure a translucent role blended over the role it reads on
        const ground = this.color(reading.on);
        const blended = Srgb.parse(Srgb.composite(this.color(name), ground));

        return Math.abs(blended.contrast(Srgb.parse(ground), measure));
    }

    /** Read a floor at the level. */
    floor(contrast: Floor): number {
        return contrast.standard + this.level * (contrast.more - contrast.standard);
    }

    /** Pick the label that reads best on a background. */
    label(palette: Palette, background: Srgb): Srgb {
        // measure each candidate on the background and take the strongest
        const dark = TEXT_LIGHTNESS.light;
        const candidates = [WHITE, palette.tone(dark), this.#palettes.base.tone(dark)];
        const reading = (candidate: Srgb) => Math.abs(candidate.contrast(background, "apca"));

        // prefer the candidates at or above the label ratio, where any are
        const legible = candidates.filter(
            (candidate) => candidate.contrast(background, "wcag") >= LABEL_RATIO,
        );

        return (legible.length > 0 ? legible : candidates).reduce((best, candidate) =>
            reading(candidate) > reading(best) ? candidate : best,
        );
    }

    /** Resolve a role from its source. */
    #resolve(role: Role): string {
        const source = role.source;
        switch (source.kind) {
            case "shift": {
                // move the role's color in lightness alone
                const color = Oklch.parse(this.color(source.from));
                const lightness = color.lightness + source.lightness[this.scheme];
                const shifted = new Oklch(lightness, color.chroma, color.hue).hex();

                return this.#translucent(shifted, source.alpha, role);
            }
            case "tone": {
                // start from the lightness and move away from the background it reads on
                const palette = this.#palettes[source.palette];
                const offset = source.lightness[this.scheme];
                const start =
                    source.from === undefined
                        ? offset
                        : Oklch.parse(this.color(source.from)).lightness + offset;

                // settle an opaque tone in lightness, a translucent one in opacity
                return source.alpha === undefined
                    ? palette.tone(this.#settle(palette, start, role)).hex
                    : this.#translucent(palette.tone(start).hex, source.alpha, role);
            }
            case "solid": {
                const palette = this.#palettes[source.palette];

                return palette.tone(this.#settle(palette, palette.lightness, role)).hex;
            }
            case "label": {
                // pick among the candidates of the palette the background grows from
                const reading = role.reading;
                if (reading === undefined) {
                    throw new RangeError("a label role needs the role it reads on");
                }
                const under = this.#roles[reading.on].source;
                const palette = this.#palettes["palette" in under ? under.palette : "base"];

                return this.label(palette, Srgb.parse(this.color(reading.on))).hex;
            }
            case "color":
                return this.#translucent(source[this.scheme], source.alpha, role);
        }
    }

    /** Apply an opacity to a color. */
    #translucent(color: string, alpha: number | undefined, role: Role): string {
        // return an opaque color, and a translucent one reading on nothing, as they are
        const reading = role.reading;
        if (alpha === undefined) {
            return color;
        } else if (reading === undefined) {
            return Srgb.translucent(color, alpha);
        }

        // find the least opacity at or above the declared one whose blend reads at the floor
        const ground = this.color(reading.on);
        const under = Srgb.parse(ground);
        const floor = this.floor(reading.contrast);
        const reads = (opacity: number) => {
            const blended = Srgb.parse(Srgb.composite(Srgb.translucent(color, opacity), ground));

            return Math.abs(blended.contrast(under, "apca")) >= floor;
        };
        let low = alpha;
        let high = 1;
        if (reads(low) || !reads(high)) {
            return Srgb.translucent(color, reads(low) ? low : high);
        }
        for (let step = 0; step < OPACITY_STEPS; step += 1) {
            const middle = (low + high) / 2;
            [low, high] = reads(middle) ? [low, middle] : [middle, high];
        }

        return Srgb.translucent(color, high);
    }

    /** Find the lightness nearest a start at which a role and its label read. */
    #settle(palette: Palette, start: number, role: Role): number {
        // require the role's reading and its label's at the level
        const { reading, labelled } = role;
        const background = reading === undefined ? undefined : Srgb.parse(this.color(reading.on));
        const passes = (color: Srgb) =>
            (reading === undefined ||
                background === undefined ||
                Math.abs(color.contrast(background, "apca")) >= this.floor(reading.contrast)) &&
            (labelled === undefined ||
                (Math.abs(this.label(palette, color).contrast(color, "apca")) >=
                    this.floor(labelled) &&
                    this.label(palette, color).contrast(color, "wcag") >= LABEL_RATIO));

        // move a solid toward its label's side, so a fill under white text deepens
        if (background === undefined || role.source.kind === "solid") {
            const seed = palette.tone(start);
            const ink = palette.tone(TEXT_LIGHTNESS.light);
            const isUnderWhite =
                Math.abs(WHITE.contrast(seed, "apca")) >= Math.abs(ink.contrast(seed, "apca"));
            const [first, second] = isUnderWhite
                ? (["darker", "lighter"] as const)
                : (["lighter", "darker"] as const);

            return (
                palette.nearest(start, passes, first) ??
                palette.nearest(start, passes, second) ??
                start
            );
        }
        const isLighter = start >= Oklch.parse(background.hex).lightness;

        return (
            palette.nearest(start, passes, isLighter ? "lighter" : "darker") ?? (isLighter ? 1 : 0)
        );
    }
}

/** The color roles every theme starts from. */
export const COLOR_ROLES: Roles = {
    // surfaces and their text
    background: tone("base", BACKGROUND_LIGHTNESS),
    foreground: readable("base", TEXT_LIGHTNESS, "background", BODY_CONTRAST),
    card: shift("background", { light: -0.011, dark: 0.026 }),
    cardForeground: readable("base", TEXT_LIGHTNESS, "card", BODY_CONTRAST),
    popover: shift("background", { light: 0, dark: 0.073 }),
    popoverForeground: readable("base", TEXT_LIGHTNESS, "popover", BODY_CONTRAST),

    // actions
    primary: solid("accent"),
    primaryForeground: label("primary"),
    primaryText: readable("accent", MUTED_LIGHTNESS, "background", CONTENT_CONTRAST),
    secondary: shift("background", SUBTLE_OFFSET),
    secondaryForeground: readable("base", TEXT_LIGHTNESS, "secondary", BODY_CONTRAST),
    muted: shift("background", SUBTLE_OFFSET),
    mutedForeground: readable("base", MUTED_LIGHTNESS, "background", CONTENT_CONTRAST),
    accent: tone("accent", SUBTLE_OFFSET, { from: "background" }),
    accentForeground: readable("accent", { light: 0.335, dark: 0.915 }, "accent", BODY_CONTRAST),

    // statuses
    destructive: solid("destructive"),
    destructiveForeground: label("destructive"),
    destructiveText: readable("destructive", MUTED_LIGHTNESS, "background", CONTENT_CONTRAST),
    success: solid("success"),
    successForeground: label("success"),
    successText: readable("success", MUTED_LIGHTNESS, "background", CONTENT_CONTRAST),
    warning: solid("warning"),
    warningForeground: label("warning"),
    warningText: readable("warning", MUTED_LIGHTNESS, "background", CONTENT_CONTRAST),
    info: solid("info"),
    infoForeground: label("info"),
    infoText: readable("info", MUTED_LIGHTNESS, "background", CONTENT_CONTRAST),

    // lines
    border: readable("base", { light: 0.886, dark: 0.348 }, "background", LINE_CONTRAST),
    input: readable("base", { light: 0.852, dark: 0.4 }, "background", LINE_CONTRAST),
    ring: readable("accent", { light: 0.736, dark: 0.532 }, "background", GRAPHIC_CONTRAST),

    // overlays
    scrim: tone("base", { light: TEXT_LIGHTNESS.light, dark: 0 }, { alpha: SCRIM_ALPHA }),

    // chart series: the accent, then four hues apart from it and from each other
    chart1: graphic("accent"),
    chart2: graphic("chart2"),
    chart3: graphic("chart3"),
    chart4: graphic("chart4"),
    chart5: graphic("chart5"),
};

/** The surface levels from the page up. */
export const SurfaceLevel = schema.enum(["base", "raised", "overlay"]);
/** A surface level. */
export type SurfaceLevel = schema.Infer<typeof SurfaceLevel>;

/** The color role each surface level shares its color with. */
export const SURFACE_ROLES: Readonly<Record<SurfaceLevel, RoleName>> = {
    base: "background",
    raised: "card",
    overlay: "popover",
};

/** The colors of a swatch: the ones a preset gives an object marked with it. */
export const SWATCH_ROLES = {
    /** The preset's solid fill, such as a tag's dot. */
    solid: "primary",
    /** The label reading on the solid fill. */
    label: "primaryForeground",
    /** The preset's subtle tint, such as a tag's background. */
    tint: "accent",
    /** The text reading on the tint. */
    text: "accentForeground",
} as const satisfies Readonly<Record<string, RoleName>>;

/** A color of a swatch. */
export type SwatchColor = keyof typeof SWATCH_ROLES;

/** The colors of a swatch, in token order. */
export const SWATCH_COLORS = [
    "solid",
    "label",
    "tint",
    "text",
] as const satisfies readonly SwatchColor[];

/** A role moving another role's color in lightness. */
function shift(from: RoleName, lightness: Lightness): Role {
    return new Role({ kind: "shift", from, lightness });
}

/** A role reading a palette's tone at a lightness. */
function tone(
    palette: PaletteName,
    lightness: Lightness,
    placement: { from?: RoleName; alpha?: number } = {},
): Role {
    return new Role({ kind: "tone", palette, lightness, ...placement });
}

/** A role reading a palette's tone that reads on a role at a contrast. */
function readable(palette: PaletteName, lightness: Lightness, on: RoleName, contrast: Floor): Role {
    return new Role({ kind: "tone", palette, lightness }, { on, contrast });
}

/** A solid role carrying a label at label contrast. */
function solid(palette: PaletteName): Role {
    return new Role({ kind: "solid", palette }, undefined, LABEL_CONTRAST);
}

/** A label role on a solid role, needing label contrast. */
function label(on: RoleName): Role {
    return new Role({ kind: "label" }, { on, contrast: LABEL_CONTRAST });
}

/** A solid graphic role standing out from the background. */
function graphic(palette: PaletteName): Role {
    return new Role({ kind: "solid", palette }, { on: "background", contrast: GRAPHIC_CONTRAST });
}
