import { intersection, relation, through } from "@destack/access";
import { unique } from "@destack/db";
import { defineObject, field, method } from "@destack/object";
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
    constraints: (favourite) => [
        unique("favourite_owner").on(
            favourite.parentPackageId,
            favourite.parentType,
            favourite.parentId,
            favourite.owner,
        ),
    ],
    permissions: { own: intersection(relation("owner"), through("parent", "favourite")) },
    methods: {
        list: method.list("own"),
        create: method.create("own"),
        delete: method.delete("own"),
    },
});
