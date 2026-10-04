import { expect, test } from "@destack/test";
import { Device, serveNotes, spaceId } from "./fixture/device.ts";
import { notebook } from "./fixture/note.ts";

test("read through methods that change nothing on the server, and refuse mutations there", async () => {
    const { connect, endpoint } = await serveNotes("sqlite");
    const alice = connect("alice");
    const device = await Device.open("alice", endpoint("alice"));
    device.online();

    // read a confirmed notebook as the server has it
    const created = device.client.mutate(notebook).create({ name: "Travel" });
    const { id } = await created.predicted;
    await created.confirmed;
    const read = await device.client.read(notebook).get({ id });

    // refuse a mutation sent as a read
    const refused = alice.replica.call({
        scope: spaceId,
        call: {
            method: "notebook.create",
            release: notebook.package.version,
            input: { spaceId, name: "Ideas" },
        },
    });
    await expect(refused).rejects.toMatchObject({
        code: "BAD_REQUEST",
        message: "notebook.create changes objects, so a client pushes it",
    });
    expect([read.name, device.errors]).toEqual(["Travel", []]);
});
