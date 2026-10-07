import { defineSetting } from "@destack/setting/declare";
import { Appearance } from "@destack/theme";

/** The appearance a person selects, overridden per package, space, installation or client. */
export const appearance = defineSetting({
    name: "appearance",
    title: "Appearance",
    description: "Use the system appearance or select a light or dark interface.",
    schema: Appearance,
    default: "system",
    scope: "user",
    overrides: ["package", "space", "installation", "client"],
    apply: "immediate",
});
