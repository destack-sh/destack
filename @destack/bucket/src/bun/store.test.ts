import { testSimulation } from "@destack/test/world";
import { join } from "node:path";
import { expect, refusal, test } from "@destack/test";
import { Digest, present } from "@destack/schema";
import { LocalBlobStorage } from "./store.ts";

/** Yield text as one chunk of bytes. */
async function* bytesOf(content: string): AsyncIterable<Uint8Array> {
    yield new TextEncoder().encode(content);
}

/** Store some bytes as one blob, answering its digest. */
async function written(store: LocalBlobStorage, body: AsyncIterable<Uint8Array>): Promise<Digest> {
    const [placement] = await store.write([{ body }]);

    return present(placement, "the written blob's placement").digest;
}

test("store a body once as a pack of its own under its digest, list the packs and delete one, twice alike", async () => {
    // store two blobs, write one's bytes again and delete the other twice
    await using simulation = testSimulation();
    const world = simulation.environment("host");
    const directory = await world.files.makeTemporaryDirectory("blobs-");
    const store = await LocalBlobStorage.open(directory, world);
    const kept = await written(store, bytesOf("kept"));
    const dropped = await written(store, bytesOf("dropped"));
    const again = await written(store, bytesOf("kept"));
    await store.delete(dropped);
    await store.delete(dropped);

    // list the one kept file, with no temporary file left
    expect({
        again,
        listed: await Array.fromAsync(store.packs()),
        files: (await world.files.readDirectory(directory)).map((entry) => entry.name),
    }).toEqual({ again: kept, listed: [kept], files: [kept] });
});

test("move a file into its blob once, refusing one whose bytes do not match its digest", async () => {
    // keep a blob by moving a file, move a second copy of it, and move a file under a wrong digest
    await using simulation = testSimulation();
    const world = simulation.environment("host");
    const { files } = world;
    const directory = await files.makeTemporaryDirectory("moves-");
    const store = await LocalBlobStorage.open(join(directory, "blobs"), world);
    const expected = await written(store, bytesOf("moved"));
    await store.delete(expected);
    await files.writeTextFile(join(directory, "first"), "moved");
    await files.writeTextFile(join(directory, "second"), "moved");
    await files.writeTextFile(join(directory, "wrong"), "other");
    const first = await store.move(join(directory, "first"), expected);
    const second = await store.move(join(directory, "second"), expected);
    const refused = await refusal(store.move(join(directory, "wrong"), expected));
    const other = await Digest.of(new TextEncoder().encode("other"), world.crypto);

    // keep the blob once, consume both copies, and leave the refused file where it was
    expect({
        moved: [first.digest, second.digest],
        blobs: await Array.fromAsync(store.packs()),
        left: (await files.readDirectory(directory)).map((entry) => entry.name).toSorted(),
        refused,
    }).toEqual({
        moved: [expected, expected],
        blobs: [expected],
        left: ["blobs", "wrong"],
        refused: ["BAD_REQUEST", "BAD_DIGEST", `expected blob ${expected}, got ${other}`],
    });
});

test("read a range of a blob at an unaligned offset across read-sized chunks, and nothing of an empty range", async () => {
    // keep a blob of two and a half read sizes
    await using simulation = testSimulation();
    const world = simulation.environment("host");
    const store = await LocalBlobStorage.open(
        await world.files.makeTemporaryDirectory("slice-"),
        world,
    );
    const bytes = new Uint8Array(160 * 1024).map((_, index) => index % 251);
    const digest = await written(
        store,
        (async function* () {
            yield bytes;
        })(),
    );

    // read a range starting mid-chunk and ending past the second chunk, and an empty one
    const chunks = await Array.fromAsync(store.slice(digest, 1000, 70_000));
    const empty = await Array.fromAsync(store.slice(digest, 1000, 0));
    const joined = new Uint8Array(chunks.reduce((length, chunk) => length + chunk.length, 0));
    let offset = 0;
    for (const chunk of chunks) {
        joined.set(chunk, offset);
        offset += chunk.length;
    }

    expect({
        sizes: chunks.map((chunk) => chunk.length),
        isExact: joined.every((byte, index) => byte === bytes[1000 + index]),
        empty: empty.length,
    }).toEqual({ sizes: [65_536, 4464], isExact: true, empty: 0 });
});
