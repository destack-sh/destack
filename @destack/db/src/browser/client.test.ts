import { expect, onTestFinished, test } from "@destack/test";
import init from "@sqlite.org/sqlite-wasm";
import { asc, eq } from "../index.ts";
import { WasmClient } from "./client.ts";
import { connectWasmSqlite } from "./connection.ts";
import { changeTables, note } from "../log/test/fixture.ts";
import { testSimulation } from "@destack/test/world";

test("declare, log and query tables on SQLite WebAssembly, as a browser's database does", async () => {
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
    await database.transaction((transaction) => transaction.insert(note).values(values).execute());
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
    expect(await database.select({ title: note.title }).from(note)).toEqual([{ title: "First" }]);
});
