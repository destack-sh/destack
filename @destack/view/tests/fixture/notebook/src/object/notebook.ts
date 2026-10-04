import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { space } from "@destack/space/object";

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
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("manage"),
        update: method.update("manage"),
        delete: method.delete("manage"),
    }),
});
