import { users } from "./fixture/user.ts";
import { expect, test } from "@destack/test";
import { serveNotes, spaceId } from "./fixture/device.ts";
import { note, notebook } from "./fixture/note.ts";

test("predict recorded calls as one mutation, which the server executes together", async () => {
    const { connect, openDevice } = await serveNotes();
    const alice = connect(users.alice);
    const device = await openDevice(users.alice);
    device.online();
    const book = device.client.mutate(notebook).create({ name: "Travel" });
    const { id: parentId } = await book.predicted;
    await book.confirmed;

    // submit two recorded note creations
    const submitted = device.client.submit([
        {
            type: note.typeReference,
            method: "create",
            release: note.package.version,
            input: { spaceId, parentId, title: "Packing" },
        },
        {
            type: note.typeReference,
            method: "create",
            release: note.package.version,
            input: { spaceId, parentId, title: "Tickets" },
        },
    ]);
    await submitted.predicted;
    const predicted = await device.titles();
    await submitted.confirmed;
    const stored = (await alice.note.list({ spaceId })).items.map((item) => item.title).toSorted();
    expect([predicted, stored, device.errors]).toEqual([
        ["Packing", "Tickets"],
        ["Packing", "Tickets"],
        [],
    ]);
});
