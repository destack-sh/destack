import { mkdtemp, readdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { expect, onTestFinished, test } from "@destack/test";
import { LocalBlobStore } from "./store.ts";

/** Yield text as one chunk of bytes. */
async function* bytesOf(content: string): AsyncIterable<Uint8Array> {
    yield new TextEncoder().encode(content);
}

test("keep a body once under its digest, list the kept blobs and delete one", async () => {
    // keep two blobs, write one's bytes again and delete the other
    const directory = await mkdtemp(join(tmpdir(), "destack-blobs-"));
    onTestFinished(() => rm(directory, { recursive: true, force: true }));
    const store = await LocalBlobStore.open(directory);
    const kept = await store.write(bytesOf("kept"));
    const dropped = await store.write(bytesOf("dropped"));
    const again = await store.write(bytesOf("kept"));
    await store.delete(dropped);

    // list the one kept file, with no temporary file left
    expect({
        again,
        listed: await Array.fromAsync(store.digests()),
        files: await readdir(directory),
    }).toEqual({ again: kept, listed: [kept], files: [kept] });
});
