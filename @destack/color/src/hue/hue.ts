import { schema } from "@destack/schema";

/** The hues colors mean things in, which each theme fills in. */
export const HUES = [
    "gray",
    "brown",
    "red",
    "orange",
    "yellow",
    "green",
    "cyan",
    "blue",
    "purple",
    "pink",
] as const;

/** A hue, such as of a tag, a status, a calendar or a package. */
export const Hue = schema.enum(HUES);
/** A hue. */
export type Hue = schema.Infer<typeof Hue>;

/** Destack's own color of each hue, which a theme may replace, from Radix Colors. */
export const DEFAULT_HUES = {
    gray: "#8b8d98",
    brown: "#ad7f58",
    red: "#e5484d",
    orange: "#f76b15",
    yellow: "#ffc53d",
    green: "#30a46c",
    cyan: "#00a2c7",
    blue: "#0090ff",
    purple: "#8e4ec6",
    pink: "#d6409f",
} as const satisfies Record<Hue, string>;
