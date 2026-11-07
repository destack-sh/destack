import { space } from "@destack/account/object";
import { relation } from "@destack/access";
import { definePackage, defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";

/** A notebook in a space, which its owner writes in. */
export const notebook = defineObject(
    {
        name: "notebook",
        plural: "notebooks",
        scope: space,
        fields: {
            /** The principal who created the notebook. */
            owner: field.subject().caller(),
            /** The title. */
            title: field.string(schema.string().min(1)),
        },
        permissions: { read: relation("owner"), manage: relation("owner") },
    },
    (method) => ({
        create: method.create("manage", { fields: ["title"] }),
    }),
);

/** The notebooks Acme publishes. */
export default definePackage({ objects: { notebook } });
