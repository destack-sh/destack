import { testSimulation } from "@destack/test/world";
import { expect, test } from "@destack/test";
import { LocalBlobStore } from "./store.ts";

/** Yield text as one chunk of bytes. */
async function* bytesOf(content: string): AsyncIterable<Uint8Array> {
    yield new TextEncoder().encode(content);
}

test("keep a body once under its digest, list the kept blobs and delete one", async () => {
    // keep two blobs, write one's bytes again and delete the other
    await using simulation = testSimulation();
    const world = simulation.environment("host");
    const directory = await world.files.makeTemporaryDirectory("blobs-");
    const store = await LocalBlobStore.open(directory, world);
    const kept = await store.write(bytesOf("kept"));
    const dropped = await store.write(bytesOf("dropped"));
    const again = await store.write(bytesOf("kept"));
    await store.delete(dropped);

    // list the one kept file, with no temporary file left
    expect({
        again,
        listed: await Array.fromAsync(store.digests()),
        files: (await world.files.readDirectory(directory)).map((entry) => entry.name),
    }).toEqual({ again: kept, listed: [kept], files: [kept] });
});

test("read a range of a blob at an unaligned offset across read-sized chunks, and nothing of an empty range", async () => {
    // keep a blob of two and a half read sizes
    await using simulation = testSimulation();
    const world = simulation.environment("host");
    const store = await LocalBlobStore.open(
        await world.files.makeTemporaryDirectory("slice-"),
        world,
    );
    const bytes = new Uint8Array(160 * 1024).map((_, index) => index % 251);
    const digest = await store.write(
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
