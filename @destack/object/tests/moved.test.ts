import { expect, onTestFinished, test } from "@destack/test";
import { Scope } from "@destack/sync";
import { principal } from "@destack/access";
import { TestDatabase } from "@destack/db/test";
import { ObjectClient } from "../src/client/index.ts";
import { serveNotes, spaceId } from "./fixture/device.ts";
import { note, notebook } from "./fixture/notes.ts";

test("send a client's writes on to the cell serving a moved scope", async () => {
    // serve the space on its source and on the cell it moves to, and fence the source
    const source = await serveNotes("sqlite");
    const target = await serveNotes("sqlite");
    await Scope.fence(source.database, spaceId, "host-b", Date.now());

    // push through the source, reconnecting to the cell its refusal refers to
    const storage = await TestDatabase.create("sqlite", ObjectClient.tables([notebook, note]));
    onTestFinished(() => storage.close());
    const cells: string[] = [];
    const client = await ObjectClient.open({
        database: storage.database,
        objects: [notebook, note],
        scope: spaceId,
        caller: principal.user.reference("universe", "alice"),
        service: source.connect("alice").replica,
        reconnect: (cell) => {
            cells.push(cell);

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

    // confirm the note on the new cell alone, without a reported failure
    await client.mutate(note).create({ title: "Ideas" }).confirmed;
    const titles = async (database: typeof source.database) =>
        (await database.select({ title: note.table.title }).from(note.table)).map(
            (row) => row.title,
        );
    expect([cells, errors, await titles(source.database), await titles(target.database)]).toEqual([
        ["host-b"],
        [],
        [],
        ["Ideas"],
    ]);
});
