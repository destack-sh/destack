import { defineSetting } from "@destack/setting/declare";
import { schema } from "@destack/schema";

/** The appearance a person selects, overridden per package, space, installation or device. */
export const appearance = defineSetting({
    name: "appearance",
    title: "Appearance",
    description: "Use the system appearance or select a light or dark interface.",
    schema: schema.enum(["system", "light", "dark"]),
    default: "system",
    scope: "user",
    overrides: ["package", "space", "installation", "device"],
    apply: "immediate",
});
