import { expect, test } from "@destack/test";
import { PUSH_MUTATIONS } from "../src/replica/replica.ts";
import { Device, serveNotes, spaceId } from "./fixture/device.ts";
import { note, notebook } from "./fixture/notes.ts";

test("push an outbox longer than one push carries, confirming every mutation across pushes", async () => {
    const { connect } = await serveNotes("sqlite");
    const alice = connect("alice");
    const device = await Device.open("alice", alice);

    // queue one mutation more than a push carries while offline
    const book = device.client.mutate(notebook).create({ name: "Travel" });
    const { id } = await book.predicted;
    const notes = Array.from({ length: PUSH_MUTATIONS + 1 }, (_, index) =>
        device.client.mutate(note).create({ parentId: id, title: `Note ${index}` }),
    );

    // confirm them all once online, the server counting every note
    device.online();
    await Promise.all([book.confirmed, ...notes.map((created) => created.confirmed)]);
    const stored = await alice.notebook.get({ spaceId, id });
    expect([stored.noteCount, device.errors]).toEqual([PUSH_MUTATIONS + 1, []]);
});
