import { mkdtemp, rm, stat } from "node:fs/promises";
import { ResourceId } from "@destack/resource";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { expect, onTestFinished, test } from "@destack/test";
import { Plan } from "@destack/resource";
import { identifier } from "@destack/schema";
import { defineDatabase } from "../declare/database.ts";
import type { DatabaseConnection } from "../database/connection.ts";
import { defineTable, sql, TABLE, text } from "../index.ts";
import type { SqliteDatabase } from "./database.ts";
import { sqliteConnector } from "./connector.ts";
import { sqliteProvider } from "./provider.ts";

/** Notes with a title. */
const note = defineTable("note", { id: text("id").primaryKey(), title: text("title") });

/** A table a copy keeps beside the notes. */
const beside = defineTable("beside", { id: text("id").primaryKey() });

/** The database holding the notes. */
const notes = defineDatabase({ name: "notes", tables: [note] });

test("provision, plan and apply a SQLite database file, connect a workload to it once applied, and destroy it", async () => {
    // provide databases below a scratch directory
    const directory = await mkdtemp(join(tmpdir(), "destack-sqlite-"));
    onTestFinished(() => rm(directory, { recursive: true }));
    const provider = sqliteProvider(pathToFileURL(`${directory}/`), "database");
    const record = {
        id: ResourceId.parse("resource-01996ab0-0000-7000-8000-000000000001"),
        scope: identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002"),
        kind: "database" as const,
        spec: { tier: "zonal" as const },
        reference: null,
    };
    const bound = (reference: string) => ({
        resource: record.id,
        kind: record.kind,
        provider: provider.code,
        reference,
    });
    const outcome = (connecting: Promise<DatabaseConnection>) =>
        connecting.then(
            async (connection) => {
                await (connection as SqliteDatabase).close();

                return "connected";
            },
            (error: Error) => error.message,
        );

    // provision the file, and refuse connecting before its tables apply
    const qualified = note[TABLE].sqlName;
    const { reference } = await provider.provision(record);
    const provisioned = { ...record, reference };
    const early = await outcome(sqliteConnector.connect(bound(reference), notes));

    // plan and apply the declared tables, then connect
    const desired = [notes.state()];
    const plan = await provider.plan(provisioned, desired);
    await provider.apply(provisioned, desired, await Plan.digest(plan));
    const applied = await outcome(sqliteConnector.connect(bound(reference), notes));

    // destroy the file
    await provider.destroy(provisioned);
    const isGone = await stat(fileURLToPath(reference)).then(
        () => false,
        () => true,
    );
    expect({ early, steps: plan.steps.map((step) => step.action), applied, isGone }).toEqual({
        early: `database notes has not applied ${qualified}`,
        steps: ["create"],
        applied: "connected",
        isGone: true,
    });
});

test("open a provisioned SQLite database over its desired tables, migrating tables beside them in and out", async () => {
    // provision and apply the notes on a database file
    const directory = await mkdtemp(join(tmpdir(), "destack-sqlite-"));
    onTestFinished(() => rm(directory, { recursive: true }));
    const provider = sqliteProvider(pathToFileURL(`${directory}/`), "database");
    const empty = {
        id: ResourceId.parse("resource-01996ab0-0000-7000-8000-000000000003"),
        scope: identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002"),
        kind: "database" as const,
        spec: { tier: "zonal" as const },
        reference: null,
    };
    const record = { ...empty, ...(await provider.provision(empty)) };
    const desired = [notes.state()];
    await provider.apply(record, desired, await Plan.digest(await provider.plan(record, desired)));

    // migrate a table beside the notes, and write into both through the handle
    const handle = await provider.open(record, desired);
    onTestFinished(() => handle.close());
    await handle.migrate([beside]);
    await handle.database.insert(beside).values({ id: "a" });
    const notesTable = handle.database.tables.find(
        (table) => table[TABLE].sqlName === note[TABLE].sqlName,
    ) as typeof note;
    await handle.database.insert(notesTable).values({ id: "n1", title: "First" });

    // drop the table beside once migrated without it, keeping the notes
    await handle.migrate([]);
    const tables = (
        await handle.database.execute<{ name: string }>(
            sql`SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE '\\_\\_%' ESCAPE '\\' AND name NOT LIKE 'sqlite\\_%' ESCAPE '\\' ORDER BY name`,
        )
    ).map((row) => row.name);
    expect([tables, await handle.database.select().from(notesTable)]).toEqual([
        [note[TABLE].sqlName],
        [{ id: "n1", title: "First" }],
    ]);
});
