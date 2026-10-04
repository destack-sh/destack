import { schema } from "@destack/schema";
import { journal } from "@destack/audit";
import { defineDatabase } from "@destack/db";
import { defineService } from "@destack/service";
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

/** Pages, their trees, sharing, links, trash and sync. */
export const pagesService = defineService("pages", { objects: { page } });

/** The database of one space's pages. */
export const pageDatabase = defineDatabase({
    name: "main",
    tables: [...page.tables, journal],
});
