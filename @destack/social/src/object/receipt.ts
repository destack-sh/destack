import { space } from "@destack/account/object";
import { intersection, relation, through } from "@destack/access";
import { type Select, unique } from "@destack/db";
import { defineObject, field } from "@destack/object";

/** A principal's private mark that it read an object, as of its last update. */
export const receipt = defineObject({
    name: "receipt",
    plural: "receipts",
    scope: space,
    nested: { in: "any", receive: "receipt" },
    fields: {
        /** The reader. */
        owner: field.subject().caller(),
    },
    constraints: (entry) => [
        unique("receipt_owner").on(
            entry.parentPackageId,
            entry.parentType,
            entry.parentId,
            entry.owner,
        ),
    ],
    permissions: { own: intersection(relation("owner"), through("parent", "receipt")) },
    methods: (method) => ({
        list: method.list("own"),
        create: method.create("own"),
        update: method.update("own"),
        delete: method.delete("own"),
    }),
});

/** A receipt as its table stores it. */
export type Receipt = Select<typeof receipt.table>;
