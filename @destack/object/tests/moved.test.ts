import { Transaction } from "@destack/db";
import { expect, onTestFinished, test } from "@destack/test";
import { Scope } from "@destack/sync";
import { principal } from "@destack/access";
import type { DatabaseConnection } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Identifier, Clock } from "@destack/schema";
import { ObjectClient } from "../src/client/index.ts";
import { serveNotes, spaceId } from "./fixture/device.ts";
import { note, notebook } from "./fixture/note.ts";

test("send a client's writes on to the machine serving a moved scope", async () => {
    // serve the space on its source and on the machine it moves to, and fence the source
    const source = await serveNotes("sqlite");
    const target = await serveNotes("sqlite");
    const moved = Identifier.create("machine");
    await Transaction.run(source.database, (transaction) =>
        Scope.fence(transaction, spaceId, moved, Date.now()),
    );

    // push through the source, reconnecting to the machine its refusal refers to
    const storage = await TestDatabase.create("sqlite", ObjectClient.tables({ notebook, note }));
    onTestFinished(() => storage.close());
    const machines: string[] = [];
    const client = await ObjectClient.open({
        clock: Clock.system,
        database: storage.database,
        objects: { notebook, note },
        scope: spaceId,
        caller: principal.user.reference("universe", "alice"),
        endpoint: source.endpoint("alice"),
        reconnect: (machine) => {
            machines.push(machine);

            return target.endpoint("alice");
        },
    });
    const stopping = new AbortController();
    const errors: unknown[] = [];
    const running = client.run(stopping.signal, (error) => errors.push(error));
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    // confirm the note on the new machine alone, without a reported failure
    await client.mutate(note).create({ title: "Ideas" }).confirmed;
    expect([
        machines,
        errors,
        await titles(source.database),
        await titles(target.database),
    ]).toEqual([[moved], [], [], ["Ideas"]]);
});

/** Read the titles of the notes a database keeps. */
async function titles(database: DatabaseConnection): Promise<string[]> {
    const rows = await database.select({ title: note.table.title }).from(note.table);

    return rows.map((row) => row.title);
}
