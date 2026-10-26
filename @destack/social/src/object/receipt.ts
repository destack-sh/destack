import { space } from "@destack/account/object";
import { intersection, relation, through } from "@destack/access";
import { defineObject, field } from "@destack/object";

/** A principal's private mark that it read an object, as of its last update, as Matrix's m.read. */
export const receipt = defineObject(
    {
        name: "receipt",
        plural: "receipts",
        scope: space,
        nested: { in: "any", receive: "receipt" },
        fields: {
            /** The reader. */
            owner: field.subject().caller(),
        },
        permissions: { own: intersection(relation("owner"), through("parent", "receipt")) },
        constraints: {
            owner: { kind: "unique", on: ["parentPackageId", "parentType", "parentId", "ownerId"] },
        },
    },
    (method) => ({
        /** Mark the object read now, once per principal. */
        create: method.create("own"),
        list: method.list("own"),
        /** Move the mark to now. */
        update: method.update("own", { visibility: "package" }),
        delete: method.delete("own"),
    }),
);
