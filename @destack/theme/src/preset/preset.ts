import { defineSchema, schema } from "@destack/schema";

/** The preset palette names, neutral grays first. */
export const PRESET_NAMES = [
    "gray",
    "mauve",
    "slate",
    "sage",
    "olive",
    "sand",
    "tomato",
    "red",
    "ruby",
    "crimson",
    "pink",
    "plum",
    "purple",
    "violet",
    "iris",
    "indigo",
    "blue",
    "cyan",
    "teal",
    "jade",
    "green",
    "grass",
    "brown",
    "bronze",
    "gold",
    "sky",
    "mint",
    "lime",
    "yellow",
    "amber",
    "orange",
] as const;

/** The neutral presets, for a theme's base. */
export const GRAY_PRESETS = ["gray", "mauve", "slate", "sage", "olive", "sand"] as const;

/** The seed each preset palette grows from. */
export const PRESETS: Readonly<Record<Preset, string>> = {
    gray: "#8d8d8d",
    mauve: "#8e8c99",
    slate: "#8b8d98",
    sage: "#868e8b",
    olive: "#898e87",
    sand: "#8d8d86",
    tomato: "#e54d2e",
    red: "#e5484d",
    ruby: "#e54666",
    crimson: "#e93d82",
    pink: "#d6409f",
    plum: "#ab4aba",
    purple: "#8e4ec6",
    violet: "#6e56cf",
    iris: "#5b5bd6",
    indigo: "#3e63dd",
    blue: "#0090ff",
    cyan: "#00a2c7",
    teal: "#12a594",
    jade: "#29a383",
    green: "#30a46c",
    grass: "#46a758",
    brown: "#ad7f58",
    bronze: "#a18072",
    gold: "#978365",
    sky: "#7ce2fe",
    mint: "#86ead4",
    lime: "#bdee63",
    yellow: "#ffe629",
    amber: "#ffc53d",
    orange: "#f76b15",
};

/** A preset palette name. */
export const Preset = defineSchema(schema.enum(PRESET_NAMES));
/** A preset palette name. */
export type Preset = schema.Infer<typeof Preset>;

/** A neutral preset. */
export type GrayPreset = (typeof GRAY_PRESETS)[number];

/** A colorful preset. */
export type AccentPreset = Exclude<Preset, GrayPreset>;
