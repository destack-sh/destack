import { ModuleMetadata } from "@destack/package";
import { eq, inArray } from "../../sql/index.ts";
import { Change } from "../../log/log.ts";
import { defineDatabase } from "../../declare/database.ts";
import { ResourceId, Plan } from "@destack/resource";
import { durableObjectConnector } from "../connector.ts";
import { DurableObjectDatabaseHost, DurableObjectMeasurement } from "../host.ts";
import { defineTable } from "../../table/table.ts";
import { text } from "../../table/column.ts";
import type { DurableObjectStorage } from "../client.ts";
import { connectDurableObject } from "../connection.ts";

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

/** Entries each namespaced database logs in the database's own scope. */
const entry = defineTable(
    "entry",
    { id: text("id").primaryKey(), title: text("title").notNull() },
    { log: {} },
    MODULE,
);

/** The database of entries a workload declares. */
export const entries = defineDatabase({ name: "entries", tables: [entry] }, MODULE);

/** The database resource the object keeps for a workload. */
const RECORD = {
    id: ResourceId.parse("database-01996ab0-0000-7000-8000-00000000d002"),
    scope: "space-1",
    spec: { copies: [] },
    reference: null,
};

/** The content keeper of operations that copy nothing, refusing every page. */
const UNKEPT = () => ({
    fetch: () => Promise.reject(new TypeError("the scenario keeps no content")),
});

/** A Durable Object keeping notes in its SQLite storage through a Destack connection. */
export class Notes {
    /** The object's storage. */
    readonly storage: DurableObjectStorage;

    /** Keep the object's storage. */
    constructor(state: { readonly storage: DurableObjectStorage }) {
        this.storage = state.storage;
    }

    /** Answer the scenario a request's path names. */
    async fetch(request: Request): Promise<Response> {
        const { pathname } = new URL(request.url);
        if (pathname === "/namespaces") {
            return await this.namespaces();
        } else if (pathname === "/host") {
            return await this.host();
        } else if (pathname === "/parameters") {
            return await this.parameters();
        } else if (pathname === "/joined") {
            return await this.joined();
        } else if (pathname === "/rows") {
            return await this.rows();
        }

        return await this.notes();
    }

    /** Write and read rows in statements binding more values than the storage takes at once. */
    async parameters(): Promise<Response> {
        // write 80 notes with a quote and a marker in their titles in one statement of 160 values
        const database = connectDurableObject(this.storage, [note]);
        await database.migrate([note]);
        const notes = Array.from({ length: 80 }, (_, index) => ({
            id: `n${index.toString().padStart(2, "0")}`,
            title: `it's ${index}?`,
        }));
        await database.insert(note).values(notes);

        // read them back in one statement of 80 values
        const rows = await database
            .select()
            .from(note)
            .where(
                inArray(
                    note.id,
                    notes.map((written) => written.id),
                ),
            )
            .orderBy(note.id);

        return Response.json({ isEqual: JSON.stringify(rows) === JSON.stringify(notes) });
    }

    /** Provision, plan through a sent operation, apply, open, write, measure and destroy a workload's database. */
    async host(): Promise<Response> {
        // provision the database and plan its tables as another object asks
        const host = new DurableObjectDatabaseHost(this.storage);
        const placed = await host.provision(RECORD);
        const record = { ...RECORD, reference: placed.reference };
        const desired = [entries.state()];
        const answered = await host.answer(
            new Request("https://instance.test/.destack/database", {
                method: "POST",
                body: JSON.stringify({ operation: "plan", record, desired }),
            }),
            UNKEPT,
        );
        const plan = Plan.parse(await answered.json());

        // apply the plan and write through the workload's connector
        await host.apply(record, desired, await Plan.digest(plan));
        const binding = {
            resource: RECORD.id,
            kind: "database",
            provider: host.provider,
            reference: placed.reference,
        };
        const rows = await (async () => {
            await using database = await durableObjectConnector(this.storage).connect(
                binding,
                entries,
            );
            await database.insert(entry).values({ id: "a", title: "Kept" });

            return await database.select().from(entry);
        })();

        // measure the object's databases as another object asks
        const measuring = await host.answer(
            new Request("https://instance.test/.destack/database", {
                method: "POST",
                body: JSON.stringify({ operation: "measure" }),
            }),
            UNKEPT,
        );
        const { bytes } = DurableObjectMeasurement.parse(await measuring.json());
        const isMeasured = bytes > 0 && bytes === this.storage.sql.databaseSize;

        // destroy the database with every relation of its namespace
        await host.destroy(record);
        const remaining = this.storage.sql
            .exec(
                "SELECT count(*) AS count FROM sqlite_schema WHERE substr(name, 1, ?) = ?",
                RECORD.id.length + 1,
                `${RECORD.id}.`,
            )
            .toArray();

        return Response.json({
            reference: placed.reference,
            steps: plan.steps.map((step) => step.target),
            rows,
            isMeasured,
            remaining,
        });
    }

    /** Count the rows the storage's statements write and read, each in the one measure following them. */
    async rows(): Promise<Response> {
        // migrate the notes, leaving the migration's rows to a first measure
        const host = new DurableObjectDatabaseHost(this.storage);
        const database = connectDurableObject(this.storage, [note]);
        await database.migrate([note]);
        host.measure();

        // write three notes, read them back, then measure nothing more
        await database.insert(note).values(["a", "b", "c"].map((id) => ({ id, title: id })));
        const written = host.measure();
        const rows = await database.select().from(note);
        const read = host.measure();
        const none = host.measure();
        const counts = [written, read, none].map((each) => [each.rowsRead, each.rowsWritten]);

        return Response.json({ counts, notes: rows.length });
    }

    /** Read and write another database of the storage from within a transaction of one, joining the storage's open transaction. */
    async joined(): Promise<Response> {
        // keep a note in each of two namespaces
        const [first, second] = [
            connectDurableObject(this.storage, [note], { namespace: "first" }),
            connectDurableObject(this.storage, [note], { namespace: "second" }),
        ];
        for (const database of [first, second]) {
            await database.migrate([note]);
            await database.insert(note).values({ id: "a", title: "Kept" });
        }

        // read and write the second database from within a transaction of the first
        const read = await first.transaction(async (transaction) => {
            await second.insert(note).values({ id: "b", title: "Joined" });

            return [
                await transaction.select().from(note),
                await second.select().from(note).orderBy(note.id),
            ];
        });

        return Response.json(read);
    }

    /** Keep two databases of one declaration in the storage, each in its own namespace with its own log. */
    async namespaces(): Promise<Response> {
        // migrate the same table in two namespaces, each logged in its own scope
        const databases = [
            connectDurableObject(this.storage, [entry], { namespace: "first" }),
            connectDurableObject(this.storage, [entry], { namespace: "second" }),
        ];
        for (const [index, database] of databases.entries()) {
            await database.log.create(`space-${index + 1}`);
            await database.migrate([entry]);
            await database.insert(entry).values({ id: "a", title: `Entry ${index + 1}` });
        }

        // read each database's rows, logged changes and plan
        const read = await Promise.all(
            databases.map(async (database) => ({
                rows: await database.select().from(entry),
                changes: (await database.log.read({ tables: [entry], after: 0 })).changes.map(
                    (change) => [change.operation, change.scope, Change.image(change)],
                ),
                steps: (await database.plan({ declared: [] })).steps.map((step) => step.target),
            })),
        );

        return Response.json(read);
    }

    /** Migrate, write, roll back a failed nested transaction, and read the notes back. */
    async notes(): Promise<Response> {
        // migrate the table and write a note
        const database = connectDurableObject(this.storage, [note]);
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
