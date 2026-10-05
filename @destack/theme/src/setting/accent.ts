import { defineSetting } from "@destack/setting/declare";
import { Preset } from "../radix/index.ts";
import { DEFAULT_PREFERENCES } from "../theme/index.ts";

/** The accent a person selects over every app's own, overridden per package, space, installation or device. */
export const accent = defineSetting({
    name: "accent",
    title: "Accent color",
    description: "Use each app's own accent, or one accent color in every app.",
    schema: Preset.nullable(),
    default: DEFAULT_PREFERENCES.accent,
    scope: "user",
    overrides: ["package", "space", "installation", "device"],
    apply: "immediate",
});
