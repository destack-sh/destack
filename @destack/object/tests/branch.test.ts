import { expect, test } from "@destack/test";
import { Device, serveNotes, spaceId } from "./fixture/device.ts";
import { note, notebook } from "./fixture/notes.ts";

test("hold mutations on local branches, shown while checked out, pushed only once merged", async () => {
    const { connect } = await serveNotes("sqlite");
    const alice = connect("alice");
    const device = await Device.open("alice", alice);
    device.online();
    const book = device.client.mutate(notebook).create({ name: "Travel" });
    const { id: parentId } = await book.predicted;
    await book.confirmed;
    const server = async () =>
        (await alice.note.list({ spaceId })).items.map((item) => item.title).sort();

    // write a note on a checked-out draft branch
    await device.client.checkout("draft");
    const drafted = device.client.mutate(note).create({ parentId, title: "Packing" });
    await drafted.predicted;
    const onDraft = await device.titles();
    await device.client.checkout(undefined);
    const onMain = await device.titles();

    // write a scratch note on another branch, then discard it
    await device.client.checkout("scratch");
    await device.client.mutate(note).create({ parentId, title: "Doodle" }).predicted;
    await device.client.discard("scratch");
    expect([onDraft, onMain, await device.titles(), await device.client.branches()]).toEqual([
        ["Packing"],
        [],
        [],
        { names: ["draft"] },
    ]);

    // push the merged draft, not the discarded branch
    expect(await server()).toEqual([]);
    await device.client.merge("draft");
    await drafted.confirmed;
    expect([
        await server(),
        await device.titles(),
        await device.client.branches(),
        device.errors,
    ]).toEqual([["Packing"], ["Packing"], { names: [] }, []]);
});
