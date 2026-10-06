import { defineSetting } from "@destack/setting/declare";
import { DEFAULT_PREFERENCES, Density } from "../theme/index.ts";

/** How tightly a person packs controls and content, overridden per package, space, installation or client. */
export const density = defineSetting({
    name: "density",
    title: "Density",
    description:
        "Pack controls and content compactly or spaciously, or keep each app's own density.",
    schema: Density.nullable(),
    default: DEFAULT_PREFERENCES.density,
    scope: "user",
    overrides: ["package", "space", "installation", "client"],
    apply: "immediate",
});
