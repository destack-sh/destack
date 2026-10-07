import { defineSchema, schema } from "@destack/schema";
import { apca, Color, HexColor, type Palette, type Scheme } from "../palette/index.ts";

/** An APCA lightness contrast Lc a role needs, at the standard contrast and at the most a person asks for. */
export interface Floor {
    /** The magnitude at the standard contrast. */
    readonly standard: number;
    /** The magnitude at the most contrast. */
    readonly more: number;
}

/** The APCA-W3 bronze level for body text, Lc 75, and its fluent level, Lc 90. */
export const BODY_CONTRAST: Floor = { standard: 75, more: 90 };

/** The APCA-W3 bronze level for content text other than body text, Lc 60, and body text's fluent level at the most. */
export const CONTENT_CONTRAST: Floor = { standard: 60, more: 90 };

/** The contrast of labels on solid fills, Lc 60, and Lc 75 at the most, where the fill keeps its hue. */
export const LABEL_CONTRAST: Floor = { standard: 60, more: 75 };

/** The contrast of solid graphics such as chart series and focus rings, Lc 45 as APCA asks of large shapes, and Lc 60. */
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
const WHITE = "#ffffff";

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
    "secondary",
    "secondaryForeground",
    "muted",
    "mutedForeground",
    "accent",
    "accentForeground",
    "destructive",
    "destructiveForeground",
    "success",
    "successForeground",
    "warning",
    "warningForeground",
    "info",
    "infoForeground",
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

/** The palette names of a theme: its base and accent, one per status and the chart series after the accent. */
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
export const PaletteName = defineSchema(schema.enum(PALETTE_NAMES));
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

/** Where a role's color comes from: another role's color moved in lightness, a tone of a palette, a palette's seed, the best label on the role it reads on, or explicit colors. */
export type RoleSource =
    | {
          /** Another role's color at a lightness offset, keeping its chroma and hue. */
          readonly kind: "shift";
          /** The role. */
          readonly from: RoleName;
          /** The lightness offset. */
          readonly lightness: Lightness;
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
          /** White, or the darkest text of the base or of the palette under it, whichever reads best on the role it reads on. */
          readonly kind: "label";
      }
    | {
          /** Explicit colors. */
          readonly kind: "color";
          /** The light appearance's six-digit sRGB hex color. */
          readonly light: string;
          /** The dark appearance's six-digit sRGB hex color. */
          readonly dark: string;
      };

/** A theme's replacement for one role: a palette's seed or tone, or explicit colors. */
export const RoleOverride = defineSchema(
    schema.union([
        schema.object({
            /** The palette. */
            palette: PaletteName,
            /** The tone's OKLCH lightness in each appearance, the palette's seed when absent. */
            lightness: schema
                .object({
                    /** The light appearance's lightness. */
                    light: schema.number().min(0).max(1),
                    /** The dark appearance's lightness. */
                    dark: schema.number().min(0).max(1),
                })
                .exactOptional(),
        }),
        schema.object({
            /** The light appearance's color. */
            light: HexColor,
            /** The dark appearance's color. */
            dark: HexColor,
        }),
    ]),
);
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

    /** Hold a role. */
    constructor(source: RoleSource, reading?: Reading, labelled?: Floor) {
        this.source = source;
        this.reading = reading;
        this.labelled = labelled;
    }

    /** Replace the color source, keeping what the role reads on and what its label needs. */
    override(override: RoleOverride): Role {
        const source: RoleSource =
            "palette" in override
                ? override.lightness === undefined
                    ? { kind: "solid", palette: override.palette }
                    : { kind: "tone", palette: override.palette, lightness: override.lightness }
                : { kind: "color", light: override.light, dark: override.dark };

        return new Role(source, this.reading, this.labelled);
    }
}

/** A color role name. */
export const RoleName = defineSchema(schema.enum(ROLE_NAMES));
/** A color role name. */
export type RoleName = schema.Infer<typeof RoleName>;

/** The color roles of a theme by name. */
export type Roles = Readonly<Record<RoleName, Role>>;

/** The colors of a theme's roles in one appearance at one contrast level, each resolved once. */
export class Resolution {
    /** The roles. */
    readonly #roles: Roles;
    /** The palettes the roles read. */
    readonly #palettes: Palettes;
    /** The appearance. */
    readonly scheme: Scheme;
    /** The contrast level from 0, the standard floors, to 1, the most. */
    readonly level: number;
    /** The colors resolved so far. */
    readonly #colors = new Map<RoleName, string>();
    /** The roles being resolved, which a role must not read through again. */
    readonly #resolving = new Set<RoleName>();

    /** Hold the roles and palettes of one appearance and level. */
    constructor(roles: Roles, palettes: Palettes, scheme: Scheme, level: number) {
        // hold what every color resolves from
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
            throw new RangeError(`role ${name} reads through itself`);
        }

        // resolve the role once
        this.#resolving.add(name);
        const color = this.#resolve(this.#roles[name]);
        this.#resolving.delete(name);
        this.#colors.set(name, color);

        return color;
    }

    /** Measure the APCA lightness contrast a role reads at on the role it reads on, as a magnitude. */
    reads(name: RoleName): number {
        const reading = this.#roles[name].reading;
        if (reading === undefined) {
            throw new RangeError(`role ${name} reads on no role`);
        }

        return Math.abs(apca(this.color(name), this.color(reading.on)));
    }

    /** Read a floor at the level. */
    floor(contrast: Floor): number {
        return contrast.standard + this.level * (contrast.more - contrast.standard);
    }

    /** Pick the label that reads best on a background: white, or the palette's or the base's darkest text. */
    label(palette: Palette, background: string): string {
        // measure each candidate on the background and keep the strongest
        const dark = TEXT_LIGHTNESS.light;
        const candidates = [WHITE, palette.tone(dark), this.#palettes.base.tone(dark)];
        const reading = (candidate: string) => Math.abs(apca(candidate, background));

        return candidates.reduce((best, candidate) =>
            reading(candidate) > reading(best) ? candidate : best,
        );
    }

    /** Resolve a role from its source, moving a tone or seed by the least lightness that meets its contrast. */
    #resolve(role: Role): string {
        const source = role.source;
        switch (source.kind) {
            case "shift": {
                // move the role's color in lightness alone
                const color = Color.parse(this.color(source.from));
                const lightness = color.lightness + source.lightness[this.scheme];

                return new Color(lightness, color.chroma, color.hue).hex();
            }
            case "tone": {
                // start from the lightness and move away from the background it reads on
                const palette = this.#palettes[source.palette];
                const offset = source.lightness[this.scheme];
                const start =
                    source.from === undefined
                        ? offset
                        : Color.parse(this.color(source.from)).lightness + offset;
                const color = palette.tone(this.#settle(palette, start, role));

                return source.alpha === undefined ? color : Color.translucent(color, source.alpha);
            }
            case "solid": {
                const palette = this.#palettes[source.palette];

                return palette.tone(this.#settle(palette, palette.lightness, role));
            }
            case "label": {
                // pick among the candidates of the palette the background grows from
                const reading = role.reading;
                if (reading === undefined) {
                    throw new RangeError("a label role needs the role it reads on");
                }
                const under = this.#roles[reading.on].source;
                const palette = this.#palettes["palette" in under ? under.palette : "base"];

                return this.label(palette, this.color(reading.on));
            }
            case "color":
                return source[this.scheme];
        }
    }

    /** Find the lightness nearest a start at which a role and its label read, the farthest from its background when none does. */
    #settle(palette: Palette, start: number, role: Role): number {
        // require the role's reading and its label's at the level
        const { reading, labelled } = role;
        const background = reading === undefined ? undefined : this.color(reading.on);
        const passes = (color: string) =>
            (reading === undefined ||
                background === undefined ||
                Math.abs(apca(color, background)) >= this.floor(reading.contrast)) &&
            (labelled === undefined ||
                Math.abs(apca(this.label(palette, color), color)) >= this.floor(labelled));

        // move a role reading on a background away from it, and a solid either way
        if (background === undefined || role.source.kind === "solid") {
            return palette.nearest(start, passes, "either") ?? start;
        }
        const isLighter = start >= Color.parse(background).lightness;

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
    secondary: shift("background", SUBTLE_OFFSET),
    secondaryForeground: readable("base", TEXT_LIGHTNESS, "secondary", BODY_CONTRAST),
    muted: shift("background", SUBTLE_OFFSET),
    mutedForeground: readable("base", MUTED_LIGHTNESS, "background", CONTENT_CONTRAST),
    accent: tone("accent", SUBTLE_OFFSET, { from: "background" }),
    accentForeground: readable("accent", { light: 0.335, dark: 0.915 }, "accent", BODY_CONTRAST),

    // statuses
    destructive: solid("destructive"),
    destructiveForeground: label("destructive"),
    success: solid("success"),
    successForeground: label("success"),
    warning: solid("warning"),
    warningForeground: label("warning"),
    info: solid("info"),
    infoForeground: label("info"),

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
export const SurfaceLevel = defineSchema(schema.enum(["base", "raised", "overlay"]));
/** A surface level. */
export type SurfaceLevel = schema.Infer<typeof SurfaceLevel>;

/** The color role each surface level shares its color with. */
export const SURFACE_ROLES: Readonly<Record<SurfaceLevel, RoleName>> = {
    base: "background",
    raised: "card",
    overlay: "popover",
};

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
