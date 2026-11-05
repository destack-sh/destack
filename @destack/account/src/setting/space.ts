import { schema } from "@destack/schema";
import { defineSetting } from "@destack/setting/declare";
import { SpaceAddress, SpaceName } from "../address/index.ts";

/** The short names a person calls spaces by, merged key by key from every scope. */
export const spaceAliases = defineSetting({
    name: "space.aliases",
    title: "Space names",
    description: "Short names for spaces, such as work for work--acme, beside home and residency.",
    schema: schema.record(SpaceName, SpaceAddress),
    default: {},
    scope: "user",
    overrides: [],
    apply: "immediate",
    merge: "key",
});
