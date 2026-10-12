import { space } from "@destack/account/object";
import { intersection, relation, through } from "@destack/access";
import { type Select } from "@destack/db";
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
    permissions: { own: intersection(relation("owner"), through("parent", "receipt")) },
    constraints: {
        owner: { kind: "unique", on: ["parentPackageId", "parentType", "parentId", "owner"] },
    },
}).methods((method) => ({
    list: method.list("own"),
    create: method.create("own"),
    update: method.update("own"),
    delete: method.delete("own"),
}));

/** A receipt as its table stores it. */
export type Receipt = Select<typeof receipt.table>;
