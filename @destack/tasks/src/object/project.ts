import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { space } from "@destack/space/object";

/** A body of work in a space, planned by its members and followed by its viewers. */
export const project = defineObject({
    name: "project",
    plural: "projects",
    scope: space,
    fields: {
        /** The project name. */
        name: field.string(schema.string().min(1).max(200)),
    },
    recoverable: { within: { days: 30 }, by: "manage" },
    shareable: {},
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("manage"),
        update: method.update("manage"),
    }),
});
