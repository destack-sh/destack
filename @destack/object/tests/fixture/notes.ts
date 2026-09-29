import { auditOutboxTables } from "@destack/audit/outbox";
import { schema } from "@destack/schema";
import { relation, through, union, principal } from "@destack/access";
import { defineDatabase } from "@destack/db/declare";
import { defineService } from "@destack/service";
import { defineJournal } from "@destack/service/database";
import { defineObject, field, method } from "../../src/index.ts";
import { user } from "../schema.ts";
import { space } from "./space.ts";

/** A collection of notes, shared with everyone who works on them. */
export const notebook = defineObject({
    name: "notebook",
    plural: "notebooks",
    scope: space,
    fields: {
        owner: field.reference(principal.user).caller(),
        name: field.string(schema.string().min(1).max(200)),
        noteCount: field.count(),
    },
    relations: {
        editor: { subjects: [user] },
        viewer: { subjects: [user] },
    },
    permissions: {
        read: union(relation("owner"), relation("editor"), relation("viewer")),
        edit: union(relation("owner"), relation("editor")),
        manage: relation("owner"),
    },
    shareable: { by: "manage" },
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("manage"),
        update: method.update("manage"),
        delete: method.delete("manage"),
    },
});

/** A plain-text note, private to its owner until shared. */
export const note = defineObject({
    name: "note",
    plural: "notes",
    scope: space,
    nested: {
        in: notebook,
        delete: "cascade",
        optional: true,
        receive: "edit",
        move: "manage",
    },
    fields: {
        owner: field.reference(principal.user).caller(),
        title: field.string(schema.string().max(500)).default(""),
        text: field.string(schema.string().max(100_000)).default(""),
        pinned: field.boolean().default(false),
    },
    recoverable: { within: { days: 30 }, by: "manage" },
    aggregates: { noteCount: { function: "count", where: { deletionRequestedAt: null } } },
    relations: {
        editor: { subjects: [user] },
        viewer: { subjects: [user] },
    },
    permissions: {
        read: union(
            relation("owner"),
            relation("editor"),
            relation("viewer"),
            through("parent", "read"),
        ),
        edit: union(relation("owner"), relation("editor"), through("parent", "edit")),
        manage: union(relation("owner"), through("parent", "manage")),
    },
    shareable: { by: "manage" },
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("edit"),
        update: method.update("edit"),
    },
});

/** Notebooks and notes, with their sharing, trash and sync. */
export const notesService = defineService("notes", { objects: { notebook, note } });

/** Replayable method requests. */
export const notesJournal = defineJournal("journal");

/** The database of one space's notebooks and notes. */
export const notesDatabase = defineDatabase({
    name: "main",
    tables: [...notebook.tables, ...note.tables, notesJournal, ...auditOutboxTables],
});
