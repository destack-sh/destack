import { principal, relation, through } from "@destack/access";
import { space } from "@destack/account/object";
import { argument, plural, t } from "@destack/locale";
import { activity, defineObject, field, nested, tracked } from "@destack/object";
import { schema } from "@destack/schema";

/** Binders their owner reads, keeping no history of who owns them. */
export const binder = defineObject({
    name: "binder",
    plural: "binders",
    title: t`${plural(argument("count"), { one: "Binder", other: "Binders" })}`,
    scope: space,
    fields: {
        /** The person owning the binder. */
        owner: field.reference(principal.user).caller(),
    },
    access: {
        permissions: { read: relation("owner") },
    },
});

/** Sheets in a binder, read through it, their changes kept as activities. */
export const sheet = defineObject({
    name: "sheet",
    plural: "sheets",
    title: t`${plural(argument("count"), { one: "Sheet", other: "Sheets" })}`,
    scope: space,
    fields: {
        /** The sheet's text. */
        text: field.string(schema.string().min(1)),
    },
    access: {
        permissions: { read: through("parent", "read") },
    },
    traits: [nested({ in: binder, add: "read" }), tracked({ by: "read", activity })],
});
