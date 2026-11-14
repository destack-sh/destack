import { through } from "@destack/access";
import { space } from "@destack/account/object";
import { argument, plural, t } from "@destack/locale";
import { activity, defineObject, field, tracked } from "@destack/object";
import { schema } from "@destack/schema";

/** Journal entries a space's members read and write, their changes kept as activities. */
export const entry = defineObject({
    name: "entry",
    plural: "entries",
    title: t`${plural(argument("count"), { one: "Entry", other: "Entries" })}`,
    scope: space,
    access: {
        permissions: { read: through("scope", "read"), write: through("scope", "write") },
    },
    fields: {
        /** The entry's text. */
        text: field.string(schema.string().min(1)),
    },
    traits: [tracked({ by: "write", activity })],
});
