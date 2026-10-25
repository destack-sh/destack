import { FileSystemError } from "@destack/environment";
import { ResourceId } from "@destack/resource";
import { fileURLToPath, pathToFileURL } from "node:url";
import { expect, onTestFinished, test } from "@destack/test";
import { Plan } from "@destack/resource";
import { present, schema } from "@destack/schema";
import { defineDatabase } from "../declare/database.ts";
import type { DatabaseConnection } from "../database/connection.ts";
import { defineTable, sql, TABLE, text } from "../index.ts";
import { bunSqliteConnector } from "./sqlite.ts";
import { databaseProvider } from "../provider/index.ts";
import { BunSqliteDatabaseHost } from "./host.ts";
import { testHost } from "@destack/test/host";

/** The host the database files live on. */
const HOST = testHost();

/** Notes with a title. */
const note = defineTable("note", { id: text("id").primaryKey(), title: text("title") });

/** A table a replica stores beside the notes. */
const beside = defineTable("beside", { id: text("id").primaryKey() });

/** The database with the notes. */
const notes = defineDatabase({ name: "notes", tables: [note] });

/** Close a connection once it opens, or read why it failed to. */
function outcome(connecting: Promise<DatabaseConnection & AsyncDisposable>): Promise<unknown> {
    return connecting.then(
        async (connection) => {
            await connection[Symbol.asyncDispose]();

            return "connected";
        },
        (error: unknown) => (error instanceof Error ? error.message : error),
    );
}

test("provision, plan and apply a SQLite database file, connect a workload to it once applied, and destroy it", async () => {
    // provide databases below a scratch directory
    const directory = await HOST.files.makeTemporaryDirectory("destack-sqlite-");
    onTestFinished(() => HOST.files.remove(directory, { recursive: true }));
    const provider = databaseProvider(
        new BunSqliteDatabaseHost(pathToFileURL(`${directory}/`), HOST),
        "database",
    );
    const record = {
        id: ResourceId.parse("resource-01996ab0-0000-7000-8000-000000000001"),
        scope: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002"),
        kind: "database" as const,
        spec: { replicated: [] },
        reference: null,
    };
    const bound = (reference: string) => ({
        resource: record.id,
        kind: record.kind,
        provider: provider.code,
        reference: present(provider.resolve, "the host's resolve")(reference),
    });

    // provision the file, and refuse connecting before its tables apply
    const qualified = note[TABLE].sqlName;
    const { reference } = await provider.provision.provision(record);
    const provisioned = { ...record, reference };
    const early = await outcome(bunSqliteConnector.connect(bound(reference), notes, HOST));

    // plan and apply the declared tables, then connect
    const desired = [notes.state()];
    const plan = await provider.reconcile.plan(provisioned, desired);
    await provider.reconcile.apply(provisioned, desired, await Plan.digest(plan, HOST.crypto));
    const applied = await outcome(bunSqliteConnector.connect(bound(reference), notes, HOST));

    // destroy the file
    await provider.provision.destroy(provisioned);
    const isGone = !(await HOST.files.exists(fileURLToPath(bound(reference).reference)));
    expect({ early, steps: plan.steps.map((step) => step.action), applied, isGone }).toEqual({
        early: `database notes has not applied ${qualified}`,
        steps: ["create"],
        applied: "connected",
        isGone: true,
    });
});

test("open a provisioned SQLite database over its desired tables, migrating tables beside them in and out", async () => {
    // provision and apply the notes on a database file
    const directory = await HOST.files.makeTemporaryDirectory("destack-sqlite-");
    onTestFinished(() => HOST.files.remove(directory, { recursive: true }));
    const provider = databaseProvider(
        new BunSqliteDatabaseHost(pathToFileURL(`${directory}/`), HOST),
        "database",
    );
    const empty = {
        id: ResourceId.parse("resource-01996ab0-0000-7000-8000-000000000003"),
        scope: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002"),
        kind: "database" as const,
        spec: { replicated: [] },
        reference: null,
    };
    const record = { ...empty, ...(await provider.provision.provision(empty)) };
    const desired = [notes.state()];
    await provider.reconcile.apply(
        record,
        desired,
        await Plan.digest(await provider.reconcile.plan(record, desired), HOST.crypto),
    );

    // migrate a table beside the notes, and write into both through the handle
    const handle = await provider.open.open(record, desired);
    onTestFinished(() => handle.close());
    await handle.migrate([beside]);
    await handle.database.insert(beside).values({ id: "a" });
    await handle.database.insert(note).values({ id: "n1", title: "First" });

    // drop the table beside once migrated without it, retaining the notes
    await handle.migrate([]);
    const tables = (
        await handle.database.execute(
            sql`SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE '\\_\\_%' ESCAPE '\\' AND name NOT LIKE 'sqlite\\_%' ESCAPE '\\' ORDER BY name`,
            schema.object({ name: schema.string() }),
        )
    ).map((row) => row.name);
    expect([tables, await handle.database.select().from(note)]).toEqual([
        [note[TABLE].sqlName],
        [{ id: "n1", title: "First" }],
    ]);
});

test("count a database's bytes as its file and its write-ahead log, and refuse a database whose file is gone", async () => {
    // provision a database below a scratch directory and write a note into it
    const directory = await HOST.files.makeTemporaryDirectory("destack-sqlite-");
    onTestFinished(() => HOST.files.remove(directory, { recursive: true }));
    const host = new BunSqliteDatabaseHost(pathToFileURL(`${directory}/`), HOST);
    const record = {
        id: ResourceId.parse("resource-01996ab0-0000-7000-8000-000000000003"),
        scope: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002"),
        kind: "database" as const,
        spec: { replicated: [] },
        reference: null,
    };
    const { reference } = await host.provision(record);
    const path = fileURLToPath(host.resolve(reference));
    const size = (file: string) =>
        HOST.files.stat(file).then(
            (status) => status.size,
            (error: unknown) =>
                FileSystemError.is(error, "NOT_FOUND") ? 0 : Promise.reject(error),
        );
    const files = async () => (await size(path)) + (await size(`${path}-wal`));

    // count the bytes, then refuse once the file is removed
    const counted = await host.bytes(reference);
    const expected = await files();
    await host.destroy({ ...record, reference });
    const refused = await host.bytes(reference).then(
        () => "counted",
        (error: unknown) => (FileSystemError.is(error, "NOT_FOUND") ? "NOT_FOUND" : error),
    );

    // count exactly the file and its log, and refuse a missing file
    expect({ isCounted: counted > 0, counted, refused }).toEqual({
        isCounted: true,
        counted: expected,
        refused: "NOT_FOUND",
    });
});

test("refer to a database below its host's root, and open it below the root it moved to", async () => {
    // provision, apply and write a database below one directory
    const before = await HOST.files.makeTemporaryDirectory("destack-sqlite-");
    const after = await HOST.files.makeTemporaryDirectory("destack-sqlite-");
    onTestFinished(() => HOST.files.remove(before, { recursive: true }));
    onTestFinished(() => HOST.files.remove(after, { recursive: true }));
    const host = new BunSqliteDatabaseHost(pathToFileURL(`${before}/`), HOST);
    const empty = {
        id: ResourceId.parse("resource-01996ab0-0000-7000-8000-000000000004"),
        scope: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002"),
        kind: "database" as const,
        spec: { replicated: [] },
        reference: null,
    };
    const record = { ...empty, ...(await host.provision(empty)) };
    const desired = [notes.state()];
    await host.apply(
        record,
        desired,
        await Plan.digest(await host.plan(record, desired), HOST.crypto),
    );
    const written = await host.open(record, desired);
    await written.database.insert(note).values({ id: "n1", title: "First" });
    await written.database.executeScript("PRAGMA wal_checkpoint(TRUNCATE)");
    await written.close();

    // move the files, then open the same reference through a host below the new root and as a workload
    await HOST.files.copy(before, after);
    const moved = new BunSqliteDatabaseHost(pathToFileURL(`${after}/`), HOST);
    const opened = await moved.open(record, desired);
    onTestFinished(() => opened.close());
    const binding = {
        resource: record.id,
        kind: record.kind,
        provider: moved.provider,
        reference: moved.resolve(record.reference),
    };
    await using workload = await bunSqliteConnector.connect(binding, notes, HOST);
    expect({
        reference: record.reference,
        resolved: binding.reference,
        hosted: await opened.database.select().from(note),
        bound: await workload.select().from(note),
    }).toEqual({
        reference: `${empty.scope}/${empty.id}.db`,
        resolved: pathToFileURL(`${after}/${empty.scope}/${empty.id}.db`).href,
        hosted: [{ id: "n1", title: "First" }],
        bound: [{ id: "n1", title: "First" }],
    });
});
