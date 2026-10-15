import { ModuleMetadata } from "@destack/package";
import { and, eq, inArray } from "../../sql/index.ts";
import { Change } from "../../log/log.ts";
import { defineDatabase } from "../../declare/database.ts";
import { ResourceId, Plan } from "@destack/resource";
import { durableObjectConnector } from "../connector.ts";
import { DATABASE_PATH, DurableObjectDatabaseHost, DurableObjectMeasurement } from "../host.ts";
import { defineTable, TABLE } from "../../table/table.ts";
import { bigint, binary, text } from "../../table/column.ts";
import type { DurableObjectStorage } from "../client.ts";
import { DurableObjectReader } from "../reader.ts";
import { Snapshot } from "../../log/snapshot.ts";

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

/** Files the reader reads by their integer size and their bytes. */
const file = defineTable(
    "file",
    {
        id: text("id").primaryKey(),
        size: bigint("size").notNull(),
        content: binary("content").notNull(),
    },
    {},
    MODULE,
);

/** The size of the file the reader reads, the largest integer a double keeps exact. */
const FILE_SIZE = 2n ** 53n - 1n;

/** A file size past the integers a number keeps exact, 2^62. */
const LARGE_SIZE = 2n ** 62n;

/** The bytes of the file the reader reads. */
const FILE_CONTENT = new Uint8Array([0, 1, 254, 255]);

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
    spec: { replicated: [] },
    reference: null,
};

/** The content keeper of operations that copy nothing, refusing every page. */
const UNKEPT = () => ({
    fetch: () => Promise.reject(new TypeError("the scenario has no content")),
});

/** A Durable Object keeping notes in its SQLite storage through a Destack connection. */
export class Notes {
    /** The databases the object keeps in its storage. */
    readonly databases: DurableObjectDatabaseHost;

    /** Keep the object's databases in its storage. */
    constructor(state: { readonly storage: DurableObjectStorage }) {
        this.databases = new DurableObjectDatabaseHost(state.storage);
    }

    /** Answer the scenario in a request's path. */
    async fetch(request: Request): Promise<Response> {
        const { pathname } = new URL(request.url);
        if (pathname === "/namespaces") {
            return await this.namespaces();
        } else if (pathname === "/host") {
            return await this.host();
        } else if (pathname === "/parameters") {
            return await this.parameters();
        } else if (pathname === "/leaked") {
            return await this.leaked();
        } else if (pathname === "/rows") {
            return await this.rows();
        } else if (pathname === "/reader") {
            return await this.reader();
        } else if (pathname === DATABASE_PATH) {
            return await this.databases.read(request);
        }

        return await this.notes();
    }

    /** Write in a transaction, then refuse its client kept past it while another runs. */
    async leaked(): Promise<Response> {
        // keep the client of a committed transaction
        const database = this.databases.connect([note]);
        await database.migrate([note]);
        const kept = await database.transaction(async (transaction) => {
            await transaction.insert(note).values({ id: "a", title: "Kept" });

            return transaction;
        });

        // refuse the kept client while a later transaction is open, which keeps only its own write
        const refused = await database.transaction(async (transaction) => {
            await transaction.insert(note).values({ id: "b", title: "Later" });

            return kept
                .insert(note)
                .values({ id: "c", title: "Leaked" })
                .then(
                    () => "written",
                    (error: unknown) => (error instanceof Error ? error.message : String(error)),
                );
        });
        const rows = await database.select().from(note).orderBy(note.id);

        return Response.json({ refused, rows });
    }

    /** Write rows binding more values or larger integers than the storage binds. */
    async parameters(): Promise<Response> {
        // write 80 notes with a quote and a marker in their titles in one statement of 160 values
        const database = this.databases.connect([note, file]);
        await database.migrate([note, file]);
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

        // write a file whose size a number cannot keep exact, and read the stored integer as text
        await database.insert(file).values({ id: "g", size: LARGE_SIZE, content: FILE_CONTENT });
        const [stored] = this.databases.storage.sql
            .exec(`SELECT CAST(size AS TEXT) AS size FROM "${file[TABLE].sqlName}"`)
            .toArray();

        return Response.json({ isEqual: JSON.stringify(rows) === JSON.stringify(notes), stored });
    }

    /** Provision, plan, apply, open, write, measure and destroy a workload database. */
    async host(): Promise<Response> {
        // provision the database and plan its tables as another object asks
        const placed = await this.databases.provision(RECORD);
        const record = { ...RECORD, reference: placed.reference };
        const desired = [entries.state()];
        const answered = await this.databases.answer(
            new Request("https://instance.test/.destack/database", {
                method: "POST",
                body: JSON.stringify({ operation: "plan", record, desired }),
            }),
            UNKEPT,
        );
        const plan = Plan.parse(await answered.json());

        // apply the plan and write through the workload's connector
        await this.databases.apply(record, desired, await Plan.digest(plan));
        const binding = {
            resource: RECORD.id,
            kind: "database",
            provider: this.databases.provider,
            reference: placed.reference,
        };
        const rows = await (async () => {
            await using database = await durableObjectConnector(this.databases).connect(
                binding,
                entries,
            );
            await database.insert(entry).values({ id: "a", title: "Kept" });

            return await database.select().from(entry);
        })();

        // measure the object's databases as another object asks
        const measuring = await this.databases.answer(
            new Request("https://instance.test/.destack/database", {
                method: "POST",
                body: JSON.stringify({ operation: "measure" }),
            }),
            UNKEPT,
        );
        const { bytes } = DurableObjectMeasurement.parse(await measuring.json());
        const isMeasured = bytes > 0 && bytes === this.databases.storage.sql.databaseSize;

        // destroy the database with every relation of its namespace
        await this.databases.destroy(record);
        const remaining = this.databases.storage.sql
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

    /** Count the rows the storage's statements write and read, each in the next measure. */
    async rows(): Promise<Response> {
        // migrate the notes, leaving the migration's rows to a first measure
        const database = this.databases.connect([note]);
        await database.migrate([note]);
        this.databases.measure();

        // write three notes, read them back, then measure nothing more
        await database.insert(note).values(["a", "b", "c"].map((id) => ({ id, title: id })));
        const written = this.databases.measure();
        const rows = await database.select().from(note);
        const read = this.databases.measure();
        const none = this.databases.measure();
        const counts = [written, read, none].map((each) => [each.rowsRead, each.rowsWritten]);

        return Response.json({ counts, notes: rows.length });
    }

    /** Read notes and a file through a reader of this object's database, refusing a write. */
    async reader(): Promise<Response> {
        // keep two notes and a file
        const database = this.databases.connect([note, file]);
        await database.migrate([note, file]);
        await database.insert(note).values(["a", "b"].map((id) => ({ id, title: `Note ${id}` })));
        await database.insert(file).values({ id: "f", size: FILE_SIZE, content: FILE_CONTENT });

        // read a note and the file by its size and bytes through the reader
        const reader = DurableObjectReader.connect({ fetch: (sent) => this.fetch(sent) }, [
            note,
            file,
        ]);
        const found = await Snapshot.live(reader)
            .read(note)
            .findFirst({ where: { id: "b" } });
        const files = await reader
            .select()
            .from(file)
            .where(and(eq(file.size, FILE_SIZE), eq(file.content, FILE_CONTENT)));

        // try to delete the notes through it
        const deleted = await new DurableObjectReader({ fetch: (sent) => this.fetch(sent) })
            .values(`DELETE FROM "${note[TABLE].sqlName}" RETURNING id`, [])
            .then(
                () => "deleted",
                (error: unknown) => (error instanceof Error ? error.message : String(error)),
            );
        const kept = await database.select().from(note).orderBy(note.id);

        // refuse any other operation sent where reads arrive
        const measured = await this.fetch(
            new Request(`https://object.test${DATABASE_PATH}`, {
                method: "POST",
                body: JSON.stringify({ operation: "measure" }),
            }),
        ).then(
            () => "measured",
            (error: unknown) => (error instanceof Error ? error.message : String(error)),
        );

        return Response.json({
            found,
            measured,
            files: files.map((row) => [row.id, row.size.toString(), row.content.toHex()]),
            deleted,
            kept: kept.map((row) => row.id),
        });
    }

    /** Keep two databases of one declaration in the storage, each with namespace and log. */
    async namespaces(): Promise<Response> {
        // migrate the same table in two namespaces, each logged in its own scope
        const databases = [
            this.databases.connect([entry], { namespace: "first" }),
            this.databases.connect([entry], { namespace: "second" }),
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
        const database = this.databases.connect([note]);
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
                readonly idFromName: (name: string) => unknown;
                readonly get: (id: unknown) => {
                    readonly fetch: (request: Request) => Promise<Response>;
                };
            };
        },
    ) {
        return environment.NOTES.get(environment.NOTES.idFromName("notes")).fetch(request);
    },
};
