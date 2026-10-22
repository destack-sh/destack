import { schema } from "@destack/schema";
import { defineDatabase } from "@destack/db";
import { definePackage } from "@destack/object";
import { defineObject, field } from "../../src/index.ts";
import { space } from "./space.ts";
import { tablesOf } from "../../src/index.ts";
import objectPackage from "../../src/package.ts";

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
        title: field.string(schema.string().max(500)).default(""),
        icon: field.string(schema.string().max(32)).optional(),
        position: field.fractionalIndex().default("a0"),
    },
    recoverable: { within: { days: 30 }, by: "manage" },
    shareable: { isPublic: true },
});

/** The package of pages, their trees, sharing, links, trash and sync. */
export const pagesPackage = definePackage({ objects: { page } });

/** The database of one space's pages. */
export const pageDatabase = defineDatabase({
    name: "main",
    ...tablesOf({
        packages: [objectPackage, { objects: { page }, tables: [] }],
        follows: [],
        tables: [],
    }),
});
