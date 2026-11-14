import { testIdentity } from "./index.ts";
import { schema } from "@destack/schema";
import { defineDatabase } from "@destack/db";
import { defineExample } from "@destack/package/declare";
import { PackageVersion } from "@destack/package";
import {
    defineObject,
    definePackage,
    field,
    nested,
    recoverable,
    shareable,
    tablesOf,
} from "../index.ts";
import { space } from "./space.ts";
import objectPackage from "../package.ts";
import { argument, plural, t } from "@destack/locale";

/** The notes package the fixture spaces serve beside the object package. */
const NOTES = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000a7",
    name: "@example/notes",
    version: "2026.9.0",
});

/** A collection of notes, shared with everyone who works on them. */
export const notebook = defineObject(
    {
        name: "notebook",
        plural: "notebooks",
        title: t`${plural(argument("count"), { one: "Notebook", other: "Notebooks" })}`,
        scope: space,
        fields: {
            name: field.string(schema.string().min(1).max(200)),
            noteCount: field.count(),
        },
        traits: [shareable({})],
    },
    (method) => ({
        create: method.create("manage"),
        update: method.update("manage"),
        delete: method.delete("manage"),
    }),
    { package: NOTES },
);

/** A plain-text note, private to its owner until shared. */
export const note = defineObject(
    {
        name: "note",
        plural: "notes",
        title: t`${plural(argument("count"), { one: "Note", other: "Notes" })}`,
        scope: space,
        fields: {
            title: field.string(schema.string().max(500)).default(""),
            text: field.string(schema.string().max(100_000)).default(""),
            starred: field.boolean().default(false),
        },
        aggregates: { noteCount: { function: "count", where: { deletedAt: null } } },
        traits: [
            nested({
                in: notebook,
                delete: "cascade",
                optional: true,
                add: "write",
                move: "manage",
            }),
            recoverable({ within: { days: 30 }, by: "manage" }),
            shareable({}),
        ],
    },
    undefined,
    { package: NOTES },
);

/** Notebooks and notes, with their sharing, trash and sync. */
export const notesPackage = definePackage(
    {
        ...testIdentity,
        objects: [notebook, note],
    },
    { package: NOTES },
);

/** The database of one space's notebooks and notes. */
export const notesDatabase = defineDatabase({
    name: "main",
    ...tablesOf({
        packages: [objectPackage, notesPackage],
        follows: [],
        tables: [],
    }),
});

/** The notes package served on its own, as object scenarios are given it. */
export const notesExample = defineExample({
    of: notesPackage,
    name: "notes",
    description: "notebooks and notes, with their sharing, trash and sync",
    render: () => notesPackage,
});
