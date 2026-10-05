import { createHash, type Hash } from "node:crypto";
import type { BlobStore } from "@destack/db/blob";
import type { BucketBody, BucketPutOptions, StringChecksums } from "../bucket/index.ts";
import { CHECKSUM_ALGORITHMS } from "../bucket/index.ts";
import { BucketError } from "../error/index.ts";
import { type ContentCipher, CustomerKey } from "./encryption.ts";

/** The hashes of a body on its way into the blob store. */
interface ContentHashes {
    /** The MD5 hash of the stored bytes. */
    readonly md5: Hash;
    /** The hash of the given bytes under the supplied checksum's algorithm, absent when the MD5 hash serves. */
    readonly checksum: Hash | undefined;
    /** The supplied checksum, as lowercase hexadecimal. */
    readonly expected: string | undefined;
    /** The given byte count so far. */
    size: number;
    /** The MD5 entity tag, once the body ended. */
    etag?: string;
    /** The checked checksum, once the body ended. */
    actual?: string;
}

/** The immutable content one write kept as a blob. */
export class Content {
    /** The write's random identifier. */
    readonly version: string;
    /** The digest of the stored bytes in the blob store. */
    readonly blob: string;
    /** The customer key's counter nonce as hexadecimal, absent for plain content. */
    readonly nonce: string | null;
    /** The length in bytes. */
    readonly size: number;
    /** The MD5 entity tag. */
    readonly etag: string;
    /** The stored content checksums. */
    readonly checksums: StringChecksums;

    /** Retain the written content's identifier, blob, length, and digest. */
    constructor(
        version: string,
        blob: string,
        nonce: string | null,
        size: number,
        etag: string,
        checksums: StringChecksums,
    ) {
        // retain the content description
        this.version = version;
        this.blob = blob;
        this.nonce = nonce;
        this.size = size;
        this.etag = etag;
        this.checksums = checksums;
    }

    /** Write contents into the blob store before the catalogue refers to them. */
    static async write(
        blobs: Pick<BlobStore, "write">,
        body: BucketBody | null,
        options: BucketPutOptions = {},
        key?: CustomerKey,
    ): Promise<Content> {
        // accept at most one supplied checksum
        const supplied = CHECKSUM_ALGORITHMS.flatMap((name) => {
            const value = options[name];

            return value === undefined ? [] : [{ name, value }];
        });
        if (supplied.length > 1) {
            throw new BucketError("INVALID_CHECKSUM", "supply at most one checksum");
        }
        const [checksum] = supplied;
        const algorithm = checksum?.name;
        const expected = checksum?.value;

        // stream the stored bytes into the store, which keeps none of a mismatched body
        const nonce = key === undefined ? null : CustomerKey.nonce();
        const hashes: ContentHashes = {
            md5: createHash("md5"),
            checksum:
                algorithm === undefined || (algorithm === "md5" && key === undefined)
                    ? undefined
                    : createHash(algorithm),
            expected: expected === undefined ? undefined : hexadecimal(expected),
            size: 0,
        };
        const cipher = key === undefined || nonce === null ? undefined : key.cipher(nonce);
        const blob = await blobs.write(encode(body, cipher, hashes));

        // keep the checked checksum of plain content
        const { etag, actual } = hashes;
        if (etag === undefined || actual === undefined) {
            throw new TypeError("content hashes are unfinished after the write");
        }
        const checksums: StringChecksums = { md5: etag };
        if (algorithm !== undefined && key === undefined) {
            checksums[algorithm] = actual;
        }

        return new Content(crypto.randomUUID(), blob, nonce, hashes.size, etag, checksums);
    }
}

/** Encrypt a body's chunks for storage while hashing them, refusing a mismatched checksum at its end. */
async function* encode(
    body: BucketBody | null,
    cipher: ContentCipher | undefined,
    hashes: ContentHashes,
): AsyncIterable<Uint8Array> {
    // read buffers and strings through a stream
    const content = ArrayBuffer.isView(body)
        ? new Uint8Array(body.buffer, body.byteOffset, body.byteLength)
        : body;
    const stream: ReadableStream<Uint8Array> | null =
        body instanceof ReadableStream ? body : new Response(content ?? new Uint8Array()).body;
    if (stream === null) {
        throw new TypeError("a buffered body has no stream");
    }

    // hash the bytes stored and the bytes given
    for await (const chunk of stream) {
        const stored = cipher === undefined ? chunk : await cipher(chunk, hashes.size);
        hashes.md5.update(stored);
        hashes.checksum?.update(chunk);
        hashes.size += chunk.byteLength;
        yield stored;
    }

    // verify the supplied checksum before the store keeps the bytes
    hashes.etag = hashes.md5.digest("hex");
    hashes.actual = hashes.checksum === undefined ? hashes.etag : hashes.checksum.digest("hex");
    if (hashes.expected !== undefined && hashes.actual !== hashes.expected) {
        throw new BucketError("INVALID_CHECKSUM", "object checksum does not match its contents");
    }
}

/** Read a supplied checksum as lowercase hexadecimal. */
function hexadecimal(checksum: string | ArrayBuffer | ArrayBufferView): string {
    return typeof checksum === "string"
        ? checksum.toLowerCase()
        : checksum instanceof ArrayBuffer
          ? new Uint8Array(checksum).toHex()
          : new Uint8Array(checksum.buffer, checksum.byteOffset, checksum.byteLength).toHex();
}
