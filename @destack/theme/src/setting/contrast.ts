import { defineSetting } from "@destack/setting/declare";
import { Contrast, DEFAULT_PREFERENCES } from "../theme/index.ts";

/** The contrast a person reads at, overridden per package, space, installation or client. */
export const contrast = defineSetting({
    name: "contrast",
    title: "Contrast",
    description: "Follow the device's contrast or set how far text, lines and graphics stand out.",
    schema: Contrast,
    default: DEFAULT_PREFERENCES.contrast,
    scope: "user",
    overrides: ["package", "space", "installation", "client"],
    apply: "immediate",
});
