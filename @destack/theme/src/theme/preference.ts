import { defineSchema, schema } from "@destack/schema";
import { Seed } from "../palette/index.ts";

/** The body size of each text size relative to the default medium, from 14 to 23 points over 16. */
export const TEXT_SCALES = {
    xSmall: 14 / 16,
    small: 15 / 16,
    medium: 1,
    large: 17 / 16,
    xLarge: 19 / 16,
    xxLarge: 21 / 16,
    xxxLarge: 23 / 16,
} as const;

/** The spacing of each density, one step of 4 pixels on a 32-pixel control apart. */
export const DENSITY_SCALES = {
    compact: 0.875,
    regular: 1,
    spacious: 1.125,
} as const;

/** The appearance a person sees: the device preference, light or dark. */
export const Appearance = defineSchema(schema.enum(["system", "light", "dark"]));
/** The appearance a person sees. */
export type Appearance = schema.Infer<typeof Appearance>;

/** The text size a person reads at. */
export const TextSize = defineSchema(
    schema.enum(["xSmall", "small", "medium", "large", "xLarge", "xxLarge", "xxxLarge"]),
);
/** The text size a person reads at. */
export type TextSize = schema.Infer<typeof TextSize>;

/** How tightly controls and content pack. */
export const Density = defineSchema(schema.enum(["compact", "regular", "spacious"]));
/** How tightly controls and content pack. */
export type Density = schema.Infer<typeof Density>;

/** How far text, lines and graphics stand out: the device preference, or a level from 0, the standard, to 1, the most. */
export const Contrast = defineSchema(
    schema.union([schema.literal("system"), schema.number().min(0).max(1)]),
);
/** How far text, lines and graphics stand out. */
export type Contrast = schema.Infer<typeof Contrast>;

/** The motion of transitions: the device preference, full or reduced to none. */
export const Motion = defineSchema(schema.enum(["system", "full", "reduced"]));
/** The motion of transitions. */
export type Motion = schema.Infer<typeof Motion>;

/** The display preferences of a person, each read from its setting. */
export const Preferences = defineSchema(
    schema.object({
        /** The text size. */
        textSize: TextSize,
        /** The density, or null for the theme's own. */
        density: Density.nullable(),
        /** The contrast. */
        contrast: Contrast,
        /** The motion. */
        motion: Motion,
        /** The accent, or null for the theme's own. */
        accent: Seed.nullable(),
    }),
);
/** The display preferences of a person. */
export type Preferences = schema.Infer<typeof Preferences>;

/** The preferences of a person who changed none: each setting's default. */
export const DEFAULT_PREFERENCES: Preferences = {
    textSize: "medium",
    density: null,
    contrast: "system",
    motion: "system",
    accent: null,
};
