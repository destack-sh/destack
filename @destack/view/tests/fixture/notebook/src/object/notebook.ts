import { space } from "@destack/account/object";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { shareable } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** A shareable notebook in a space. */
export const notebook = defineObject(
    {
        name: "notebook",
        plural: "notebooks",
        title: t`${plural(argument("count"), { one: "Notebook", other: "Notebooks" })}`,
        scope: space,
        fields: {
            /** The notebook name. */
            name: field.string(schema.string().min(1).max(200)),
        },
        traits: [shareable({})],
    },
    (method) => ({
        create: method.create("manage"),
        update: method.update("manage"),
        delete: method.delete("manage"),
    }),
);
