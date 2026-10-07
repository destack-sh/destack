import { defineSetting } from "@destack/setting/declare";
import { DEFAULT_PREFERENCES, TextSize } from "@destack/theme";

/** The text size a person reads at, overridden per package, space, installation or client. */
export const textSize = defineSetting({
    name: "textSize",
    title: "Text size",
    description: "Read text smaller or larger.",
    schema: TextSize,
    default: DEFAULT_PREFERENCES.textSize,
    scope: "user",
    overrides: ["package", "space", "installation", "client"],
    apply: "immediate",
});
