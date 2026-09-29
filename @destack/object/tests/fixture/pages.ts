import { auditOutboxTables } from "@destack/audit/outbox";
import { schema } from "@destack/schema";
import { anyone, permission, relation, through, union, principal } from "@destack/access";
import { defineDatabase } from "@destack/db/declare";
import { defineService } from "@destack/service";
import { defineJournal } from "@destack/service/database";
import { defineObject, field, method } from "../../src/index.ts";
import { user } from "../schema.ts";
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
        owner: field.reference(principal.user).caller(),
        title: field.string(schema.string().max(500)).default(""),
        icon: field.string(schema.string().max(32)).optional(),
        position: field.position().default("a0"),
    },
    recoverable: { within: { days: 30 }, by: "manage" },
    relations: {
        editor: { subjects: [user] },
        viewer: { subjects: [user, anyone.all()] },
    },
    permissions: {
        "read-direct": union(relation("owner"), relation("editor"), relation("viewer")),
        "edit-direct": union(relation("owner"), relation("editor")),
        "manage-direct": relation("owner"),
        read: union(permission("read-direct"), through("parent", "read-direct", true)),
        edit: union(permission("edit-direct"), through("parent", "edit-direct", true)),
        manage: union(permission("manage-direct"), through("parent", "manage-direct", true)),
    },
    shareable: { by: "manage" },
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("edit"),
        update: method.update("edit"),
    },
});

/** Pages, their trees, sharing, links, trash and sync. */
export const pagesService = defineService("pages", { objects: { page } });

/** Replayable method requests. */
export const pagesJournal = defineJournal("journal");

/** The database of one space's pages. */
export const pageDatabase = defineDatabase({
    name: "main",
    tables: [...page.tables, pagesJournal, ...auditOutboxTables],
});
