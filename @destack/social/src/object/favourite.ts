import { intersection, relation, through } from "@destack/access";
import { unique } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { space } from "@destack/space/object";

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
    constraints: (entry) => [
        unique("favourite_owner").on(
            entry.parentPackageId,
            entry.parentType,
            entry.parentId,
            entry.owner,
        ),
    ],
    permissions: { own: intersection(relation("owner"), through("parent", "favourite")) },
    methods: (method) => ({
        list: method.list("own"),
        create: method.create("own"),
        delete: method.delete("own"),
    }),
});
