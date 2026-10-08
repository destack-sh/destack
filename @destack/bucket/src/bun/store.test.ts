import { mkdtemp, rm, utimes } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { expect, onTestFinished, test } from "@destack/test";
import { LocalBlobStore } from "./store.ts";

/** An hour, the grace the sweep spares blobs used within, in milliseconds. */
const HOUR_MILLISECONDS = 60 * 60 * 1000;

/** Yield text as one chunk of bytes. */
async function* bytesOf(content: string): AsyncIterable<Uint8Array> {
    yield new TextEncoder().encode(content);
}

test("sweep the blobs outside the retained set last used before a moment, sparing a blob a write used again", async () => {
    // keep three blobs last used two hours ago
    const directory = await mkdtemp(join(tmpdir(), "destack-blobs-"));
    onTestFinished(() => rm(directory, { recursive: true, force: true }));
    const store = await LocalBlobStore.open(directory);
    const [retained, reused, dropped] = [
        await store.write(bytesOf("retained")),
        await store.write(bytesOf("reused")),
        await store.write(bytesOf("dropped")),
    ];
    const earlier = new Date(Date.now() - 2 * HOUR_MILLISECONDS);
    for (const digest of [retained, reused, dropped]) {
        await utimes(store.path(digest), earlier, earlier);
    }

    // use one again by writing its bytes, then sweep what was last used over an hour ago
    await store.write(bytesOf("reused"));
    await store.sweep(new Set([retained]), new Date(Date.now() - HOUR_MILLISECONDS));
    expect(await store.missing([retained, reused, dropped])).toEqual([dropped]);
});
