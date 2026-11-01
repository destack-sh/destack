import type { Random } from "@destack/environment";
import assert from "node:assert/strict";
import { Digest } from "@destack/schema";
import type { Bucket } from "../../src/bucket/index.ts";
import { R2BlobStorage } from "../../src/cloudflare/store.ts";

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

/** Store blobs in packs below a prefix, and read, list and delete them. */
export async function exerciseBlobStorage(bucket: Bucket, random: Random): Promise<void> {
    // store a short body alone under its digest, and a long one in two parts
    const store = new R2BlobStorage(bucket, "bucket-source/", random);
    const short = new TextEncoder().encode("short body");
    const long = new Uint8Array(PART_BYTES + 1024).map((_, index) => index % 251);
    const [alone] = await store.write([
        { body: chunked(short), digest: SHORT_DIGEST, size: short.byteLength },
    ]);
    const [longAlone] = await store.write([{ body: chunked(long) }]);

    // store both in one pack of fresh name, the short body first
    const packed = await store.write([{ body: chunked(short) }, { body: chunked(long) }]);
    const pack = packed[0]?.pack ?? "";

    // refuse a body read under another digest, leaving the packs as they were
    const before = (await Array.fromAsync(store.packs())).toSorted();
    await assert.rejects(store.write([{ body: chunked(long), digest: SHORT_DIGEST }]), {
        message: `expected blob ${SHORT_DIGEST}, got ${LONG_DIGEST}`,
    });
    assert.deepStrictEqual((await Array.fromAsync(store.packs())).toSorted(), before);

    // read a lone blob whole, and a range across the packed long blob's parts
    assert.deepStrictEqual(
        [
            alone,
            longAlone?.digest,
            packed.map(({ digest, offset }) => [digest, offset]),
            pack.startsWith("pack-"),
            await joined(store.slice(SHORT_DIGEST, 0, short.byteLength)),
            await joined(store.slice(pack, short.byteLength + PART_BYTES - 2, 4)),
        ],
        [
            { digest: SHORT_DIGEST, pack: SHORT_DIGEST, offset: 0, size: short.byteLength },
            LONG_DIGEST,
            [
                [SHORT_DIGEST, 0],
                [LONG_DIGEST, short.byteLength],
            ],
            true,
            short,
            long.subarray(PART_BYTES - 2, PART_BYTES + 2),
        ],
    );

    // write a stored lone blob again, storing it once, then delete a pack and clear the prefix
    await store.write([{ body: chunked(short), digest: SHORT_DIGEST, size: short.byteLength }]);
    const listed = (await Array.fromAsync(store.packs())).toSorted();
    await store.delete(pack);
    const left = (await Array.fromAsync(store.packs())).toSorted();
    await store.clear();
    const longPack = longAlone?.pack ?? "";
    assert.deepStrictEqual(
        [listed, left, (await bucket.list()).files.map((file) => file.key)],
        [[pack, SHORT_DIGEST, longPack].toSorted(), [SHORT_DIGEST, longPack].toSorted(), []],
    );
}
