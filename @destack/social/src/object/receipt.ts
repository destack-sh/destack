import { intersection, relation, through } from "@destack/access";
import { type Select, unique } from "@destack/db";
import { defineObject, field, method } from "@destack/object";
import { space } from "@destack/space/object";

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
    constraints: (receipt) => [
        unique("receipt_owner").on(
            receipt.parentPackageId,
            receipt.parentType,
            receipt.parentId,
            receipt.owner,
        ),
    ],
    permissions: { own: intersection(relation("owner"), through("parent", "receipt")) },
    methods: {
        list: method.list("own"),
        create: method.create("own"),
        update: method.update("own"),
        delete: method.delete("own"),
    },
});

/** A receipt as its table holds it. */
export type ReceiptRow = Select<typeof receipt.table>;
