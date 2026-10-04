import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { space } from "@destack/space/object";

/** A page in a tree of pages, shared with its subpages and publishable through links. */
export const page = defineObject({
    name: "page",
    plural: "pages",
    scope: space,
    nested: {
        in: "self",
        optional: true,
        receive: "edit",
        move: "manage",
    },
    fields: {
        /** The page title. */
        title: field.string(schema.string().max(500)).default(""),
        /** The emoji or icon name shown beside the title. */
        icon: field.string(schema.string().max(32)).optional(),
        /** The page's place among its siblings, as a fractional index. */
        position: field.position().default("a0"),
    },
    recoverable: { within: { days: 30 }, by: "manage" },
    shareable: { isPublic: true },
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("edit"),
        update: method.update("edit"),
    }),
});
