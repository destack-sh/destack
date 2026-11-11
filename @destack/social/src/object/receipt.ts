import { space } from "@destack/account/object";
import { intersection, relation, through } from "@destack/access";
import { defineObject, field } from "@destack/object";
import { nested } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** A principal's private mark that it read an object, as of its last update, as Matrix's m.read. */
export const receipt = defineObject(
    {
        name: "receipt",
        plural: "receipts",
        title: t`${plural(argument("count"), { one: "Receipt", other: "Receipts" })}`,
        scope: space,
        fields: {
            /** The reader. */
            owner: field.subject().caller(),
        },
        constraints: {
            owner: { kind: "unique", on: ["parentPackageId", "parentType", "parentId", "ownerId"] },
        },
        access: {
            permissions: { own: intersection(relation("owner"), through("parent", "receipt")) },
        },
        traits: [nested({ in: "any", add: "receipt" })],
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
