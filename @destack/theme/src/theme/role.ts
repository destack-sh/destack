import { defineSchema, schema } from "@destack/schema";
import { Color, HexColor, Scale, STEPS, type Scheme, type Step } from "../palette/index.ts";
import { VARIABLE_PREFIX } from "../token/index.ts";
import type { Contrast } from "./preference.ts";

/** The APCA-W3 bronze level for body text: Lc 75 at least. */
export const BODY_CONTRAST = 75;

/** The APCA-W3 bronze level for content text other than body text: Lc 60 at least. */
export const CONTENT_CONTRAST = 60;

/** The custom property the stylesheet sets to 1 when the device asks for more contrast. */
const CONTRAST_VARIABLE = `${VARIABLE_PREFIX}-contrast`;

/** The opacity of the scrim behind modal surfaces: black at half strength. */
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
] as const;

/** The scales a theme's roles read from: its gray and accent, and one per status. */
export const ROLE_SCALES = ["gray", "accent", "red", "green", "amber", "blue"] as const;

/** A scale a theme's roles read from. */
export type RoleScale = (typeof ROLE_SCALES)[number];

/** The scales of a theme by the name its roles read them under. */
export type RoleScales = Readonly<Record<RoleScale, Scale>>;

/** The contrast a role resolves under, after the person's contrast setting. */
export type RoleContrast = "standard" | "more";

/** The background a text role reads on and the contrast it needs there. */
export interface RoleText {
    /** The color role of the background. */
    readonly on: RoleName;
    /** The APCA lightness contrast Lc the text needs, as a magnitude. */
    readonly contrast: number;
}

/** Where a role's color comes from: a scale step, the best label on a background role, or explicit colors. */
export type RoleSource =
    | {
          /** A step of a scale. */
          readonly kind: "step";
          /** The scale. */
          readonly scale: RoleScale;
          /** The light step. */
          readonly step: Step;
          /** The dark step when it differs from the light one. */
          readonly dark?: Step;
          /** The step under more contrast. */
          readonly more?: Step;
          /** The opacity from 0 to 1, opaque when absent. */
          readonly alpha?: number;
      }
    | {
          /** White, the scale's or the gray's darkest step, whichever reads best on the role it reads on. */
          readonly kind: "label";
          /** The scale. */
          readonly scale: RoleScale;
      }
    | {
          /** Explicit colors. */
          readonly kind: "color";
          /** The light appearance's six-digit sRGB hex color. */
          readonly light: string;
          /** The dark appearance's six-digit sRGB hex color. */
          readonly dark: string;
      };

/** A theme's replacement for one role: a scale step or explicit colors. */
export const RoleOverride = defineSchema(
    schema.union([
        schema.object({
            /** The scale. */
            scale: schema.enum(ROLE_SCALES),
            /** The step in both appearances. */
            step: schema.literal(STEPS),
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

/** A color role with the background it reads on when it is text. */
export class Role {
    /** Where the color comes from. */
    readonly source: RoleSource;
    /** The role this one is text on and the APCA lightness contrast it needs there, for text roles. */
    readonly text: RoleText | undefined;

    /** Hold a role. */
    constructor(source: RoleSource, text?: RoleText) {
        this.source = source;
        this.text = text;
    }

    /** Replace the color source, keeping the text requirement. */
    override(override: RoleOverride): Role {
        const source: RoleSource =
            "scale" in override
                ? { kind: "step", scale: override.scale, step: override.step }
                : { kind: "color", light: override.light, dark: override.dark };

        return new Role(source, this.text);
    }

    /** Format the color in both appearances under the person's contrast, following the device for `system`. */
    css(roles: Roles, scales: RoleScales, contrast: Contrast): string {
        // resolve both appearances under the explicit or the standard contrast
        const pair = (resolved: RoleContrast) =>
            [
                this.color(roles, scales, "light", resolved),
                this.color(roles, scales, "dark", resolved),
            ] as const;
        const [light, dark] = pair(contrast === "more" ? "more" : "standard");
        const isContrasted = this.source.kind === "step" && this.source.more !== undefined;
        if (contrast !== "system" || !isContrasted) {
            return light === dark ? light : `light-dark(${light}, ${dark})`;
        }

        // mix toward the more contrasted step as far as the device contrast the stylesheet sets
        const [lightMore, darkMore] = pair("more");
        const amount = `calc(var(${CONTRAST_VARIABLE}, 0) * 100%)`;
        const mix = (standard: string, more: string) =>
            `color-mix(in oklab, ${standard}, ${more} ${amount})`;

        return `light-dark(${mix(light, lightMore)}, ${mix(dark, darkMore)})`;
    }

    /** Resolve the color in one appearance under a contrast, reading label backgrounds from the roles. */
    color(roles: Roles, scales: RoleScales, scheme: Scheme, contrast: RoleContrast): string {
        const source = this.source;
        switch (source.kind) {
            case "step": {
                const standard = scheme === "dark" ? (source.dark ?? source.step) : source.step;
                const resolved = contrast === "more" ? (source.more ?? standard) : standard;
                const color = scales[source.scale].color(resolved, scheme);

                return source.alpha === undefined ? color : Color.translucent(color, source.alpha);
            }
            case "label": {
                // pick the candidate that reads best on the resolved background role
                if (this.text === undefined) {
                    throw new RangeError("a label role needs the role it reads on");
                }
                const background = roles[this.text.on].color(roles, scales, scheme, contrast);

                return scales[source.scale].label(background, scales.gray.color(12, "light"));
            }
            case "color":
                return source[scheme];
        }
    }
}

/** A color role name. */
export const RoleName = defineSchema(schema.enum(ROLE_NAMES));
/** A color role name. */
export type RoleName = schema.Infer<typeof RoleName>;

/** The color roles of a theme by name. */
export type Roles = Readonly<Record<RoleName, Role>>;

/** The color roles every theme starts from. */
export const COLOR_ROLES: Roles = {
    // surfaces and their text
    background: step("gray", 1),
    foreground: step("gray", 12, {}, { on: "background", contrast: BODY_CONTRAST }),
    card: step("gray", 2),
    cardForeground: step("gray", 12, {}, { on: "card", contrast: BODY_CONTRAST }),
    popover: step("gray", 1, { dark: 3 }),
    popoverForeground: step("gray", 12, {}, { on: "popover", contrast: BODY_CONTRAST }),

    // actions
    primary: step("accent", 9),
    primaryForeground: label("accent", "primary"),
    secondary: step("gray", 3),
    secondaryForeground: step("gray", 12, {}, { on: "secondary", contrast: BODY_CONTRAST }),
    muted: step("gray", 3),
    mutedForeground: step(
        "gray",
        11,
        { more: 12 },
        { on: "background", contrast: CONTENT_CONTRAST },
    ),
    accent: step("accent", 3),
    accentForeground: step("accent", 12, {}, { on: "accent", contrast: BODY_CONTRAST }),

    // statuses
    destructive: step("red", 9),
    destructiveForeground: label("red", "destructive"),
    success: step("green", 9),
    successForeground: label("green", "success"),
    warning: step("amber", 9),
    warningForeground: label("amber", "warning"),
    info: step("blue", 9),
    infoForeground: label("blue", "info"),

    // lines
    border: step("gray", 6, { more: 8 }),
    input: step("gray", 7, { more: 8 }),
    ring: step("accent", 8),

    // overlays
    scrim: step("gray", 12, { dark: 1, alpha: SCRIM_ALPHA }),
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

/** A role reading a scale step. */
function step(
    scale: RoleScale,
    light: Step,
    steps: { dark?: Step; more?: Step; alpha?: number } = {},
    text?: RoleText,
): Role {
    return new Role({ kind: "step", scale, step: light, ...steps }, text);
}

/** A label role on a background role, needing content contrast. */
function label(scale: RoleScale, on: RoleName): Role {
    return new Role({ kind: "label", scale }, { on, contrast: CONTENT_CONTRAST });
}
