import { testIdentity } from "../../src/test/index.ts";
import { schema } from "@destack/schema";
import { defineDatabase } from "@destack/db";
import { invitation } from "../../src/access/index.ts";
import { definePackage } from "@destack/object";
import { defineObject, field } from "../../src/index.ts";
import { space } from "../../src/test/space.ts";
import { tablesOf } from "../../src/index.ts";
import objectPackage from "../../src/package.ts";
import { nested, recoverable, shareable } from "../../src/index.ts";
import { argument, plural, t } from "@destack/locale";

/** A page in a tree of pages, shared with its subpages and publishable through links. */
export const page = defineObject({
    name: "page",
    plural: "pages",
    title: t`${plural(argument("count"), { one: "Page", other: "Pages" })}`,
    scope: space,
    fields: {
        title: field.string(schema.string().max(500)).default(""),
        icon: field.string(schema.string().max(32)).optional(),
        position: field.fractionalIndex().default("a0"),
    },
    traits: [
        nested({
            in: "self",
            optional: true,
            add: "write",
            move: "manage",
        }),
        recoverable({ within: { days: 30 }, by: "manage" }),
        shareable({ isPublic: true }),
    ],
});

/** The package of pages, their trees, sharing, links, trash and sync. */
export const pagesPackage = definePackage({
    ...testIdentity,
    objects: [page, invitation],
});

/** The database of one space's pages. */
export const pageDatabase = defineDatabase({
    name: "main",
    ...tablesOf({
        packages: [objectPackage, { objects: { page }, tables: [] }],
        follows: [],
        tables: [],
    }),
});
