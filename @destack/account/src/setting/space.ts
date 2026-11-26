import { schema } from "@destack/schema";
import { defineSetting } from "@destack/setting/declare";
import { SpaceAddress, SpaceName } from "../address/index.ts";
import { t } from "@destack/locale";

/** The short names a person calls spaces by, merged key by key from every scope. */
export const spaceAliases = defineSetting({
    name: "space.aliases",
    title: t.context("title")`Space names`,
    description: t`Short names for spaces, such as work for work--example, beside home.`,
    schema: schema.record(SpaceName, SpaceAddress),
    default: {},
    scope: "user",
    merge: "key",
});
