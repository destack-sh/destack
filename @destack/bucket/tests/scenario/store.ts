import type { Random } from "@destack/environment";
import assert from "node:assert/strict";
import { Digest } from "@destack/schema";
import type { Bucket } from "../../src/bucket/index.ts";
import { R2BlobStore } from "../../src/cloudflare/store.ts";

/** The bytes of one stored part, as the store splits long bodies. */
const PART_BYTES = 8 * 1024 * 1024;

/** The SHA-256 digest of "short body". */
const SHORT_DIGEST = Digest.parse(
    "7e2e6dcff29af8ad6739aa1004ef7b42d319eff908018b4058a3ab4aed5524f2",
);

/** The SHA-256 digest of the long body: one part and 1 KiB of bytes counting modulo 251. */
const LONG_DIGEST = Digest.parse(
    "04c406f52c861202b6eda815818baa62cbe71d750ea921fd66088f90595726cb",
);

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

/** Keep blobs by digest below a prefix, and read, list, copy and delete them. */
export async function exerciseBlobStore(bucket: Bucket, random: Random): Promise<void> {
    // keep a short body under its digest, and a long one staged in two parts
    const store = new R2BlobStore(bucket, "bucket-source/", random);
    const short = new TextEncoder().encode("short body");
    const long = new Uint8Array(PART_BYTES + 1024).map((_, index) => index % 251);
    assert.deepStrictEqual(
        [await store.write(chunked(short)), await store.write(chunked(long))],
        [SHORT_DIGEST, LONG_DIGEST],
    );

    // refuse a body read under another digest, keeping nothing of it
    await assert.rejects(store.write(chunked(long), SHORT_DIGEST), {
        message: `expected blob ${SHORT_DIGEST}, got ${LONG_DIGEST}`,
    });
    assert.deepStrictEqual(
        (await bucket.list({ prefix: "bucket-source/staging/" })).files.map((file) => file.key),
        [],
    );

    // read a blob whole, and a range across the long blob's parts
    assert.deepStrictEqual(
        [
            await joined(store.read(SHORT_DIGEST)),
            await joined(store.slice(LONG_DIGEST, PART_BYTES - 2, 4)),
        ],
        [short, long.subarray(PART_BYTES - 2, PART_BYTES + 2)],
    );

    // list the blobs, then delete the staged body an interrupted write left
    await bucket.put("bucket-source/staging/interrupted", "partial");
    const listed = (await Array.fromAsync(store.digests())).toSorted();
    await store.clean();
    assert.deepStrictEqual(
        [listed, await bucket.head("bucket-source/staging/interrupted")],
        [[SHORT_DIGEST, LONG_DIGEST].toSorted(), null],
    );

    // write a kept body again, keeping its one copy
    assert.deepStrictEqual(
        [await store.write(chunked(short)), (await Array.fromAsync(store.digests())).toSorted()],
        [SHORT_DIGEST, [SHORT_DIGEST, LONG_DIGEST].toSorted()],
    );

    // copy the blobs another prefix lacks, then delete one blob and clear both prefixes
    const target = new R2BlobStore(bucket, "bucket-target/", random);
    await target.fetch([SHORT_DIGEST, LONG_DIGEST, SHORT_DIGEST], store);
    const copied = await target.missing([SHORT_DIGEST, LONG_DIGEST]);
    await store.delete(SHORT_DIGEST);
    const deleted = await store.missing([SHORT_DIGEST, LONG_DIGEST]);
    await store.clear();
    await target.clear();
    assert.deepStrictEqual(
        [copied, deleted, (await bucket.list()).files.map((file) => file.key)],
        [[], [SHORT_DIGEST], []],
    );
}
