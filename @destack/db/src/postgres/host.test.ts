import { TestDatabase } from "../test/database.ts";
import postgres from "postgres";
import { Plan, ResourceId } from "@destack/resource";
import { Digest, schema, Uuid } from "@destack/schema";
import { expect, onTestFinished, test, refusal } from "@destack/test";
import type { DatabaseConnection } from "../database/connection.ts";
import { defineDatabase } from "../declare/database.ts";
import { DatabaseError } from "../error/error.ts";
import { databaseProvider } from "../provider/provider.ts";
import { text } from "../table/column.ts";
import { defineTable, TABLE } from "../table/table.ts";
import { MemoryBlobStorage } from "@destack/resource";
import { connectPostgres } from "./connection.ts";
import { reportNotices } from "./pool.ts";
import { PostgresDatabaseHost } from "./host.ts";
import { testHost } from "@destack/test/host";

/** Notes with a title. */
const note = defineTable("note", { id: text("id").primaryKey(), title: text("title") });

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

/** Store databases in the test server, dropping some records' schemas after the test. */
function openHost(address: string, ids: readonly ResourceId[]): PostgresDatabaseHost {
    // close the host's pool, then drop the schemas a failed test leaves behind
    const host = new PostgresDatabaseHost(new URL(address), testHost());
    onTestFinished(async () => {
        await host.close();
        const administration = postgres(address, {
            max: 1,
            onnotice: reportNotices(testHost().telemetry),
        });
        for (const id of ids) {
            await administration.unsafe(`DROP SCHEMA IF EXISTS "${id}" CASCADE`);
        }
        await administration.end();
    });

    return host;
}

/** Describe an unprovisioned database resource of a new identifier. */
function recordOf(id: ResourceId) {
    return {
        id,
        scope: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002"),
        kind: "database" as const,
        spec: { replicated: [] },
        reference: null,
    };
}

test("provision a PostgreSQL schema twice with the same log, plan and apply it, connect a workload to it once applied, count its bytes, and destroy it", async () => {
    // run on the real host, whose identifiers stay unique across runs
    const system = testHost();

    // provide databases in the test server
    const id = ResourceId.parse(`resource-${Uuid.v7(system)}`);
    const host = openHost(TestDatabase.address(), [id]);
    const provider = databaseProvider(host, "database");
    const record = recordOf(id);
    const bound = (reference: string) => ({
        resource: record.id,
        kind: record.kind,
        provider: provider.code,
        reference,
    });

    // provision the schema twice, and refuse connecting before its tables apply
    const qualified = note[TABLE].sqlName;
    const epoch = async (schemaName: string) => {
        await using connection = await connectPostgres(host.pool, [], {
            environment: system,
            schema: schemaName,
        });

        return (await connection.log.position()).epoch;
    };
    const { reference } = await provider.provision.provision(record);
    const epochs = [await epoch(reference)];
    const again = await provider.provision.provision(record);
    epochs.push(await epoch(again.reference));
    const provisioned = { ...record, reference };
    const early = await outcome(host.connector.connect(bound(reference), notes, system));

    // refuse a plan other than the reviewed one, then apply the reviewed plan and connect
    const desired = [notes.state()];
    const plan = await provider.reconcile.plan(provisioned, desired);
    const stale = await refusal(
        provider.reconcile.apply(provisioned, desired, Digest.parse("0".repeat(64))),
    );
    await provider.reconcile.apply(provisioned, desired, await Plan.digest(plan, system.crypto));
    const applied = await outcome(host.connector.connect(bound(reference), notes, system));

    // count the bytes, then destroy the schema
    const bytes = await host.bytes(reference);
    await provider.provision.destroy(provisioned);
    const refused = await host.bytes(reference).then(
        () => "counted",
        (error: unknown) =>
            DatabaseError.is(error) && error.cause instanceof Error
                ? [error.reason, error.cause.message]
                : error,
    );

    // name the schema by the resource, and connect only between applying and destroying
    expect({
        reference,
        again: again.reference,
        isLogUnchanged: epochs[0] === epochs[1],
        early,
        steps: plan.steps,
        stale,
        applied,
        isCounted: bytes > 0,
        refused,
    }).toEqual({
        reference: id,
        again: id,
        isLogUnchanged: true,
        early: `database notes has not applied ${qualified}`,
        steps: [
            {
                action: "create",
                target: `table/${qualified}`,
                risk: "safe",
                detail: "create table",
                statements: [
                    `CREATE TABLE "${qualified}" (\n    "id" text COLLATE "C" NOT NULL,\n    "title" text COLLATE "C",\n    CONSTRAINT "${qualified}_id_pk" PRIMARY KEY ("id")\n)`,
                ],
            },
        ],
        stale: ["INTERNAL_SERVER_ERROR", "PLAN_CHANGED", `plan of ${id} changed since review`],
        applied: "connected",
        isCounted: true,
        refused: ["QUERY_FAILED", `schema "${id}" does not exist`],
    });
});

test("snapshot a PostgreSQL database's rows and restore them into another", async () => {
    // run on the real host, whose identifiers stay unique across runs
    const system = testHost();

    // provision and apply the notes on two schemas
    const ids = [
        ResourceId.parse(`resource-${Uuid.v7(system)}`),
        ResourceId.parse(`resource-${Uuid.v7(system)}`),
    ];
    const host = openHost(TestDatabase.address(), ids);
    const desired = [notes.state()];
    const [source, target] = await Promise.all(
        ids.map(async (id) => {
            const empty = recordOf(id);
            const record = { ...empty, ...(await host.provision(empty)) };
            await host.apply(
                record,
                desired,
                await Plan.digest(await host.plan(record, desired), system.crypto),
            );

            return record;
        }),
    );
    if (source === undefined || target === undefined) {
        throw new Error("the test provisioned no two databases");
    }

    // write notes into the source
    const handle = await host.open(source, desired);
    await handle.database.insert(note).values([
        { id: "n1", title: "First" },
        { id: "n2", title: "Second" },
    ]);
    await handle.close();

    // snapshot the source and restore it into the target
    const store = new MemoryBlobStorage(system.crypto);
    const digest = await host.snapshot(source, desired, store);
    await host.restore(target, desired, digest, store);
    const restored = await host.open(target, desired);
    onTestFinished(() => restored.close());
    expect(await restored.database.select().from(note).orderBy(note.id)).toEqual([
        { id: "n1", title: "First" },
        { id: "n2", title: "Second" },
    ]);
});
