import { expect, test } from "@destack/test";
import { Device, serveNotes, spaceId } from "./fixture/device.ts";
import { notebook } from "./fixture/notes.ts";

test("predict recorded calls as one mutation, which the server executes together", async () => {
    const { connect, endpoint } = await serveNotes("sqlite");
    const alice = connect("alice");
    const device = await Device.open("alice", endpoint("alice"));
    device.online();
    const book = device.client.mutate(notebook).create({ name: "Travel" });
    const { id: parentId } = await book.predicted;
    await book.confirmed;

    // submit two recorded note creations
    const submitted = device.client.submit([
        { method: "note.create", input: { spaceId, parentId, title: "Packing" } },
        { method: "note.create", input: { spaceId, parentId, title: "Tickets" } },
    ]);
    await submitted.predicted;
    const predicted = await device.titles();
    await submitted.confirmed;
    const stored = (await alice.note.list({ spaceId })).items.map((item) => item.title).sort();
    expect([predicted, stored, device.errors]).toEqual([
        ["Packing", "Tickets"],
        ["Packing", "Tickets"],
        [],
    ]);
});
