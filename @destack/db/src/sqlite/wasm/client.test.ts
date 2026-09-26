import { expect, onTestFinished, test } from "@destack/test";
import init from "@sqlite.org/sqlite-wasm";
import { migrate } from "../../migration/database.ts";
import { asc, eq } from "../../index.ts";
import { connectShared, serveDatabase, type Message } from "../../shared/shared.ts";
import { WasmClient } from "./client.ts";
import { changeTables, note } from "../../log/test/fixture.ts";
import { channelHub } from "../../test/channel.ts";

test("declare, log and query tables on SQLite WebAssembly through a channel, as a browser tab does", async () => {
    // serve an in-memory WebAssembly database to a party
    const sqlite = await init();
    const client = new WasmClient(new sqlite.oo1.DB(":memory:"));
    const join = channelHub<Message>();
    const stop = serveDatabase(client, join());
    onTestFinished(async () => {
        stop();
        await client.close();
    });
    const database = connectShared(join(), "tab-1", changeTables);
    onTestFinished(() => database.close());

    // declare the tables with their log triggers, and log a write
    await migrate(database, changeTables);
    const values = {
        id: "a",
        title: "First",
        folder: "inbox",
        summary: null,
        views: 9_007_199_254_740_993n,
        labels: ["draft"],
        editedAt: new Date("2026-09-24T10:00:00.123Z"),
        attachment: new Uint8Array([1, 2, 3]),
    };
    await database.insert(note).values(values);
    expect(await database.log.latest()).toBe(1);

    // read exact integers, dates and binary values back
    const [row] = await database.select().from(note).orderBy(asc(note.id));
    expect(row).toEqual(values);

    // roll back a failing transaction as a whole
    await expect(
        database.transaction(async (transaction) => {
            await transaction.update(note).set({ title: "Changed" }).where(eq(note.id, "a"));
            throw new Error("undo");
        }),
    ).rejects.toThrow("undo");
    expect((await database.select({ title: note.title }).from(note))[0]!.title).toBe("First");
});
