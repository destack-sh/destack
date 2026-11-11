import { space } from "@destack/account/object";
import { intersection, relation, through } from "@destack/access";
import { defineObject, field } from "@destack/object";
import { nested } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** A principal's private star on an object. */
export const favourite = defineObject(
    {
        name: "favourite",
        plural: "favourites",
        title: t`${plural(argument("count"), { one: "Favourite", other: "Favourites" })}`,
        scope: space,
        fields: {
            /** The principal. */
            owner: field.subject().caller(),
        },
        constraints: {
            owner: { kind: "unique", on: ["parentPackageId", "parentType", "parentId", "ownerId"] },
        },
        access: {
            permissions: { own: intersection(relation("owner"), through("parent", "favourite")) },
        },
        traits: [nested({ in: "any", add: "favourite" })],
    },
    (method) => ({
        /** Star the object, once per principal. */
        create: method.create("own"),
        list: method.list("own"),
        delete: method.delete("own"),
    }),
);
