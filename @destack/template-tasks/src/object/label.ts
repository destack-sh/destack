import { through } from "@destack/access";
import { space } from "@destack/account/object";
import { argument, plural, t } from "@destack/locale";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";

/** A tag in a space that sorts tasks across projects. */
export const label = defineObject({
    name: "label",
    plural: "labels",
    title: t`${plural(argument("count"), { one: "Label", other: "Labels" })}`,
    scope: space,
    access: {
        permissions: { read: through("scope", "read"), write: through("scope", "write") },
    },
    fields: {
        /** The label's name. */
        name: field.string(schema.string().min(1).max(100)),
        /** The label's color, as a CSS color. */
        color: field.string(schema.string().max(32)).default("gray"),
    },
});
