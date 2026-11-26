import { type Message, t } from "@destack/locale";
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

/** A hue, such as of a tag, a status, a calendar or a package, with its choice in pickers. */
export const Hue = Object.assign(schema.enum(HUES), {
    /** Each hue as a picker offers it: its name in the person's language, shown in its color. */
    choices: {
        gray: { title: t.context("option")`Gray`, color: "gray" },
        brown: { title: t.context("option")`Brown`, color: "brown" },
        red: { title: t.context("option")`Red`, color: "red" },
        orange: { title: t.context("option")`Orange`, color: "orange" },
        yellow: { title: t.context("option")`Yellow`, color: "yellow" },
        green: { title: t.context("option")`Green`, color: "green" },
        cyan: { title: t.context("option")`Cyan`, color: "cyan" },
        blue: { title: t.context("option")`Blue`, color: "blue" },
        purple: { title: t.context("option")`Purple`, color: "purple" },
        pink: { title: t.context("option")`Pink`, color: "pink" },
    } satisfies Record<
        (typeof HUES)[number],
        { readonly title: Message; readonly color: (typeof HUES)[number] }
    >,
});
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
