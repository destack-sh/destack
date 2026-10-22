import { schema } from "@destack/schema";
import { defineDatabase } from "@destack/db";
import { definePackage } from "@destack/object";
import { defineObject, field } from "../../src/index.ts";
import { space } from "./space.ts";
import { tablesOf } from "../../src/index.ts";
import objectPackage from "../../src/package.ts";

/** A collection of notes, shared with everyone who works on them. */
export const notebook = defineObject(
    {
        name: "notebook",
        plural: "notebooks",
        scope: space,
        fields: {
            name: field.string(schema.string().min(1).max(200)),
            noteCount: field.count(),
        },
        shareable: {},
    },
    (method) => ({
        create: method.create("manage"),
        update: method.update("manage"),
        delete: method.delete("manage"),
    }),
);

/** A plain-text note, private to its owner until shared. */
export const note = defineObject({
    name: "note",
    plural: "notes",
    scope: space,
    nested: {
        in: notebook,
        delete: "cascade",
        optional: true,
        receive: "write",
        move: "manage",
    },
    fields: {
        title: field.string(schema.string().max(500)).default(""),
        text: field.string(schema.string().max(100_000)).default(""),
        starred: field.boolean().default(false),
    },
    recoverable: { within: { days: 30 }, by: "manage" },
    aggregates: { noteCount: { function: "count", where: { deletedAt: null } } },
    shareable: {},
});

/** Notebooks and notes, with their sharing, trash and sync. */
export const notesPackage = definePackage({ objects: { notebook, note } });

/** The database of one space's notebooks and notes. */
export const notesDatabase = defineDatabase({
    name: "main",
    ...tablesOf({
        packages: [objectPackage, notesPackage],
        follows: [],
        tables: [],
    }),
});
