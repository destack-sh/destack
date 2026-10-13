import { space } from "@destack/account/object";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";

/** A shareable notebook in a space. */
export const notebook = defineObject({
    name: "notebook",
    plural: "notebooks",
    scope: space,
    fields: {
        /** The notebook name. */
        name: field.string(schema.string().min(1).max(200)),
    },
    shareable: {},
}).methods((method) => ({
    create: method.create("manage"),
    update: method.update("manage"),
    delete: method.delete("manage"),
}));
