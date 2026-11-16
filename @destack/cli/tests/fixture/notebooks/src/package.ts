import { space } from "@destack/account/object";
import { relation } from "@destack/access";
import { activity, definePackage, defineObject, field, tracked } from "@destack/object";
import { schema } from "@destack/schema";
import { argument, plural, t } from "@destack/locale";

/** A notebook in a space, which its owner writes in, its changes kept as activities. */
export const notebook = defineObject(
    {
        name: "notebook",
        plural: "notebooks",
        title: t`${plural(argument("count"), { one: "Notebook", other: "Notebooks" })}`,
        scope: space,
        fields: {
            /** The principal who created the notebook. */
            owner: field.subject().caller(),
            /** The title. */
            title: field.string(schema.string().min(1)),
        },
        access: {
            permissions: { read: relation("owner"), manage: relation("owner") },
        },
        traits: [tracked({ by: "manage", activity })],
    },
    (method) => ({
        create: method.create("manage", { fields: ["title"] }),
    }),
);

/** The notebooks Acme publishes. */
export default definePackage({
    title: t`Notebooks`,
    description: t`Keep notebooks.`,
    icon: "notebook",
    color: "yellow",
    objects: [notebook],
});
