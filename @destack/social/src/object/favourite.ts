import { space } from "@destack/account/object";
import { intersection, relation, through } from "@destack/access";
import { defineObject, field } from "@destack/object";

/** A principal's private star on an object. */
export const favourite = defineObject({
    name: "favourite",
    plural: "favourites",
    scope: space,
    nested: { in: "any", receive: "favourite" },
    fields: {
        /** The principal. */
        owner: field.subject().caller(),
    },
    permissions: { own: intersection(relation("owner"), through("parent", "favourite")) },
    constraints: {
        owner: { kind: "unique", on: ["parentPackageId", "parentType", "parentId", "owner"] },
    },
}).methods((method) => ({
    create: method.create("own"),
    list: method.list("own"),
    delete: method.delete("own"),
}));
