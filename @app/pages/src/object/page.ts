import { space } from "@destack/account/object";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";

/** A page in a tree of pages, shared with its subpages and publishable through links. */
export const page = defineObject({
    name: "page",
    plural: "pages",
    scope: space,
    nested: {
        in: "self",
        optional: true,
        receive: "write",
        move: "manage",
    },
    fields: {
        /** The page title. */
        title: field.string(schema.string().max(500)).default(""),
        /** The emoji or icon name shown beside the title. */
        icon: field.string(schema.string().max(32)).optional(),
        /** The page's place among its siblings, as a fractional index. */
        position: field.fractionalIndex().default("a0"),
    },
    recoverable: { within: { days: 30 }, by: "manage" },
    shareable: { isPublic: true },
});
