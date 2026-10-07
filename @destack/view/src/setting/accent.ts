import { defineSetting } from "@destack/setting/declare";
import { Seed } from "@destack/theme";
import { DEFAULT_PREFERENCES } from "@destack/theme";

/** The accent a person selects over every app's own, overridden per package, space, installation or client. */
export const accent = defineSetting({
    name: "accent",
    title: "Accent color",
    description: "Use each app's own accent, or one accent color in every app.",
    schema: Seed.nullable(),
    default: DEFAULT_PREFERENCES.accent,
    scope: "user",
    overrides: ["package", "space", "installation", "client"],
    apply: "immediate",
});
