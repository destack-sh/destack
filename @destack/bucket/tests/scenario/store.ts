import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import type { Bucket } from "../../src/index.ts";
import { R2ContentStore } from "../../src/cloudflare/store.ts";

/** The bytes of one stored part, as the store splits long bodies. */
const PART_BYTES = 8 * 1024 * 1024;

/** Stream bytes in chunks of at most a mebibyte, as a request body arrives. */
async function* chunked(bytes: Uint8Array): AsyncIterable<Uint8Array> {
    for (let offset = 0; offset < bytes.byteLength; offset += 1024 * 1024) {
        yield bytes.subarray(offset, offset + 1024 * 1024);
    }
}

/** Read every chunk into one array. */
async function joined(chunks: AsyncIterable<Uint8Array>): Promise<Uint8Array> {
    const read = await Array.fromAsync(chunks);
    const bytes = new Uint8Array(read.reduce((size, chunk) => size + chunk.byteLength, 0));
    let offset = 0;
    for (const chunk of read) {
        bytes.set(chunk, offset);
        offset += chunk.byteLength;
    }

    return bytes;
}

/** Keep blobs by digest below a prefix, staging bodies longer than one part, and read, copy and delete them. */
export async function exerciseContentStore(bucket: Bucket): Promise<void> {
    // keep a short body under its digest, and a long one staged in two parts
    const store = new R2ContentStore(bucket, "bucket-source/");
    const short = new TextEncoder().encode("short body");
    const long = new Uint8Array(PART_BYTES + 1024).map((_, index) => index % 251);
    const [shortDigest, longDigest] = [
        createHash("sha256").update(short).digest("hex"),
        createHash("sha256").update(long).digest("hex"),
    ];
    assert.deepStrictEqual(
        [await store.write(chunked(short)), await store.write(chunked(long))],
        [shortDigest, longDigest],
    );

    // refuse a body read under another digest, keeping nothing of it
    await assert.rejects(store.write(chunked(long), shortDigest), {
        message: `blob ${shortDigest} read as ${longDigest}`,
    });
    assert.deepStrictEqual(
        (await bucket.list({ prefix: "bucket-source/staging/" })).files.map((file) => file.key),
        [],
    );

    // read a blob whole, and a range across the long blob's parts
    assert.deepStrictEqual(
        [
            await joined(store.read(shortDigest)),
            await joined(store.slice(longDigest, PART_BYTES - 2, 4)),
        ],
        [short, long.subarray(PART_BYTES - 2, PART_BYTES + 2)],
    );

    // list the blobs, then delete the staged body an interrupted write left
    await bucket.put("bucket-source/staging/interrupted", "partial");
    const listed = (await Array.fromAsync(store.digests())).toSorted();
    await store.clean();
    assert.deepStrictEqual(
        [listed, await bucket.head("bucket-source/staging/interrupted")],
        [[shortDigest, longDigest].toSorted(), null],
    );

    // copy the blobs another prefix lacks, then delete one blob and clear both prefixes
    const target = new R2ContentStore(bucket, "bucket-target/");
    await target.fetch([shortDigest, longDigest, shortDigest], store);
    const copied = await target.missing([shortDigest, longDigest]);
    await store.delete(shortDigest);
    const deleted = await store.missing([shortDigest, longDigest]);
    await store.clear();
    await target.clear();
    assert.deepStrictEqual(
        [copied, deleted, (await bucket.list()).files.map((file) => file.key)],
        [[], [shortDigest], []],
    );
}
