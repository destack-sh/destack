import { expect, onTestFinished, test } from "@destack/test";
import { principal, Scope } from "@destack/access";
import { TestDatabase } from "@destack/db/test";
import { ObjectClient } from "../src/client/index.ts";
import { serveNotes, spaceId } from "./fixture/device.ts";
import { note, notebook } from "./fixture/notes.ts";

test("send a client's writes on to the holder a moved scope names", async () => {
    // serve the space on its source and on the holder it moves to, and fence the source
    const source = await serveNotes("sqlite");
    const target = await serveNotes("sqlite");
    await Scope.fence(source.database, spaceId, "host-b", Date.now());

    // push through the source, reconnecting to the holder its refusal names
    const storage = await TestDatabase.create("sqlite", ObjectClient.tables([notebook, note]));
    onTestFinished(() => storage.close());
    const holders: string[] = [];
    const client = await ObjectClient.open({
        database: storage.database,
        objects: [notebook, note],
        scope: spaceId,
        caller: principal.user.reference("global", "alice"),
        service: source.connect("alice").replica,
        reconnect: (holder) => {
            holders.push(holder);

            return target.connect("alice").replica;
        },
    });
    const stopping = new AbortController();
    const errors: unknown[] = [];
    const running = client.run(stopping.signal, (error) => errors.push(error));
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    // confirm the note on the new holder alone, without a reported failure
    await client.mutate(note).create({ title: "Ideas" }).confirmed;
    const titles = async (database: typeof source.database) =>
        (await database.select({ title: note.table.title }).from(note.table)).map(
            (row) => row.title,
        );
    expect([holders, errors, await titles(source.database), await titles(target.database)]).toEqual(
        [["host-b"], [], [], ["Ideas"]],
    );
});
