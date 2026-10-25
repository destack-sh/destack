import { users } from "./fixture/user.ts";
import { expect, refusal, test } from "@destack/test";
import { serveNotes, spaceId } from "./fixture/device.ts";
import { notebook } from "./fixture/note.ts";

test("read through methods that change nothing on the server, and refuse mutations there", async () => {
    const { connect, openDevice } = await serveNotes();
    const alice = connect(users.alice);
    const device = await openDevice(users.alice);
    device.online();

    // read a confirmed notebook as the server has it
    const created = device.client.mutate(notebook).create({ name: "Travel" });
    const { id } = await created.predicted;
    await created.confirmed;
    const read = await device.client.read(notebook).get({ id });

    // refuse a mutation sent as a read
    const refused = alice.store.call({
        scope: spaceId,
        call: {
            type: notebook.typeReference,
            method: "create",
            release: notebook.package.version,
            input: { spaceId, name: "Ideas" },
        },
    });
    expect(await refusal(refused)).toEqual([
        "BAD_REQUEST",
        "PUSH_REQUIRED",
        "notebook.create changes objects: push it from a client",
    ]);
    expect([read.name, device.errors]).toEqual(["Travel", []]);
});
