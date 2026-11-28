import { expect, onTestFinished, testOn, vi } from "@destack/test";
import init from "@sqlite.org/sqlite-wasm";
import { asc, eq } from "../index.ts";
import { WasmClient } from "./client.ts";
import { connectWasmSqlite } from "./connection.ts";
import { changeTables, note } from "../log/test/fixture.ts";
import { testSimulation } from "@destack/test/world";

/** Test against SQLite alone, as the file tests its own connector or host. */
const onSqlite = testOn("sqlite");

onSqlite(
    "declare, log and query tables on SQLite WebAssembly, as a browser's database does",
    async () => {
        // run in a world of its own
        const world = testSimulation().environment("test");

        // connect to an in-memory database
        const sqlite = await init();
        const client = new WasmClient(new sqlite.oo1.DB(":memory:"), world);
        const database = connectWasmSqlite(client, changeTables, {
            name: "memory",
            environment: world,
        });
        onTestFinished(() => database.close());

        // migrate the tables and log a write
        await database.migrate(changeTables);
        const values = {
            id: "a",
            title: "First",
            scope: "inbox",
            subtitle: null,
            views: 9_007_199_254_740_993n,
            labels: ["draft"],
            editedAt: 1790244000123,
            attachment: new Uint8Array([1, 2, 3]),
        };
        await database.transaction((transaction) =>
            transaction.insert(note).values(values).execute(),
        );
        expect((await database.log.position()).sequence).toBe(1);

        // read exact values back
        const [row] = await database.select().from(note).orderBy(asc(note.id));
        expect(row).toEqual(values);

        // roll back a failing transaction
        await expect(
            database.transaction(async (transaction) => {
                await transaction.update(note).set({ title: "Changed" }).where(eq(note.id, "a"));
                throw new Error("undo");
            }),
        ).rejects.toThrow("undo");
        expect(await database.select({ title: note.title }).from(note)).toEqual([
            { title: "First" },
        ]);
    },
);

onSqlite(
    "prepare each statement text of a browser database once, reading its rows each run",
    async () => {
        // follow the statements a browser database prepares
        const world = testSimulation().environment("test");
        const sqlite = await init();
        const database = new sqlite.oo1.DB(":memory:");
        const prepare = vi.spyOn(database, "prepare");
        const client = new WasmClient(database, world);
        onTestFinished(() => client.close());

        // read the same text twice with other values
        const first = await client.values("SELECT ? + 1", [1]);
        const second = await client.values("SELECT ? + 1", [41]);

        // expect both reads answered by one prepared statement
        expect({ first, second, prepared: prepare.mock.calls.length }).toEqual({
            first: [[2]],
            second: [[42]],
            prepared: 1,
        });
    },
);

onSqlite("checkpoint a browser database once its commits rest, never inside a commit", async () => {
    // follow the statements a browser database runs, as its client sends them
    const simulation = testSimulation();
    const world = simulation.environment("test");
    const sqlite = await init();
    const database = new sqlite.oo1.DB(":memory:");
    const exec = vi.spyOn(database, "exec");
    const client = new WasmClient(database, world);
    onTestFinished(() => client.close());
    const checkpoints = () =>
        exec.mock.calls
            .map(([statement]): unknown => statement)
            .filter(
                (statement) => typeof statement === "string" && statement.startsWith("PRAGMA wal"),
            );

    // commit twice in a burst, then let the commits rest
    const commit = () =>
        client.transactionAsync(async (query) => await query.run("SELECT 1", [])).immediate();
    await commit();
    await commit();
    const during = checkpoints().length;
    await simulation.jump(world.clock.now() + 250);

    // expect one checkpoint after the burst, and none in it
    expect({ during, after: checkpoints() }).toEqual({
        during: 0,
        after: ["PRAGMA wal_checkpoint(PASSIVE)"],
    });
});
