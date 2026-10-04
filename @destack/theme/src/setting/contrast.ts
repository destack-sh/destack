import { defineSetting } from "@destack/setting/declare";
import { Contrast, DEFAULT_PREFERENCES } from "../theme/index.ts";

/** The contrast a person reads at, overridden per package, space, installation or device. */
export const contrast = defineSetting({
    name: "contrast",
    title: "Contrast",
    description: "Use the system contrast or darken borders and secondary text.",
    schema: Contrast,
    default: DEFAULT_PREFERENCES.contrast,
    scope: "user",
    overrides: ["package", "space", "installation", "device"],
    apply: "immediate",
});
