import { ModuleMetadata } from "@destack/package";
import { eq } from "../../../sql/index.ts";
import { defineTable } from "../../../table/table.ts";
import { text } from "../../../table/column.ts";
import type { DurableObjectStorage } from "../client.ts";
import { connect } from "../connection.ts";

/** The module the scenario's table belongs to. */
const MODULE = ModuleMetadata.parse({
    package: {
        id: "package-01996ab0-0000-7000-8000-00000000d001",
        name: "@destack/durable-scenario",
        version: "2026.9.0",
    },
});

/** Notes the scenario writes. */
const note = defineTable(
    "note",
    { id: text("id").primaryKey(), title: text("title").notNull() },
    {},
    MODULE,
);

/** A Durable Object keeping notes in its SQLite storage through a Destack connection. */
export class Notes {
    /** The object's storage. */
    readonly storage: DurableObjectStorage;

    /** Keep the object's storage. */
    constructor(state: { readonly storage: DurableObjectStorage }) {
        this.storage = state.storage;
    }

    /** Migrate, write, roll back a failed nested transaction, and read the notes back. */
    async fetch(): Promise<Response> {
        // migrate the table and write a note
        const database = connect(this.storage, [note]);
        await database.migrate([note]);
        await database.insert(note).values({ id: "a", title: "Kept" });

        // keep the outer write when a nested transaction fails
        await database.transaction(async (outer) => {
            await outer.insert(note).values({ id: "b", title: "Outer" });
            await outer
                .transaction(async (inner) => {
                    await inner.insert(note).values({ id: "c", title: "Undone" });
                    throw new Error("undo the inner write");
                })
                .catch(() => undefined);
        });
        await database.update(note).set({ title: "Changed" }).where(eq(note.id, "a"));

        // read every note
        const rows = await database.select().from(note).orderBy(note.id);

        return Response.json(rows);
    }
}

/** The Worker routing every request to the one notes object. */
export default {
    /** Forward each request to the one notes object. */
    fetch(
        request: Request,
        environment: {
            readonly NOTES: {
                idFromName(name: string): unknown;
                get(id: unknown): { fetch(request: Request): Promise<Response> };
            };
        },
    ) {
        return environment.NOTES.get(environment.NOTES.idFromName("notes")).fetch(request);
    },
};
