import { callTables } from "../../src/test/index.ts";
import { schema } from "@destack/schema";
import { defineDatabase } from "@destack/db";
import { definePackage } from "@destack/runtime";
import { defineObject, field } from "../../src/index.ts";
import { space } from "./space.ts";

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
        title: field.string(schema.string().max(500)).default(""),
        icon: field.string(schema.string().max(32)).optional(),
        position: field.fractionalIndex().default("a0"),
    },
    recoverable: { within: { days: 30 }, by: "manage" },
    shareable: { isPublic: true },
}).methods((method) => ({
    create: method.create("edit"),
    update: method.update("edit"),
}));

/** The package of pages, their trees, sharing, links, trash and sync. */
export const pagesPackage = definePackage({ objects: { page } });

/** The database of one space's pages. */
export const pageDatabase = defineDatabase({
    name: "main",
    tables: [...page.tables, ...callTables],
});
